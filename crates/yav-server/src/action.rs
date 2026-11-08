//! Route loaders, actions, and typed route execution context.
//!
//! Provides the Remix/Next.js-style loader and action paradigm for Yav applications:
//! - [`RouteContext`]: Extracted path parameters, query parameters, and headers.
//! - [`Loader`]: Type-safe async data loader providing context to SSR components.
//! - [`FormAction`]: Typed form mutation handler with integrated validation and response handling.

use axum::{extract::Request, http::HeaderMap};
use std::{collections::HashMap, future::Future, sync::Arc};
use yav_core::context::provide_context;

use crate::error::YavError;
use crate::forms::{ActionResponse, FormErrors};

/// Typed path parameters extracted from the route matching engine and stored in request extensions.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RouteParams(pub HashMap<String, String>);

/// Context provided to route loaders and actions containing extracted request metadata.
#[derive(Debug, Clone, Default)]
pub struct RouteContext {
    /// Request URI path.
    pub path: String,
    /// Extracted path parameters (e.g., `:id` -> `"42"`).
    pub params: HashMap<String, String>,
    /// Extracted query string parameters (e.g., `?sort=asc` -> `"sort"`: `"asc"`).
    pub query: HashMap<String, String>,
    /// Incoming HTTP request headers.
    pub headers: HeaderMap,
    /// Optional internationalization dictionary and locale resolver.
    pub i18n: Option<Arc<yav_core::I18n>>,
    /// Optional user session extracted from request extensions.
    pub session: Option<crate::session::Session>,
    /// Stable per-request cryptographic CSP nonce.
    pub nonce: String,
    /// Shared application state map injected via `YavApp::state`.
    pub app_state: yav_core::ContextMap,
}

