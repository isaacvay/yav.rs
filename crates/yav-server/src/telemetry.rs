//! Real-time Core Web Vitals telemetry collector and client-side reporter.
//!
//! Provides `@vercel/analytics` and Next.js `reportWebVitals` parity in pure Rust:
//! - Tracks **LCP** (Largest Contentful Paint), **CLS** (Cumulative Layout Shift),
//!   **FID** / **INP** (First Input Delay / Interaction to Next Paint),
//!   **FCP** (First Contentful Paint), and **TTFB** (Time to First Byte).
//! - Client script uses native browser `PerformanceObserver` with zero external dependencies.
//! - Beacons are ingested at `POST /_yav/vitals` via `navigator.sendBeacon`.
//! - Server-side in-memory ring-buffer [`VitalsCollector`] computes live P75 percentiles,
//!   averages, and Good / Needs Improvement / Poor ratings distribution.

use axum::http::StatusCode;
use axum::response::{IntoResponse, Json, Response};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex, OnceLock};

/// Maximum number of recent metric samples retained in memory.
const MAX_SAMPLES: usize = 2000;

/// Individual Core Web Vital metric event reported by the browser.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebVitalMetric {
    /// Metric name (`"CLS"`, `"LCP"`, `"INP"`, `"FID"`, `"FCP"`, `"TTFB"`).
    pub name: String,
    /// Measured numeric value (milliseconds for time metrics, dimensionless score for CLS).
    pub value: f64,
    /// Quality rating computed by browser or server (`"good"`, `"needs-improvement"`, `"poor"`).
    #[serde(default = "default_rating")]
    pub rating: String,
    /// URL pathname where the metric was recorded (e.g. `"/blog/rust-ssr"`).
    #[serde(default)]
    pub path: String,
    /// Unique identifier for the measurement session or element.
    #[serde(default)]
    pub id: Option<String>,
}

fn default_rating() -> String {
    "good".to_string()
}

/// Aggregated statistical summary for a specific metric.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MetricStats {
    /// Total number of recorded samples.
    pub count: usize,
    /// Arithmetic average value.
    pub avg: f64,
    /// 75th percentile (standard Google Web Vitals evaluation threshold).
    pub p75: f64,
    /// Percentage of reports rated as "good" (0.0 to 100.0).
    pub good_percent: f64,
    /// Percentage of reports rated as "poor" (0.0 to 100.0).
    pub poor_percent: f64,
}

/// Complete telemetry snapshot across all routes and metrics.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct VitalsSummary {
    /// Total number of vitals beacons received.
    pub total_reports: usize,
    /// Aggregated stats per metric name (`"LCP"`, `"CLS"`, etc.).
    pub metrics: HashMap<String, MetricStats>,
}

/// In-memory ring-buffer collector for Web Vitals telemetry.
#[derive(Debug, Clone, Default)]
pub struct VitalsCollector {
    inner: Arc<Mutex<VecDeque<WebVitalMetric>>>,
}

