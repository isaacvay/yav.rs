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

pub use fs_router::{file_path_to_route_pattern, DiscoveredRoute, FileSystemRouter};
pub use matcher::{
    compare_route_priority, find_best_match, match_route, parse_query_string, percent_decode,
    split_path_and_query,
};
pub use route::{
    parse_segments, to_axum_pattern, Route, RouteMatch, RouteSegment, SegmentPriority,
};
pub use table::RouteTable;

/// Convenience prelude module.
pub mod prelude {
    pub use crate::fs_router::{file_path_to_route_pattern, DiscoveredRoute, FileSystemRouter};
    pub use crate::route::{to_axum_pattern, Route, RouteMatch, RouteSegment};
    pub use crate::table::RouteTable;
}
