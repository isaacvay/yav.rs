//! Fine-grained reactive Signals & Declarative Client State Scopes (`yav::signals`).
//!
//! Provides both:
//! 1. **Rust-native reactive primitives**: [`Signal<T>`], [`ReadSignal<T>`], [`WriteSignal<T>`],
//!    [`Computed<T>`], and [`create_signal`] / [`create_memo`] with automatic subscriber notifications.
//! 2. **Declarative SSR-to-Client reactive scopes**: [`SignalScope`] for building CSP-safe
//!    reactive islands (`yav:state`, `yav:bind`, `yav:text`, `yav:show`, `yav:on-click`) without `unsafe-eval`.

use std::fmt;
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc, Mutex, RwLock,
};

use crate::node::{Element, IntoNode, Node};

type Subscriber<T> = Arc<dyn Fn(&T) + Send + Sync + 'static>;

/// Unique identifier for an active Signal subscription.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SubscriptionId(pub u64);

struct SignalInner<T> {
    value: RwLock<T>,
    version: AtomicU64,
    subscribers: Mutex<Vec<(u64, Subscriber<T>)>>,
}

/// A thread-safe reactive state container.
///
/// Can be read, mutated, and subscribed to across components and async tasks.
pub struct Signal<T> {
    inner: Arc<SignalInner<T>>,
}

impl<T> Clone for Signal<T> {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}

impl<T: fmt::Debug + Clone> fmt::Debug for Signal<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let val = self
            .inner
            .value
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .clone();
        let ver = self.inner.version.load(Ordering::SeqCst);
        f.debug_struct("Signal")
            .field("value", &val)
            .field("version", &ver)
            .finish()
    }
}

impl<T: Clone + Send + Sync + 'static> Signal<T> {
    /// Create a new reactive [`Signal`] initialized with `initial`.
    pub fn new(initial: T) -> Self {
        Self {
            inner: Arc::new(SignalInner {
                value: RwLock::new(initial),
                version: AtomicU64::new(0),
                subscribers: Mutex::new(Vec::new()),
            }),
        }
    }

    /// Return a cloned snapshot of the current signal value.
    pub fn get(&self) -> T {
        self.inner
            .value
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    /// Inspect the current value by reference without cloning.
    pub fn with<R>(&self, f: impl FnOnce(&T) -> R) -> R {
        let guard = self.inner.value.read().unwrap_or_else(|e| e.into_inner());
        f(&*guard)
    }

    /// Set a new value and notify all registered subscribers.
    pub fn set(&self, new_value: T) {
        {
            let mut guard = self.inner.value.write().unwrap_or_else(|e| e.into_inner());
            *guard = new_value;
        }
        self.inner.version.fetch_add(1, Ordering::SeqCst);
        self.notify_subscribers();
    }

    /// Mutate the value in-place and notify subscribers.
    pub fn update(&self, f: impl FnOnce(&mut T)) {
        {
            let mut guard = self.inner.value.write().unwrap_or_else(|e| e.into_inner());
            f(&mut *guard);
        }
        self.inner.version.fetch_add(1, Ordering::SeqCst);
        self.notify_subscribers();
    }

    /// Monotonically increasing revision number bumped on every mutation.
    pub fn version(&self) -> u64 {
        self.inner.version.load(Ordering::SeqCst)
    }

    /// Register a callback invoked whenever the signal's value changes.
    /// Returns a [`SubscriptionId`] that can be used to unsubscribe.
    pub fn subscribe(&self, callback: impl Fn(&T) + Send + Sync + 'static) -> SubscriptionId {
        static SUB_COUNTER: AtomicU64 = AtomicU64::new(1);
        let id = SubscriptionId(SUB_COUNTER.fetch_add(1, Ordering::Relaxed));
        let mut subs = self
            .inner
            .subscribers
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        subs.push((id.0, Arc::new(callback)));
        id
    }

    /// Unregister an active subscription by its [`SubscriptionId`]. Returns `true` if removed.
    pub fn unsubscribe(&self, id: SubscriptionId) -> bool {
        let mut subs = self
            .inner
            .subscribers
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let initial_len = subs.len();
        subs.retain(|(sub_id, _)| *sub_id != id.0);
        subs.len() < initial_len
    }

    /// Split into a read-only [`ReadSignal`] and write-only [`WriteSignal`].
    pub fn split(&self) -> (ReadSignal<T>, WriteSignal<T>) {
        (
            ReadSignal {
                signal: self.clone(),
            },
            WriteSignal {
                signal: self.clone(),
            },
        )
    }

    fn notify_subscribers(&self) {
        let snapshot = self.get();
        let subs: Vec<Subscriber<T>> = {
            let subs_guard = self
                .inner
                .subscribers
                .lock()
                .unwrap_or_else(|e| e.into_inner());
            subs_guard.iter().map(|(_, sub)| Arc::clone(sub)).collect()
        };
        for sub in subs {
            sub(&snapshot);
        }
    }
}

