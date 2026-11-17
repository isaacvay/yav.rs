//! Automated Security Auditor, Header Heuristic Grader & W3C CSP Violation Reporter.
//!
//! Provides automated inspection of HTTP security headers (HSTS, CSP, X-Frame-Options,
//! Referrer-Policy) and a collector endpoint (`POST /_yav/csp-report`) for browser CSP
//! violation telemetry.
//!
//! # Important: this is NOT an OWASP certification
//!
//! The `A+`→`F` grade and `0..100` score produced by [`SecurityAuditor::audit`] are a
//! **local, hardcoded heuristic** over the presence/shape of a handful of response
//! headers (fixed `+25`/`+15`/`+20`/… weights). They are inspired by common guidance
//! (Mozilla Observatory, OWASP Secure Headers) but are computed entirely offline from
//! the supplied [`HeaderMap`]: there is **no external audit, no OWASP validation, no
//! certification, and no guarantee of real-world security**. Use it as a quick
//! developer hint only.

use axum::{
    body::Body,
    http::{header::HeaderMap, Request, StatusCode},
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, OnceLock, RwLock};

/// W3C / RFC 6797 standard CSP violation payload emitted by modern browsers.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CspViolation {
    #[serde(rename = "document-uri", default)]
    pub document_uri: String,
    #[serde(default)]
    pub referrer: Option<String>,
    #[serde(rename = "blocked-uri", default)]
    pub blocked_uri: String,
    #[serde(rename = "violated-directive", default)]
    pub violated_directive: String,
    #[serde(rename = "effective-directive", default)]
    pub effective_directive: String,
    #[serde(rename = "original-policy", default)]
    pub original_policy: String,
    #[serde(rename = "status-code", default)]
    pub status_code: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CspReportEnvelope {
    #[serde(rename = "csp-report")]
    pub csp_report: CspViolation,
}

/// Central store collecting and tracking CSP violations in memory.
#[derive(Debug, Clone)]
pub struct CspReporter {
    reports: Arc<RwLock<Vec<CspViolation>>>,
    max_history: usize,
}

impl Default for CspReporter {
    fn default() -> Self {
        Self::new(200)
    }
}

impl CspReporter {
    /// Create a new reporter with a bounded circular buffer limit.
    pub fn new(max_history: usize) -> Self {
        Self {
            reports: Arc::new(RwLock::new(Vec::new())),
            max_history,
        }
    }

    /// Record an incoming violation report.
    pub fn record(&self, report: CspViolation) {
        let mut lock = self.reports.write().expect("CspReporter lock poisoned");
        if lock.len() >= self.max_history {
            lock.remove(0);
        }
        lock.push(report);
    }

    /// List recent recorded violations.
    pub fn list(&self) -> Vec<CspViolation> {
        self.reports
            .read()
            .expect("CspReporter lock poisoned")
            .clone()
    }

    /// Clear all stored violations.
    pub fn clear(&self) {
        self.reports
            .write()
            .expect("CspReporter lock poisoned")
            .clear();
    }
}

/// Global singleton instance of [`CspReporter`].
pub fn global_csp_reporter() -> &'static CspReporter {
    static INSTANCE: OnceLock<CspReporter> = OnceLock::new();
    INSTANCE.get_or_init(CspReporter::default)
}

/// Axum handler capturing client CSP violations (`POST /_yav/csp-report`).
pub async fn csp_report_handler(req: Request<Body>) -> Response {
    let bytes = match axum::body::to_bytes(req.into_body(), 64 * 1024).await {
        Ok(b) => b,
        Err(_) => return StatusCode::BAD_REQUEST.into_response(),
    };

    if let Ok(envelope) = serde_json::from_slice::<CspReportEnvelope>(&bytes) {
        global_csp_reporter().record(envelope.csp_report);
    } else if let Ok(direct) = serde_json::from_slice::<CspViolation>(&bytes) {
        global_csp_reporter().record(direct);
    }

    StatusCode::NO_CONTENT.into_response()
}

/// **Heuristic** letter grade derived from [`SecurityAuditor::audit`].
///
/// Despite the `A+`/`A`/`B`/`C`/`F` styling, this is **not** an OWASP certification
/// or a compliance grade: it is a label for a local header-check score.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SecurityGrade {
    APlus,
    A,
    B,
    C,
    F,
}

impl SecurityGrade {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::APlus => "A+",
            Self::A => "A",
            Self::B => "B",
            Self::C => "C",
            Self::F => "F",
        }
    }
}

/// Result of a **local header heuristic**, not a security certification.
///
/// The field names `score`/`grade` are kept for backwards compatibility. Prefer the
/// explicit [`SecurityAuditReport::header_heuristic_score`] accessor when the
/// heuristic nature matters. This report does **not** assert OWASP compliance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityAuditReport {
    /// Heuristic score from 0 to 100 (sum of fixed header-check weights).
    pub score: u32,
    /// Heuristic letter grade computed from `score`. Not an OWASP grade.
    pub grade: SecurityGrade,
    /// List of satisfied local header checks.
    pub passed_checks: Vec<String>,
    /// Actionable hardening recommendations.
    pub recommendations: Vec<String>,
}

impl SecurityAuditReport {
    /// Unambiguously named accessor for the local header-heuristic score.
    ///
    /// Equivalent to [`SecurityAuditReport::score`]; exists so callers can avoid the
    /// ambiguous `score` name while preserving full serialization compatibility.
    #[must_use]
    pub fn header_heuristic_score(&self) -> u32 {
        self.score
    }
}

/// Automated HTTP response **header heuristic** auditor (not a compliance certifier).
pub struct SecurityAuditor;

