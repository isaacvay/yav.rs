//! Enterprise Feature Flags, A/B Testing, Multi-Tenancy, ABAC Policy Engine & Tamper-Evident Audit Trail (`yav::flags`).
//!
//! Provides the B2B SaaS governance stack rivaling LaunchDarkly, Statsig, WorkOS, and AWS Cedar:
//! - [`FlagEngine`] & [`FeatureFlag`]: Deterministic percentage rollouts (`SHA-256`), user/tenant allowlists, and A/B test bucketing.
//! - [`TenantContext`]: Multi-tenant organization resolution from subdomains (`acme.app.com`), headers (`x-tenant-id`), or paths.
//! - [`PolicyEngine`]: Attribute-Based & Relationship-Based Access Control (ABAC/ReBAC) evaluator.
//! - [`AuditTrail`]: Append-only, SHA-256 hash-chained immutable audit log with cryptographic integrity verification.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::{Arc, RwLock};

// ─────────────────────────────────────────────────────────────────────────────
// 1. Feature Flags & A/B Testing (`FlagEngine`)
// ─────────────────────────────────────────────────────────────────────────────

/// Definition of a feature flag with percentage rollout and targeted allowlists.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureFlag {
    pub key: String,
    pub enabled: bool,
    /// Percentage rollout (`0..=100`). Evaluated deterministically per `user_id`.
    pub rollout_percentage: u8,
    pub allowed_users: HashSet<String>,
    pub allowed_tenants: HashSet<String>,
}

impl FeatureFlag {
    pub fn new(key: impl Into<String>, enabled: bool) -> Self {
        Self {
            key: key.into(),
            enabled,
            rollout_percentage: if enabled { 100 } else { 0 },
            allowed_users: HashSet::new(),
            allowed_tenants: HashSet::new(),
        }
    }

    #[must_use]
    pub fn rollout(mut self, percentage: u8) -> Self {
        self.rollout_percentage = percentage.min(100);
        self
    }

    #[must_use]
    pub fn allow_user(mut self, user_id: impl Into<String>) -> Self {
        self.allowed_users.insert(user_id.into());
        self
    }

    #[must_use]
    pub fn allow_tenant(mut self, tenant_id: impl Into<String>) -> Self {
        self.allowed_tenants.insert(tenant_id.into());
        self
    }
}

/// Thread-safe feature flag and A/B experiment evaluation engine.
#[derive(Clone, Default)]
pub struct FlagEngine {
    flags: Arc<RwLock<HashMap<String, FeatureFlag>>>,
}

impl FlagEngine {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register or update a [`FeatureFlag`].
    pub fn register(&self, flag: FeatureFlag) {
        self.flags
            .write()
            .unwrap_or_else(|e| e.into_inner())
            .insert(flag.key.clone(), flag);
    }

    /// Evaluate whether `flag_key` is enabled for the given `user_id` and optional `tenant_id`.
    pub fn is_enabled(
        &self,
        flag_key: &str,
        user_id: Option<&str>,
        tenant_id: Option<&str>,
    ) -> bool {
        let guard = self.flags.read().unwrap_or_else(|e| e.into_inner());
        let Some(flag) = guard.get(flag_key) else {
            return false;
        };
        if !flag.enabled {
            return false;
        }
        if let Some(t) = tenant_id {
            if flag.allowed_tenants.contains(t) {
                return true;
            }
        }
        if let Some(u) = user_id {
            if flag.allowed_users.contains(u) {
                return true;
            }
            if flag.rollout_percentage >= 100 {
                return true;
            }
            if flag.rollout_percentage == 0 {
                return false;
            }
            let bucket = deterministic_bucket(&flag.key, u, 100);
            return bucket < (flag.rollout_percentage as u64);
        }
        flag.rollout_percentage >= 100
    }

    /// Deterministically assign `user_id` to one of `variants` for A/B testing (e.g. `["control", "variant_a", "variant_b"]`).
    pub fn assign_variant<'a>(
        &self,
        experiment_key: &str,
        user_id: &str,
        variants: &'a [&'a str],
    ) -> Option<&'a str> {
        if variants.is_empty() {
            return None;
        }
        let idx = deterministic_bucket(experiment_key, user_id, variants.len() as u64) as usize;
        variants.get(idx).copied()
    }
}

fn deterministic_bucket(key: &str, subject: &str, modulo: u64) -> u64 {
    let mut hasher = Sha256::new();
    hasher.update(key.as_bytes());
    hasher.update(b":");
    hasher.update(subject.as_bytes());
    let digest = hasher.finalize();
    let mut bytes = [0u8; 8];
    bytes.copy_from_slice(&digest[..8]);
    u64::from_le_bytes(bytes) % modulo.max(1)
}