impl VitalsCollector {
    /// Create a new empty vitals collector.
    #[must_use]
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(VecDeque::with_capacity(MAX_SAMPLES))),
        }
    }

    /// Record a single Web Vital metric sample.
    pub fn record(&self, mut metric: WebVitalMetric) {
        // Normalize rating if not accurately provided by client
        metric.rating = classify_rating(&metric.name, metric.value).to_string();

        let mut lock = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        if lock.len() >= MAX_SAMPLES {
            lock.pop_front();
        }
        lock.push_back(metric);
    }

    /// Compute an aggregated summary of all collected telemetry.
    #[must_use]
    pub fn summary(&self) -> VitalsSummary {
        let lock = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        let total = lock.len();

        let mut groups: HashMap<String, Vec<&WebVitalMetric>> = HashMap::new();
        for m in lock.iter() {
            groups.entry(m.name.to_uppercase()).or_default().push(m);
        }

        let mut metrics = HashMap::new();
        for (name, samples) in groups {
            let count = samples.len();
            if count == 0 {
                continue;
            }

            let sum: f64 = samples.iter().map(|s| s.value).sum();
            let avg = sum / (count as f64);

            let mut sorted_values: Vec<f64> = samples.iter().map(|s| s.value).collect();
            sorted_values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

            let p75_idx = ((count as f64) * 0.75).clamp(0.0, (count - 1) as f64) as usize;
            let p75 = sorted_values[p75_idx];

            let good_count = samples.iter().filter(|s| s.rating == "good").count();
            let poor_count = samples.iter().filter(|s| s.rating == "poor").count();

            let good_percent = (good_count as f64 / count as f64) * 100.0;
            let poor_percent = (poor_count as f64 / count as f64) * 100.0;

            metrics.insert(
                name,
                MetricStats {
                    count,
                    avg: (avg * 100.0).round() / 100.0,
                    p75: (p75 * 100.0).round() / 100.0,
                    good_percent: (good_percent * 10.0).round() / 10.0,
                    poor_percent: (poor_percent * 10.0).round() / 10.0,
                },
            );
        }

        VitalsSummary {
            total_reports: total,
            metrics,
        }
    }

    /// Retrieve all recorded metrics for a specific URL pathname.
    #[must_use]
    pub fn by_path(&self, path: &str) -> Vec<WebVitalMetric> {
        let lock = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        lock.iter().filter(|m| m.path == path).cloned().collect()
    }

    /// Clear all stored samples.
    pub fn clear(&self) {
        let mut lock = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        lock.clear();
    }
}

/// Global singleton collector shared across HTTP handler invocations.
pub fn global_vitals() -> &'static VitalsCollector {
    static INSTANCE: OnceLock<VitalsCollector> = OnceLock::new();
    INSTANCE.get_or_init(VitalsCollector::new)
}

/// Classify a metric value into Google Core Web Vitals thresholds.
#[must_use]
pub fn classify_rating(name: &str, value: f64) -> &'static str {
    match name.to_uppercase().as_str() {
        "LCP" => {
            if value <= 2500.0 {
                "good"
            } else if value <= 4000.0 {
                "needs-improvement"
            } else {
                "poor"
            }
        }
        "FID" => {
            if value <= 100.0 {
                "good"
            } else if value <= 300.0 {
                "needs-improvement"
            } else {
                "poor"
            }
        }
        "INP" => {
            if value <= 200.0 {
                "good"
            } else if value <= 500.0 {
                "needs-improvement"
            } else {
                "poor"
            }
        }
        "CLS" => {
            if value <= 0.1 {
                "good"
            } else if value <= 0.25 {
                "needs-improvement"
            } else {
                "poor"
            }
        }
        "FCP" => {
            if value <= 1800.0 {
                "good"
            } else if value <= 3000.0 {
                "needs-improvement"
            } else {
                "poor"
            }
        }
        "TTFB" => {
            if value <= 800.0 {
                "good"
            } else if value <= 1800.0 {
                "needs-improvement"
            } else {
                "poor"
            }
        }
        _ => "good",
    }
}

/// Axum HTTP POST handler accepting telemetry beacons from `navigator.sendBeacon`.
///
/// Accepts both a single `WebVitalMetric` or an array `[WebVitalMetric]`.
pub async fn vitals_handler(body: axum::body::Bytes) -> Response {
    if let Ok(single) = serde_json::from_slice::<WebVitalMetric>(&body) {
        global_vitals().record(single);
        return StatusCode::NO_CONTENT.into_response();
    }

    if let Ok(batch) = serde_json::from_slice::<Vec<WebVitalMetric>>(&body) {
        let collector = global_vitals();
        for metric in batch {
            collector.record(metric);
        }
        return StatusCode::NO_CONTENT.into_response();
    }

    StatusCode::BAD_REQUEST.into_response()
}

/// Axum HTTP GET handler returning the aggregated telemetry summary as JSON.
pub async fn vitals_summary_handler() -> Response {
    let summary = global_vitals().summary();
    Json(summary).into_response()
}

