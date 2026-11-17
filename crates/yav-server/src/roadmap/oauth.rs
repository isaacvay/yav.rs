//! OAuth2 / OIDC authorization-code client built on `oauth2` 5.0.
//!
//! Compiled only with the `oauth` Cargo feature. Tokens are obtained through
//! the real oauth2 reqwest async client — no value is ever synthesized.
#![cfg(feature = "oauth")]

use oauth2::{
    basic::BasicClient, AuthUrl, AuthorizationCode, ClientId, ClientSecret, CsrfToken,
    EndpointNotSet, EndpointSet, PkceCodeChallenge, PkceCodeVerifier, RedirectUrl, Scope,
    TokenResponse, TokenUrl,
};
use std::sync::Mutex;

/// A `BasicClient` with both the authorization and token endpoints configured.
type ConfiguredBasicClient =
    BasicClient<EndpointSet, EndpointNotSet, EndpointNotSet, EndpointNotSet, EndpointSet>;

/// An OAuth2 authorization-code client with CSRF `state` and PKCE `S256`.
///
/// The PKCE verifier produced by [`authorization_url`](Self::authorization_url)
/// is kept inside the client and consumed by
/// [`exchange_code`](Self::exchange_code). This means one pending authorization
/// flow is tracked at a time per client instance; create a fresh client for
/// concurrent independent flows.
pub struct OAuthTokenClient {
    client: ConfiguredBasicClient,
    pkce_verifier: Mutex<Option<PkceCodeVerifier>>,
}

impl OAuthTokenClient {
    /// Build a client from the standard OAuth2 endpoint configuration.
    ///
    /// Returns `Err` when any URL is rejected by the `oauth2`/`url` parser.
    pub fn new(
        client_id: &str,
        client_secret: &str,
        auth_url: &str,
        token_url: &str,
        redirect_url: &str,
    ) -> Result<Self, String> {
        let client = BasicClient::new(ClientId::new(client_id.to_string()))
            .set_client_secret(ClientSecret::new(client_secret.to_string()))
            .set_auth_uri(AuthUrl::new(auth_url.to_string()).map_err(|err| err.to_string())?)
            .set_token_uri(TokenUrl::new(token_url.to_string()).map_err(|err| err.to_string())?)
            .set_redirect_uri(
                RedirectUrl::new(redirect_url.to_string()).map_err(|err| err.to_string())?,
            );

        Ok(Self {
            client,
            pkce_verifier: Mutex::new(None),
        })
    }

    /// Build the authorization redirect URL.
    ///
    /// The caller-supplied `state` is used verbatim as the CSRF token (do not
    /// reuse it across sessions). A fresh PKCE `S256` challenge is attached and
    /// the matching verifier is stored for the subsequent token exchange.
    pub fn authorization_url(&self, state: &str, scopes: &[&str]) -> String {
        let state = state.to_string();
        let mut request = self.client.authorize_url(move || CsrfToken::new(state));

        for scope in scopes {
            request = request.add_scope(Scope::new((*scope).to_string()));
        }

        let (challenge, verifier) = PkceCodeChallenge::new_random_sha256();
        if let Ok(mut stored) = self.pkce_verifier.lock() {
            *stored = Some(verifier);
        }

        request.set_pkce_challenge(challenge).url().0.to_string()
    }

    /// Exchange an authorization code for an access token via oauth2's async
    /// reqwest client.
    ///
    /// The stored PKCE verifier (if any) is attached and consumed. Redirects
    /// are disabled on the HTTP client to avoid SSRF surprises. All failures —
    /// HTTP, parsing, or provider `error` responses — are returned as `Err`.
    pub async fn exchange_code(&self, code: &str) -> Result<String, String> {
        let http_client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(|err| err.to_string())?;

        let verifier = self
            .pkce_verifier
            .lock()
            .ok()
            .and_then(|mut stored| stored.take());

        let mut request = self
            .client
            .exchange_code(AuthorizationCode::new(code.to_string()));
        if let Some(verifier) = verifier {
            request = request.set_pkce_verifier(verifier);
        }

        let token = request
            .request_async(&http_client)
            .await
            .map_err(|err| err.to_string())?;

        Ok(token.access_token().secret().clone())
    }

    /// Fetch OIDC `userinfo` (or any bearer-protected JSON resource).
    ///
    /// This is a plain `reqwest` GET with an `Authorization: Bearer` header;
    /// non-2xx responses and invalid JSON are surfaced as `Err`.
    pub async fn userinfo(
        &self,
        access_token: &str,
        url: &str,
    ) -> Result<serde_json::Value, String> {
        let client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(|err| err.to_string())?;

        let response = client
            .get(url)
            .bearer_auth(access_token)
            .send()
            .await
            .map_err(|err| err.to_string())?
            .error_for_status()
            .map_err(|err| err.to_string())?;

        let bytes = response.bytes().await.map_err(|err| err.to_string())?;
        serde_json::from_slice(&bytes).map_err(|err| err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn client() -> OAuthTokenClient {
        OAuthTokenClient::new(
            "client-123",
            "secret",
            "https://provider.example.com/authorize",
            "https://provider.example.com/token",
            "https://app.example.com/callback",
        )
        .expect("all URLs are valid")
    }

    #[test]
    fn authorization_url_carries_state_scopes_and_pkce() {
        let client = client();
        let url = client.authorization_url("csrf-state-42", &["openid", "email"]);

        assert!(url.contains("state=csrf-state-42"));
        assert!(url.contains("scope=openid%20email") || url.contains("scope=openid+email"));
        assert!(url.contains("code_challenge_method=S256"));
        assert!(url.contains("code_challenge="));
    }

    #[test]
    fn invalid_endpoint_is_rejected() {
        assert!(
            OAuthTokenClient::new("id", "secret", "not a url", "not a url", "not a url").is_err()
        );
    }
}
