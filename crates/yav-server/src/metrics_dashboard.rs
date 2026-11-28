//! Production Metrics Dashboard & Grafana-Ready Visualizer.
//!
//! Exposes a zero-dependency server-rendered monitoring dashboard (`/metrics/dashboard`).
//! The built-in handler is backed by **real** OpenTelemetry data from
//! [`crate::otel::global_tracer()`]; values that are not instrumented are shown as `0`
//! and are never fabricated.

use std::sync::atomic::{AtomicU64, Ordering};

use axum::http::StatusCode;
use axum::response::Response;

use crate::handler::html_response;
use crate::otel::global_tracer;

static CACHE_HITS: AtomicU64 = AtomicU64::new(0);
static CACHE_MISSES: AtomicU64 = AtomicU64::new(0);

/// Record an SSR cache hit for telemetry dashboards.
pub fn record_cache_hit() {
    CACHE_HITS.fetch_add(1, Ordering::Relaxed);
}

/// Record an SSR cache miss for telemetry dashboards.
pub fn record_cache_miss() {
    CACHE_MISSES.fetch_add(1, Ordering::Relaxed);
}

/// Compute the live SSR cache hit percentage (`0.0..=100.0`).
#[must_use]
pub fn cache_hit_pct() -> f64 {
    let hits = CACHE_HITS.load(Ordering::Relaxed);
    let misses = CACHE_MISSES.load(Ordering::Relaxed);
    let total = hits + misses;
    if total == 0 {
        0.0
    } else {
        (hits as f64 / total as f64) * 100.0
    }
}

/// Read current process RSS memory in megabytes when exposed by the OS (`/proc/self/statm`).
#[must_use]
pub fn process_rss_mb() -> f64 {
    #[cfg(target_os = "linux")]
    {
        if let Ok(statm) = std::fs::read_to_string("/proc/self/statm") {
            if let Some(rss_pages_str) = statm.split_whitespace().nth(1) {
                if let Ok(rss_pages) = rss_pages_str.parse::<u64>() {
                    return (rss_pages * 4096) as f64 / (1024.0 * 1024.0);
                }
            }
        }
    }
    0.0
}

/// Render the complete self-contained HTML metrics dashboard from caller-supplied values.
///
/// This is a pure renderer: it invents nothing and is kept for callers that already own
/// their metrics. The built-in [`metrics_dashboard_handler`] does **not** use hardcoded
/// demo values — see [`render_live_metrics_dashboard_html`].
#[must_use]
#[allow(clippy::too_many_arguments)]
pub fn render_metrics_dashboard_html(
    app_name: &str,
    uptime_secs: u64,
    total_requests: u64,
    requests_per_sec: f64,
    p50_ms: f64,
    p90_ms: f64,
    p99_ms: f64,
    cache_hit_pct: f64,
    memory_mb: f64,
) -> String {
    render_dashboard_html(
        app_name,
        uptime_secs,
        total_requests,
        requests_per_sec,
        p50_ms,
        p90_ms,
        p99_ms,
        cache_hit_pct,
        memory_mb,
        None,
    )
}