impl RouteContext {
    /// Create an empty route context.
    pub fn new(path: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            params: HashMap::new(),
            query: HashMap::new(),
            headers: HeaderMap::new(),
            i18n: None,
            session: None,
            nonce: crate::middleware::CspNonce::generate().0,
            app_state: yav_core::ContextMap::new(),
        }
    }

    /// Extract a [`RouteContext`] from an incoming Axum [`Request`].
    pub fn from_request(req: &Request) -> Self {
        let uri = req.uri();
        let path = uri.path().to_string();

        let query = uri
            .query()
            .map(yav_router::parse_query_string)
            .unwrap_or_default();

        let mut params = HashMap::new();
        if let Some(route_params) = req.extensions().get::<RouteParams>() {
            params.clone_from(&route_params.0);
        }

        let i18n = req.extensions().get::<Arc<yav_core::I18n>>().cloned();
        let session = req.extensions().get::<crate::session::Session>().cloned();
        let app_state = req
            .extensions()
            .get::<yav_core::ContextMap>()
            .cloned()
            .unwrap_or_default();
        let nonce = req
            .headers()
            .get("x-nonce")
            .or_else(|| req.headers().get("csp-nonce"))
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string())
            .or_else(|| {
                req.extensions()
                    .get::<crate::middleware::CspNonce>()
                    .map(|n| n.0.clone())
            })
            .unwrap_or_else(|| crate::middleware::CspNonce::generate().0);

        Self {
            path,
            params,
            query,
            headers: req.headers().clone(),
            i18n,
            session,
            nonce,
            app_state,
        }
    }

    /// Attach path parameters into this context (builder pattern).
    pub fn with_param(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.params.insert(key.into(), value.into());
        self
    }

    /// Attach an [`I18n`](yav_core::I18n) dictionary to this context.
    pub fn with_i18n(mut self, i18n: impl Into<Arc<yav_core::I18n>>) -> Self {
        self.i18n = Some(i18n.into());
        self
    }

    /// Attach a [`Session`](crate::session::Session) to this context.
    pub fn with_session(mut self, session: crate::session::Session) -> Self {
        self.session = Some(session);
        self
    }

    /// Retrieve reference to the session if present.
    pub fn session(&self) -> Option<&crate::session::Session> {
        self.session.as_ref()
    }

    /// Retrieve the current session, or return a new empty fallback session if none was attached.
    pub fn get_session(&self) -> crate::session::Session {
        self.session.clone().unwrap_or_default()
    }

    /// Retrieve a path parameter by name.
    pub fn param(&self, key: &str) -> Option<&str> {
        self.params.get(key).map(|s| s.as_str())
    }

    /// Retrieve a query parameter by name.
    pub fn query_val(&self, key: &str) -> Option<&str> {
        self.query.get(key).map(|s| s.as_str())
    }

    /// Retrieve a header value as UTF-8 string by header name.
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers.get(name).and_then(|v| v.to_str().ok())
    }

    /// Retrieve a cookie value by name from the `Cookie` header.
    pub fn cookie(&self, name: &str) -> Option<String> {
        let cookie_header = self.header("cookie")?;
        for pair in cookie_header.split(';') {
            let mut parts = pair.splitn(2, '=');
            if let (Some(k), Some(v)) = (parts.next(), parts.next()) {
                if k.trim() == name {
                    return Some(v.trim().to_string());
                }
            }
        }
        None
    }

    /// Retrieve or generate a cryptographic CSP nonce for securing inline scripts.
    /// Returns a stable nonce across all calls within the same request.
    pub fn csp_nonce(&self) -> String {
        if !self.nonce.is_empty() {
            return self.nonce.clone();
        }
        if let Some(val) = self
            .headers
            .get("x-nonce")
            .or_else(|| self.headers.get("csp-nonce"))
        {
            if let Ok(s) = val.to_str() {
                if !s.is_empty() {
                    return s.to_string();
                }
            }
        }
        crate::middleware::CspNonce::generate().0
    }

    /// Resolve the most appropriate locale for this request.
    ///
    /// Respects `?lang=...` query parameter, `:lang` path parameter,
    /// `yav_locale` / `lang` cookie, and HTTP `Accept-Language` header, matching against supported locales
    /// if an [`I18n`](yav_core::I18n) catalog is attached. Defaults to `"en"`.
    pub fn locale(&self) -> String {
        let cookie_pref = self.cookie("yav_locale").or_else(|| self.cookie("lang"));
        let preferred = self
            .query_val("lang")
            .or_else(|| self.param("lang"))
            .or(cookie_pref.as_deref());
        let accept_header = self.header("accept-language");
        if let Some(ref i18n) = self.i18n {
            i18n.resolve_locale(accept_header, preferred)
        } else if let Some(pref) = preferred {
            pref.to_string()
        } else if let Some(hdr) = accept_header {
            for entry in hdr.split(',') {
                let tag = entry.split(';').next().unwrap_or("").trim();
                if !tag.is_empty() {
                    let base = tag.split('-').next().unwrap_or(tag);
                    return base.to_string();
                }
            }
            "en".to_string()
        } else {
            "en".to_string()
        }
    }

    /// Translate a key for the current request's resolved locale.
    pub fn t(&self, key: &str) -> String {
        self.t_with(key, &[])
    }

    /// Translate a key with variable interpolation (`{var_name}`) for the current request's locale.
    pub fn t_with(&self, key: &str, vars: &[(&str, &str)]) -> String {
        let loc = self.locale();
        if let Some(ref i18n) = self.i18n {
            i18n.translate(&loc, key, vars)
        } else {
            let mut out = key.to_string();
            for (k, v) in vars {
                out = out.replace(&format!("{{{k}}}"), v);
            }
            out
        }
    }

    /// Translate with pluralization rules for the current request's locale.
    pub fn t_plural(&self, count: usize, key_prefix: &str, vars: &[(&str, &str)]) -> String {
        let loc = self.locale();
        if let Some(ref i18n) = self.i18n {
            i18n.plural(&loc, count, key_prefix, vars)
        } else {
            let mut all_vars = vars.to_vec();
            let count_str = count.to_string();
            all_vars.push(("count", &count_str));
            self.t_with(key_prefix, &all_vars)
        }
    }

    /// Construct a [`ContextMap`](yav_core::ContextMap) populated with this [`RouteContext`],
    /// its [`I18n`](yav_core::I18n) catalog, [`Session`](crate::session::Session), and [`CspNonce`](crate::middleware::CspNonce).
    pub fn ssr_context_map(&self) -> yav_core::ContextMap {
        let mut map = self.app_state.clone();
        map.extend(&yav_core::current_context());
        map.insert(self.clone());
        let current_locale = self.locale();
        map.insert(current_locale);
        map.insert(crate::middleware::CspNonce(self.csp_nonce()));
        if let Some(ref i18n) = self.i18n {
            map.insert(i18n.clone());
            map.insert((**i18n).clone());
        }
        if let Some(ref session) = self.session {
            map.insert(session.clone());
        }
        map
    }

    /// Execute a closure with this route context, its [`I18n`](yav_core::I18n), and [`Session`](crate::session::Session)
    /// injected into the SSR [`use_context`](yav_core::use_context) scope.
    pub fn with_ssr_context<R>(&self, f: impl FnOnce() -> R) -> R {
        yav_core::with_context_map(self.ssr_context_map(), f)
    }
}

