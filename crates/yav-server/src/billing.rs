//! # YAV Billing, Exact Money, Webhooks & Idempotency
//!
//! Native financial primitives, Stripe/Svix webhook verification, idempotency
//! key enforcement, and metered SaaS billing (rivaling Stripe SDK, Dinero.js,
//! and Svix).

use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{SystemTime, UNIX_EPOCH};

use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha256;

use crate::auth::constant_time_eq;

type HmacSha256 = Hmac<Sha256>;

fn to_hex(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for &b in bytes {
        use std::fmt::Write;
        let _ = write!(&mut s, "{b:02x}");
    }
    s
}

/// ISO 4217 currency descriptor with minor unit exponent.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Currency {
    USD,
    EUR,
    GBP,
    JPY,
    Custom { code: String, decimals: u8 },
}

impl Currency {
    /// Returns the 3-letter ISO currency code.
    pub fn code(&self) -> &str {
        match self {
            Currency::USD => "USD",
            Currency::EUR => "EUR",
            Currency::GBP => "GBP",
            Currency::JPY => "JPY",
            Currency::Custom { code, .. } => code.as_str(),
        }
    }

    /// Returns the number of decimal places for minor units (e.g. 2 for USD/EUR, 0 for JPY).
    pub fn decimals(&self) -> u8 {
        match self {
            Currency::USD | Currency::EUR | Currency::GBP => 2,
            Currency::JPY => 0,
            Currency::Custom { decimals, .. } => *decimals,
        }
    }

    /// Returns the currency symbol prefix.
    pub fn symbol(&self) -> &str {
        match self {
            Currency::USD => "$",
            Currency::EUR => "€",
            Currency::GBP => "£",
            Currency::JPY => "¥",
            Currency::Custom { code, .. } => code.as_str(),
        }
    }
}

/// Exact integer-backed monetary value (minor units, e.g. cents) eliminating
/// IEEE 754 floating-point rounding bugs (Dinero.js equivalent).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Money {
    pub amount_minor: i128,
    pub currency: Currency,
}

impl Money {
    /// Create a new monetary value in minor units (e.g., `Money::usd(1999)` = `$19.99`).
    pub fn new(amount_minor: i128, currency: Currency) -> Self {
        Self {
            amount_minor,
            currency,
        }
    }

    /// Shorthand for USD cents.
    pub fn usd(cents: i128) -> Self {
        Self::new(cents, Currency::USD)
    }

    /// Shorthand for EUR cents.
    pub fn eur(cents: i128) -> Self {
        Self::new(cents, Currency::EUR)
    }

    /// Add two monetary values of the same currency.
    pub fn add(&self, other: &Money) -> Result<Money, String> {
        if self.currency != other.currency {
            return Err(format!(
                "currency mismatch: {} vs {}",
                self.currency.code(),
                other.currency.code()
            ));
        }
        Ok(Money::new(
            self.amount_minor + other.amount_minor,
            self.currency.clone(),
        ))
    }

    /// Subtract two monetary values of the same currency.
    pub fn sub(&self, other: &Money) -> Result<Money, String> {
        if self.currency != other.currency {
            return Err(format!(
                "currency mismatch: {} vs {}",
                self.currency.code(),
                other.currency.code()
            ));
        }
        Ok(Money::new(
            self.amount_minor - other.amount_minor,
            self.currency.clone(),
        ))
    }

    /// Multiply by basis points (10,000 bps = 100.00%) with half-up rounding.
    pub fn multiply_bps(&self, bps: i128) -> Money {
        let product = self.amount_minor * bps;
        let rounded = if product >= 0 {
            (product + 5_000) / 10_000
        } else {
            (product - 5_000) / 10_000
        };
        Money::new(rounded, self.currency.clone())
    }