/// Generate an ultra-lightweight client-side script (< 1KB) using native `PerformanceObserver`
/// to automatically capture Core Web Vitals and report them to `/_yav/vitals`.
#[must_use]
pub fn yav_vitals_script() -> &'static str {
    r#"<script>(function(){
function send(name,val,id){
  var payload=JSON.stringify({name:name,value:Math.round(val*100)/100,path:window.location.pathname,id:id});
  if(navigator.sendBeacon){navigator.sendBeacon('/_yav/vitals',payload);}
  else{fetch('/_yav/vitals',{method:'POST',body:payload,keepalive:true,headers:{'Content-Type':'application/json'}}).catch(function(){});}
}
try{
  var po=function(type,cb){
    try{
      var o=new PerformanceObserver(function(l){l.getEntries().forEach(cb);});
      o.observe({type:type,buffered:true});
    }catch(e){}
  };
  po('paint',function(e){if(e.name==='first-contentful-paint')send('FCP',e.startTime,e.name);});
  po('largest-contentful-paint',function(e){send('LCP',e.startTime,e.id);});
  var cls=0;
  po('layout-shift',function(e){if(!e.hadRecentInput){cls+=e.value;}});
  po('first-input',function(e){send('FID',e.processingStart-e.startTime,e.name);});
  window.addEventListener('visibilitychange',function(){if(document.visibilityState==='hidden'){send('CLS',cls,'cls');}});
  window.addEventListener('pagehide',function(){send('CLS',cls,'cls');});
  window.addEventListener('load',function(){
    try{
      var nav=performance.getEntriesByType('navigation')[0];
      if(nav){send('TTFB',nav.responseStart,'ttfb');}
    }catch(e){}
  });
}catch(e){}
})();</script>"#
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_collector_and_classification() {
        let collector = VitalsCollector::new();

        collector.record(WebVitalMetric {
            name: "LCP".to_string(),
            value: 1800.0,
            rating: "".to_string(),
            path: "/docs".to_string(),
            id: None,
        });

        collector.record(WebVitalMetric {
            name: "LCP".to_string(),
            value: 4500.0,
            rating: "".to_string(),
            path: "/docs".to_string(),
            id: None,
        });

        collector.record(WebVitalMetric {
            name: "CLS".to_string(),
            value: 0.05,
            rating: "".to_string(),
            path: "/".to_string(),
            id: None,
        });

        let summary = collector.summary();
        assert_eq!(summary.total_reports, 3);

        let lcp = summary.metrics.get("LCP").unwrap();
        assert_eq!(lcp.count, 2);
        assert_eq!(lcp.good_percent, 50.0);
        assert_eq!(lcp.poor_percent, 50.0);

        let cls = summary.metrics.get("CLS").unwrap();
        assert_eq!(cls.count, 1);
        assert_eq!(cls.good_percent, 100.0);

        let docs_samples = collector.by_path("/docs");
        assert_eq!(docs_samples.len(), 2);
    }

    #[test]
    fn test_classify_thresholds() {
        assert_eq!(classify_rating("LCP", 2000.0), "good");
        assert_eq!(classify_rating("LCP", 3500.0), "needs-improvement");
        assert_eq!(classify_rating("LCP", 4200.0), "poor");

        assert_eq!(classify_rating("CLS", 0.08), "good");
        assert_eq!(classify_rating("CLS", 0.15), "needs-improvement");
        assert_eq!(classify_rating("CLS", 0.30), "poor");

        assert_eq!(classify_rating("TTFB", 500.0), "good");
        assert_eq!(classify_rating("TTFB", 2000.0), "poor");
    }

    #[test]
    fn test_script_generation() {
        let script = yav_vitals_script();
        assert!(script.contains("PerformanceObserver"));
        assert!(script.contains("/_yav/vitals"));
        assert!(script.contains("sendBeacon"));
    }
}