impl<S> axum::extract::FromRequestParts<S> for RouteContext
where
    S: Send + Sync,
{
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        _state: &S,
    ) -> Result<Self, Self::Rejection> {
        let path = parts.uri.path().to_string();
        let query = parts
            .uri
            .query()
            .map(yav_router::parse_query_string)
            .unwrap_or_default();
        let mut params = HashMap::new();
        if let Some(route_params) = parts.extensions.get::<RouteParams>() {
            params.clone_from(&route_params.0);
        }
        let i18n = parts.extensions.get::<Arc<yav_core::I18n>>().cloned();
        let session = parts.extensions.get::<crate::session::Session>().cloned();
        let app_state = parts
            .extensions
            .get::<yav_core::ContextMap>()
            .cloned()
            .unwrap_or_default();
        let nonce = parts
            .headers
            .get("x-nonce")
            .or_else(|| parts.headers.get("csp-nonce"))
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string())
            .or_else(|| {
                parts
                    .extensions
                    .get::<crate::middleware::CspNonce>()
                    .map(|n| n.0.clone())
            })
            .unwrap_or_else(|| crate::middleware::CspNonce::generate().0);

        Ok(Self {
            path,
            params,
            query,
            headers: parts.headers.clone(),
            i18n,
            session,
            nonce,
            app_state,
        })
    }
}

/// A pinned, boxed async function for route data loading.
pub type BoxLoaderFn<T> = Arc<
    dyn Fn(RouteContext) -> std::pin::Pin<Box<dyn Future<Output = Result<T, YavError>> + Send>>
        + Send
        + Sync,
>;

/// An async data loader that executes before route rendering.
#[derive(Clone)]
pub struct Loader<T> {
    load_fn: BoxLoaderFn<T>,
}

impl<T: Send + Sync + 'static> Loader<T> {
    /// Create a new loader with an asynchronous load closure.
    pub fn new<F, Fut>(f: F) -> Self
    where
        F: Fn(RouteContext) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<T, YavError>> + Send + 'static,
    {
        Self {
            load_fn: Arc::new(move |ctx| Box::pin(f(ctx))),
        }
    }

    /// Execute the loader with the given route context.
    pub async fn load(&self, ctx: RouteContext) -> Result<T, YavError> {
        (self.load_fn)(ctx).await
    }

    /// Execute the loader and provide the resulting data to the SSR context during render execution.
    pub async fn load_and_render<R>(
        &self,
        ctx: RouteContext,
        render: impl FnOnce() -> R,
    ) -> Result<R, YavError>
    where
        T: Clone + Send + Sync + 'static,
    {
        let data = self.load(ctx.clone()).await?;
        Ok(ctx.with_ssr_context(|| provide_context(data, render)))
    }
}

/// Pinned validator function for form actions.
pub type ActionValidatorFn<T> = Box<dyn Fn(&T, &mut FormErrors) + Send + Sync>;

/// A server form action validator and handler.
pub struct FormAction<T> {
    validator: ActionValidatorFn<T>,
}

impl<T> FormAction<T> {
    /// Create a new form action with a validation callback.
    pub fn new(validator: impl Fn(&T, &mut FormErrors) + Send + Sync + 'static) -> Self {
        Self {
            validator: Box::new(validator),
        }
    }

    /// Validate the submitted form data, returning [`FormErrors`] if any checks fail.
    pub fn validate(&self, input: &T) -> Result<(), FormErrors> {
        let mut errors = FormErrors::new();
        (self.validator)(input, &mut errors);
        if errors.has_errors() {
            Err(errors)
        } else {
            Ok(())
        }
    }

