//! Cryptographic Proof-of-Work (PoW) Anti-Bot Shield & Invisible Honeypots.
//!
//! Provides privacy-friendly, zero-Google bot protection (alternative to reCAPTCHA/Cloudflare Turnstile)
//! using client-side SHA-256 puzzles and CSS-hidden honeypot traps.

use hmac::{Hmac, Mac};
use sha2::{Digest, Sha256};
use std::time::{SystemTime, UNIX_EPOCH};

type HmacSha256 = Hmac<Sha256>;

/// A cryptographic Proof-of-Work challenge sent to the client.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct PowChallenge {
    /// Random salt (hex-encoded).
    pub salt: String,
    /// Number of required leading zero hexadecimal characters (e.g. 3 = ~4096 hashes).
    pub difficulty: u32,
    /// Expiration timestamp in seconds.
    pub expires_at: u64,
    /// HMAC-SHA256 signature preventing client tampering.
    pub signature: String,
}

/// Proof-of-Work Shield protecting endpoints against automated bots and scrapers.
#[derive(Debug, Clone)]
pub struct PowShield {
    secret: Vec<u8>,
    difficulty: u32,
    ttl_seconds: u64,
}

impl PowShield {
    /// Create a new `PowShield` with HMAC secret key.
    pub fn new(secret: impl Into<String>) -> Self {
        Self {
            secret: secret.into().into_bytes(),
            difficulty: 3,
            ttl_seconds: 300, // 5 minutes
        }
    }

    /// Set required difficulty (number of leading zeros in hex hash; recommended: 3 or 4).
    #[must_use]
    pub fn with_difficulty(mut self, difficulty: u32) -> Self {
        self.difficulty = difficulty;
        self
    }

    /// Set validity duration of challenge in seconds.
    #[must_use]
    pub fn with_ttl(mut self, ttl_seconds: u64) -> Self {
        self.ttl_seconds = ttl_seconds;
        self
    }

    fn sign_payload(&self, salt: &str, difficulty: u32, expires_at: u64) -> String {
        let mut mac = HmacSha256::new_from_slice(&self.secret).expect("HMAC can take any key size");
        let payload = format!("{salt}:{difficulty}:{expires_at}");
        mac.update(payload.as_bytes());
        hex_encode(&mac.finalize().into_bytes())
    }

    /// Generate a fresh signed [`PowChallenge`].
    pub fn create_challenge(&self) -> PowChallenge {
        static SHIELD_COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
        let count = SHIELD_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let now_nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let now_secs = (now_nanos / 1_000_000_000) as u64;
        let expires_at = now_secs + self.ttl_seconds;

        // Generate 16 bytes of unique random salt
        let mut hasher = Sha256::new();
        hasher.update(now_nanos.to_le_bytes());
        hasher.update(count.to_le_bytes());
        hasher.update(&self.secret);
        let salt = hex_encode(&hasher.finalize()[..16]);

        let signature = self.sign_payload(&salt, self.difficulty, expires_at);

        PowChallenge {
            salt,
            difficulty: self.difficulty,
            expires_at,
            signature,
        }
    }

    /// Verify that the client's nonce satisfies the puzzle difficulty and signature is authentic.
    pub fn verify(&self, challenge: &PowChallenge, nonce: &str) -> Result<(), &'static str> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        if now > challenge.expires_at {
            return Err("Challenge has expired");
        }

        let expected_sig =
            self.sign_payload(&challenge.salt, challenge.difficulty, challenge.expires_at);
        if !crate::auth::constant_time_eq(expected_sig.as_bytes(), challenge.signature.as_bytes()) {
            return Err("Invalid challenge signature");
        }

        // Check hash(salt + nonce) has required leading zeros
        let mut hasher = Sha256::new();
        hasher.update(challenge.salt.as_bytes());
        hasher.update(nonce.as_bytes());
        let digest = hex_encode(&hasher.finalize());

        let target_prefix = "0".repeat(challenge.difficulty as usize);
        if !digest.starts_with(&target_prefix) {
            return Err("Proof-of-work hash does not meet required difficulty");
        }

        Ok(())
    }
}

/// Helper generating client-side JavaScript solving the PoW in the background.
pub fn pow_client_script() -> &'static str {
    r#"<script id="yav-pow-solver">(function(){
