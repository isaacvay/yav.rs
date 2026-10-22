//! URL pattern matching engine.
//!
//! This module provides pattern matching against [`Route`] definitions,
//! path parameter extraction, query string parsing, percent-decoding,
//! and priority-based conflict resolution.

use std::collections::HashMap;

use crate::route::{Route, RouteMatch, RouteSegment};

/// Percent-decode a URL-encoded string.
///
/// Converts `%XX` hexadecimal sequences to their byte equivalents and `+` to space.
/// Invalid sequences are preserved as-is.
///
/// # Examples
/// ```
/// use yav_router::matcher::percent_decode;
///
/// assert_eq!(percent_decode("hello%20world"), "hello world");
/// assert_eq!(percent_decode("tag%2B1"), "tag+1");
/// assert_eq!(percent_decode("foo+bar"), "foo bar");
/// ```
pub fn percent_decode(input: &str) -> String {
    let mut bytes = Vec::with_capacity(input.len());
    let mut chars = input.bytes();

    while let Some(b) = chars.next() {
        match b {
            b'%' => {
                let h1 = chars.next();
                let h2 = chars.next();
                if let (Some(h1), Some(h2)) = (h1, h2) {
                    if let Ok(hex_str) = std::str::from_utf8(&[h1, h2]) {
                        if let Ok(byte) = u8::from_str_radix(hex_str, 16) {
                            bytes.push(byte);
                            continue;
                        }
                    }
                    bytes.push(b'%');
                    bytes.push(h1);
                    bytes.push(h2);
                } else {
                    bytes.push(b'%');
                    if let Some(h1) = h1 {
                        bytes.push(h1);
                    }
                }
            }
            b'+' => bytes.push(b' '),
            _ => bytes.push(b),
        }
    }

    String::from_utf8(bytes).unwrap_or_else(|e| String::from_utf8_lossy(e.as_bytes()).into_owned())
}

/// Decode a percent-encoded URL path segment.
///
/// Unlike query string decoding, `+` is preserved as a literal `+` (not converted to space),
/// and encoded slashes (`%2F`, `%5C`) are NOT decoded into path separators or directory traversals.
pub fn percent_decode_path_segment(input: &str) -> String {
    let mut bytes = Vec::with_capacity(input.len());
    let mut chars = input.bytes();

    while let Some(b) = chars.next() {
        match b {
            b'%' => {
                let h1 = chars.next();
                let h2 = chars.next();
                if let (Some(h1), Some(h2)) = (h1, h2) {
                    if let Ok(hex_str) = std::str::from_utf8(&[h1, h2]) {
                        if let Ok(byte) = u8::from_str_radix(hex_str, 16) {
                            // Do not decode encoded '/' (0x2F) or '\' (0x5C) to prevent directory traversal
                            if byte == b'/' || byte == b'\\' {
                                bytes.push(b'%');
                                bytes.push(h1);
                                bytes.push(h2);
                                continue;
                            }
                            bytes.push(byte);
                            continue;
                        }
                    }
                    bytes.push(b'%');
                    bytes.push(h1);
                    bytes.push(h2);
                } else {
                    bytes.push(b'%');
                    if let Some(h1) = h1 {
                        bytes.push(h1);
                    }
                }
            }
            // In path segments, '+' is literal '+', not space
            _ => bytes.push(b),
        }
    }

    String::from_utf8(bytes).unwrap_or_else(|e| String::from_utf8_lossy(e.as_bytes()).into_owned())
}

/// Parse a raw query string into a [`HashMap`].
///
/// Leading `?` is stripped if present. Keys and values are percent-decoded.
/// Keys without values (e.g., `?flag`) are stored with an empty string `""`.
///
/// # Examples
/// ```
/// use yav_router::matcher::parse_query_string;
///
/// let params = parse_query_string("q=rust&page=2");
/// assert_eq!(params.get("q"), Some(&"rust".to_string()));
/// assert_eq!(params.get("page"), Some(&"2".to_string()));
/// ```
pub fn parse_query_string(query: &str) -> HashMap<String, String> {
    let mut params = HashMap::new();
    let query_clean = query.strip_prefix('?').unwrap_or(query);

    if query_clean.is_empty() {
        return params;
    }

    for pair in query_clean.split('&') {
        if pair.is_empty() {
            continue;
        }
        let (k, v) = match pair.split_once('=') {
            Some((key, val)) => (percent_decode(key), percent_decode(val)),
            None => (percent_decode(pair), String::new()),
        };
        if !k.is_empty() {
            params.insert(k, v);
        }
    }

    params
}

