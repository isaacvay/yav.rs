//! Shared data models and sample dataset for projects routes.

#[derive(Clone, Debug, serde::Serialize)]
pub struct ProjectSummary {
    pub id: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    pub status: &'static str,
    pub latency: &'static str,
    pub nodes: usize,
}

pub fn get_sample_projects() -> Vec<ProjectSummary> {
    vec![
        ProjectSummary {
            id: "yav-auth-service",
            name: "Yav Auth Service",
            description: "High-performance OAuth2 & session validator handling 12k req/s.",
            status: "Healthy",
            latency: "1.2 ms",
            nodes: 6,
        },
        ProjectSummary {
            id: "media-transcoder",
            name: "Media Transcoder",
            description: "Asynchronous image optimizer and WebP/AVIF transformer.",
            status: "Degraded",
            latency: "14.8 ms",
            nodes: 3,
        },
        ProjectSummary {
            id: "edge-analytics",
            name: "Edge Analytics",
            description: "Streaming telemetry collector built on hyper and tokio.",
            status: "Healthy",
            latency: "0.8 ms",
            nodes: 12,
        },
    ]
}
