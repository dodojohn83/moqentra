//! OIDC discovery and JWKS-backed token validation.
//!
//! Implements discovery from `.well-known/openid-configuration`, RSA JWKS
//! caching with rotation, issuer/audience/nonce validation, clock skew
//! tolerance and a fail-degraded cache fallback.

use crate::jwt::{map_roles, AuthSession, TokenClaims, TokenValidator};
use jsonwebtoken::{decode, decode_header, Algorithm, DecodingKey, TokenData, Validation};
use moqentra_types::{Error, UserId};
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// OIDC configuration used to validate browser-issued ID tokens.
#[derive(Debug, Clone)]
pub struct OidcConfig {
    pub issuer: String,
    pub audience: String,
    pub expected_nonce: Option<String>,
    pub jwks_uri: Option<String>,
    pub cache_ttl: Duration,
    pub clock_skew_seconds: u64,
}

impl OidcConfig {
    pub fn new(issuer: impl Into<String>, audience: impl Into<String>) -> Self {
        Self {
            issuer: issuer.into(),
            audience: audience.into(),
            expected_nonce: None,
            jwks_uri: None,
            cache_ttl: Duration::from_secs(300),
            clock_skew_seconds: 60,
        }
    }

    pub fn with_nonce(mut self, nonce: impl Into<String>) -> Self {
        self.expected_nonce = Some(nonce.into());
        self
    }

    pub fn with_jwks_uri(mut self, uri: impl Into<String>) -> Self {
        self.jwks_uri = Some(uri.into());
        self
    }
}

#[derive(Debug, Deserialize)]
struct OidcDiscovery {
    jwks_uri: String,
}

#[derive(Debug, Deserialize)]
struct Jwks {
    keys: Vec<Jwk>,
}

#[derive(Debug, Deserialize)]
struct Jwk {
    kty: String,
    kid: Option<String>,
    #[serde(default)]
    n: Option<String>,
    #[serde(default)]
    e: Option<String>,
}

#[derive(Clone)]
struct CacheEntry {
    keys: HashMap<String, DecodingKey>,
    expires_at: Instant,
}

/// JWKS-backed token validator with discovery, caching and rotation.
#[derive(Clone)]
pub struct JwkSetValidator {
    config: OidcConfig,
    client: reqwest::Client,
    cache: Arc<RwLock<CacheEntry>>,
    last_failed: Arc<RwLock<Instant>>,
}

impl std::fmt::Debug for JwkSetValidator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("JwkSetValidator").field("config", &self.config).finish()
    }
}

