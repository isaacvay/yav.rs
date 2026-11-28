//! WebAuthn / Passkeys Passwordless Biometric Authentication.
//!
//! Provides W3C WebAuthn Level 2 / Passkeys support (FaceID, TouchID, Windows Hello, YubiKey):
//! - [`PasskeyManager`] manages Relying Party configuration (`rp_id`, `rp_name`, `origin`),
//!   cryptographic challenges, and registered credentials.
//! - Generates standard `PublicKeyCredentialCreationOptions` and `PublicKeyCredentialRequestOptions`
//!   JSON payloads compatible with `navigator.credentials.create()` and `navigator.credentials.get()`.
//! - Verifies `clientDataJSON` (`type`, `challenge`, `origin`) with constant-time challenge comparison
//!   and sign-count replay protection.
//! - **Really verifies ES256 (COSE alg -7) assertions** with P-256 ECDSA/SHA-256
//!   over `authenticatorData || SHA-256(clientDataJSON)` instead of accepting any
//!   non-empty byte string as a valid signature.
//! - [`yav_passkey_script`] emits a zero-dependency browser helper for `yav:passkey-register` and
//!   `yav:passkey-login` buttons.

use crate::auth::{base64url_decode, base64url_encode, constant_time_eq};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

// ---------------------------------------------------------------------------
// Minimal CBOR reader for COSE_Key (RFC 8152). WebAuthn stores the credential
// public key as a CBOR map: {1: 2 (kty EC2), 3: -7 (alg ES256), -1: 1 (crv
// P-256), -2: x (32 bytes), -3: y (32 bytes)}. Only the data types COSE uses
// are supported; anything else yields `None` and the assertion is rejected.
// ---------------------------------------------------------------------------

#[derive(Debug)]
enum Cbor<'a> {
    UInt(u64),
    NegInt(i64),
    Bytes(&'a [u8]),
    Map(Vec<(Cbor<'a>, Cbor<'a>)>),
    /// Any well-formed CBOR item that is irrelevant to a COSE_Key (text,
    /// arrays, tags, booleans, ...). Its bytes are still consumed so the
    /// reader stays in sync; only integers, byte strings and maps are kept.
    Ignored,
}

fn cbor_take<'a>(data: &'a [u8], pos: &mut usize, len: u64) -> Option<&'a [u8]> {
    let len = usize::try_from(len).ok()?;
    let end = pos.checked_add(len)?;
    let slice = data.get(*pos..end)?;
    *pos = end;
    Some(slice)
}

fn read_cbor<'a>(data: &'a [u8], pos: &mut usize) -> Option<Cbor<'a>> {
    let initial = *data.get(*pos)?;
    *pos += 1;
    let major = initial >> 5;
    let additional = initial & 0x1f;
    let arg: u64 = match additional {
        0..=23 => additional as u64,
        24 => {
            let b = cbor_take(data, pos, 1)?;
            b[0] as u64
        }
        25 => {
            let b = cbor_take(data, pos, 2)?;
            u16::from_be_bytes([b[0], b[1]]) as u64
        }
        26 => {
            let b = cbor_take(data, pos, 4)?;
            u32::from_be_bytes([b[0], b[1], b[2], b[3]]) as u64
        }
        27 => {
            let b = cbor_take(data, pos, 8)?;
            u64::from_be_bytes([b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7]])
        }
        _ => return None,
    };
    match major {
        0 => Some(Cbor::UInt(arg)),
        // RFC 8949: a major-1 item encodes `-1 - n`. Parentheses are required:
        // `-1i64.saturating_sub(n)` parses as `-(1i64.saturating_sub(n))`.
        1 => {
            let n = i64::try_from(arg).ok()?;
            Some(Cbor::NegInt((-1i64).checked_sub(n)?))
        }
        2 => Some(Cbor::Bytes(cbor_take(data, pos, arg)?)),
        3 => {
            cbor_take(data, pos, arg)?;
            Some(Cbor::Ignored)
        }
        4 => {
            for _ in 0..arg {
                read_cbor(data, pos)?;
            }
            Some(Cbor::Ignored)
        }
        5 => {
            let mut items = Vec::new();
            for _ in 0..arg {
                let key = read_cbor(data, pos)?;
                let value = read_cbor(data, pos)?;
                items.push((key, value));
            }
            Some(Cbor::Map(items))
        }
        6 => {
            // CBOR tag: the argument is the tag number, one data item follows.
            read_cbor(data, pos)?;
            Some(Cbor::Ignored)
        }
        7 => Some(Cbor::Ignored),
        _ => None,
    }
}

