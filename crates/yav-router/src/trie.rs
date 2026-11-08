//! High-performance Radix Prefix Tree (Trie) for $O(L)$ route resolution.
//!
//! Provides deterministic logarithmic-time prefix routing matching Next.js / Axum priority:
//! `Static (3) > Dynamic (2) > CatchAll (1)`.

use std::collections::HashMap;

use crate::matcher::{compare_route_priority, match_route, parse_query_string};
use crate::route::{Route, RouteMatch, RouteSegment};

/// A node in the Radix Route Trie.
#[derive(Debug, Clone, Default)]
pub struct RouteTrieNode {
    /// Static path children indexed by segment literal (e.g. "about", "posts")
    pub static_children: HashMap<String, RouteTrieNode>,
    /// Dynamic parameter child (e.g. ":id", ":slug") with parameter name and subtree
    pub dynamic_child: Option<(String, Box<RouteTrieNode>)>,
    /// Catch-all terminal child (e.g. "*path") with parameter name and subtree
    pub catch_all_child: Option<(String, Box<RouteTrieNode>)>,
    /// End-of-path routes registered at this node (differentiated by HTTP method)
    pub routes: Vec<Route>,
}

/// High-performance Prefix Route Trie for $O(L)$ route dispatching.
#[derive(Debug, Clone, Default)]
pub struct RouteTrie {
    root: RouteTrieNode,
}

impl RouteTrie {
    /// Create a new, empty route trie.
    #[must_use]
    pub fn new() -> Self {
        Self {
            root: RouteTrieNode::default(),
        }
    }

    /// Insert a route into the prefix trie.
    pub fn insert(&mut self, route: Route) {
        let mut curr = &mut self.root;

        for seg in &route.segments {
            match seg {
                RouteSegment::Static(s) => {
                    curr = curr.static_children.entry(s.clone()).or_default();
                }
                RouteSegment::Dynamic(param) => {
                    if curr.dynamic_child.is_none() {
                        curr.dynamic_child =
                            Some((param.clone(), Box::new(RouteTrieNode::default())));
                    }
                    let (_, ref mut next_node) = curr.dynamic_child.as_mut().unwrap();
                    curr = next_node;
                }
                RouteSegment::CatchAll(param) => {
                    if curr.catch_all_child.is_none() {
                        curr.catch_all_child =
                            Some((param.clone(), Box::new(RouteTrieNode::default())));
                    }
                    let (_, ref mut next_node) = curr.catch_all_child.as_mut().unwrap();
                    curr = next_node;
                }
            }
        }

        curr.routes.push(route);
    }

    /// Resolve an incoming HTTP method and URL in $O(L)$ trie traversal.
    #[must_use]
    pub fn resolve(&self, method: &str, raw_url: &str) -> Option<RouteMatch> {
        let url_without_hash = raw_url.split('#').next().unwrap_or(raw_url);
        let (path_part, query_str) = match url_without_hash.split_once('?') {
            Some((p, q)) => (p, Some(q)),
            None => (url_without_hash, None),
        };
        let req_segments: Vec<&str> = path_part.split('/').filter(|s| !s.is_empty()).collect();

        let best_route = self.match_node(&self.root, &req_segments, 0, method)?;
        let match_result = match_route(&best_route, method, raw_url)?;
        let query = query_str
            .map(parse_query_string)
            .unwrap_or(match_result.query);

        Some(RouteMatch::new(best_route, match_result.params, query))
    }

    fn match_node<'a>(
        &'a self,
        node: &'a RouteTrieNode,
        segments: &[&str],
        index: usize,
        method: &str,
    ) -> Option<Route> {
        let mut candidates = Vec::new();

        // 1. If at end of segments, check terminal routes on this node
        if index == segments.len() {
            for r in &node.routes {
                if r.matches_method(method) {
                    candidates.push(r.clone());
                }
            }
        }

        if index < segments.len() {
            let seg = segments[index];

            // 2. Try static branch (Priority: Highest)
            if let Some(static_child) = node.static_children.get(seg) {
                if let Some(r) = self.match_node(static_child, segments, index + 1, method) {
                    candidates.push(r);
                }
            }

            // 3. Try dynamic branch (Priority: Medium)
            if let Some((_, ref dynamic_child)) = node.dynamic_child {
                if let Some(r) = self.match_node(dynamic_child, segments, index + 1, method) {
                    candidates.push(r);
                }
            }
        }

        // 4. Try catch-all branch (Priority: Lowest)
        if let Some((_, ref catch_all_node)) = node.catch_all_child {
            for r in &catch_all_node.routes {
                if r.matches_method(method) {
                    candidates.push(r.clone());
                }
            }
            if index < segments.len() {
                for i in index + 1..=segments.len() {
                    if let Some(r) = self.match_node(catch_all_node, segments, i, method) {
                        candidates.push(r);
                    }
                }
            }
        }

        candidates.into_iter().max_by(compare_route_priority)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_route_trie_static_and_dynamic() {
        let mut trie = RouteTrie::new();
        trie.insert(Route::new("GET", "/", "root"));
        trie.insert(Route::new("GET", "/about", "about"));
        trie.insert(Route::new("GET", "/users/:id", "user_detail"));
        trie.insert(Route::new("GET", "/users/:id/settings", "user_settings"));
        trie.insert(Route::new("GET", "/files/*path", "files"));

        let m1 = trie.resolve("GET", "/").unwrap();
        assert_eq!(m1.route.handler, "root");

        let m2 = trie.resolve("GET", "/about").unwrap();
        assert_eq!(m2.route.handler, "about");

        let m3 = trie.resolve("GET", "/users/42").unwrap();
        assert_eq!(m3.route.handler, "user_detail");
        assert_eq!(m3.params.get("id"), Some(&"42".to_string()));

        let m4 = trie.resolve("GET", "/users/99/settings").unwrap();
        assert_eq!(m4.route.handler, "user_settings");
        assert_eq!(m4.params.get("id"), Some(&"99".to_string()));

        let m5 = trie.resolve("GET", "/files/docs/report.pdf").unwrap();
        assert_eq!(m5.route.handler, "files");
        assert_eq!(m5.params.get("path"), Some(&"docs/report.pdf".to_string()));
    }
}
