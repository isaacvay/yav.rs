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
    let mut url = pattern.to_string();
    let mut sorted_params = params.to_vec();
    sorted_params.sort_by_key(|a| std::cmp::Reverse(a.0.len()));

    for (k, v) in sorted_params {
        let colon = format!(":{k}");
        let star = format!("*{k}");
        let bracket = format!("[{k}]");
        let encoded_seg = matcher::percent_encode(v, false);
        let encoded_catch = matcher::percent_encode(v, true);
        url = replace_param_token(&url, &colon, &encoded_seg);
        url = replace_param_token(&url, &star, &encoded_catch);
        url = url.replace(&bracket, &encoded_seg);
    }
    url
}

fn replace_param_token(input: &str, token: &str, replacement: &str) -> String {
    let mut result = String::with_capacity(input.len());
    let mut rem = input;
    while let Some(pos) = rem.find(token) {
        let after_idx = pos + token.len();
        let is_boundary = if let Some(next_char) = rem[after_idx..].chars().next() {
            !next_char.is_ascii_alphanumeric() && next_char != '_'
        } else {
            true
        };

        if is_boundary {
            result.push_str(&rem[..pos]);
            result.push_str(replacement);
            rem = &rem[after_idx..];
        } else {
            result.push_str(&rem[..after_idx]);
            rem = &rem[after_idx..];
        }
    }
    result.push_str(rem);
    result
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
