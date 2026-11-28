//! # YAV Mock Server & Realistic Data Faker
//!
//! Provides static schema-based mock API responses, deterministic data fuzzing/faking,
//! and contract stubbing for offline development and testing.
//!
//! `MockEndpoint::delay_ms` is a **real** simulated latency: the response path
//! [`MockServer::handle_with_latency`] blocks for that duration before returning.

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use serde_json::{json, Value};

/// Deterministic faker for generating realistic mock fixtures without external libraries.
pub struct MockFaker;

impl MockFaker {
    const NAMES: &'static [&'static str] = &[
        "Alice Vance",
        "Bob Stone",
        "Charlie Green",
        "Diana Prince",
        "Ethan Hunt",
        "Fiona Gallagher",
        "Gabriel Logan",
        "Hannah Abbott",
    ];

    const COMPANIES: &'static [&'static str] = &[
        "Acme Global Corp",
        "Vance & Stone Tech",
        "Cyberdyne Systems",
        "Stark Enterprises",
        "Wayne Logistics",
        "Initech Solutions",
    ];

    const CITIES: &'static [&'static str] = &[
        "Paris",
        "London",
        "Berlin",
        "Tokyo",
        "New York",
        "San Francisco",
        "Toronto",
        "Sydney",
    ];

    /// Deterministically pick a name based on a seed or index.
    #[must_use]
    pub fn name(seed: usize) -> &'static str {
        Self::NAMES[seed % Self::NAMES.len()]
    }

    /// Generate an email from a name seed.
    #[must_use]
    pub fn email(seed: usize) -> String {
        let name = Self::name(seed).to_lowercase().replace(' ', ".");
        format!("{name}@example.com")
    }

    /// Deterministically pick a company name.
    #[must_use]
    pub fn company(seed: usize) -> &'static str {
        Self::COMPANIES[seed % Self::COMPANIES.len()]
    }

    /// Deterministically pick a city.
    #[must_use]
    pub fn city(seed: usize) -> &'static str {
        Self::CITIES[seed % Self::CITIES.len()]
    }

    /// Generate a pseudo-UUID v4 string.
    #[must_use]
    pub fn uuid(seed: u64) -> String {
        let h1 = ((seed.wrapping_mul(0x517cc1b727220a95)) >> 32) as u32;
        let h2 = ((seed.wrapping_mul(0x4de64b9d03426799)) >> 32) as u16;
        let h3 = (((seed.wrapping_mul(0x6b772c695b2d56a3)) >> 32) as u16 & 0x0fff) | 0x4000;
        let h4 = (((seed.wrapping_mul(0x19a4c116b5c0fbcf)) >> 32) as u16 & 0x3fff) | 0x8000;
        let h5 = ((seed.wrapping_mul(0x243185be550c7dc3)) >> 16) & 0xffffffffffff;

        format!("{h1:08x}-{h2:04x}-{h3:04x}-{h4:04x}-{h5:012x}")
    }

    /// Generate an integer amount in range [min, max].
    #[must_use]
    pub fn amount(seed: u64, min: u64, max: u64) -> u64 {
        if min >= max {
            return min;
        }
        let span = max - min + 1;
        min + (seed.wrapping_mul(6364136223846793005) % span)
    }

    /// Generate a mock ISO date string.
    #[must_use]
    pub fn date_iso(seed: usize) -> String {
        let month = (seed % 12) + 1;
        let day = (seed % 28) + 1;
        format!("2026-{month:02}-{day:02}")
    }
}

/// A configured mock endpoint route.
#[derive(Debug, Clone, PartialEq)]
pub struct MockEndpoint {
    pub method: String,
    pub path: String,
    pub status: u16,
    pub body: Value,
    /// Simulated response latency in milliseconds, consumed by
    /// [`MockServer::handle_with_latency`] (and [`MockServer::handle`]).
    pub delay_ms: u64,
}