async function solvePow(salt, diff){
  var prefix = '0'.repeat(diff);
  var nonce = 0;
  var enc = new TextEncoder();
  while(true){
    var str = salt + nonce;
    var buf = await crypto.subtle.digest('SHA-256', enc.encode(str));
    var arr = Array.from(new Uint8Array(buf));
    var hex = arr.map(b => b.toString(16).padStart(2, '0')).join('');
    if(hex.startsWith(prefix)) return nonce.toString();
    nonce++;
  }
}
window.addEventListener('DOMContentLoaded', async function(){
  var f = document.querySelector('form[data-yav-pow]');
  if(!f) return;
  var salt = f.getAttribute('data-pow-salt');
  var diff = parseInt(f.getAttribute('data-pow-diff')||'3', 10);
  if(!salt) return;
  var nonce = await solvePow(salt, diff);
  var inp = f.querySelector('input[name="pow_nonce"]');
  if(!inp){
    inp = document.createElement('input');
    inp.type = 'hidden';
    inp.name = 'pow_nonce';
    f.appendChild(inp);
  }
  inp.value = nonce;
});
})();</script>"#
}

/// Invisible honeypot trap catching automated form spammers.
pub struct Honeypot;

impl Honeypot {
    /// Default hidden field name mimicking an enterprise attribute.
    pub const DEFAULT_FIELD: &'static str = "hp_company_website";

    /// Render hidden form input element designed to trap dumb bot parsers.
    pub fn render_input(field_name: Option<&str>) -> String {
        let name = field_name.unwrap_or(Self::DEFAULT_FIELD);
        format!(
            r#"<div style="position:absolute;left:-9999px;opacity:0;pointer-events:none;height:0;overflow:hidden;" aria-hidden="true" tabindex="-1">
  <label for="{name}">Do not fill this field</label>
  <input type="text" id="{name}" name="{name}" value="" tabindex="-1" autocomplete="off" />
</div>"#
        )
    }

    /// Check whether a bot filled the honeypot input (returns `true` if spammer detected).
    pub fn is_bot(
        form_fields: &std::collections::HashMap<String, String>,
        field_name: Option<&str>,
    ) -> bool {
        let name = field_name.unwrap_or(Self::DEFAULT_FIELD);
        form_fields
            .get(name)
            .map(|v| !v.trim().is_empty())
            .unwrap_or(false)
    }
}

fn hex_encode(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        use std::fmt::Write;
        let _ = write!(s, "{:02x}", b);
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pow_challenge_creation_and_solving() {
        let shield = PowShield::new("super-secret-shield-key").with_difficulty(2);
        let challenge = shield.create_challenge();

        assert_eq!(challenge.difficulty, 2);
        assert!(!challenge.salt.is_empty());

        // Solve the puzzle on test runner
        let mut nonce = 0;
        let found_nonce = loop {
            let mut hasher = Sha256::new();
            hasher.update(challenge.salt.as_bytes());
            hasher.update(nonce.to_string().as_bytes());
            let hex = hex_encode(&hasher.finalize());
            if hex.starts_with("00") {
                break nonce.to_string();
            }
            nonce += 1;
        };

        // Verification should succeed
        assert!(shield.verify(&challenge, &found_nonce).is_ok());

        // Ensure invalid nonce fails deterministically (avoid 1/256 random collision)
        let mut invalid_nonce = "wrong_nonce_value".to_string();
        let mut count = 0;
        loop {
            let mut hasher = Sha256::new();
            hasher.update(challenge.salt.as_bytes());
            hasher.update(invalid_nonce.as_bytes());
            let hex = hex_encode(&hasher.finalize());
            if !hex.starts_with("00") {
                break;
            }
            count += 1;
            invalid_nonce = format!("wrong_nonce_{count}");
        }
        assert!(shield.verify(&challenge, &invalid_nonce).is_err());
    }

    #[test]
    fn test_honeypot_detection() {
        let mut clean_form = std::collections::HashMap::new();
        clean_form.insert("email".to_string(), "user@example.com".to_string());
        clean_form.insert(Honeypot::DEFAULT_FIELD.to_string(), "".to_string());
        assert!(!Honeypot::is_bot(&clean_form, None));

        let mut bot_form = std::collections::HashMap::new();
        bot_form.insert("email".to_string(), "bot@spammer.org".to_string());
        bot_form.insert(
            Honeypot::DEFAULT_FIELD.to_string(),
            "https://spam.xyz".to_string(),
        );
        assert!(Honeypot::is_bot(&bot_form, None));
    }
}