impl<T: fmt::Display + Clone + Send + Sync + 'static> IntoNode for Signal<T> {
    fn into_node(self) -> Node {
        Node::Text(self.get().to_string())
    }
}

/// Read-only handle to a [`Signal`].
#[derive(Clone)]
pub struct ReadSignal<T> {
    signal: Signal<T>,
}

impl<T: Clone + Send + Sync + 'static> ReadSignal<T> {
    /// Get a clone of the current value.
    pub fn get(&self) -> T {
        self.signal.get()
    }

    /// Borrow the current value inside a closure.
    pub fn with<R>(&self, f: impl FnOnce(&T) -> R) -> R {
        self.signal.with(f)
    }

    /// Current revision version.
    pub fn version(&self) -> u64 {
        self.signal.version()
    }
}

impl<T: fmt::Display + Clone + Send + Sync + 'static> IntoNode for ReadSignal<T> {
    fn into_node(self) -> Node {
        Node::Text(self.get().to_string())
    }
}

/// Write-only handle to a [`Signal`].
#[derive(Clone)]
pub struct WriteSignal<T> {
    signal: Signal<T>,
}

impl<T: Clone + Send + Sync + 'static> WriteSignal<T> {
    /// Set a new signal value.
    pub fn set(&self, value: T) {
        self.signal.set(value);
    }

    /// Mutate the current signal value in place.
    pub fn update(&self, f: impl FnOnce(&mut T)) {
        self.signal.update(f);
    }
}

/// Create a `(ReadSignal<T>, WriteSignal<T>)` pair initialized with `initial`.
pub fn create_signal<T: Clone + Send + Sync + 'static>(
    initial: T,
) -> (ReadSignal<T>, WriteSignal<T>) {
    Signal::new(initial).split()
}

/// A memoized derived signal that automatically updates when its source [`Signal`] changes.
#[derive(Clone)]
pub struct Computed<U> {
    derived: Signal<U>,
}

impl<U: Clone + Send + Sync + 'static> Computed<U> {
    /// Get the current derived value.
    pub fn get(&self) -> U {
        self.derived.get()
    }
}

impl<U: fmt::Display + Clone + Send + Sync + 'static> IntoNode for Computed<U> {
    fn into_node(self) -> Node {
        Node::Text(self.get().to_string())
    }
}

/// Create a derived [`Computed`] signal from a source [`Signal`] and a pure transformation function.
pub fn create_memo<T, U, F>(source: &Signal<T>, compute: F) -> Computed<U>
where
    T: Clone + Send + Sync + 'static,
    U: Clone + Send + Sync + 'static,
    F: Fn(&T) -> U + Send + Sync + 'static,
{
    let initial = source.with(&compute);
    let derived = Signal::new(initial);
    let derived_clone = derived.clone();
    source.subscribe(move |val| {
        derived_clone.set(compute(val));
    });
    Computed { derived }
}

/// Create a derived [`Computed`] signal from two source signals and a combiner function.
pub fn create_memo2<T1, T2, U, F>(s1: &Signal<T1>, s2: &Signal<T2>, compute: F) -> Computed<U>
where
    T1: Clone + Send + Sync + 'static,
    T2: Clone + Send + Sync + 'static,
    U: Clone + Send + Sync + 'static,
    F: Fn(&T1, &T2) -> U + Send + Sync + 'static,
{
    let compute = Arc::new(compute);
    let initial = compute(&s1.get(), &s2.get());
    let derived = Signal::new(initial);

    let d1 = derived.clone();
    let s2_clone = s2.clone();
    let c1 = Arc::clone(&compute);
    s1.subscribe(move |v1| {
        d1.set(c1(v1, &s2_clone.get()));
    });

    let d2 = derived.clone();
    let s1_clone = s1.clone();
    let c2 = Arc::clone(&compute);
    s2.subscribe(move |v2| {
        d2.set(c2(&s1_clone.get(), v2));
    });

    Computed { derived }
}

