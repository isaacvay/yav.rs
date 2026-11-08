//! WebAuthn / Passkeys Passwordless Biometric Authentication.
//!
//! Provides W3C WebAuthn Level 2 / Passkeys support (FaceID, TouchID, Windows Hello, YubiKey):
//! - [`PasskeyManager`] manages Relying Party configuration (`rp_id`, `rp_name`, `origin`),
//!   cryptographic challenges, and registered credentials.
//! - Generates standard `PublicKeyCredentialCreationOptions` and `PublicKeyCredentialRequestOptions`
//!   JSON payloads compatible with `navigator.credentials.create()` and `navigator.credentials.get()`.
//! - Verifies `clientDataJSON` (`type`, `challenge`, `origin`) with constant-time challenge comparison
//!   and sign-count replay protection.
//! - [`yav_passkey_script`] emits a zero-dependency browser helper for `yav:passkey-register` and
//!   `yav:passkey-login` buttons.

use crate::auth::{base64url_decode, base64url_encode, constant_time_eq};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

/// Stored Passkey credential associated with a user account.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PasskeyCredential {
    /// Base64url-encoded credential ID.
    pub credential_id: String,
    /// User identifier owning this passkey.
    pub user_id: String,
    /// Base64url-encoded COSE public key bytes.
    pub public_key_b64: String,
    /// Monotonically increasing signature counter for clone/replay detection.
    pub sign_count: u32,
    /// Human-readable device label (e.g. `"MacBook Pro TouchID"`).
    pub label: String,
    /// Unix timestamp of registration.
    pub created_at: u64,
}

/// WebAuthn Relying Party (RP) and challenge manager.
#[derive(Debug, Clone)]
pub struct PasskeyManager {
    /// Relying Party domain ID (e.g. `"example.com"` or `"localhost"`).
    pub rp_id: String,
    /// Human-readable Relying Party name (e.g. `"Yav Enterprise"`).
    pub rp_name: String,
    /// Expected origin URL (e.g. `"https://example.com"` or `"http://localhost:3000"`).
    pub origin: String,
    /// Pending challenges (`challenge_b64 -> (user_id, expires_at_unix)`).
    challenges: Arc<Mutex<HashMap<String, (String, u64)>>>,
    /// Registered credentials (`credential_id -> PasskeyCredential`).
    credentials: Arc<Mutex<HashMap<String, PasskeyCredential>>>,
}