impl SecurityAuditor {
    /// Score an HTTP header map with a **local, hardcoded heuristic**.
    ///
    /// Inspired by Mozilla Observatory / OWASP Secure Headers guidance, but computed
    /// entirely offline from fixed weights (`+25` CSP, `+20` HSTS, `+15` each for
    /// nosniff / X-Frame-Options / Referrer-Policy, `+10` Permissions-Policy).
    /// This is **not** an OWASP certification, audit, or proof of real-world security.
    pub fn audit(headers: &HeaderMap) -> SecurityAuditReport {
        let mut score: u32 = 0;
        let mut passed = Vec::new();
        let mut recs = Vec::new();

        // 1. Content-Security-Policy (25 pts)
        if let Some(csp) = headers
            .get("content-security-policy")
            .and_then(|v| v.to_str().ok())
        {
            if !csp.contains("unsafe-eval") && !csp.contains("unsafe-inline") {
                score += 25;
                passed.push(
                    "Strict Content-Security-Policy without unsafe directives (+25)".to_string(),
                );
            } else {
                score += 15;
                passed.push(
                    "Content-Security-Policy present with relaxed directives (+15)".to_string(),
                );
                recs.push(
                    "Remove 'unsafe-inline' and 'unsafe-eval' from CSP using nonces or hashes"
                        .to_string(),
                );
            }
        } else {
            recs.push(
                "Add 'Content-Security-Policy' to mitigate Cross-Site Scripting (XSS)".to_string(),
            );
        }

        // 2. Strict-Transport-Security HSTS (20 pts)
        if let Some(hsts) = headers
            .get("strict-transport-security")
            .and_then(|v| v.to_str().ok())
        {
            if hsts.contains("max-age=") {
                score += 20;
                passed.push("Strict-Transport-Security (HSTS) enabled (+20)".to_string());
            }
        } else {
            recs.push(
                "Add 'Strict-Transport-Security: max-age=63072000; includeSubDomains; preload'"
                    .to_string(),
            );
        }

        // 3. X-Content-Type-Options: nosniff (15 pts)
        if headers
            .get("x-content-type-options")
            .and_then(|v| v.to_str().ok())
            == Some("nosniff")
        {
            score += 15;
            passed.push("X-Content-Type-Options set to 'nosniff' (+15)".to_string());
        } else {
            recs.push(
                "Set 'X-Content-Type-Options: nosniff' to prevent MIME-confusion attacks"
                    .to_string(),
            );
        }

        // 4. X-Frame-Options (15 pts)
        if let Some(xfo) = headers.get("x-frame-options").and_then(|v| v.to_str().ok()) {
            if xfo.eq_ignore_ascii_case("DENY") || xfo.eq_ignore_ascii_case("SAMEORIGIN") {
                score += 15;
                passed.push(format!("X-Frame-Options protected ({xfo}) (+15)"));
            }
        } else {
            recs.push(
                "Set 'X-Frame-Options: DENY' or 'SAMEORIGIN' to prevent clickjacking".to_string(),
            );
        }

        // 5. Referrer-Policy (15 pts)
        if let Some(rp) = headers.get("referrer-policy").and_then(|v| v.to_str().ok()) {
            if rp.contains("strict-origin") || rp.contains("no-referrer") {
                score += 15;
                passed.push(format!("Referrer-Policy hardened ({rp}) (+15)"));
            }
        } else {
            recs.push("Set 'Referrer-Policy: strict-origin-when-cross-origin'".to_string());
        }

        // 6. Permissions-Policy (10 pts)
        if headers.contains_key("permissions-policy") {
            score += 10;
            passed.push("Permissions-Policy configured (+10)".to_string());
        } else {
            recs.push(
                "Set 'Permissions-Policy' to restrict microphone, camera, and geolocation"
                    .to_string(),
            );
        }

        let grade = match score {
            95..=100 => SecurityGrade::APlus,
            80..=94 => SecurityGrade::A,
            65..=79 => SecurityGrade::B,
            50..=64 => SecurityGrade::C,
            _ => SecurityGrade::F,
        };

        SecurityAuditReport {
            score,
            grade,
            passed_checks: passed,
            recommendations: recs,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderValue;

    #[test]
    fn test_security_auditor_grading() {
        let mut headers = HeaderMap::new();
        headers.insert(
            "content-security-policy",
            HeaderValue::from_static("default-src 'self'"),
        );
        headers.insert(
            "strict-transport-security",
            HeaderValue::from_static("max-age=31536000; includeSubDomains"),
        );
        headers.insert(
            "x-content-type-options",
            HeaderValue::from_static("nosniff"),
        );
        headers.insert("x-frame-options", HeaderValue::from_static("DENY"));
        headers.insert(
            "referrer-policy",
            HeaderValue::from_static("strict-origin-when-cross-origin"),
        );
        headers.insert(
            "permissions-policy",
            HeaderValue::from_static("camera=(), microphone=()"),
        );

        let report = SecurityAuditor::audit(&headers);
        assert_eq!(report.score, 100);
        assert_eq!(report.grade, SecurityGrade::APlus);
        assert!(report.recommendations.is_empty());
    }

    #[test]
    fn test_csp_reporter_buffer() {
        let reporter = CspReporter::new(2);
        let sample = CspViolation {
            document_uri: "https://yav.rs/app".to_string(),
            referrer: None,
            blocked_uri: "https://evil.com/script.js".to_string(),
            violated_directive: "script-src".to_string(),
            effective_directive: "script-src".to_string(),
            original_policy: "default-src 'self'".to_string(),
            status_code: 200,
        };

        reporter.record(sample.clone());
        assert_eq!(reporter.list().len(), 1);

        reporter.record(sample.clone());
        reporter.record(sample.clone());
        // Max history is 2
        assert_eq!(reporter.list().len(), 2);
    }
}