    /// Penny-preserving proportional allocation across ratios (e.g. `&[1, 1, 1]` splits
    /// `$1.00` into `[$0.34, $0.33, $0.33]` without losing a single cent).
    pub fn allocate(&self, ratios: &[usize]) -> Vec<Money> {
        let total_ratio: usize = ratios.iter().sum();
        if total_ratio == 0 || ratios.is_empty() {
            return Vec::new();
        }
        let total = total_ratio as i128;
        let mut remainder = self.amount_minor;
        let mut shares: Vec<i128> = ratios
            .iter()
            .map(|&r| {
                let share = (self.amount_minor * (r as i128)) / total;
                remainder -= share;
                share
            })
            .collect();

        let step = if remainder >= 0 { 1 } else { -1 };
        let len = shares.len();
        let mut i = 0;
        while remainder != 0 {
            if ratios[i % len] > 0 {
                shares[i % len] += step;
                remainder -= step;
            }
            i += 1;
        }

        shares
            .into_iter()
            .map(|amt| Money::new(amt, self.currency.clone()))
            .collect()
    }

    /// Format for human display (e.g. `"$19.99"` or `"¥1500"`).
    pub fn format_display(&self) -> String {
        let decimals = self.currency.decimals();
        let sign = if self.amount_minor < 0 { "-" } else { "" };
        let abs = self.amount_minor.unsigned_abs();
        if decimals == 0 {
            format!("{}{}{}", sign, self.currency.symbol(), abs)
        } else {
            let factor = 10u128.pow(decimals as u32);
            let major = abs / factor;
            let minor = abs % factor;
            format!(
                "{}{}{}.{:0width$}",
                sign,
                self.currency.symbol(),
                major,
                minor,
                width = decimals as usize
            )
        }
    }
}

/// Cryptographic Stripe/Svix webhook verifier (`Stripe-Signature: t=...,v1=...`)
/// with replay-window protection.
#[derive(Debug, Clone)]
pub struct WebhookVerifier {
    secret: Vec<u8>,
    tolerance_secs: u64,
}

impl WebhookVerifier {
    /// Create a new verifier with the endpoint signing secret and replay window (default 300s).
    pub fn new(secret: impl Into<Vec<u8>>, tolerance_secs: u64) -> Self {
        Self {
            secret: secret.into(),
            tolerance_secs,
        }
    }

    /// Compute a valid `t=<timestamp>,v1=<hex_sig>` header for testing or outbound webhooks.
    pub fn sign_payload(&self, timestamp: u64, payload: &[u8]) -> String {
        let mut mac =
            HmacSha256::new_from_slice(&self.secret).expect("HMAC accepts keys of any length");
        mac.update(timestamp.to_string().as_bytes());
        mac.update(b".");
        mac.update(payload);
        let sig = to_hex(&mac.finalize().into_bytes());
        format!("t={timestamp},v1={sig}")
    }

    /// Verify a `Stripe-Signature` header against `payload` at `now_epoch_secs`.
    pub fn verify_at(
        &self,
        signature_header: &str,
        payload: &[u8],
        now_epoch_secs: u64,
    ) -> Result<(), String> {
        let mut timestamp: Option<u64> = None;
        let mut v1_sigs: Vec<&str> = Vec::new();

        for part in signature_header.split(',') {
            let mut kv = part.trim().splitn(2, '=');
            if let (Some(k), Some(v)) = (kv.next(), kv.next()) {
                match k {
                    "t" => timestamp = v.parse::<u64>().ok(),
                    "v1" => v1_sigs.push(v),
                    _ => {}
                }
            }
        }

        let ts = timestamp.ok_or_else(|| "missing webhook timestamp (t=)".to_string())?;
        if v1_sigs.is_empty() {
            return Err("missing v1 signature in header".to_string());
        }

        let age = now_epoch_secs.abs_diff(ts);
        if age > self.tolerance_secs {
            return Err(format!(
                "webhook timestamp outside tolerance window ({}s > {}s)",
                age, self.tolerance_secs
            ));
        }

        let mut mac =
            HmacSha256::new_from_slice(&self.secret).expect("HMAC accepts keys of any length");
        mac.update(ts.to_string().as_bytes());
        mac.update(b".");
        mac.update(payload);
        let expected_hex = to_hex(&mac.finalize().into_bytes());

        let matched = v1_sigs
            .iter()
            .any(|sig| constant_time_eq(sig.as_bytes(), expected_hex.as_bytes()));
        if !matched {
            return Err("webhook HMAC signature mismatch".to_string());
        }

        Ok(())
    }

    /// Verify using the current wall-clock time.
    pub fn verify(&self, signature_header: &str, payload: &[u8]) -> Result<(), String> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        self.verify_at(signature_header, payload, now)
    }
}