    /// Validate input, execute the mutation if valid, and return an [`ActionResponse`].
    pub async fn execute<F, Fut>(&self, input: &T, mutation: F) -> ActionResponse
    where
        F: FnOnce(&T) -> Fut,
        Fut: Future<Output = ActionResponse>,
    {
        if let Err(errors) = self.validate(input) {
            ActionResponse::errors(errors)
        } else {
            mutation(input).await
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use yav_core::context::use_context;

    #[derive(Clone, Debug, PartialEq)]
    struct UserProfile {
        id: u64,
        name: String,
    }

    #[tokio::test]
    async fn test_route_context_and_loader() {
        let ctx = RouteContext::new("/users/42").with_param("id", "42");

        assert_eq!(ctx.param("id"), Some("42"));
        assert_eq!(ctx.param("missing"), None);

        let loader = Loader::new(|ctx| async move {
            let id = ctx.param("id").unwrap_or("0").parse::<u64>().unwrap_or(0);
            Ok(UserProfile {
                id,
                name: format!("User #{}", id),
            })
        });

        let rendered = loader
            .load_and_render(ctx, || {
                let user = use_context::<UserProfile>().expect("UserProfile should be in context");
                format!("Profile: {} ({})", user.name, user.id)
            })
            .await
            .expect("loader must succeed");

        assert_eq!(rendered, "Profile: User #42 (42)");
    }

    #[tokio::test]
    async fn test_form_action_validation() {
        #[derive(Debug)]
        struct CreatePost {
            title: String,
        }

        let action = FormAction::new(|input: &CreatePost, errors: &mut FormErrors| {
            errors.require("title", &input.title, "Title is required");
        });

        // Invalid submission
        let invalid = CreatePost {
            title: "   ".into(),
        };
        let resp = action
            .execute(&invalid, |_| async { ActionResponse::redirect("/posts") })
            .await;
        match resp {
            ActionResponse::Errors(errs) => {
                assert!(errs.has_errors());
                assert_eq!(errs.first("title"), Some("Title is required"));
            }
            _ => panic!("Expected errors"),
        }

        // Valid submission
        let valid = CreatePost {
            title: "Hello Yav".into(),
        };
        let resp = action
            .execute(&valid, |_| async { ActionResponse::redirect("/posts/1") })
            .await;
        match resp {
            ActionResponse::Redirect(url) => assert_eq!(url, "/posts/1"),
            _ => panic!("Expected redirect"),
        }
    }

    #[test]
    fn test_route_context_query_percent_decoding() {
        let req = axum::extract::Request::builder()
            .uri("/search?q=hello%20world&filter=rust%2Baxum")
            .body(axum::body::Body::empty())
            .unwrap();

        let ctx = RouteContext::from_request(&req);
        assert_eq!(ctx.query.get("q"), Some(&"hello world".to_string()));
        assert_eq!(ctx.query.get("filter"), Some(&"rust+axum".to_string()));
    }

    #[tokio::test]
    async fn test_route_context_path_params_raw_params() {
        use tower::ServiceExt;

        let app = crate::app::YavApp::new()
            .get(
                "/users/:id/posts/:slug",
                |req: axum::extract::Request| async move {
                    let ctx = RouteContext::from_request(&req);
                    assert_eq!(ctx.param("id"), Some("42"));
                    assert_eq!(ctx.param("slug"), Some("hello world"));
                    "ok"
                },
            )
            .into_router();

        let req = axum::extract::Request::builder()
            .uri("/users/42/posts/hello%20world")
            .body(axum::body::Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), axum::http::StatusCode::OK);
    }

    #[test]
    fn test_route_context_i18n_integration() {
        let i18n = yav_core::I18n::new("en")
            .supported_locales(["en", "fr"])
            .add_translation("en", "welcome", "Welcome, {name}!")
            .add_translation("fr", "welcome", "Bienvenue, {name}!")
            .add_translation("en", "cart_zero", "No items")
            .add_translation("en", "cart_one", "1 item")
            .add_translation("en", "cart_other", "{count} items");

        let mut ctx = RouteContext::new("/dashboard").with_i18n(i18n);

        // Fallback default
        assert_eq!(ctx.locale(), "en");
        assert_eq!(
            ctx.t_with("welcome", &[("name", "Alice")]),
            "Welcome, Alice!"
        );
        assert_eq!(ctx.t_plural(0, "cart", &[]), "No items");
        assert_eq!(ctx.t_plural(1, "cart", &[]), "1 item");
        assert_eq!(ctx.t_plural(5, "cart", &[]), "5 items");

        // French header
        ctx.headers.insert(
            "accept-language",
            "fr-FR,fr;q=0.9,en;q=0.8".parse().unwrap(),
        );
        assert_eq!(ctx.locale(), "fr");
        assert_eq!(ctx.t_with("welcome", &[("name", "Bob")]), "Bienvenue, Bob!");
    }
}