/// Declarative builder for client-side reactive scopes (`yav:state`).
///
/// Compiles initial state into a safe JSON payload on the container element,
/// powering zero-bundle, CSP-safe client reactivity (`yav:text`, `yav:bind`, `yav:show`, `yav:on-click`).
#[derive(Debug, Clone, Default)]
pub struct SignalScope {
    tag: String,
    class: Option<String>,
    entries: Vec<(String, String)>,
    children: Vec<Node>,
}

impl SignalScope {
    /// Create a new client signal scope container (`<div yav:state="...">`).
    pub fn new() -> Self {
        Self {
            tag: "div".to_string(),
            class: None,
            entries: Vec::new(),
            children: Vec::new(),
        }
    }

    /// Customize the HTML wrapper tag (defaults to `"div"`).
    pub fn tag(mut self, tag: impl Into<String>) -> Self {
        self.tag = tag.into();
        self
    }

    /// Add CSS classes to the scope container.
    pub fn class(mut self, class: impl Into<String>) -> Self {
        self.class = Some(class.into());
        self
    }

    /// Declare an integer or float reactive state variable.
    pub fn state_num(mut self, key: impl Into<String>, value: impl fmt::Display) -> Self {
        self.entries.push((key.into(), value.to_string()));
        self
    }

    /// Declare a boolean reactive state variable.
    pub fn state_bool(mut self, key: impl Into<String>, value: bool) -> Self {
        self.entries
            .push((key.into(), if value { "true" } else { "false" }.to_string()));
        self
    }

    /// Declare a string reactive state variable.
    pub fn state_str(mut self, key: impl Into<String>, value: impl AsRef<str>) -> Self {
        let escaped = value
            .as_ref()
            .replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('\n', "\\n");
        self.entries.push((key.into(), format!("\"{escaped}\"")));
        self
    }

    /// Add a child node inside this reactive scope.
    pub fn child(mut self, node: impl IntoNode) -> Self {
        self.children.push(node.into_node());
        self
    }

    /// Render the reactive scope into a [`Node`].
    pub fn render(self) -> Node {
        let pairs: Vec<String> = self
            .entries
            .iter()
            .map(|(k, v)| format!("\"{}\":{}", k.replace('"', "\\\""), v))
            .collect();
        let json_state = format!("{{{}}}", pairs.join(","));

        let mut el = Element::new(self.tag).attr("yav:state", json_state);
        if let Some(cls) = self.class {
            el = el.attr("class", cls);
        }
        el = el.children(self.children);
        Node::Element(el)
    }
}

impl IntoNode for SignalScope {
    fn into_node(self) -> Node {
        self.render()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::render_to_string;

    #[test]
    fn test_signal_read_write_and_subscription() {
        let count = Signal::new(10);
        assert_eq!(count.get(), 10);
        assert_eq!(count.version(), 0);

        let observed = Arc::new(Mutex::new(Vec::new()));
        let obs_clone = Arc::clone(&observed);
        let sub_id = count.subscribe(move |v| {
            obs_clone.lock().unwrap().push(*v);
        });

        count.set(25);
        assert_eq!(count.get(), 25);
        assert_eq!(count.version(), 1);

        count.update(|v| *v += 5);
        assert_eq!(count.get(), 30);
        assert_eq!(count.version(), 2);

        let history = observed.lock().unwrap().clone();
        assert_eq!(history, vec![25, 30]);

        assert!(count.unsubscribe(sub_id));
        count.set(40);
        let history_after = observed.lock().unwrap().clone();
        assert_eq!(history_after, vec![25, 30]);
    }

    #[test]
    fn test_create_memo2_multi_signal() {
        let a = Signal::new(5);
        let b = Signal::new(10);
        let sum = create_memo2(&a, &b, |x, y| *x + *y);
        assert_eq!(sum.get(), 15);

        a.set(7);
        assert_eq!(sum.get(), 17);

        b.set(20);
        assert_eq!(sum.get(), 27);
    }

    #[test]
    fn test_create_memo_derived_computation() {
        let base = Signal::new(4);
        let doubled = create_memo(&base, |v| *v * 2);
        assert_eq!(doubled.get(), 8);

        base.set(15);
        assert_eq!(doubled.get(), 30);
    }

    #[test]
    fn test_signal_scope_rendering() {
        let scope = SignalScope::new()
            .class("p-4")
            .state_num("count", 0)
            .state_bool("open", true)
            .state_str("label", "Items")
            .child(Element::new("span").attr("yav:text", "count"))
            .render();

        let html = render_to_string(&scope);
        assert!(html.contains(r#"yav:state="{&quot;count&quot;:0,&quot;open&quot;:true,&quot;label&quot;:&quot;Items&quot;}""#));
        assert!(html.contains(r#"yav:text="count""#));
    }
}
