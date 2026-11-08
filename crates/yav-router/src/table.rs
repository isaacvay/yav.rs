//! Route table that maps paths to handlers.
//!
//! The [`RouteTable`] manages a collection of [`Route`] definitions and provides
//! request matching with parameter extraction, query string parsing, and
//! specificity-based priority ordering.

use crate::matcher::find_best_match;
use crate::route::{Route, RouteMatch};
use crate::trie::RouteTrie;

/// A table holding registered routes, supporting request resolution.
///
/// # Example
/// ```
/// use yav_router::{Route, RouteTable};
///
/// let mut table = RouteTable::new();
/// table.add(Route::new("GET", "/", "index"));
/// table.add(Route::new("GET", "/users/:id", "user_detail"));
/// table.add(Route::new("GET", "/files/*path", "serve_file"));
///
/// let m = table.resolve("GET", "/users/42").unwrap();
/// assert_eq!(m.params.get("id"), Some(&"42".to_string()));
/// ```
#[derive(Debug, Clone, Default)]
pub struct RouteTable {
    routes: Vec<Route>,
    trie: RouteTrie,
}

impl RouteTable {
    /// Create a new, empty route table.
    pub fn new() -> Self {
        Self {
            routes: Vec::new(),
            trie: RouteTrie::new(),
        }
    }

    /// Register a route in the table.
    ///
    /// # Example
    /// ```
    /// use yav_router::{Route, RouteTable};
    ///
    /// let mut table = RouteTable::new();
    /// table.add(Route::new("GET", "/about", "about_page"));
    /// assert_eq!(table.len(), 1);
    /// ```
    pub fn add(&mut self, route: Route) {
        tracing::debug!(
            method = %route.method,
            pattern = %route.pattern,
            handler = %route.handler,
            "Added route to table"
        );
        self.trie.insert(route.clone());
        self.routes.push(route);
    }

    /// Match an incoming HTTP request method and path against registered routes.
    ///
    /// First attempts fast $O(L)$ Radix Trie traversal, falling back to full matcher if needed.
    pub fn resolve(&self, method: &str, path: &str) -> Option<RouteMatch> {
        tracing::debug!(method = %method, path = %path, "Resolving request");
        if let Some(m) = self.trie.resolve(method, path) {
            return Some(m);
        }
        find_best_match(&self.routes, method, path)
    }

    /// Return a slice of all registered routes in registration order.
    pub fn routes(&self) -> &[Route] {
        &self.routes
    }

    /// Return the number of registered routes.
    pub fn len(&self) -> usize {
        self.routes.len()
    }

    /// Check if the route table is empty.
    pub fn is_empty(&self) -> bool {
        self.routes.is_empty()
    }
}

impl FromIterator<Route> for RouteTable {
    fn from_iter<T: IntoIterator<Item = Route>>(iter: T) -> Self {
        let mut table = Self::new();
        for route in iter {
            table.add(route);
        }
        table
    }
}