impl MockEndpoint {
    #[must_use]
    pub fn new(
        method: impl Into<String>,
        path: impl Into<String>,
        status: u16,
        body: Value,
    ) -> Self {
        Self {
            method: method.into().to_uppercase(),
            path: path.into(),
            status,
            body,
            delay_ms: 0,
        }
    }

    /// Set the simulated latency (milliseconds) applied by the response path.
    #[must_use]
    pub fn delay(mut self, ms: u64) -> Self {
        self.delay_ms = ms;
        self
    }
}

/// In-memory mock server for offline contract testing and client simulation.
///
/// Use [`MockServer::handle_with_latency`] (or [`MockServer::handle`]) to serve a
/// request: those methods actually consume [`MockEndpoint::delay_ms`].
#[derive(Debug, Clone, Default)]
pub struct MockServer {
    endpoints: Arc<RwLock<HashMap<String, MockEndpoint>>>,
}

impl MockServer {
    #[must_use]
    pub fn new() -> Self {
        Self {
            endpoints: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register a mock endpoint (with no latency).
    pub fn mock(&self, method: &str, path: &str, status: u16, body: Value) -> &Self {
        let key = format!("{}:{}", method.to_uppercase(), path);
        let ep = MockEndpoint::new(method, path, status, body);
        let mut map = self.endpoints.write().expect("Lock poisoned");
        map.insert(key, ep);
        self
    }

    /// Register a fully-configured [`MockEndpoint`], including its `delay_ms`.
    ///
    /// This is the way to install an endpoint built with [`MockEndpoint::delay`];
    /// [`MockServer::mock`] always registers a zero-delay endpoint.
    pub fn register(&self, endpoint: MockEndpoint) -> &Self {
        let key = format!("{}:{}", endpoint.method.to_uppercase(), endpoint.path);
        let mut map = self.endpoints.write().expect("Lock poisoned");
        map.insert(key, endpoint);
        self
    }

    /// Pure lookup against registered mocks (does **not** apply latency).
    ///
    /// To actually serve a request and honour `delay_ms`, use
    /// [`MockServer::handle_with_latency`] instead.
    pub fn resolve(&self, method: &str, path: &str) -> Option<MockEndpoint> {
        let key = format!("{}:{}", method.to_uppercase(), path);
        let map = self.endpoints.read().expect("Lock poisoned");
        map.get(&key).cloned()
    }

    /// Serve a request through the mock server, applying the configured latency.
    ///
    /// This is the primary response path: unlike [`MockServer::resolve`] (a pure
    /// lookup), it **consumes** [`MockEndpoint::delay_ms`] by blocking for that many
    /// milliseconds before returning the matched endpoint. The module is deliberately
    /// synchronous, so latency is simulated with `std::thread::sleep`; no async runtime
    /// is required. A missing route returns `None` immediately (no sleep).
    pub fn handle_with_latency(&self, method: &str, path: &str) -> Option<MockEndpoint> {
        let endpoint = self.resolve(method, path)?;
        if endpoint.delay_ms > 0 {
            std::thread::sleep(std::time::Duration::from_millis(endpoint.delay_ms));
        }
        Some(endpoint)
    }

    /// Alias for [`MockServer::handle_with_latency`], the latency-aware response path.
    pub fn handle(&self, method: &str, path: &str) -> Option<MockEndpoint> {
        self.handle_with_latency(method, path)
    }

    /// Auto-generate realistic mock data from a JSON Schema type definition.
    #[must_use]
    pub fn generate_from_schema(schema: &Value, seed: usize) -> Value {
        match schema
            .get("type")
            .and_then(Value::as_str)
            .unwrap_or("object")
        {
            "string" => {
                let format = schema.get("format").and_then(Value::as_str).unwrap_or("");
                match format {
                    "email" => json!(MockFaker::email(seed)),
                    "uuid" => json!(MockFaker::uuid(seed as u64)),
                    "date" => json!(MockFaker::date_iso(seed)),
                    _ => json!(MockFaker::name(seed)),
                }
            }
            "integer" | "number" => {
                let min = schema.get("minimum").and_then(Value::as_u64).unwrap_or(1);
                let max = schema.get("maximum").and_then(Value::as_u64).unwrap_or(100);
                json!(MockFaker::amount(seed as u64, min, max))
            }
            "boolean" => json!(seed.is_multiple_of(2)),
            "array" => {
                let default_item = json!({"type": "string"});
                let item_schema = schema.get("items").unwrap_or(&default_item);
                let items: Vec<Value> = (0..3)
                    .map(|i| Self::generate_from_schema(item_schema, seed + i))
                    .collect();
                json!(items)
            }
            _ => {
                // Object schema
                let mut map = serde_json::Map::new();
                if let Some(props) = schema.get("properties").and_then(Value::as_object) {
                    for (idx, (prop_name, prop_schema)) in props.iter().enumerate() {
                        let val = Self::generate_from_schema(prop_schema, seed + idx * 7);
                        map.insert(prop_name.clone(), val);
                    }
                }
                Value::Object(map)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_faker_deterministic() {
        assert_eq!(MockFaker::name(0), "Alice Vance");
        assert_eq!(MockFaker::name(1), "Bob Stone");
        assert_eq!(MockFaker::email(0), "alice.vance@example.com");
        assert_eq!(MockFaker::company(0), "Acme Global Corp");
        assert_eq!(MockFaker::city(0), "Paris");

        let u1 = MockFaker::uuid(42);
        let u2 = MockFaker::uuid(42);
        assert_eq!(u1, u2);
        assert_eq!(u1.len(), 36);

        let amt = MockFaker::amount(10, 50, 100);
        assert!((50..=100).contains(&amt));
    }

    #[test]
    fn test_mock_server_matching_and_schema_gen() {
        let server = MockServer::new();
        server.mock(
            "GET",
            "/api/v1/users",
            200,
            json!([{"id": 1, "name": "Alice"}]),
        );

        let res = server
            .resolve("GET", "/api/v1/users")
            .expect("Route should resolve");
        assert_eq!(res.status, 200);
        assert_eq!(res.body[0]["name"], "Alice");

        assert!(server.resolve("POST", "/api/v1/users").is_none());

        // Test schema generation
        let schema = json!({
            "type": "object",
            "properties": {
                "name": { "type": "string" },
                "email": { "type": "string", "format": "email" },
                "age": { "type": "integer", "minimum": 18, "maximum": 90 }
            }
        });

        let mock_obj = MockServer::generate_from_schema(&schema, 1);
        assert!(mock_obj["email"].as_str().unwrap().contains('@'));
        let age = mock_obj["age"].as_u64().unwrap();
        assert!((18..=90).contains(&age));
    }

    #[test]
    fn test_mock_server_delay_is_really_applied() {
        use std::time::{Duration, Instant};

        let server = MockServer::new();
        server.register(MockEndpoint::new("GET", "/fast", 200, json!({"ok": true})));
        server.register(MockEndpoint::new("GET", "/slow", 200, json!({"ok": true})).delay(25));

        let fast_start = Instant::now();
        let fast = server
            .handle_with_latency("GET", "/fast")
            .expect("fast route must resolve");
        let fast_elapsed = fast_start.elapsed();

        let slow_start = Instant::now();
        let slow = server
            .handle_with_latency("GET", "/slow")
            .expect("slow route must resolve");
        let slow_elapsed = slow_start.elapsed();

        assert_eq!(fast.delay_ms, 0);
        assert_eq!(slow.delay_ms, 25);
        assert_eq!(slow.status, 200);

        // Generous margin: sleep(25ms) must add at least 20ms of wall-clock latency.
        assert!(
            slow_elapsed >= Duration::from_millis(20),
            "expected >= 20ms latency, slow={slow_elapsed:?}, fast={fast_elapsed:?}"
        );

        // `handle` is the same latency-aware path; `resolve` stays a pure lookup.
        assert_eq!(server.handle("GET", "/slow").map(|e| e.delay_ms), Some(25));
        assert!(server.resolve("GET", "/missing").is_none());
    }
}