// ─────────────────────────────────────────────────────────────────────────────
// 2. Multi-Tenant B2B Context (`TenantContext`)
// ─────────────────────────────────────────────────────────────────────────────

/// Represents the resolved B2B tenant/organization for the current request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TenantContext {
    pub tenant_id: String,
    pub plan: String,
}

impl TenantContext {
    pub fn new(tenant_id: impl Into<String>, plan: impl Into<String>) -> Self {
        Self {
            tenant_id: tenant_id.into(),
            plan: plan.into(),
        }
    }

    /// Resolve a tenant identifier from an HTTP `Host` header subdomain (`acme.example.com` -> `Some("acme")`),
    /// an explicit `X-Tenant-Id` header, or a `/t/<tenant>/...` path prefix.
    pub fn resolve(
        host_header: Option<&str>,
        tenant_header: Option<&str>,
        path: &str,
        root_domain: &str,
    ) -> Option<String> {
        if let Some(th) = tenant_header {
            let trimmed = th.trim();
            if !trimmed.is_empty() {
                return Some(trimmed.to_string());
            }
        }
        if let Some(rest) = path.strip_prefix("/t/") {
            let seg = rest.split('/').next().unwrap_or("");
            if !seg.is_empty() {
                return Some(seg.to_string());
            }
        }
        if let Some(host) = host_header {
            let clean_host = host.split(':').next().unwrap_or(host);
            if let Some(sub) = clean_host.strip_suffix(&format!(".{root_domain}")) {
                if !sub.is_empty() && sub != "www" {
                    return Some(sub.to_string());
                }
            }
        }
        None
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// 3. Attribute-Based Access Control (`PolicyEngine` — AWS Cedar / CASL equivalent)
// ─────────────────────────────────────────────────────────────────────────────

/// Subject requesting access to a resource.
#[derive(Debug, Clone)]
pub struct PolicySubject {
    pub user_id: String,
    pub tenant_id: String,
    pub roles: Vec<String>,
}

/// Target resource being accessed.
#[derive(Debug, Clone)]
pub struct PolicyResource {
    pub kind: String,
    pub owner_id: Option<String>,
    pub tenant_id: String,
    pub locked: bool,
}

type PolicyPredicate = Box<dyn Fn(&PolicySubject, &str, &PolicyResource) -> bool + Send + Sync>;

/// Fine-grained ABAC / ReBAC policy evaluation engine.
#[derive(Default)]
pub struct PolicyEngine {
    allow_rules: Vec<PolicyPredicate>,
    forbid_rules: Vec<PolicyPredicate>,
}

impl PolicyEngine {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add an `allow` rule (`permit(subject, action, resource) when { ... }`).
    #[must_use]
    pub fn permit<F>(mut self, rule: F) -> Self
    where
        F: Fn(&PolicySubject, &str, &PolicyResource) -> bool + Send + Sync + 'static,
    {
        self.allow_rules.push(Box::new(rule));
        self
    }

    /// Add an explicit `forbid` rule (`forbid` always overrides `permit`, matching AWS Cedar semantics).
    #[must_use]
    pub fn forbid<F>(mut self, rule: F) -> Self
    where
        F: Fn(&PolicySubject, &str, &PolicyResource) -> bool + Send + Sync + 'static,
    {
        self.forbid_rules.push(Box::new(rule));
        self
    }

    /// Evaluate whether `subject` is authorized to perform `action` on `resource`.
    /// Default-deny unless at least one `permit` rule matches and zero `forbid` rules match.
    pub fn evaluate(
        &self,
        subject: &PolicySubject,
        action: &str,
        resource: &PolicyResource,
    ) -> bool {
        if self
            .forbid_rules
            .iter()
            .any(|rule| rule(subject, action, resource))
        {
            return false;
        }
        self.allow_rules
            .iter()
            .any(|rule| rule(subject, action, resource))
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// 4. Tamper-Evident Cryptographic Audit Trail (`AuditTrail`)
// ─────────────────────────────────────────────────────────────────────────────

/// An immutable, hash-chained audit log record.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AuditEvent {
    pub sequence: u64,
    pub actor_id: String,
    pub tenant_id: String,
    pub action: String,
    pub resource: String,
    pub metadata: BTreeMap<String, String>,
    pub prev_hash: String,
    pub event_hash: String,
}

/// Append-only, SHA-256 hash-chained audit log (SOC2 / HIPAA compliance ready).
#[derive(Clone, Default)]
pub struct AuditTrail {
    events: Arc<RwLock<Vec<AuditEvent>>>,
}

impl AuditTrail {
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a new audit event, chaining its SHA-256 digest to the previous event's hash.
    pub fn record(
        &self,
        actor_id: impl Into<String>,
        tenant_id: impl Into<String>,
        action: impl Into<String>,
        resource: impl Into<String>,
        metadata: BTreeMap<String, String>,
    ) -> AuditEvent {
        let mut guard = self.events.write().unwrap_or_else(|e| e.into_inner());
        let seq = (guard.len() + 1) as u64;
        let prev_hash = guard
            .last()
            .map(|e| e.event_hash.clone())
            .unwrap_or_else(|| "0".repeat(64));

        let actor = actor_id.into();
        let tenant = tenant_id.into();
        let act = action.into();
        let res = resource.into();

        let event_hash =
            compute_audit_hash(seq, &actor, &tenant, &act, &res, &metadata, &prev_hash);
        let ev = AuditEvent {
            sequence: seq,
            actor_id: actor,
            tenant_id: tenant,
            action: act,
            resource: res,
            metadata,
            prev_hash,
            event_hash,
        };
        guard.push(ev.clone());
        ev
    }

    /// Verify the cryptographic SHA-256 hash chain across all recorded audit events.
    /// Returns `false` if any record was modified, deleted, or reordered.
    pub fn verify_integrity(&self) -> bool {
        let guard = self.events.read().unwrap_or_else(|e| e.into_inner());
        let mut expected_prev = "0".repeat(64);
        for (idx, ev) in guard.iter().enumerate() {
            if ev.sequence != (idx + 1) as u64 || ev.prev_hash != expected_prev {
                return false;
            }
            let recomputed = compute_audit_hash(
                ev.sequence,
                &ev.actor_id,
                &ev.tenant_id,
                &ev.action,
                &ev.resource,
                &ev.metadata,
                &ev.prev_hash,
            );
            if recomputed != ev.event_hash {
                return false;
            }
            expected_prev = ev.event_hash.clone();
        }
        true
    }

    /// Return all audit events.
    pub fn events(&self) -> Vec<AuditEvent> {
        self.events
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }
}

fn compute_audit_hash(
    seq: u64,
    actor: &str,
    tenant: &str,
    action: &str,
    resource: &str,
    metadata: &BTreeMap<String, String>,
    prev_hash: &str,
) -> String {
    let mut hasher = Sha256::new();
    hasher.update(seq.to_le_bytes());
    hasher.update(actor.as_bytes());
    hasher.update(tenant.as_bytes());
    hasher.update(action.as_bytes());
    hasher.update(resource.as_bytes());
    for (k, v) in metadata {
        hasher.update(k.as_bytes());
        hasher.update(v.as_bytes());
    }
    hasher.update(prev_hash.as_bytes());
    let digest = hasher.finalize();
    let mut hex = String::with_capacity(64);
    for b in digest {
        use std::fmt::Write;
        let _ = write!(&mut hex, "{:02x}", b);
    }
    hex
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_feature_flags_tenant_abac_and_audit_chain() {
        let engine = FlagEngine::new();
        engine.register(
            FeatureFlag::new("ai_assistant", true)
                .rollout(50)
                .allow_tenant("acme_corp"),
        );

        assert!(engine.is_enabled("ai_assistant", Some("user_1"), Some("acme_corp")));
        let v1 = engine.assign_variant("checkout_ab", "user_42", &["control", "one_click"]);
        let v2 = engine.assign_variant("checkout_ab", "user_42", &["control", "one_click"]);
        assert_eq!(v1, v2);

        // Tenant resolution
        assert_eq!(
            TenantContext::resolve(Some("acme.yav.rs:3000"), None, "/dashboard", "yav.rs"),
            Some("acme".to_string())
        );

        // ABAC policy evaluation (same tenant + not locked)
        let policy = PolicyEngine::new()
            .permit(|sub, _act, res| sub.tenant_id == res.tenant_id)
            .forbid(|_sub, act, res| act == "delete" && res.locked);

        let sub = PolicySubject {
            user_id: "u1".to_string(),
            tenant_id: "acme".to_string(),
            roles: vec!["editor".to_string()],
        };
        let res = PolicyResource {
            kind: "document".to_string(),
            owner_id: Some("u1".to_string()),
            tenant_id: "acme".to_string(),
            locked: true,
        };
        assert!(policy.evaluate(&sub, "read", &res));
        assert!(!policy.evaluate(&sub, "delete", &res));

        // Cryptographic Audit Trail
        let audit = AuditTrail::new();
        audit.record("u1", "acme", "document.read", "doc_99", BTreeMap::new());
        audit.record("u1", "acme", "document.export", "doc_99", BTreeMap::new());
        assert!(audit.verify_integrity());
        assert_eq!(audit.events().len(), 2);
    }
}