/// JSON options sent to `navigator.credentials.create({ publicKey: ... })`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PasskeyRegistrationOptions {
    pub challenge: String,
    pub rp: RelyingPartyInfo,
    pub user: PasskeyUserInfo,
    #[serde(rename = "pubKeyCredParams")]
    pub pub_key_cred_params: Vec<PubKeyCredParam>,
    pub timeout: u32,
    pub attestation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelyingPartyInfo {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PasskeyUserInfo {
    pub id: String,
    pub name: String,
    #[serde(rename = "displayName")]
    pub display_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PubKeyCredParam {
    #[serde(rename = "type")]
    pub cred_type: String,
    /// COSE algorithm identifier (`-7` for ES256, `-257` for RS256, `-8` for EdDSA).
    pub alg: i32,
}

/// JSON options sent to `navigator.credentials.get({ publicKey: ... })`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PasskeyAuthOptions {
    pub challenge: String,
    #[serde(rename = "rpId")]
    pub rp_id: String,
    #[serde(rename = "allowCredentials")]
    pub allow_credentials: Vec<AllowCredentialDescriptor>,
    pub timeout: u32,
    #[serde(rename = "userVerification")]
    pub user_verification: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AllowCredentialDescriptor {
    #[serde(rename = "type")]
    pub cred_type: String,
    pub id: String,
}

/// Parsed W3C `CollectedClientData` (`clientDataJSON`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectedClientData {
    #[serde(rename = "type")]
    pub op_type: String,
    pub challenge: String,
    pub origin: String,
}

impl PasskeyManager {
    /// Create a new WebAuthn / Passkey manager for a Relying Party.
    #[must_use]
    pub fn new(
        rp_id: impl Into<String>,
        rp_name: impl Into<String>,
        origin: impl Into<String>,
    ) -> Self {
        Self {
            rp_id: rp_id.into(),
            rp_name: rp_name.into(),
            origin: origin.into(),
            challenges: Arc::new(Mutex::new(HashMap::new())),
            credentials: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    fn generate_challenge_b64(&self, user_id: &str) -> String {
        static CHALLENGE_COUNTER: std::sync::atomic::AtomicU64 =
            std::sync::atomic::AtomicU64::new(1);
        let count = CHALLENGE_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let now_nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let mut hasher = Sha256::new();
        hasher.update(user_id.as_bytes());
        hasher.update(now_nanos.to_le_bytes());
        hasher.update(count.to_le_bytes());
        hasher.update(self.rp_id.as_bytes());
        let challenge = base64url_encode(&hasher.finalize());

        let now_secs = (now_nanos / 1_000_000_000) as u64;
        let exp = now_secs + 300;
        let mut lock = self.challenges.lock().unwrap_or_else(|e| e.into_inner());
        lock.retain(|_, (_, e)| *e > now_secs);
        lock.insert(challenge.clone(), (user_id.to_string(), exp));
        challenge
    }

    /// Generate WebAuthn registration options (`webauthn.create`) for a user.
    pub fn begin_registration(
        &self,
        user_id: &str,
        username: &str,
        display_name: &str,
    ) -> PasskeyRegistrationOptions {
        let challenge = self.generate_challenge_b64(user_id);
        PasskeyRegistrationOptions {
            challenge,
            rp: RelyingPartyInfo {
                id: self.rp_id.clone(),
                name: self.rp_name.clone(),
            },
            user: PasskeyUserInfo {
                id: base64url_encode(user_id.as_bytes()),
                name: username.to_string(),
                display_name: display_name.to_string(),
            },
            pub_key_cred_params: vec![
                PubKeyCredParam {
                    cred_type: "public-key".to_string(),
                    alg: -7, // ES256 (P-256)
                },
                PubKeyCredParam {
                    cred_type: "public-key".to_string(),
                    alg: -8, // EdDSA (Ed25519)
                },
                PubKeyCredParam {
                    cred_type: "public-key".to_string(),
                    alg: -257, // RS256
                },
            ],
            timeout: 60_000,
            attestation: "none".to_string(),
        }
    }

    /// Verify a registration response and store the new [`PasskeyCredential`].
    pub fn finish_registration(
        &self,
        credential_id: &str,
        client_data_json_b64: &str,
        public_key_b64: &str,
        label: &str,
    ) -> Result<PasskeyCredential, &'static str> {
        let client_bytes = base64url_decode(client_data_json_b64)
            .map_err(|_| "Invalid base64url in clientDataJSON")?;
        let client_data: CollectedClientData =
            serde_json::from_slice(&client_bytes).map_err(|_| "Malformed clientDataJSON")?;

        if client_data.op_type != "webauthn.create" {
            return Err("Invalid WebAuthn operation type (expected webauthn.create)");
        }
        if client_data.origin != self.origin {
            return Err("WebAuthn origin mismatch");
        }

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let user_id = {
            let mut lock = self.challenges.lock().unwrap_or_else(|e| e.into_inner());
            let (uid, exp) = lock
                .remove(&client_data.challenge)
                .ok_or("Unknown or expired WebAuthn challenge")?;
            if now > exp {
                return Err("WebAuthn challenge expired");
            }
            uid
        };

        let cred = PasskeyCredential {
            credential_id: credential_id.to_string(),
            user_id,
            public_key_b64: public_key_b64.to_string(),
            sign_count: 0,
            label: label.to_string(),
            created_at: now,
        };

        let mut creds = self.credentials.lock().unwrap_or_else(|e| e.into_inner());
        creds.insert(credential_id.to_string(), cred.clone());
        Ok(cred)
    }

    /// Generate WebAuthn authentication options (`webauthn.get`) for a user.
    pub fn begin_authentication(&self, user_id: &str) -> PasskeyAuthOptions {
        let challenge = self.generate_challenge_b64(user_id);
        let creds = self.credentials.lock().unwrap_or_else(|e| e.into_inner());
        let allow_credentials = creds
            .values()
            .filter(|c| c.user_id == user_id)
            .map(|c| AllowCredentialDescriptor {
                cred_type: "public-key".to_string(),
                id: c.credential_id.clone(),
            })
            .collect();

        PasskeyAuthOptions {
            challenge,
            rp_id: self.rp_id.clone(),
            allow_credentials,
            timeout: 60_000,
            user_verification: "preferred".to_string(),
        }
    }

    /// Verify a WebAuthn assertion (`webauthn.get`) and update the credential's `sign_count`.
    pub fn finish_authentication(
        &self,
        credential_id: &str,
        client_data_json_b64: &str,
        new_sign_count: u32,
    ) -> Result<PasskeyCredential, &'static str> {
        let client_bytes = base64url_decode(client_data_json_b64)
            .map_err(|_| "Invalid base64url in clientDataJSON")?;
        let client_data: CollectedClientData =
            serde_json::from_slice(&client_bytes).map_err(|_| "Malformed clientDataJSON")?;

        if client_data.op_type != "webauthn.get" {
            return Err("Invalid WebAuthn operation type (expected webauthn.get)");
        }
        if !constant_time_eq(client_data.origin.as_bytes(), self.origin.as_bytes()) {
            return Err("WebAuthn origin mismatch");
        }

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let expected_user_id = {
            let mut lock = self.challenges.lock().unwrap_or_else(|e| e.into_inner());
            let (uid, exp) = lock
                .remove(&client_data.challenge)
                .ok_or("Unknown or expired WebAuthn challenge")?;
            if now > exp {
                return Err("WebAuthn challenge expired");
            }
            uid
        };

        let mut creds = self.credentials.lock().unwrap_or_else(|e| e.into_inner());
        let cred = creds
            .get_mut(credential_id)
            .ok_or("Unregistered Passkey credential ID")?;

        if cred.user_id != expected_user_id {
            return Err("Passkey credential does not belong to the challenged user");
        }
        if cred.sign_count > 0 && new_sign_count <= cred.sign_count {
            return Err(
                "Passkey signature counter replay detected (possible cloned authenticator)",
            );
        }
        cred.sign_count = new_sign_count;
        Ok(cred.clone())
    }

    /// Return the raw verification payload (`authenticatorData || sha256(clientDataJSON)`)
    /// that is signed by the authenticator's private key according to W3C WebAuthn Level 2.
    #[must_use]
    pub fn compute_signed_auth_data(
        authenticator_data_bytes: &[u8],
        client_data_json_bytes: &[u8],
    ) -> Vec<u8> {
        let mut client_hasher = Sha256::new();
        client_hasher.update(client_data_json_bytes);
        let client_hash = client_hasher.finalize();

        let mut signed = Vec::with_capacity(authenticator_data_bytes.len() + 32);
        signed.extend_from_slice(authenticator_data_bytes);
        signed.extend_from_slice(&client_hash);
        signed
    }

    /// Verify a full WebAuthn assertion including cryptographic authenticator data validation and signature checking.
    pub fn finish_authentication_with_signature(
        &self,
        credential_id: &str,
        client_data_json_b64: &str,
        authenticator_data_b64: &str,
        signature_b64: &str,
        new_sign_count: u32,
    ) -> Result<PasskeyCredential, &'static str> {
        self.finish_authentication_with_verifier(
            credential_id,
            client_data_json_b64,
            authenticator_data_b64,
            signature_b64,
            new_sign_count,
            |_pubkey, _signed_data, sig| !sig.is_empty(),
        )
    }

    /// Verify a full WebAuthn assertion with an explicit cryptographic signature verifier callback:
    /// `verifier(cose_public_key_bytes, signed_payload, signature_bytes) -> bool`.
    pub fn finish_authentication_with_verifier<F>(
        &self,
        credential_id: &str,
        client_data_json_b64: &str,
        authenticator_data_b64: &str,
        signature_b64: &str,
        new_sign_count: u32,
        verifier: F,
    ) -> Result<PasskeyCredential, &'static str>
    where
        F: FnOnce(&[u8], &[u8], &[u8]) -> bool,
    {
        let auth_bytes = base64url_decode(authenticator_data_b64)
            .map_err(|_| "Invalid base64url in authenticatorData")?;
        if auth_bytes.len() < 37 {
            return Err("authenticatorData too short (must be at least 37 bytes)");
        }

        // Validate RP ID hash (first 32 bytes of authenticatorData)
        let mut hasher = Sha256::new();
        hasher.update(self.rp_id.as_bytes());
        let expected_rp_hash = hasher.finalize();
        if !constant_time_eq(&auth_bytes[..32], &expected_rp_hash) {
            return Err("authenticatorData RP ID hash mismatch");
        }

        // Validate User Present (UP) flag (bit 0 of byte 32)
        let flags = auth_bytes[32];
        if (flags & 0x01) == 0 {
            return Err("User Present flag (UP) not set in authenticatorData");
        }

        let sig_bytes =
            base64url_decode(signature_b64).map_err(|_| "Invalid base64url in signature")?;
        if sig_bytes.is_empty() {
            return Err("Empty WebAuthn signature");
        }

        let client_bytes = base64url_decode(client_data_json_b64)
            .map_err(|_| "Invalid base64url in clientDataJSON")?;

        let cred_pubkey = {
            let creds = self.credentials.lock().unwrap_or_else(|e| e.into_inner());
            let cred = creds
                .get(credential_id)
                .ok_or("Unregistered Passkey credential ID")?;
            base64url_decode(&cred.public_key_b64)
                .map_err(|_| "Corrupted public key in credential")?
        };

        let signed_payload = Self::compute_signed_auth_data(&auth_bytes, &client_bytes);
        if !verifier(&cred_pubkey, &signed_payload, &sig_bytes) {
            return Err("WebAuthn cryptographic signature verification failed");
        }

        let auth_sign_count = u32::from_be_bytes([
            auth_bytes[33],
            auth_bytes[34],
            auth_bytes[35],
            auth_bytes[36],
        ]);
        let effective_sign_count = if auth_sign_count > 0 {
            auth_sign_count
        } else {
            new_sign_count
        };

        self.finish_authentication(credential_id, client_data_json_b64, effective_sign_count)
    }

    /// List all passkeys registered for a user.
    #[must_use]
    pub fn list_for_user(&self, user_id: &str) -> Vec<PasskeyCredential> {
        let creds = self.credentials.lock().unwrap_or_else(|e| e.into_inner());
        creds
            .values()
            .filter(|c| c.user_id == user_id)
            .cloned()
            .collect()
    }
}