/// Split a raw request URL into its path component and parsed query parameters.
///
/// Also strips any fragment `#...` from the URL.
///
/// # Examples
/// ```
/// use yav_router::matcher::split_path_and_query;
///
/// let (path, query) = split_path_and_query("/search?q=rust#top");
/// assert_eq!(path, "/search");
/// assert_eq!(query.get("q"), Some(&"rust".to_string()));
/// ```
pub fn split_path_and_query(raw_url: &str) -> (&str, HashMap<String, String>) {
    let url_without_hash = raw_url.split('#').next().unwrap_or(raw_url);

    match url_without_hash.split_once('?') {
        Some((path, query_str)) => (path, parse_query_string(query_str)),
        None => (url_without_hash, HashMap::new()),
    }
}

/// Attempt to match a single [`Route`] against an HTTP method and request URL.
///
/// Returns `Some(RouteMatch)` on successful match, or `None` if the route does not match.
pub fn match_route(route: &Route, method: &str, raw_url: &str) -> Option<RouteMatch> {
    if !route.matches_method(method) {
        return None;
    }

    let (path_part, query) = split_path_and_query(raw_url);
    let req_segments: Vec<&str> = path_part.split('/').filter(|s| !s.is_empty()).collect();

    let catch_all_pos = route.segments.iter().position(|s| s.is_catch_all());

    match catch_all_pos {
        None => {
            // Exact number of segments required when there is no catch-all
            if req_segments.len() != route.segments.len() {
                return None;
            }

            let mut params = HashMap::new();
            for (route_seg, req_seg) in route.segments.iter().zip(req_segments.iter()) {
                match route_seg {
                    RouteSegment::Static(expected) => {
                        if expected != *req_seg {
                            return None;
                        }
                    }
                    RouteSegment::Dynamic(param_name) => {
                        params.insert(param_name.clone(), percent_decode_path_segment(req_seg));
                    }
                    RouteSegment::CatchAll(_) => unreachable!(),
                }
            }

            Some(RouteMatch::new(route.clone(), params, query))
        }
        Some(pos) => {
            let prefix_len = pos;
            let suffix_len = route.segments.len() - 1 - pos;

            if req_segments.len() < prefix_len + suffix_len {
                return None;
            }

            let mut params = HashMap::new();

            // Match prefix segments (before catch-all)
            for (i, req_seg) in req_segments.iter().enumerate().take(prefix_len) {
                match &route.segments[i] {
                    RouteSegment::Static(expected) => {
                        if expected != *req_seg {
                            return None;
                        }
                    }
                    RouteSegment::Dynamic(param_name) => {
                        params.insert(param_name.clone(), percent_decode_path_segment(req_seg));
                    }
                    RouteSegment::CatchAll(_) => unreachable!(),
                }
            }

            // Match suffix segments (after catch-all, if any)
            for i in 0..suffix_len {
                let route_idx = pos + 1 + i;
                let req_idx = req_segments.len() - suffix_len + i;
                match &route.segments[route_idx] {
                    RouteSegment::Static(expected) => {
                        if expected != req_segments[req_idx] {
                            return None;
                        }
                    }
                    RouteSegment::Dynamic(param_name) => {
                        params.insert(
                            param_name.clone(),
                            percent_decode_path_segment(req_segments[req_idx]),
                        );
                    }
                    RouteSegment::CatchAll(_) => unreachable!(),
                }
            }

            // Extract catch-all segment(s)
            let catch_all_name = match &route.segments[pos] {
                RouteSegment::CatchAll(name) => name.as_str(),
                _ => "path",
            };

            let remaining_slice = &req_segments[prefix_len..req_segments.len() - suffix_len];
            let remaining_decoded: Vec<String> = remaining_slice
                .iter()
                .map(|seg| percent_decode_path_segment(seg))
                .collect();
            params.insert(catch_all_name.to_string(), remaining_decoded.join("/"));

            Some(RouteMatch::new(route.clone(), params, query))
        }
    }
}

/// Compare two routes by specificity priority.
///
/// Returns `Ordering::Greater` if `a` has higher priority than `b`.
/// Returns `Ordering::Less` if `b` has higher priority than `a`.
/// Returns `Ordering::Equal` if they have equal priority.
///
/// Priority rule:
/// `static (3) > dynamic (2) > catch-all (1)` evaluated segment-by-segment from left to right.
/// Routes without catch-all beat catch-all routes when all compared segments are equal.
pub fn compare_route_priority(a: &Route, b: &Route) -> std::cmp::Ordering {
    let max_len = a.segments.len().max(b.segments.len());
    for i in 0..max_len {
        let seg_a = a.segments.get(i);
        let seg_b = b.segments.get(i);

        match (seg_a, seg_b) {
            (Some(sa), Some(sb)) => {
                let ord = sa.priority().cmp(&sb.priority());
                if ord != std::cmp::Ordering::Equal {
                    return ord;
                }
            }
            (None, Some(sb)) => {
                // `a` has fewer segments. An exact route beats a trailing catch-all,
                // while a longer path with specific segments beats a shorter prefix.
                return if sb.is_catch_all() {
                    std::cmp::Ordering::Greater
                } else {
                    std::cmp::Ordering::Less
                };
            }
            (Some(sa), None) => {
                return if sa.is_catch_all() {
                    std::cmp::Ordering::Less
                } else {
                    std::cmp::Ordering::Greater
                };
            }
            (None, None) => break,
        }
    }
    std::cmp::Ordering::Equal
}

