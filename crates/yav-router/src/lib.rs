//! # YAV Router
//!
//! URL pattern matching and routing system for the Yav web framework.
//!
//! This crate provides:
//! - [`Route`]: Definition of a route pattern, HTTP method, handler name, and metadata.
//! - [`RouteSegment`]: Represents static, dynamic, or catch-all path segments.
//! - [`RouteMatch`]: Results of a matched route with path params and query parameters.
//! - [`RouteTable`]: Router table that stores routes and resolves requests.
//!
//! ## Pattern Matching Features
//! - Static paths: `/about`, `/contact`
//! - Dynamic segments: `/users/:id`, `/posts/:slug`
//! - Catch-all wildcards: `/files/*path`
//! - Nested paths: `/dashboard/settings`
//! - Specificity priority: `static > dynamic > catch-all`
//! - Query string parsing: `?query=rust&page=1`
//! - HTTP method dispatch: `GET`, `POST`, `PUT`, `DELETE`, etc.
//!
//! ## Example
//! ```
//! use yav_router::{Route, RouteTable};
//!
//! let mut table = RouteTable::new();
//! table.add(Route::new("GET", "/", "index"));
//! table.add(Route::new("GET", "/users/:id", "user_detail"));
//! table.add(Route::new("GET", "/files/*path", "serve_file"));
//!
//! let m = table.resolve("GET", "/users/42").unwrap();
//! assert_eq!(m.params.get("id"), Some(&"42".to_string()));
//! ```

pub mod fs_router;
pub mod matcher;
pub mod route;
pub mod table;
pub mod trie;

pub use fs_router::{
    extract_intercept_info, extract_parallel_slots, file_path_to_route_pattern, DiscoveredRoute,
    FileSystemRouter, InterceptLevel,
};
pub use matcher::{
    compare_route_priority, find_best_match, match_route, parse_query_string, percent_decode,
    split_path_and_query,
};
pub use route::{
    parse_segments, to_axum_pattern, Route, RouteMatch, RouteSegment, SegmentPriority,
};
pub use table::RouteTable;
pub use trie::{RouteTrie, RouteTrieNode};

/// Helper to construct parameterised URLs safely (type-safe reverse routing).
///
/// Replaces `:param`, `*slug`, and `[param]` placeholders with supplied values.
///
/// # Examples
/// ```
/// use yav_router::url_for;
///
/// assert_eq!(url_for("/users/:id", &[("id", "42")]), "/users/42");
/// assert_eq!(url_for("/files/*path", &[("path", "images/logo.png")]), "/files/images/logo.png");
/// ```
pub fn url_for(pattern: &str, params: &[(&str, &str)]) -> String {
    let mut param_map = std::collections::HashMap::with_capacity(params.len());
    for (k, v) in params {
        param_map.insert(*k, *v);
    }

    let bytes = pattern.as_bytes();
    let mut out = String::with_capacity(pattern.len());
    let mut i = 0;

    while i < bytes.len() {
        let ch = pattern[i..].chars().next().unwrap();
        if ch == ':' || ch == '*' {
            let is_catch = ch == '*';
            let rest = &pattern[i + 1..];
            let name_len = rest
                .find(|c: char| !c.is_ascii_alphanumeric() && c != '_')
                .unwrap_or(rest.len());
            if name_len > 0 {
                let key = &rest[..name_len];
                if let Some(val) = param_map.get(key) {
                    out.push_str(&matcher::percent_encode(val, is_catch));
                    i += 1 + name_len;
                    continue;
                }
            }
        } else if ch == '[' {
            let rest = &pattern[i + 1..];
            if let Some(close_idx) = rest.find(']') {
                let raw_key = &rest[..close_idx];
                let (is_catch, key) = match raw_key.strip_prefix("...") {
                    Some(k) => (true, k),
                    None => (false, raw_key),
                };
                if !key.is_empty() {
                    if let Some(val) = param_map.get(key) {
                        out.push_str(&matcher::percent_encode(val, is_catch));
                        i += 1 + close_idx + 1;
                        continue;
                    }
                }
            }
        }
        out.push(ch);
        i += ch.len_utf8();
    }
    out
}

/// Convenience prelude module.
pub mod prelude {
    pub use crate::fs_router::{file_path_to_route_pattern, DiscoveredRoute, FileSystemRouter};
    pub use crate::route::{to_axum_pattern, Route, RouteMatch, RouteSegment};
    pub use crate::table::RouteTable;
    pub use crate::url_for;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_url_for_percent_encoding_and_injection_prevention() {
        let u1 = url_for("/users/:id", &[("id", "42")]);
        assert_eq!(u1, "/users/42");

        let u2 = url_for("/users/:name", &[("name", "Jean Dupont")]);
        assert_eq!(u2, "/users/Jean%20Dupont");

        let u3 = url_for("/search/:query", &[("query", "admin/delete?all=true")]);
        assert_eq!(u3, "/search/admin%2Fdelete%3Fall=true");

        let u4 = url_for("/files/*path", &[("path", "docs/intro/getting-started.md")]);
        assert_eq!(u4, "/files/docs/intro/getting-started.md");
    }
}
