//! Route definition types.
//!
//! This module defines the core data structures used by the router:
//! - [`Route`]: represents a registered route pattern, method, handler, and metadata.
//! - [`RouteSegment`]: represents an individual segment of a route path pattern.
//! - [`RouteMatch`]: represents a successfully resolved route with extracted parameters.

use std::collections::HashMap;
use std::fmt;

use serde::{Deserialize, Serialize};
use yav_core::PageMeta;

/// A segment in a route path pattern.
///
/// Route patterns are composed of one or more segments separated by slashes.
///
/// # Examples
/// - Static: `about` in `/about`
/// - Dynamic: `:id` in `/users/:id`
/// - Catch-all: `*path` in `/files/*path`
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RouteSegment {
    /// A fixed literal path segment, e.g. `about` or `users`.
    Static(String),
    /// A named variable segment, e.g. `:id` or `:slug`.
    Dynamic(String),
    /// A catch-all wildcard segment matching the remainder of the path, e.g. `*path`.
    CatchAll(String),
}

impl RouteSegment {
    /// Parse a single path segment string into a [`RouteSegment`].
    ///
    /// Leading `:` denotes a dynamic segment.
    /// Leading `*` denotes a catch-all segment.
    /// Everything else is a static segment.
    pub fn parse(segment: &str) -> Self {
        if let Some(param) = segment.strip_prefix(':') {
            RouteSegment::Dynamic(param.to_string())
        } else if let Some(param) = segment.strip_prefix('*') {
            let name = if param.is_empty() { "path" } else { param };
            RouteSegment::CatchAll(name.to_string())
        } else if segment.starts_with('{') && segment.ends_with('}') && segment.len() >= 2 {
            let inner = &segment[1..segment.len() - 1];
            if let Some(slug) = inner.strip_prefix('*') {
                let name = if slug.is_empty() { "path" } else { slug };
                RouteSegment::CatchAll(name.to_string())
            } else {
                RouteSegment::Dynamic(inner.to_string())
            }
        } else {
            RouteSegment::Static(segment.to_string())
        }
    }

    /// Check if this segment is static.
    pub fn is_static(&self) -> bool {
        matches!(self, RouteSegment::Static(_))
    }

    /// Check if this segment is dynamic.
    pub fn is_dynamic(&self) -> bool {
        matches!(self, RouteSegment::Dynamic(_))
    }

    /// Check if this segment is catch-all.
    pub fn is_catch_all(&self) -> bool {
        matches!(self, RouteSegment::CatchAll(_))
    }

    /// Return the priority rank for this segment.
    /// Higher values indicate higher priority: Static (3) > Dynamic (2) > CatchAll (1).
    pub fn priority(&self) -> SegmentPriority {
        match self {
            RouteSegment::Static(_) => SegmentPriority::Static,
            RouteSegment::Dynamic(_) => SegmentPriority::Dynamic,
            RouteSegment::CatchAll(_) => SegmentPriority::CatchAll,
        }
    }
}

impl fmt::Display for RouteSegment {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RouteSegment::Static(s) => write!(f, "{s}"),
            RouteSegment::Dynamic(s) => write!(f, ":{s}"),
            RouteSegment::CatchAll(s) => write!(f, "*{s}"),
        }
    }
}

/// Priority rank for route segments during conflict resolution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SegmentPriority {
    /// Catch-all wildcard segments have the lowest priority.
    CatchAll = 1,
    /// Dynamic named segments have medium priority.
    Dynamic = 2,
    /// Static literal segments have the highest priority.
    Static = 3,
}

/// A registered route definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Route {
    /// HTTP method (e.g., "GET", "POST", "PUT", "DELETE").
    pub method: String,
    /// Path pattern string (e.g., `/users/:id`, `/files/*path`).
    pub pattern: String,
    /// Path pattern alias (identical to `pattern`).
    pub path: String,
    /// Handler identifier or name (e.g., "index", "user_detail").
    pub handler: String,
    /// Metadata associated with the page or route.
    #[serde(skip)]
    pub metadata: PageMeta,
    /// Parsed route segments.
    pub segments: Vec<RouteSegment>,
}