fn cbor_int(value: &Cbor<'_>) -> Option<i64> {
    match value {
        Cbor::UInt(v) => i64::try_from(*v).ok(),
        Cbor::NegInt(v) => Some(*v),
        _ => None,
    }
}

/// Validate a raw uncompressed P-256 point (`0x04 || x || y`) and return
/// `x || y`. Rejects off-curve / invalid points instead of accepting arbitrary
/// 64 bytes as a "public key".
fn validated_p256_point(sec1: &[u8]) -> Option<[u8; 64]> {
    if sec1.len() != 65 || sec1[0] != 0x04 {
        return None;
    }
    p256::ecdsa::VerifyingKey::from_sec1_bytes(sec1).ok()?;
    let mut out = [0u8; 64];
    out.copy_from_slice(&sec1[1..]);
    Some(out)
}

/// Extract the uncompressed P-256 point (`x || y`, 64 bytes) from a COSE_Key or
/// from a raw SEC1 point. Returns `None` for anything that is not a valid ES256
/// (P-256) public key: the key type, algorithm, curve and coordinates are all
/// checked, and the resulting point must actually lie on the NIST P-256 curve.
pub fn cose_es256_public_key(cose: &[u8]) -> Option<[u8; 64]> {
    // Convenience forms accepted for tests and for callers that already hold a
    // decoded point: 65-byte SEC1 (0x04 || x || y) or raw 64-byte x || y.
    if cose.len() == 65 && cose[0] == 0x04 {
        return validated_p256_point(cose);
    }
    if cose.len() == 64 {
        let mut sec1 = Vec::with_capacity(65);
        sec1.push(0x04);
        sec1.extend_from_slice(cose);
        return validated_p256_point(&sec1);
    }

    let mut pos = 0;
    let root = read_cbor(cose, &mut pos)?;
    if pos != cose.len() {
        return None; // trailing data is not a canonical COSE_Key
    }
    let Cbor::Map(entries) = root else {
        return None;
    };

    let mut kty: Option<i64> = None;
    let mut alg: Option<i64> = None;
    let mut crv: Option<i64> = None;
    let mut x: Option<&[u8]> = None;
    let mut y: Option<&[u8]> = None;
    for (key, value) in &entries {
        let Some(label) = cbor_int(key) else {
            continue;
        };
        match label {
            1 | 3 | -1 => {
                let slot = match label {
                    1 => &mut kty,
                    3 => &mut alg,
                    _ => &mut crv,
                };
                if slot.is_some() {
                    return None; // duplicate label
                }
                *slot = Some(cbor_int(value)?);
            }
            -2 => {
                if x.is_some() {
                    return None;
                }
                let Cbor::Bytes(bytes) = value else {
                    return None;
                };
                if bytes.len() != 32 {
                    return None;
                }
                x = Some(bytes);
            }
            -3 => {
                if y.is_some() {
                    return None;
                }
                let Cbor::Bytes(bytes) = value else {
                    return None;
                };
                if bytes.len() != 32 {
                    return None;
                }
                y = Some(bytes);
            }
            _ => {}
        }
    }
    // RFC 8152 / W3C WebAuthn: kty=EC2 (2), alg=ES256 (-7), crv=P-256 (1).
    // All three must be present and correct; missing metadata is not accepted.
    if kty != Some(2) || alg != Some(-7) || crv != Some(1) {
        return None;
    }
    let (x, y) = (x?, y?);
    let mut sec1 = Vec::with_capacity(65);
    sec1.push(0x04);
    sec1.extend_from_slice(x);
    sec1.extend_from_slice(y);
    validated_p256_point(&sec1)
}