/// Find the highest-priority matching route from a slice of routes.
///
/// Evaluates each route against the incoming method and path, returning the match
/// with the highest specificity score (`static > dynamic > catch-all`).
pub fn find_best_match(routes: &[Route], method: &str, raw_url: &str) -> Option<RouteMatch> {
    let mut best_match: Option<RouteMatch> = None;

    for route in routes {
        if let Some(m) = match_route(route, method, raw_url) {
            match &best_match {
                None => {
                    best_match = Some(m);
                }
                Some(current_best) => {
                    if compare_route_priority(&m.route, &current_best.route)
                        == std::cmp::Ordering::Greater
                    {
                        best_match = Some(m);
                    }
                }
            }
        }
    }

    best_match
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn percent_decoding() {
        assert_eq!(percent_decode("hello%20world"), "hello world");
        assert_eq!(percent_decode("c%2B%2B"), "c++");
        assert_eq!(percent_decode("foo+bar"), "foo bar");
        assert_eq!(percent_decode("plain"), "plain");
        assert_eq!(percent_decode("%2Fpath%2Fto"), "/path/to");
        assert_eq!(percent_decode("%ZZ%1"), "%ZZ%1");
        assert_eq!(percent_decode("%"), "%");
    }

    #[test]
    fn parse_query() {
        let q = parse_query_string("foo=bar&baz=qux&num=42");
        assert_eq!(q.get("foo"), Some(&"bar".to_string()));
        assert_eq!(q.get("baz"), Some(&"qux".to_string()));
        assert_eq!(q.get("num"), Some(&"42".to_string()));

        let q_empty = parse_query_string("");
        assert!(q_empty.is_empty());

        let q_flags = parse_query_string("flag&active=true");
        assert_eq!(q_flags.get("flag"), Some(&"".to_string()));
        assert_eq!(q_flags.get("active"), Some(&"true".to_string()));
    }

    #[test]
    fn split_url_with_fragment_and_query() {
        let (path, query) = split_path_and_query("/users?tab=info#section");
        assert_eq!(path, "/users");
        assert_eq!(query.get("tab"), Some(&"info".to_string()));

        let (path2, query2) = split_path_and_query("/simple/path");
        assert_eq!(path2, "/simple/path");
        assert!(query2.is_empty());
    }

    #[test]
    fn match_static() {
        let route = Route::get("/about", "about_page");
        let m = match_route(&route, "GET", "/about").unwrap();
        assert_eq!(m.route.handler, "about_page");
        assert!(m.params.is_empty());

        assert!(match_route(&route, "GET", "/contact").is_none());
        assert!(match_route(&route, "POST", "/about").is_none());
    }

    #[test]
    fn match_dynamic() {
        let route = Route::get("/users/:id", "user_detail");
        let m = match_route(&route, "GET", "/users/100").unwrap();
        assert_eq!(m.params.get("id"), Some(&"100".to_string()));

        // Different segment count should not match
        assert!(match_route(&route, "GET", "/users").is_none());
        assert!(match_route(&route, "GET", "/users/100/extra").is_none());
    }

    #[test]
    fn match_catch_all() {
        let route = Route::get("/files/*path", "serve_file");
        let m = match_route(&route, "GET", "/files/assets/css/style.css").unwrap();
        assert_eq!(
            m.params.get("path"),
            Some(&"assets/css/style.css".to_string())
        );

        let m_single = match_route(&route, "GET", "/files/logo.png").unwrap();
        assert_eq!(m_single.params.get("path"), Some(&"logo.png".to_string()));

        let m_empty = match_route(&route, "GET", "/files").unwrap();
        assert_eq!(m_empty.params.get("path"), Some(&"".to_string()));
    }

    #[test]
    fn priority_comparison() {
        let r_static = Route::get("/users/admin", "static");
        let r_dynamic = Route::get("/users/:id", "dynamic");
        let r_catchall = Route::get("/users/*path", "catchall");

        assert_eq!(
            compare_route_priority(&r_static, &r_dynamic),
            std::cmp::Ordering::Greater
        );
        assert_eq!(
            compare_route_priority(&r_dynamic, &r_catchall),
            std::cmp::Ordering::Greater
        );
        assert_eq!(
            compare_route_priority(&r_static, &r_catchall),
            std::cmp::Ordering::Greater
        );
    }
}