impl Route {
    /// Create a new route with the given HTTP method, path pattern, and handler name.
    ///
    /// # Example
    /// ```
    /// use yav_router::Route;
    ///
    /// let route = Route::new("GET", "/users/:id", "user_detail");
    /// assert_eq!(route.method, "GET");
    /// assert_eq!(route.pattern, "/users/:id");
    /// assert_eq!(route.handler, "user_detail");
    /// ```
    pub fn new(
        method: impl Into<String>,
        pattern: impl Into<String>,
        handler: impl Into<String>,
    ) -> Self {
        let pattern_str = pattern.into();
        let segments = parse_segments(&pattern_str);
        let method_str = method.into().to_uppercase();
        Self {
            method: method_str,
            path: pattern_str.clone(),
            pattern: pattern_str,
            handler: handler.into(),
            metadata: PageMeta::default(),
            segments,
        }
    }

    /// Create a GET route.
    pub fn get(pattern: impl Into<String>, handler: impl Into<String>) -> Self {
        Self::new("GET", pattern, handler)
    }

    /// Create a POST route.
    pub fn post(pattern: impl Into<String>, handler: impl Into<String>) -> Self {
        Self::new("POST", pattern, handler)
    }

    /// Create a PUT route.
    pub fn put(pattern: impl Into<String>, handler: impl Into<String>) -> Self {
        Self::new("PUT", pattern, handler)
    }

    /// Create a DELETE route.
    pub fn delete(pattern: impl Into<String>, handler: impl Into<String>) -> Self {
        Self::new("DELETE", pattern, handler)
    }

    /// Create a PATCH route.
    pub fn patch(pattern: impl Into<String>, handler: impl Into<String>) -> Self {
        Self::new("PATCH", pattern, handler)
    }

    /// Create a HEAD route.
    pub fn head(pattern: impl Into<String>, handler: impl Into<String>) -> Self {
        Self::new("HEAD", pattern, handler)
    }

    /// Create an OPTIONS route.
    pub fn options(pattern: impl Into<String>, handler: impl Into<String>) -> Self {
        Self::new("OPTIONS", pattern, handler)
    }

    /// Attach metadata to this route.
    pub fn with_metadata(mut self, metadata: PageMeta) -> Self {
        self.metadata = metadata;
        self
    }

    /// Alias for [`Self::with_metadata`].
    pub fn with_meta(mut self, metadata: PageMeta) -> Self {
        self.metadata = metadata;
        self
    }

    /// Check if this route matches the given HTTP method (case-insensitive).
    pub fn matches_method(&self, method: &str) -> bool {
        self.method.eq_ignore_ascii_case(method)
    }
}

impl PartialEq for Route {
    fn eq(&self, other: &Self) -> bool {
        self.method == other.method
            && self.pattern == other.pattern
            && self.handler == other.handler
            && self.segments == other.segments
            && self.metadata.title == other.metadata.title
            && self.metadata.description == other.metadata.description
            && self.metadata.css == other.metadata.css
            && self.metadata.scripts == other.metadata.scripts
            && self.metadata.status == other.metadata.status
    }
}

impl Eq for Route {}

impl fmt::Display for Route {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {} -> {}", self.method, self.pattern, self.handler)
    }
}

/// The result of successfully matching an incoming HTTP request to a route.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteMatch {
    /// The matched route definition.
    pub route: Route,
    /// Extracted path parameters (e.g., `id` => `"42"`).
    pub params: HashMap<String, String>,
    /// Extracted query string parameters (e.g., `q` => `"rust"`).
    pub query: HashMap<String, String>,
    /// Alias for `query`.
    pub query_params: HashMap<String, String>,
}

impl RouteMatch {
    /// Create a new `RouteMatch`.
    pub fn new(
        route: Route,
        params: HashMap<String, String>,
        query: HashMap<String, String>,
    ) -> Self {
        Self {
            route,
            params,
            query_params: query.clone(),
            query,
        }
    }

    /// Get a path parameter by name.
    pub fn param(&self, name: &str) -> Option<&str> {
        self.params.get(name).map(|s| s.as_str())
    }