/// Verify a W3C WebAuthn ES256 assertion signature.
///
/// `public_key` is the stored COSE_Key (or a raw SEC1 point), `payload` is
/// `authenticatorData || SHA-256(clientDataJSON)` and `signature` is the ASN.1
/// DER-encoded ECDSA signature returned by the authenticator.
#[must_use]
pub fn verify_es256_signature(
    public_key: &[u8],
    signed_payload: &[u8],
    der_signature: &[u8],
) -> bool {
    use p256::ecdsa::signature::Verifier;

    let Some(xy) = cose_es256_public_key(public_key) else {
        return false;
    };
    let mut sec1 = Vec::with_capacity(65);
    sec1.push(0x04);
    sec1.extend_from_slice(&xy);
    let Ok(verifying_key) = p256::ecdsa::VerifyingKey::from_sec1_bytes(&sec1) else {
        return false;
    };
    let Ok(signature) = p256::ecdsa::Signature::from_der(der_signature) else {
        return false;
    };
    verifying_key.verify(signed_payload, &signature).is_ok()
}

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
        // WebAuthn challenges must be unpredictable. Deriving them from a
        // timestamp + counter (the previous implementation) let an attacker
        // pre-compute a challenge and forge a ceremony; use the OS CSPRNG.
        let mut entropy = [0u8; 32];
        if getrandom::getrandom(&mut entropy).is_err() {
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
            entropy.copy_from_slice(&hasher.finalize());
            tracing::warn!("OS CSPRNG unavailable; WebAuthn challenge uses reduced entropy");
        }
        let challenge = base64url_encode(&entropy);

        let now_secs = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
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
            // Only advertise algorithms this implementation can actually
            // verify. Previously EdDSA (-8) and RS256 (-257) were offered to
            // the authenticator but every resulting credential was rejected at
            // registration/assertion time (or, worse, accepted by the old
            // "any non-empty signature" stub). ES256 is verified with real
            // P-256 ECDSA in `verify_es256_signature`.
            pub_key_cred_params: vec![PubKeyCredParam {
                cred_type: "public-key".to_string(),
                alg: -7, // ES256 (P-256 / SHA-256)
            }],
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
        if !constant_time_eq(client_data.origin.as_bytes(), self.origin.as_bytes()) {
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

        // Reject unusable keys at registration time instead of storing a
        // credential that can never produce a valid assertion.
        let key_bytes =
            base64url_decode(public_key_b64).map_err(|_| "Invalid base64url in COSE public key")?;
        if cose_es256_public_key(&key_bytes).is_none() {
            return Err("Unsupported or malformed COSE public key (expected ES256 / P-256)");
        }

        let cred = PasskeyCredential {
            credential_id: credential_id.to_string(),
            user_id,
            public_key_b64: public_key_b64.to_string(),
            sign_count: 0,
            label: label.to_string(),
            created_at: now,
        };

        let mut creds = self.credentials.lock().unwrap_or_else(|e| e.into_inner());
        const MAX_PASSKEY_CREDENTIALS: usize = 50_000;
        if creds.len() >= MAX_PASSKEY_CREDENTIALS && !creds.contains_key(credential_id) {
            if let Some(oldest) = creds
                .iter()
                .min_by_key(|(_, c)| c.created_at)
                .map(|(k, _)| k.clone())
            {
                creds.remove(&oldest);
            }
        }
        creds.insert(credential_id.to_string(), cred.clone());
        Ok(cred)
    }

    /// Strict W3C registration variant that also validates `authenticatorData` (`rpIdHash` and UP flag).
    pub fn finish_registration_with_auth_data(
        &self,
        credential_id: &str,
        client_data_json_b64: &str,
        authenticator_data_b64: &str,
        public_key_b64: &str,
        label: &str,
    ) -> Result<PasskeyCredential, &'static str> {
        let auth_bytes = base64url_decode(authenticator_data_b64)
            .map_err(|_| "Invalid base64url in authenticatorData")?;
        if auth_bytes.len() < 37 {
            return Err("authenticatorData too short (must be at least 37 bytes)");
        }
        let mut hasher = Sha256::new();
        hasher.update(self.rp_id.as_bytes());
        let expected_rp_hash = hasher.finalize();
        if !constant_time_eq(&auth_bytes[..32], &expected_rp_hash) {
            return Err("authenticatorData RP ID hash mismatch");
        }
        if (auth_bytes[32] & 0x01) == 0 {
            return Err("User Present flag (UP) not set in authenticatorData");
        }
        self.finish_registration(credential_id, client_data_json_b64, public_key_b64, label)
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

    /// Validate the ceremony metadata of a WebAuthn assertion (`webauthn.get`)
    /// and update the credential's `sign_count`.
    ///
    /// # Security
    /// INTERNAL helper only: it performs **no cryptographic signature
    /// verification**. It is called exclusively by
    /// [`PasskeyManager::finish_authentication_with_signature`] *after* the
    /// ES256 assertion has been verified. It is deliberately private so that
    /// callers cannot mistake it for a complete authentication check — an
    /// attacker who only supplies `clientDataJSON` must never be accepted.
    fn verify_ceremony_metadata(
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

    /// Verify a full WebAuthn assertion: RP ID hash, user-presence flag,
    /// challenge/origin binding, **real P-256 ECDSA signature verification**
    /// and signature-counter replay protection.
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
            verify_es256_signature,
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
        // Only trust the signed `auth_sign_count` inside `authenticatorData`;
        // never accept an unauthenticated non-zero `new_sign_count` when the
        // signed authenticatorData reports 0 (software/stateless authenticators stay at 0).
        if auth_sign_count == 0 && new_sign_count > 0 {
            return Err("Unauthenticated sign_count escalation rejected");
        }
        let effective_sign_count = auth_sign_count;

        self.verify_ceremony_metadata(credential_id, client_data_json_b64, effective_sign_count)
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
document.addEventListener('click',function(e){
  var regBtn=e.target&&e.target.closest&&e.target.closest('[yav\\:passkey-register]');
  if(regBtn){document.dispatchEvent(new CustomEvent('yav:passkey-register',{detail:{element:regBtn}}));return;}
  var loginBtn=e.target&&e.target.closest&&e.target.closest('[yav\\:passkey-login]');
  if(loginBtn){document.dispatchEvent(new CustomEvent('yav:passkey-login',{detail:{element:loginBtn}}));}
});
})();</script>"#
}