/// Stored idempotent response for `Idempotency-Key` deduplication.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdempotentRecord {
    pub key: String,
    pub status_code: u16,
    pub body: String,
    pub created_at: u64,
}

/// Thread-safe idempotency store preventing double-charges or duplicate mutations.
#[derive(Debug, Clone, Default)]
pub struct IdempotencyStore {
    records: Arc<RwLock<HashMap<String, IdempotentRecord>>>,
}

impl IdempotencyStore {
    pub fn new() -> Self {
        Self::default()
    }

    /// Retrieve a previously recorded response for `key`.
    pub fn get(&self, key: &str) -> Option<IdempotentRecord> {
        self.records.read().ok()?.get(key).cloned()
    }

    /// Record or return existing response atomically. Returns `(record, was_cached)`.
    pub fn execute_once<F>(&self, key: impl Into<String>, f: F) -> (IdempotentRecord, bool)
    where
        F: FnOnce() -> (u16, String),
    {
        let key_str = key.into();
        if let Some(existing) = self.get(&key_str) {
            return (existing, true);
        }
        let (status_code, body) = f();
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let record = IdempotentRecord {
            key: key_str.clone(),
            status_code,
            body,
            created_at: now,
        };
        if let Ok(mut guard) = self.records.write() {
            let entry = guard.entry(key_str).or_insert_with(|| record.clone());
            return (entry.clone(), false);
        }
        (record, false)
    }
}

/// Metered SaaS usage accumulator per `(tenant_id, metric)` (Stripe Billing Meters equivalent).
#[derive(Debug, Clone, Default)]
pub struct UsageMeter {
    counters: Arc<RwLock<HashMap<(String, String), u64>>>,
}

impl UsageMeter {
    pub fn new() -> Self {
        Self::default()
    }

    /// Increment usage for `tenant_id` and `metric` by `units`.
    pub fn record(&self, tenant_id: &str, metric: &str, units: u64) -> u64 {
        let mut guard = self.counters.write().expect("usage meter lock");
        let entry = guard
            .entry((tenant_id.to_string(), metric.to_string()))
            .or_insert(0);
        *entry += units;
        *entry
    }

    /// Read current accumulated usage for `tenant_id` and `metric`.
    pub fn usage(&self, tenant_id: &str, metric: &str) -> u64 {
        self.counters
            .read()
            .ok()
            .and_then(|g| g.get(&(tenant_id.to_string(), metric.to_string())).copied())
            .unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exact_money_and_penny_allocation() {
        let price = Money::usd(100); // $1.00
        let shares = price.allocate(&[1, 1, 1]);
        assert_eq!(shares.len(), 3);
        assert_eq!(shares[0].amount_minor, 34);
        assert_eq!(shares[1].amount_minor, 33);
        assert_eq!(shares[2].amount_minor, 33);
        assert_eq!(shares[0].format_display(), "$0.34");
    }

    #[test]
    fn test_webhook_verifier_and_replay_window() {
        let verifier = WebhookVerifier::new(b"whsec_test_secret", 300);
        let payload = br#"{"id":"evt_123","type":"invoice.paid"}"#;
        let header = verifier.sign_payload(1_700_000_000, payload);
        assert!(verifier.verify_at(&header, payload, 1_700_000_120).is_ok());
        // Replay attack outside 300s window fails:
        assert!(verifier.verify_at(&header, payload, 1_700_000_500).is_err());
        // Tampered payload fails:
        assert!(verifier
            .verify_at(
                &header,
                br#"{"id":"evt_123","type":"hacked"}"#,
                1_700_000_100
            )
            .is_err());
    }

    #[test]
    fn test_idempotency_and_usage_meter() {
        let store = IdempotencyStore::new();
        let (rec1, cached1) = store.execute_once("idem_abc", || (200, "charged".to_string()));
        let (rec2, cached2) = store.execute_once("idem_abc", || (500, "double".to_string()));
        assert!(!cached1);
        assert!(cached2);
        assert_eq!(rec1.body, rec2.body);

        let meter = UsageMeter::new();
        assert_eq!(meter.record("org_1", "tokens", 1500), 1500);
        assert_eq!(meter.record("org_1", "tokens", 500), 2000);
        assert_eq!(meter.usage("org_1", "tokens"), 2000);
    }
}