    /// Get a query parameter by name.
    pub fn query_val(&self, name: &str) -> Option<&str> {
        self.query.get(name).map(|s| s.as_str())
    }

    /// Get the handler identifier.
    pub fn handler(&self) -> &str {
        &self.route.handler
    }
}

impl PartialEq for RouteMatch {
    fn eq(&self, other: &Self) -> bool {
        self.route == other.route && self.params == other.params && self.query == other.query
    }
}

impl Eq for RouteMatch {}

/// Parse a path pattern into a vector of [`RouteSegment`]s.
pub fn parse_segments(pattern: &str) -> Vec<RouteSegment> {
    pattern
        .split('/')
        .filter(|s| !s.is_empty())
        .map(RouteSegment::parse)
        .collect()
}

/// Converts any route pattern (`:id`, `*slug`, `{id}`, `{*slug}`) into an Axum 0.8 compatible path pattern.
///
/// # Examples
/// ```
/// use yav_router::route::to_axum_pattern;
///
/// assert_eq!(to_axum_pattern("/users/:id"), "/users/{id}");
/// assert_eq!(to_axum_pattern("/files/*path"), "/files/{*path}");
/// assert_eq!(to_axum_pattern("/already/{id}"), "/already/{id}");
/// ```
pub fn to_axum_pattern(pattern: &str) -> String {
    if pattern.is_empty() || pattern == "/" {
        return "/".to_string();
    }
    let mut segments = Vec::new();
    for seg in pattern.split('/') {
        if seg.is_empty() {
            continue;
        }
        if let Some(param) = seg.strip_prefix(':') {
            segments.push(format!("{{{param}}}"));
        } else if let Some(slug) = seg.strip_prefix('*') {
            let name = if slug.is_empty() { "path" } else { slug };
            segments.push(format!("{{*{name}}}"));
        } else {
            segments.push(seg.to_string());
        }
    }
    format!("/{}", segments.join("/"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_segments_basic() {
        let segs = parse_segments("/users/:id/posts/*slug");
        assert_eq!(
            segs,
            vec![
                RouteSegment::Static("users".into()),
                RouteSegment::Dynamic("id".into()),
                RouteSegment::Static("posts".into()),
                RouteSegment::CatchAll("slug".into()),
            ]
        );
    }

    #[test]
    fn parse_segments_root() {
        let segs = parse_segments("/");
        assert!(segs.is_empty());

        let segs_empty = parse_segments("");
        assert!(segs_empty.is_empty());
    }

    #[test]
    fn route_new_fields() {
        let r = Route::new("get", "/users/:id", "user_detail");
        assert_eq!(r.method, "GET");
        assert_eq!(r.pattern, "/users/:id");
        assert_eq!(r.path, "/users/:id");
        assert_eq!(r.handler, "user_detail");
        assert_eq!(r.segments.len(), 2);
    }

    #[test]
    fn route_helpers() {
        let r = Route::post("/users", "create_user");
        assert_eq!(r.method, "POST");
        assert!(r.matches_method("post"));
        assert!(r.matches_method("POST"));
        assert!(!r.matches_method("GET"));
    }

    #[test]
    fn route_metadata() {
        let meta = PageMeta {
            title: Some("User Detail".into()),
            ..Default::default()
        };
        let r = Route::get("/user", "user").with_meta(meta);
        assert_eq!(r.metadata.title, Some("User Detail".into()));
    }

    #[test]
    fn route_match_param_access() {
        let r = Route::new("GET", "/users/:id", "detail");
        let mut params = HashMap::new();
        params.insert("id".into(), "42".into());
        let mut query = HashMap::new();
        query.insert("tab".into(), "overview".into());

        let m = RouteMatch::new(r, params, query);
        assert_eq!(m.param("id"), Some("42"));
        assert_eq!(m.param("missing"), None);
        assert_eq!(m.params.get("id"), Some(&"42".to_string()));
        assert_eq!(m.query.get("tab"), Some(&"overview".to_string()));
        assert_eq!(m.query_params.get("tab"), Some(&"overview".to_string()));
        assert_eq!(m.handler(), "detail");
    }
}
