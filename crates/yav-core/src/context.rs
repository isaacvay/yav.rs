//! SSR Request Context API for Yav applications.
//!
//! Provides dependency-injection and state sharing across component trees
//! during server-side rendering without prop-drilling.
//!
//! # Example
//! ```
//! use yav_core::context::{provide_context, use_context};
//!
//! #[derive(Clone, Debug, PartialEq)]
//! struct UserSession {
//!     username: String,
//! }
//!
//! let session = UserSession { username: "Alice".into() };
//!
//! let rendered = provide_context(session.clone(), || {
//!     // Deep inside any child component:
//!     let current = use_context::<UserSession>().expect("session must be present");
//!     format!("Hello, {}", current.username)
//! });
//!
//! assert_eq!(rendered, "Hello, Alice");
//! ```

use std::{
    any::{Any, TypeId},
    cell::RefCell,
    collections::HashMap,
    sync::Arc,
};

/// A thread-safe container storing heterogeneous context values keyed by type.
#[derive(Clone, Default)]
pub struct ContextMap {
    data: HashMap<TypeId, Arc<dyn Any + Send + Sync>>,
}

impl ContextMap {
    /// Create an empty context map.
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert or overwrite a context value of type `T`.
    pub fn insert<T: Send + Sync + 'static>(&mut self, value: T) {
        self.data.insert(TypeId::of::<T>(), Arc::new(value));
    }

    /// Retrieve a cloned instance of type `T` if present.
    pub fn get<T: Clone + 'static>(&self) -> Option<T> {
        self.data
            .get(&TypeId::of::<T>())
            .and_then(|arc| arc.downcast_ref::<T>())
            .cloned()
    }

    /// Check if a type `T` is registered in this context map.
    pub fn contains<T: 'static>(&self) -> bool {
        self.data.contains_key(&TypeId::of::<T>())
    }
}

thread_local! {
    static CONTEXT_STACK: RefCell<Vec<ContextMap>> = const { RefCell::new(Vec::new()) };
}

struct PopGuard;

impl Drop for PopGuard {
    fn drop(&mut self) {
        CONTEXT_STACK.with(|stack| {
            stack.borrow_mut().pop();
        });
    }
}

/// Run a closure with the specified context value available to all child components.
pub fn provide_context<T, R>(value: T, f: impl FnOnce() -> R) -> R
where
    T: Clone + Send + Sync + 'static,
{
    CONTEXT_STACK.with(|stack| {
        let mut map = stack.borrow().last().cloned().unwrap_or_default();
        map.insert(value);
        stack.borrow_mut().push(map);
    });

    let _guard = PopGuard;
    f()
}

/// Retrieve a value of type `T` from the current rendering context.
///
/// Returns `None` if no context of this type was provided.
pub fn use_context<T: Clone + 'static>() -> Option<T> {
    CONTEXT_STACK.with(|stack| stack.borrow().last().and_then(|map| map.get::<T>()))
}

/// Capture a clone of the current active context map, suitable for transfer across async tasks or threads.
pub fn current_context() -> ContextMap {
    CONTEXT_STACK.with(|stack| stack.borrow().last().cloned().unwrap_or_default())
}

/// Run a closure with the specified [`ContextMap`] as the active rendering context.
///
/// Useful for propagating captured context into new threads or across async boundaries.
pub fn with_context_map<R>(map: ContextMap, f: impl FnOnce() -> R) -> R {
    CONTEXT_STACK.with(|stack| {
        stack.borrow_mut().push(map);
    });

    let _guard = PopGuard;
    f()
}

/// Retrieve a value of type `T`, or return a fallback default if absent.
pub fn use_context_or_default<T: Clone + Default + 'static>() -> T {
    use_context::<T>().unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Debug, PartialEq)]
    struct Theme(String);

    #[derive(Clone, Debug, PartialEq)]
    struct User {
        name: String,
        is_admin: bool,
    }

    #[test]
    fn test_context_basic() {
        let user = User {
            name: "John Doe".into(),
            is_admin: true,
        };

        let result = provide_context(user.clone(), || {
            let u = use_context::<User>().expect("User must be in context");
            assert_eq!(u.name, "John Doe");
            assert!(u.is_admin);
            "OK"
        });

        assert_eq!(result, "OK");
        // Outside context, should be None
        assert!(use_context::<User>().is_none());
    }

    #[test]
    fn test_nested_context_overrides() {
        let theme = Theme("dark".into());

        provide_context(theme, || {
            assert_eq!(use_context::<Theme>().unwrap().0, "dark");

            // Override in inner scope
            provide_context(Theme("light".into()), || {
                assert_eq!(use_context::<Theme>().unwrap().0, "light");
            });

            // Back to outer scope
            assert_eq!(use_context::<Theme>().unwrap().0, "dark");
        });
    }

    #[test]
    fn test_multiple_distinct_types() {
        provide_context(Theme("nord".into()), || {
            provide_context(
                User {
                    name: "Bob".into(),
                    is_admin: false,
                },
                || {
                    assert_eq!(use_context::<Theme>().unwrap().0, "nord");
                    assert_eq!(use_context::<User>().unwrap().name, "Bob");
                },
            );
        });
    }

    #[test]
    fn test_provide_context_panic_safety() {
        let panic_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            provide_context(Theme("temporary".into()), || {
                assert_eq!(use_context::<Theme>().unwrap().0, "temporary");
                panic!("intentional panic inside component rendering");
            });
        }));

        assert!(panic_result.is_err());
        // Verify stack was cleanly popped and no context leak remains
        assert_eq!(use_context::<Theme>(), None);
    }

    #[test]
    fn test_current_context_and_cross_thread_propagation() {
        let theme = Theme("synthwave".into());

        provide_context(theme, || {
            let captured = current_context();

            // Run in a separate thread transferring the captured context
            let handle = std::thread::spawn(move || {
                with_context_map(captured, || {
                    let t = use_context::<Theme>().expect("Theme must be accessible across thread");
                    assert_eq!(t.0, "synthwave");
                });
            });

            handle.join().expect("thread finished ok");
        });
    }
}