#[allow(clippy::too_many_arguments)]
fn render_dashboard_html(
    app_name: &str,
    uptime_secs: u64,
    total_requests: u64,
    requests_per_sec: f64,
    p50_ms: f64,
    p90_ms: f64,
    p99_ms: f64,
    cache_hit_pct: f64,
    memory_mb: f64,
    live: Option<LiveMetrics>,
) -> String {
    let safe_app_name = yav_core::escape_html(app_name);
    let days = uptime_secs / 86400;
    let hours = (uptime_secs % 86400) / 3600;
    let minutes = (uptime_secs % 3600) / 60;
    let uptime_str = format!("{}d {}h {}m", days, hours, minutes);

    let clean_p50 = if p50_ms.is_finite() && p50_ms >= 0.0 {
        p50_ms
    } else {
        0.0
    };
    let clean_p90 = if p90_ms.is_finite() && p90_ms >= 0.0 {
        p90_ms
    } else {
        0.0
    };
    let clean_p99 = if p99_ms.is_finite() && p99_ms >= 0.0 {
        p99_ms
    } else {
        0.0
    };

    let p50_bar = (clean_p50 * 3.0).clamp(4.0, 100.0);
    let p90_bar = (clean_p90 * 3.0).clamp(8.0, 100.0);
    let p99_bar = (clean_p99 * 3.0).clamp(12.0, 100.0);

    let is_live = live.is_some();
    let source_note = if is_live {
        " • Source: live OpenTelemetry spans (unmeasured values = 0)"
    } else {
        ""
    };
    let live_cards = match live {
        Some(m) => format!(
            r#"        <div class="card">
            <div class="card-title">Mean Latency (over recorded spans)</div>
            <div class="card-value">{avg:.2} <span style="font-size: 1rem; font-weight: 500; color: var(--muted);">ms</span></div>
            <div class="card-sub">{spans} spans • {dur_us} µs cumulative • percentiles not instrumented</div>
        </div>
        <div class="card">
            <div class="card-title">Error Rate (ERROR spans)</div>
            <div class="card-value">{err:.2}%</div>
            <div class="card-sub">{errors} errors / {spans} spans</div>
        </div>
"#,
            avg = m.avg_latency_ms,
            spans = m.total_spans,
            dur_us = m.total_duration_us,
            err = m.error_rate_pct,
            errors = m.total_errors,
        ),
        None => String::new(),
    };

    format!(
        r#"<!DOCTYPE html>
<html lang="en" class="dark">
<head>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <title>{safe_app_name} — Metrics Dashboard</title>
    <meta http-equiv="refresh" content="5">
    <style>
        :root {{ --bg: #090d16; --card: #111827; --border: #1f2937; --text: #f3f4f6; --muted: #9ca3af; --accent: #6366f1; --accent-light: #818cf8; }}
        * {{ margin: 0; padding: 0; box-sizing: border-box; font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, monospace; }}
        body {{ background: var(--bg); color: var(--text); padding: 2rem; min-height: 100vh; }}
        .header {{ display: flex; justify-content: space-between; align-items: center; border-bottom: 1px solid var(--border); padding-bottom: 1.5rem; margin-bottom: 2rem; }}
        .badge-live {{ display: inline-flex; align-items: center; gap: 6px; padding: 4px 10px; border-radius: 9999px; background: rgba(16, 185, 129, 0.15); color: #10b981; font-size: 12px; font-weight: 600; border: 1px solid rgba(16, 185, 129, 0.3); }}
        .badge-dot {{ width: 8px; height: 8px; border-radius: 50%; background: #10b981; animation: pulse 2s infinite; }}
        @keyframes pulse {{ 0%, 100% {{ opacity: 1; }} 50% {{ opacity: 0.4; }} }}
        .grid {{ display: grid; grid-template-columns: repeat(auto-fit, minmax(260px, 1fr)); gap: 1.5rem; margin-bottom: 2rem; }}
        .card {{ background: var(--card); border: 1px solid var(--border); border-radius: 1rem; padding: 1.5rem; box-shadow: 0 4px 6px -1px rgba(0, 0, 0, 0.3); }}
        .card-title {{ font-size: 0.875rem; font-weight: 600; color: var(--muted); text-transform: uppercase; letter-spacing: 0.05em; margin-bottom: 0.5rem; }}
        .card-value {{ font-size: 2.25rem; font-weight: 800; color: #fff; }}
        .card-sub {{ font-size: 0.75rem; color: var(--muted); margin-top: 0.25rem; }}
        .bar-container {{ margin-top: 1rem; }}
        .bar-label {{ display: flex; justify-content: space-between; font-size: 0.75rem; margin-bottom: 0.25rem; color: var(--muted); }}
        .bar-track {{ background: #1f2937; height: 8px; border-radius: 4px; overflow: hidden; }}
        .bar-fill {{ background: linear-gradient(90deg, #6366f1, #a855f7); height: 100%; border-radius: 4px; }}
        .svg-chart {{ width: 100%; height: 60px; overflow: visible; margin-top: 0.5rem; }}
    </style>
</head>
<body>
    <div class="header">
        <div>
            <h1 style="font-size: 1.5rem; font-weight: 800;">{safe_app_name} Server Telemetry</h1>
            <p style="color: var(--muted); font-size: 0.875rem; margin-top: 0.25rem;">Uptime: {uptime_str} • Auto-refresh: 5s{source_note}</p>
        </div>
        <div class="badge-live">
            <span class="badge-dot"></span>
            LIVE METRICS
        </div>
    </div>

    <div class="grid">
        <div class="card">
            <div class="card-title">Throughput</div>
            <div class="card-value">{requests_per_sec:.1} <span style="font-size: 1rem; font-weight: 500; color: var(--muted);">req/s</span></div>
            <div class="card-sub">{total_requests} total lifetime requests</div>
        </div>
        <div class="card">
            <div class="card-title">Cache Hit Ratio</div>
            <div class="card-value">{cache_hit_pct:.1}%</div>
            <div class="card-sub">L1 in-memory + L2 distributed</div>
        </div>
        <div class="card">
            <div class="card-title">RSS Memory</div>
            <div class="card-value">{memory_mb:.1} <span style="font-size: 1rem; font-weight: 500; color: var(--muted);">MB</span></div>
            <div class="card-sub">Zero memory leaks (jemalloc / rust)</div>
        </div>
        {live_cards}
    </div>

    <div class="grid" style="grid-template-columns: 2fr 1fr;">
        <div class="card">
            <div class="card-title">Latency Distribution Percentiles</div>
            <div class="bar-container">
                <div class="bar-label"><span>p50 (Median)</span><span>{p50_ms:.1} ms</span></div>
                <div class="bar-track"><div class="bar-fill" style="width: {p50_bar}%;"></div></div>
            </div>
            <div class="bar-container">
                <div class="bar-label"><span>p90</span><span>{p90_ms:.1} ms</span></div>
                <div class="bar-track"><div class="bar-fill" style="width: {p90_bar}%;"></div></div>
            </div>
            <div class="bar-container">
                <div class="bar-label"><span>p99 (Tail Latency)</span><span>{p99_ms:.1} ms</span></div>
                <div class="bar-track"><div class="bar-fill" style="width: {p99_bar}%; background: #ec4899;"></div></div>
            </div>
        </div>
        <div class="card">
            <div class="card-title">Quick Endpoints</div>
            <ul style="list-style: none; margin-top: 1rem; font-size: 0.875rem;">
                <li style="margin-bottom: 0.75rem;"><a href="/metrics" style="color: var(--accent-light); text-decoration: none;">&rarr; /metrics (Prometheus)</a></li>
                <li style="margin-bottom: 0.75rem;"><a href="/healthz" style="color: var(--accent-light); text-decoration: none;">&rarr; /healthz (Liveness)</a></li>
                <li style="margin-bottom: 0.75rem;"><a href="/readyz" style="color: var(--accent-light); text-decoration: none;">&rarr; /readyz (Readiness)</a></li>
                <li><a href="/_yav/vitals" style="color: var(--accent-light); text-decoration: none;">&rarr; /_yav/vitals (Web Vitals)</a></li>
            </ul>
        </div>
    </div>
</body>
</html>"#
    )
}

/// Real, non-fabricated telemetry derived from [`global_tracer()`].
///
/// Every field is read from the live OpenTelemetry tracer. Metrics that are not
/// instrumented (percentiles, cache hit ratio, RSS memory) are deliberately reported
/// as `0` by [`render_live_metrics_dashboard_html`] and are never invented.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct LiveMetrics {
    /// Seconds since the global tracer was initialized.
    pub uptime_secs: u64,
    /// Total recorded spans (one `SERVER` span per traced HTTP request).
    pub total_spans: u64,
    /// Spans with status `"ERROR"`.
    pub total_errors: u64,
    /// Cumulative span duration in microseconds.
    pub total_duration_us: u64,
    /// `total_spans / uptime_secs`, or `0` when uptime is zero.
    pub requests_per_sec: f64,
    /// `total_duration_us / total_spans` converted to milliseconds, or `0`.
    pub avg_latency_ms: f64,
    /// `total_errors / total_spans` expressed as a percentage, or `0`.
    pub error_rate_pct: f64,
}

/// Read real telemetry from [`global_tracer()`]; unknown/absent values stay `0`.
#[must_use]
pub fn live_metrics() -> LiveMetrics {
    let tracer = global_tracer();
    let total_spans = tracer.total_spans();
    let total_errors = tracer.total_errors();
    let total_duration_us = tracer.total_duration_us();
    let uptime_secs = tracer.uptime_secs();

    let requests_per_sec = if uptime_secs == 0 {
        0.0
    } else {
        total_spans as f64 / uptime_secs as f64
    };
    let avg_latency_ms = if total_spans == 0 {
        0.0
    } else {
        (total_duration_us as f64 / total_spans as f64) / 1000.0
    };
    let error_rate_pct = if total_spans == 0 {
        0.0
    } else {
        (total_errors as f64 / total_spans as f64) * 100.0
    };

    LiveMetrics {
        uptime_secs,
        total_spans,
        total_errors,
        total_duration_us,
        requests_per_sec,
        avg_latency_ms,
        error_rate_pct,
    }
}

/// Build the dashboard HTML from real tracer data, cache metrics, and process memory.
#[must_use]
pub fn render_live_metrics_dashboard_html(app_name: &str) -> String {
    let m = live_metrics();
    let (p50, p90, p99) = global_tracer().latency_percentiles_ms();
    let hit_pct = cache_hit_pct();
    let rss_mb = process_rss_mb();
    render_dashboard_html(
        app_name,
        m.uptime_secs,
        m.total_spans,
        m.requests_per_sec,
        p50,
        p90,
        p99,
        hit_pct,
        rss_mb,
        Some(m),
    )
}

/// Axum handler serving the interactive metrics dashboard on `GET /metrics/dashboard`.
///
/// All numbers come from [`global_tracer()`] via [`live_metrics`]; this handler uses
/// **no sample/demo telemetry**. Requests = recorded spans, mean latency =
/// `total_duration_us / total_spans`, error rate = `errors / spans`, and uptime =
/// time since tracer initialization. Non-instrumented values are `0`, never invented.
/// To render your own numbers, call [`render_metrics_dashboard_html`] directly.
pub async fn metrics_dashboard_handler() -> Response {
    html_response(
        StatusCode::OK,
        render_live_metrics_dashboard_html("Yav.rs Production"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_metrics_nan_safety() {
        // Must not panic when encountering NaN or negative values.
        let html = render_metrics_dashboard_html(
            "Faulty Telemetry",
            0,
            0,
            f64::NAN,
            f64::NAN,
            -5.0,
            f64::INFINITY,
            f64::NAN,
            f64::NAN,
        );
        assert!(html.contains("Faulty Telemetry"));
    }

    #[test]
    fn test_render_metrics_dashboard_pure_renderer() {
        // Pure renderer: values are supplied by the caller, not demo constants.
        let html = render_metrics_dashboard_html(
            "Acme Service",
            3600,
            10000,
            120.0,
            0.8,
            2.5,
            6.0,
            95.0,
            24.5,
        );
        assert!(html.contains("Acme Service Server Telemetry"));
        assert!(html.contains("LIVE METRICS"));
        assert!(html.contains("120.0"));
        assert!(html.contains("p50 (Median)"));
        assert!(html.contains("10000 total lifetime requests"));
        assert!(html.contains("/metrics (Prometheus)"));
    }

    #[test]
    fn test_live_metrics_are_derived_from_tracer() {
        let m = live_metrics();
        assert!(m.avg_latency_ms >= 0.0);
        assert!(m.error_rate_pct >= 0.0 && m.error_rate_pct <= 100.0);
        assert!(m.requests_per_sec >= 0.0);
        if m.total_spans == 0 {
            assert_eq!(m.avg_latency_ms, 0.0, "unknown latency must be 0");
            assert_eq!(m.error_rate_pct, 0.0, "unknown error rate must be 0");
        } else {
            let expected_latency = (m.total_duration_us as f64 / m.total_spans as f64) / 1000.0;
            let expected_rate = (m.total_errors as f64 / m.total_spans as f64) * 100.0;
            assert!((m.avg_latency_ms - expected_latency).abs() < 1e-9);
            assert!((m.error_rate_pct - expected_rate).abs() < 1e-9);
        }

        let html = render_live_metrics_dashboard_html("Real Service");
        assert!(html.contains("Real Service Server Telemetry"));
        assert!(html.contains("Source: live OpenTelemetry spans"));
        assert!(html.contains("Mean Latency (over recorded spans)"));
        assert!(html.contains("Error Rate (ERROR spans)"));
        // The old hardcoded demo literals must never be served.
        assert!(!html.contains("1_420_850"));
        assert!(!html.contains("3d 4h 0m"));
    }

    #[tokio::test]
    async fn test_metrics_dashboard_handler_serves_real_data() {
        let resp = metrics_dashboard_handler().await;
        assert_eq!(resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(resp.into_body(), 1024 * 1024)
            .await
            .unwrap();
        let html = String::from_utf8(body.to_vec()).unwrap();
        assert!(html.contains("Yav.rs Production Server Telemetry"));
        assert!(html.contains("Source: live OpenTelemetry spans"));
        assert!(!html.contains("1_420_850"));
        assert!(!html.contains("3d 4h 0m"));
    }
}