impl Extend<Route> for RouteTable {
    fn extend<T: IntoIterator<Item = Route>>(&mut self, iter: T) {
        for route in iter {
            self.add(route);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prompt_example_usage() {
        let mut table = RouteTable::new();
        table.add(Route::new("GET", "/", "index"));
        table.add(Route::new("GET", "/users/:id", "user_detail"));
        table.add(Route::new("GET", "/files/*path", "serve_file"));

        let m = table.resolve("GET", "/users/42").unwrap();
        assert_eq!(m.params.get("id"), Some(&"42".to_string()));
    }

    #[test]
    fn static_route_matching() {
        let mut table = RouteTable::new();
        table.add(Route::new("GET", "/", "index"));
        table.add(Route::new("GET", "/about", "about"));
        table.add(Route::new("GET", "/contact", "contact"));
        table.add(Route::new("GET", "/dashboard/settings", "settings"));

        let m1 = table.resolve("GET", "/").unwrap();
        assert_eq!(m1.route.handler, "index");

        let m2 = table.resolve("GET", "/about").unwrap();
        assert_eq!(m2.route.handler, "about");

        let m3 = table.resolve("GET", "/contact").unwrap();
        assert_eq!(m3.route.handler, "contact");

        let m4 = table.resolve("GET", "/dashboard/settings").unwrap();
        assert_eq!(m4.route.handler, "settings");

        // Trailing slash normalization
        let m5 = table.resolve("GET", "/about/").unwrap();
        assert_eq!(m5.route.handler, "about");
    }

    #[test]
    fn dynamic_segment_extraction() {
        let mut table = RouteTable::new();
        table.add(Route::new("GET", "/users/:id", "user_detail"));
        table.add(Route::new("GET", "/posts/:slug", "post_detail"));
        table.add(Route::new(
            "GET",
            "/users/:user_id/posts/:post_id",
            "user_post",
        ));

        let m1 = table.resolve("GET", "/users/42").unwrap();
        assert_eq!(m1.route.handler, "user_detail");
        assert_eq!(m1.params.get("id"), Some(&"42".to_string()));

        let m2 = table.resolve("GET", "/posts/hello-world").unwrap();
        assert_eq!(m2.route.handler, "post_detail");
        assert_eq!(m2.params.get("slug"), Some(&"hello-world".to_string()));

        let m3 = table.resolve("GET", "/users/10/posts/99").unwrap();
        assert_eq!(m3.route.handler, "user_post");
        assert_eq!(m3.params.get("user_id"), Some(&"10".to_string()));
        assert_eq!(m3.params.get("post_id"), Some(&"99".to_string()));
    }

    #[test]
    fn catch_all_matching() {
        let mut table = RouteTable::new();
        table.add(Route::new("GET", "/files/*path", "serve_file"));
        table.add(Route::new("GET", "/*all", "fallback"));

        let m1 = table.resolve("GET", "/files/docs/readme.txt").unwrap();
        assert_eq!(m1.route.handler, "serve_file");
        assert_eq!(m1.params.get("path"), Some(&"docs/readme.txt".to_string()));

        let m2 = table.resolve("GET", "/files/single.png").unwrap();
        assert_eq!(m2.route.handler, "serve_file");
        assert_eq!(m2.params.get("path"), Some(&"single.png".to_string()));

        let m3 = table.resolve("GET", "/any/unknown/deep/path").unwrap();
        assert_eq!(m3.route.handler, "fallback");
        assert_eq!(
            m3.params.get("all"),
            Some(&"any/unknown/deep/path".to_string())
        );
    }

    #[test]
    fn priority_ordering() {
        // Register in reverse order (catch-all first, then dynamic, then static)
        // to prove resolution is priority-driven, not insertion-order dependent.
        let mut table = RouteTable::new();
        table.add(Route::new("GET", "/*catchall", "catchall"));
        table.add(Route::new("GET", "/users/*path", "user_catchall"));
        table.add(Route::new("GET", "/users/:id", "user_dynamic"));
        table.add(Route::new("GET", "/users/admin", "user_static"));
        table.add(Route::new(
            "GET",
            "/dashboard/:section",
            "dashboard_dynamic",
        ));
        table.add(Route::new("GET", "/dashboard/settings", "dashboard_static"));

        // Static wins over dynamic and catch-all
        let m1 = table.resolve("GET", "/users/admin").unwrap();
        assert_eq!(m1.route.handler, "user_static");

        // Dynamic wins over catch-all
        let m2 = table.resolve("GET", "/users/42").unwrap();
        assert_eq!(m2.route.handler, "user_dynamic");
        assert_eq!(m2.params.get("id"), Some(&"42".to_string()));

        // Subpath catch-all matches multi-segment
        let m3 = table.resolve("GET", "/users/42/details/extra").unwrap();
        assert_eq!(m3.route.handler, "user_catchall");
        assert_eq!(m3.params.get("path"), Some(&"42/details/extra".to_string()));

        // Nested static beats dynamic
        let m4 = table.resolve("GET", "/dashboard/settings").unwrap();
        assert_eq!(m4.route.handler, "dashboard_static");

        let m5 = table.resolve("GET", "/dashboard/analytics").unwrap();
        assert_eq!(m5.route.handler, "dashboard_dynamic");
        assert_eq!(m5.params.get("section"), Some(&"analytics".to_string()));

        // General fallback
        let m6 = table.resolve("GET", "/other/page").unwrap();
        assert_eq!(m6.route.handler, "catchall");
    }

    #[test]
    fn no_match_returns_none() {
        let mut table = RouteTable::new();
        table.add(Route::new("GET", "/users", "list_users"));
        table.add(Route::new("GET", "/users/:id", "user_detail"));

        // Path does not exist
        assert!(table.resolve("GET", "/posts").is_none());
        assert!(table.resolve("GET", "/").is_none());

        // Too many segments for non-catchall route
        assert!(table.resolve("GET", "/users/42/extra").is_none());

        // Wrong method
        assert!(table.resolve("POST", "/users").is_none());
        assert!(table.resolve("DELETE", "/users/42").is_none());
    }

    #[test]
    fn query_string_parsing() {
        let mut table = RouteTable::new();
        table.add(Route::new("GET", "/search", "search"));

        let m = table
            .resolve("GET", "/search?q=rust&page=2&sort=desc")
            .unwrap();
        assert_eq!(m.route.handler, "search");
        assert_eq!(m.query.get("q"), Some(&"rust".to_string()));
        assert_eq!(m.query.get("page"), Some(&"2".to_string()));
        assert_eq!(m.query.get("sort"), Some(&"desc".to_string()));
        assert_eq!(m.query_params.get("q"), Some(&"rust".to_string()));

        // URL decoding in query string
        let m2 = table
            .resolve("GET", "/search?q=hello%20world&tag=c%2B%2B")
            .unwrap();
        assert_eq!(m2.query.get("q"), Some(&"hello world".to_string()));
        assert_eq!(m2.query.get("tag"), Some(&"c++".to_string()));

        // Flags without values
        let m3 = table.resolve("GET", "/search?active&verified=").unwrap();
        assert_eq!(m3.query.get("active"), Some(&"".to_string()));
        assert_eq!(m3.query.get("verified"), Some(&"".to_string()));

        // Stripping fragment
        let m4 = table.resolve("GET", "/search?q=test#heading").unwrap();
        assert_eq!(m4.query.get("q"), Some(&"test".to_string()));

        // Path without query has empty map
        let m5 = table.resolve("GET", "/search").unwrap();
        assert!(m5.query.is_empty());
    }

    #[test]
    fn multiple_methods_on_same_path() {
        let mut table = RouteTable::new();
        table.add(Route::new("GET", "/items", "list_items"));
        table.add(Route::new("POST", "/items", "create_item"));
        table.add(Route::new("PUT", "/items", "update_items"));
        table.add(Route::new("DELETE", "/items", "delete_items"));

        assert_eq!(
            table.resolve("GET", "/items").unwrap().route.handler,
            "list_items"
        );
        assert_eq!(
            table.resolve("POST", "/items").unwrap().route.handler,
            "create_item"
        );
        assert_eq!(
            table.resolve("PUT", "/items").unwrap().route.handler,
            "update_items"
        );
        assert_eq!(
            table.resolve("DELETE", "/items").unwrap().route.handler,
            "delete_items"
        );

        // Unhandled method
        assert!(table.resolve("PATCH", "/items").is_none());
        assert!(table.resolve("HEAD", "/items").is_none());
    }

    #[test]
    fn routes_listing() {
        let mut table = RouteTable::new();
        table.add(Route::new("GET", "/", "index"));
        table.add(Route::new("GET", "/about", "about"));
        table.add(Route::new("POST", "/submit", "submit"));

        let list = table.routes();
        assert_eq!(list.len(), 3);
        assert_eq!(list[0].handler, "index");
        assert_eq!(list[1].handler, "about");
        assert_eq!(list[2].handler, "submit");
    }

    #[test]
    fn empty_table() {
        let table = RouteTable::new();
        assert!(table.is_empty());
        assert_eq!(table.len(), 0);
        assert!(table.resolve("GET", "/").is_none());
    }
}