/// Nonce-aware variant of [`yav_passkey_script`] for strict CSP policies.
#[must_use]
pub fn yav_passkey_script_with_nonce(nonce: &str) -> String {
    let escaped = yav_core::escape_attribute(nonce);
    yav_passkey_script().replacen("<script ", &format!(r#"<script nonce="{escaped}" "#), 1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use p256::ecdsa::{signature::Signer, Signature, SigningKey};

    const RP_ID: &str = "yav.rs";
    const ORIGIN: &str = "https://yav.rs";
    const USER: &str = "usr_42";
    const CRED: &str = "cred_apple_touchid_1";

    /// Deterministic P-256 test key. Real signatures are produced with it, so
    /// these tests exercise the same ECDSA code path as a real authenticator.
    fn test_signing_key() -> SigningKey {
        SigningKey::from_slice(&[0x2a; 32]).expect("fixed scalar is a valid P-256 secret key")
    }

    fn another_signing_key() -> SigningKey {
        SigningKey::from_slice(&[0x7b; 32]).expect("fixed scalar is a valid P-256 secret key")
    }

    /// Build a canonical RFC 8152 COSE_Key for ES256:
    /// {1: 2, 3: -7, -1: 1, -2: x(32), -3: y(32)}.
    fn cose_key(key: &SigningKey) -> Vec<u8> {
        let sec1 = key.verifying_key().to_encoded_point(false);
        let sec1 = sec1.as_bytes();
        assert_eq!(sec1.len(), 65);
        assert_eq!(sec1[0], 0x04);
        let mut cose = vec![
            0xa5, // map with 5 entries
            0x01, 0x02, // 1: kty = EC2
            0x03, 0x26, // 3: alg = -7 (ES256)
            0x20, 0x01, // -1: crv = 1 (P-256)
            0x21, 0x58, 0x20, // -2: bytes(32)
        ];
        cose.extend_from_slice(&sec1[1..33]);
        cose.extend_from_slice(&[0x22, 0x58, 0x20]); // -3: bytes(32)
        cose.extend_from_slice(&sec1[33..65]);
        cose
    }

    fn authenticator_data(rp_id: &str, flags: u8, sign_count: u32) -> Vec<u8> {
        let mut hasher = Sha256::new();
        hasher.update(rp_id.as_bytes());
        let mut out = hasher.finalize().to_vec();
        out.push(flags);
        out.extend_from_slice(&sign_count.to_be_bytes());
        out
    }

    /// Sign `authenticatorData || SHA-256(clientDataJSON)` exactly as a WebAuthn
    /// authenticator does, and return the ASN.1 DER ECDSA signature.
    fn sign_assertion(key: &SigningKey, auth_data: &[u8], client_json: &[u8]) -> Vec<u8> {
        let payload = PasskeyManager::compute_signed_auth_data(auth_data, client_json);
        let signature: Signature = key.sign(&payload);
        signature.to_der().as_bytes().to_vec()
    }

    fn new_manager() -> PasskeyManager {
        PasskeyManager::new(RP_ID, "Yav Cloud", ORIGIN)
    }

    fn register_credential(manager: &PasskeyManager, key: &SigningKey) {
        let opts = manager.begin_registration(USER, "alice@yav.rs", "Alice");
        let client_json = serde_json::json!({
            "type": "webauthn.create",
            "challenge": opts.challenge,
            "origin": ORIGIN
        })
        .to_string();
        manager
            .finish_registration(
                CRED,
                &base64url_encode(client_json.as_bytes()),
                &base64url_encode(&cose_key(key)),
                "MacBook TouchID",
            )
            .expect("registration with a real ES256 COSE_Key must succeed");
    }

    /// Returns `(clientDataJSON bytes, base64url(clientDataJSON), options)`.
    fn begin_auth(manager: &PasskeyManager) -> (Vec<u8>, String, PasskeyAuthOptions) {
        let opts = manager.begin_authentication(USER);
        let client_json = serde_json::json!({
            "type": "webauthn.get",
            "challenge": opts.challenge,
            "origin": ORIGIN
        })
        .to_string()
        .into_bytes();
        let b64 = base64url_encode(&client_json);
        (client_json, b64, opts)
    }

    #[test]
    fn test_cbor_negative_integers_follow_rfc_8949() {
        // Regression guard: a major-1 item with argument n means -1 - n.
        // (An unparenthesised `-1i64.saturating_sub(n)` silently yields n-1.)
        let mut pos = 0;
        assert!(matches!(
            read_cbor(&[0x20], &mut pos),
            Some(Cbor::NegInt(-1))
        ));
        let mut pos = 0;
        assert!(matches!(
            read_cbor(&[0x26], &mut pos),
            Some(Cbor::NegInt(-7))
        ));
        let mut pos = 0;
        assert!(matches!(
            read_cbor(&[0x21], &mut pos),
            Some(Cbor::NegInt(-2))
        ));
    }

    #[test]
    fn test_cose_es256_public_key_validation() {
        let key = test_signing_key();
        let cose = cose_key(&key);
        assert!(cose_es256_public_key(&cose).is_some());

        // The old fake key used by the previous test suite must be rejected.
        assert!(cose_es256_public_key(b"cose_pubkey_xyz").is_none());
        // Truncated CBOR, trailing garbage, and an off-curve point.
        assert!(cose_es256_public_key(&cose[..cose.len() - 1]).is_none());
        let mut with_junk = cose.clone();
        with_junk.push(0x00);
        assert!(cose_es256_public_key(&with_junk).is_none());
        let mut off_curve = cose.clone();
        for b in off_curve.iter_mut().skip(10) {
            *b = 0;
        }
        assert!(cose_es256_public_key(&off_curve).is_none());
        // Missing metadata labels must not be silently accepted.
        let no_alg = vec![0xa1, 0x01, 0x02];
        assert!(cose_es256_public_key(&no_alg).is_none());
        // Wrong algorithm (EdDSA / -8) and wrong curve (P-384 / 2).
        let mut wrong_alg = cose.clone();
        wrong_alg[4] = 0x27; // 3: -8
        assert!(cose_es256_public_key(&wrong_alg).is_none());
        let mut wrong_crv = cose.clone();
        wrong_crv[6] = 0x02; // -1: 2
        assert!(cose_es256_public_key(&wrong_crv).is_none());
    }

    #[test]
    fn test_registration_rejects_fake_and_unsupported_keys() {
        let manager = new_manager();
        let opts = manager.begin_registration(USER, "alice@yav.rs", "Alice");
        let client_json = serde_json::json!({
            "type": "webauthn.create",
            "challenge": opts.challenge,
            "origin": ORIGIN
        })
        .to_string();
        let err = manager.finish_registration(
            CRED,
            &base64url_encode(client_json.as_bytes()),
            &base64url_encode(b"cose_pubkey_xyz"),
            "fake",
        );
        assert_eq!(
            err.unwrap_err(),
            "Unsupported or malformed COSE public key (expected ES256 / P-256)"
        );
        assert!(manager.list_for_user(USER).is_empty());
    }

    #[test]
    fn test_real_es256_assertion_is_accepted() {
        let manager = new_manager();
        let key = test_signing_key();
        register_credential(&manager, &key);

        let (client_json, client_b64, opts) = begin_auth(&manager);
        assert_eq!(opts.rp_id, RP_ID);
        assert_eq!(opts.allow_credentials.len(), 1);
        assert_eq!(opts.allow_credentials[0].id, CRED);

        let auth = authenticator_data(RP_ID, 0x01, 2);
        let sig = sign_assertion(&key, &auth, &client_json);
        let verified = manager
            .finish_authentication_with_signature(
                CRED,
                &client_b64,
                &base64url_encode(&auth),
                &base64url_encode(&sig),
                2,
            )
            .expect("a genuine ES256 assertion must be accepted");
        assert_eq!(verified.sign_count, 2);
        assert_eq!(verified.user_id, USER);
    }

    #[test]
    fn test_forged_signatures_are_rejected() {
        // 1. Arbitrary bytes are not a signature any more.
        {
            let manager = new_manager();
            let key = test_signing_key();
            register_credential(&manager, &key);
            let (client_json, client_b64, _) = begin_auth(&manager);
            let auth = authenticator_data(RP_ID, 0x01, 1);
            let err = manager
                .finish_authentication_with_signature(
                    CRED,
                    &client_b64,
                    &base64url_encode(&auth),
                    &base64url_encode(b"valid_ecdsa_signature_bytes"),
                    1,
                )
                .unwrap_err();
            assert_eq!(err, "WebAuthn cryptographic signature verification failed");
            // Keep `client_json` used so the test mirrors a real ceremony.
            assert!(!client_json.is_empty());
        }

        // 2. A valid DER signature made by a *different* private key.
        {
            let manager = new_manager();
            let key = test_signing_key();
            register_credential(&manager, &key);
            let (client_json, client_b64, _) = begin_auth(&manager);
            let auth = authenticator_data(RP_ID, 0x01, 1);
            let sig = sign_assertion(&another_signing_key(), &auth, &client_json);
            assert!(manager
                .finish_authentication_with_signature(
                    CRED,
                    &client_b64,
                    &base64url_encode(&auth),
                    &base64url_encode(&sig),
                    1,
                )
                .is_err());
        }

        // 3. A valid signature over *different* authenticatorData / clientData.
        {
            let manager = new_manager();
            let key = test_signing_key();
            register_credential(&manager, &key);
            let (client_json, client_b64, _) = begin_auth(&manager);
            let auth = authenticator_data(RP_ID, 0x01, 1);
            let tampered_auth = authenticator_data(RP_ID, 0x05, 99); // UV + BE, other counter
            let sig = sign_assertion(&key, &tampered_auth, &client_json);
            assert!(manager
                .finish_authentication_with_signature(
                    CRED,
                    &client_b64,
                    &base64url_encode(&auth),
                    &base64url_encode(&sig),
                    1,
                )
                .is_err());
        }

        // 4. An empty signature is rejected early.
        {
            let manager = new_manager();
            let key = test_signing_key();
            register_credential(&manager, &key);
            let (_, client_b64, _) = begin_auth(&manager);
            let auth = authenticator_data(RP_ID, 0x01, 1);
            assert_eq!(
                manager
                    .finish_authentication_with_signature(
                        CRED,
                        &client_b64,
                        &base64url_encode(&auth),
                        &base64url_encode(&[]),
                        1,
                    )
                    .unwrap_err(),
                "Empty WebAuthn signature"
            );
        }
    }

    #[test]
    fn test_authenticator_data_flags_and_rp_hash_are_enforced() {
        let cases: [(&str, u8, &str); 2] = [
            (
                RP_ID,
                0x00,
                "User Present flag (UP) not set in authenticatorData",
            ),
            (
                "evil.example",
                0x01,
                "authenticatorData RP ID hash mismatch",
            ),
        ];
        for (rp, flags, expected) in cases {
            let manager = new_manager();
            let key = test_signing_key();
            register_credential(&manager, &key);
            let (client_json, client_b64, _) = begin_auth(&manager);
            let auth = authenticator_data(rp, flags, 1);
            let sig = sign_assertion(&key, &auth, &client_json);
            assert_eq!(
                manager
                    .finish_authentication_with_signature(
                        CRED,
                        &client_b64,
                        &base64url_encode(&auth),
                        &base64url_encode(&sig),
                        1,
                    )
                    .unwrap_err(),
                expected
            );
        }

        // Too-short authenticatorData is rejected before any crypto.
        let manager = new_manager();
        let key = test_signing_key();
        register_credential(&manager, &key);
        let (_, client_b64, _) = begin_auth(&manager);
        assert_eq!(
            manager
                .finish_authentication_with_signature(
                    CRED,
                    &client_b64,
                    &base64url_encode(&[0u8; 36]),
                    &base64url_encode(&[1u8; 8]),
                    1,
                )
                .unwrap_err(),
            "authenticatorData too short (must be at least 37 bytes)"
        );
    }

    #[test]
    fn test_custom_verifier_callback_and_replay_protection() {
        let manager = new_manager();
        let key = test_signing_key();
        let cose = cose_key(&key);
        register_credential(&manager, &key);

        let (client_json, client_b64, _) = begin_auth(&manager);
        let auth = authenticator_data(RP_ID, 0x01, 3);
        let sig = sign_assertion(&key, &auth, &client_json);
        let verified = manager
            .finish_authentication_with_verifier(
                CRED,
                &client_b64,
                &base64url_encode(&auth),
                &base64url_encode(&sig),
                3,
                |pubkey, signed_payload, signature| {
                    assert_eq!(pubkey, cose.as_slice());
                    assert_eq!(
                        signed_payload,
                        PasskeyManager::compute_signed_auth_data(&auth, &client_json)
                    );
                    verify_es256_signature(pubkey, signed_payload, signature)
                },
            )
            .expect("custom verifier must receive the real COSE key and payload");
        assert_eq!(verified.sign_count, 3);

        // Replay with a stale counter is rejected even with another valid challenge.
        let (_, client_b64_2, _) = begin_auth(&manager);
        assert_eq!(
            manager
                .verify_ceremony_metadata(CRED, &client_b64_2, 3)
                .unwrap_err(),
            "Passkey signature counter replay detected (possible cloned authenticator)"
        );
    }

    #[test]
    fn test_challenges_are_unique_and_ceremony_metadata_is_checked() {
        let manager = new_manager();
        let a = manager.begin_registration(USER, "alice@yav.rs", "Alice");
        let b = manager.begin_authentication(USER);
        assert_ne!(a.challenge, b.challenge);
        assert!(!a.challenge.is_empty());
        // Only ES256 is advertised: we must not promise algorithms we cannot verify.
        assert_eq!(a.pub_key_cred_params.len(), 1);
        assert_eq!(a.pub_key_cred_params[0].alg, -7);

        let key = test_signing_key();
        register_credential(&manager, &key);
        let (_, client_b64, _) = begin_auth(&manager);
        let wrong_origin = serde_json::json!({
            "type": "webauthn.get",
            "challenge": manager.begin_authentication(USER).challenge,
            "origin": "https://evil.example"
        })
        .to_string();
        assert_eq!(
            manager
                .verify_ceremony_metadata(CRED, &base64url_encode(wrong_origin.as_bytes()), 1)
                .unwrap_err(),
            "WebAuthn origin mismatch"
        );
        // The valid ceremony is still usable after the rejected one.
        assert_eq!(
            manager
                .verify_ceremony_metadata(CRED, &client_b64, 1)
                .expect("ceremony metadata validation")
                .sign_count,
            1
        );
    }

    #[test]
    fn test_passkey_script_is_csp_nonce_compatible() {
        // The runtime script must be rewritable by the HTML nonce injector,
        // otherwise strict CSP silently breaks passkey buttons.
        let injected = crate::handler::inject_csp_nonce_into_scripts(yav_passkey_script(), "N0nce");
        assert!(injected.contains("nonce=\"N0nce\""));
        assert!(injected.contains(r#"id="yav-passkey-runtime""#));
    }
}