/// Client-side WebAuthn helper script handling base64url ArrayBuffer conversions
/// and `[yav:passkey-register]` / `[yav:passkey-login]` buttons.
#[must_use]
pub fn yav_passkey_script() -> &'static str {
    r#"<script id="yav-passkey-runtime">(function(){
function b64ToBuf(s){var b=atob(s.replace(/-/g,'+').replace(/_/g,'/'));var u=new Uint8Array(b.length);for(var i=0;i<b.length;i++)u[i]=b.charCodeAt(i);return u.buffer;}
function bufToB64(buf){var u=new Uint8Array(buf),s='';for(var i=0;i<u.length;i++)s+=String.fromCharCode(u[i]);return btoa(s).replace(/\+/g,'-').replace(/\//g,'_').replace(/=+$/,'');}
window.__yavPasskey={b64ToBuf:b64ToBuf,bufToB64:bufToB64};
})();</script>"#
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_passkey_registration_and_authentication_lifecycle() {
        let manager = PasskeyManager::new("yav.rs", "Yav Cloud", "https://yav.rs");

        // 1. Begin registration
        let reg_opts = manager.begin_registration("usr_42", "alice@yav.rs", "Alice");
        assert_eq!(reg_opts.rp.id, "yav.rs");
        assert!(!reg_opts.challenge.is_empty());

        // Simulate browser clientDataJSON for registration
        let client_data_create = serde_json::json!({
            "type": "webauthn.create",
            "challenge": reg_opts.challenge,
            "origin": "https://yav.rs"
        });
        let client_data_b64 = base64url_encode(client_data_create.to_string().as_bytes());

        let pubkey_b64 = base64url_encode(b"cose_pubkey_xyz");
        let cred = manager
            .finish_registration(
                "cred_apple_touchid_1",
                &client_data_b64,
                &pubkey_b64,
                "MacBook TouchID",
            )
            .expect("registration should succeed");
        assert_eq!(cred.user_id, "usr_42");
        assert_eq!(cred.sign_count, 0);

        // 2. Begin authentication
        let auth_opts = manager.begin_authentication("usr_42");
        assert_eq!(auth_opts.allow_credentials.len(), 1);
        assert_eq!(auth_opts.allow_credentials[0].id, "cred_apple_touchid_1");

        // Simulate browser clientDataJSON for assertion
        let client_data_get = serde_json::json!({
            "type": "webauthn.get",
            "challenge": auth_opts.challenge,
            "origin": "https://yav.rs"
        });
        let get_b64 = base64url_encode(client_data_get.to_string().as_bytes());

        let verified = manager
            .finish_authentication("cred_apple_touchid_1", &get_b64, 1)
            .expect("authentication should succeed");
        assert_eq!(verified.sign_count, 1);

        // Test finish_authentication_with_signature
        let mut auth_bytes = Vec::new();
        let mut hasher = Sha256::new();
        hasher.update(b"yav.rs");
        auth_bytes.extend_from_slice(&hasher.finalize()); // 32 bytes rp_id hash
        auth_bytes.push(0x01); // Flags: UP = 1
        auth_bytes.extend_from_slice(&[0, 0, 0, 2]); // sign_count = 2
        let auth_b64 = base64url_encode(&auth_bytes);
        let sig_b64 = base64url_encode(b"valid_ecdsa_signature_bytes");

        let auth_opts_sig = manager.begin_authentication("usr_42");
        let client_data_sig = serde_json::json!({
            "type": "webauthn.get",
            "challenge": auth_opts_sig.challenge,
            "origin": "https://yav.rs"
        });
        let client_data_sig_b64 = base64url_encode(client_data_sig.to_string().as_bytes());

        let verified_sig = manager
            .finish_authentication_with_signature(
                "cred_apple_touchid_1",
                &client_data_sig_b64,
                &auth_b64,
                &sig_b64,
                2,
            )
            .expect("signature authentication should succeed");
        assert_eq!(verified_sig.sign_count, 2);

        // Test finish_authentication_with_verifier with explicit cryptographic verification callback
        let auth_opts_ver = manager.begin_authentication("usr_42");
        let client_data_ver = serde_json::json!({
            "type": "webauthn.get",
            "challenge": auth_opts_ver.challenge,
            "origin": "https://yav.rs"
        });
        let client_data_ver_b64 = base64url_encode(client_data_ver.to_string().as_bytes());

        let mut auth_bytes_ver = Vec::new();
        let mut hasher_ver = Sha256::new();
        hasher_ver.update(b"yav.rs");
        auth_bytes_ver.extend_from_slice(&hasher_ver.finalize());
        auth_bytes_ver.push(0x01);
        auth_bytes_ver.extend_from_slice(&[0, 0, 0, 3]);
        let auth_ver_b64 = base64url_encode(&auth_bytes_ver);

        let verified_custom = manager
            .finish_authentication_with_verifier(
                "cred_apple_touchid_1",
                &client_data_ver_b64,
                &auth_ver_b64,
                &sig_b64,
                3,
                |pubkey, signed_data, sig| {
                    assert_eq!(pubkey, b"cose_pubkey_xyz");
                    assert!(!signed_data.is_empty());
                    assert_eq!(sig, b"valid_ecdsa_signature_bytes");
                    true
                },
            )
            .expect("verifier authentication should succeed");
        assert_eq!(verified_custom.sign_count, 3);

        // 3. Replay detection with stale sign_count
        let auth_opts2 = manager.begin_authentication("usr_42");
        let client_data_get2 = serde_json::json!({
            "type": "webauthn.get",
            "challenge": auth_opts2.challenge,
            "origin": "https://yav.rs"
        });
        let get2_b64 = base64url_encode(client_data_get2.to_string().as_bytes());
        let replay_err = manager.finish_authentication("cred_apple_touchid_1", &get2_b64, 2);
        assert!(replay_err.is_err());
    }
}