impl JwkSetValidator {
    pub fn new(config: OidcConfig) -> Result<Self, Error> {
        let client = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(5))
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|e| Error::internal(format!("failed to build http client: {e}")))?;
        Ok(Self {
            config,
            client,
            cache: Arc::new(RwLock::new(CacheEntry {
                keys: HashMap::new(),
                expires_at: Instant::now() - Duration::from_secs(1),
            })),
            last_failed: Arc::new(RwLock::new(Instant::now())),
        })
    }

    pub fn is_configured(&self) -> bool {
        !self.config.issuer.is_empty() && !self.config.audience.is_empty()
    }

    async fn discover_jwks_uri(&self) -> Result<String, Error> {
        let issuer = self.config.issuer.trim_end_matches('/');
        let discovery_url = format!("{}/.well-known/openid-configuration", issuer);
        let resp = self
            .client
            .get(&discovery_url)
            .timeout(Duration::from_secs(10))
            .send()
            .await
            .map_err(|e| Error::unauthenticated(format!("OIDC discovery failed: {e}")))?;
        let doc: OidcDiscovery = resp
            .json()
            .await
            .map_err(|e| Error::unauthenticated(format!("OIDC discovery parse failed: {e}")))?;
        if doc.jwks_uri.is_empty() {
            return Err(Error::unauthenticated("OIDC discovery missing jwks_uri"));
        }
        Ok(doc.jwks_uri)
    }

    async fn fetch_jwks(&self, uri: &str) -> Result<HashMap<String, DecodingKey>, Error> {
        let resp = self
            .client
            .get(uri)
            .timeout(Duration::from_secs(10))
            .send()
            .await
            .map_err(|e| Error::unauthenticated(format!("JWKS fetch failed: {e}")))?;
        let jwks: Jwks = resp
            .json()
            .await
            .map_err(|e| Error::unauthenticated(format!("JWKS parse failed: {e}")))?;

        let mut keys = HashMap::new();
        for jwk in jwks.keys {
            if jwk.kty != "RSA" {
                continue;
            }
            if let (Some(n), Some(e), Some(kid)) = (jwk.n, jwk.e, jwk.kid) {
                let key = DecodingKey::from_rsa_components(&n, &e)
                    .map_err(|e| Error::unauthenticated(format!("invalid RSA JWK: {e}")))?;
                keys.insert(kid, key);
            }
        }
        Ok(keys)
    }

    async fn refresh_cache(&self, force: bool) -> Result<(), Error> {
        let now = Instant::now();
        let mut cache = self.cache.write().await;
        if !force && cache.expires_at > now {
            return Ok(());
        }

        let jwks_uri = match self.config.jwks_uri.clone() {
            Some(uri) => uri,
            None => self.discover_jwks_uri().await?,
        };

        let keys = self.fetch_jwks(&jwks_uri).await?;
        if keys.is_empty() {
            return Err(Error::unauthenticated("JWKS contained no usable RSA keys"));
        }
        *cache = CacheEntry {
            keys,
            expires_at: now + self.config.cache_ttl,
        };
        Ok(())
    }

    async fn resolve_key(&self, kid: &str) -> Result<DecodingKey, Error> {
        let cache = self.cache.read().await;
        if let Some(key) = cache.keys.get(kid) {
            return Ok(key.clone());
        }
        drop(cache);

        // Degrade: avoid hammering the identity provider.
        let last_failed = *self.last_failed.read().await;
        if last_failed.elapsed() < Duration::from_secs(5) {
            return Err(Error::unauthenticated(
                "OIDC key not found and failure cooldown active",
            ));
        }

        if let Err(e) = self.refresh_cache(true).await {
            *self.last_failed.write().await = Instant::now();
            return Err(e);
        }

        let cache = self.cache.read().await;
        cache
            .keys
            .get(kid)
            .cloned()
            .ok_or_else(|| Error::unauthenticated("OIDC signing key not found"))
    }

    fn build_validation(&self) -> Validation {
        let mut validation = Validation::new(Algorithm::RS256);
        validation.set_issuer(&[&self.config.issuer]);
        validation.set_audience(&[&self.config.audience]);
        validation.leeway = self.config.clock_skew_seconds;
        validation
    }

    /// Validates an OIDC ID token, returning an authenticated session.
    pub async fn validate_session(&self, token: &str) -> Result<AuthSession, Error> {
        let header = decode_header(token)
            .map_err(|e| Error::unauthenticated(format!("invalid token header: {e}")))?;
        if !matches!(
            header.alg,
            Algorithm::RS256 | Algorithm::RS384 | Algorithm::RS512
        ) {
            return Err(Error::unauthenticated("OIDC tokens must be RSA-signed"));
        }

        let kid = header.kid.ok_or_else(|| Error::unauthenticated("token header missing kid"))?;

        // Background refresh on cache miss/refresh to avoid synchronous I/O.
        let key = self.resolve_key(&kid).await?;
        let validation = self.build_validation();

        let TokenData { claims, .. } = decode::<TokenClaims>(token, &key, &validation)
            .map_err(|e| Error::unauthenticated(format!("token validation failed: {e}")))?;

        if let Some(expected) = &self.config.expected_nonce {
            let actual = claims.nonce.as_deref().unwrap_or_default();
            if actual != expected.as_str() {
                return Err(Error::unauthenticated("token nonce mismatch"));
            }
        }

        let user_id = UserId::try_from(claims.sub.as_str())?;
        Ok(AuthSession {
            principal: moqentra_types::Principal::user(user_id),
            roles: map_roles(&claims.roles),
            tenant_ids: claims.tenant_ids,
        })
    }
}

#[async_trait::async_trait]
impl TokenValidator for JwkSetValidator {
    async fn validate(&self, token: &str) -> Result<moqentra_types::Principal, Error> {
        self.validate_session(token).await.map(|s| s.principal)
    }
}
