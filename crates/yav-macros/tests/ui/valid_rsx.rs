//! Fixture compiled (and run) by `trybuild_pass.rs` via `t.pass(...)`.
//!
//! Uses the autonomous `yav_macros::rsx!` macro directly. Its expansion only
//! needs `yav_core`'s public `Node`/`Element`/`Attribute` types — no `yav`
//! facade — so it compiles with `yav-core` as the sole extra dev-dependency.

use yav_core::Node;
use yav_macros::rsx;

fn main() {
    let node: Node = rsx! {
        <div class="contract">
            <h1>"Contract OK"</h1>
        </div>
    };

    match node {
        Node::Element(element) => {
            assert_eq!(element.tag.as_ref(), "div");
            assert_eq!(element.children.len(), 1);
        }
        other => panic!("expected an element node, got {other:?}"),
    }
}
