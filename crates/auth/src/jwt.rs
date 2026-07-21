//! JWT/OIDC validation and principal extraction.

use jsonwebtoken::{decode, decode_header, Algorithm, DecodingKey, TokenData, Validation};
use moqentra_types::{Error, Principal, UserId};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Claims expected in a Moqentra-issued or upstream OIDC token.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenClaims {
    pub sub: String,
    pub iss: String,
    pub aud: String,
    pub exp: usize,
    #[serde(default)]
    pub iat: usize,
    #[serde(default)]
    pub roles: Vec<String>,
    #[serde(default)]
    pub tenant_ids: Vec<String>,
}

/// Validator for bearer tokens.
pub trait TokenValidator: Send + Sync {
    /// Validates a token and returns the authenticated principal.
    fn validate(&self, token: &str) -> Result<Principal, Error>;
}

/// HMAC-based validator for local development and tests.
#[derive(Clone)]
pub struct HmacValidator {
    secret: String,
    issuer: String,
    audience: String,
}

impl HmacValidator {
    pub fn new(
        secret: impl Into<String>,
        issuer: impl Into<String>,
        audience: impl Into<String>,
    ) -> Self {
        Self {
            secret: secret.into(),
            issuer: issuer.into(),
            audience: audience.into(),
        }
    }
}

impl HmacValidator {
    /// Validate token and return both principal and full claims (for role seeding).
    pub fn validate_claims(&self, token: &str) -> Result<(Principal, TokenClaims), Error> {
        if self.secret.is_empty() {
            return Err(Error::unauthenticated("HMAC secret must not be empty"));
        }
        let header = decode_header(token)
            .map_err(|e| Error::unauthenticated(format!("invalid token header: {}", e)))?;
        let algorithm = header.alg;
        if !matches!(
            algorithm,
            Algorithm::HS256 | Algorithm::HS384 | Algorithm::HS512
        ) {
            return Err(Error::unauthenticated("only HMAC tokens are accepted"));
        }
        let key = DecodingKey::from_secret(self.secret.as_bytes());

        let mut validation = Validation::new(algorithm);
        validation.set_issuer(&[&self.issuer]);
        validation.set_audience(&[&self.audience]);
        validation.validate_exp = true;
        validation.required_spec_claims = {
            let mut s = HashSet::new();
            s.insert("exp".to_string());
            s.insert("iss".to_string());
            s.insert("aud".to_string());
            s
        };

        let TokenData { claims, .. } = decode::<TokenClaims>(token, &key, &validation)
            .map_err(|e| Error::unauthenticated(format!("token validation failed: {}", e)))?;

        let user_id = UserId::try_from(claims.sub.as_str())?;
        Ok((Principal::user(user_id), claims))
    }
}

impl TokenValidator for HmacValidator {
    fn validate(&self, token: &str) -> Result<Principal, Error> {
        self.validate_claims(token).map(|(p, _)| p)
    }
}

/// Service-account validator using a shared secret (client-credentials style).
#[derive(Clone)]
pub struct ServiceAccountValidator {
    credentials: std::sync::Arc<std::collections::HashMap<String, String>>,
}

impl ServiceAccountValidator {
    pub fn new(credentials: std::collections::HashMap<String, String>) -> Self {
        Self {
            credentials: std::sync::Arc::new(credentials),
        }
    }
}

impl TokenValidator for ServiceAccountValidator {
    fn validate(&self, token: &str) -> Result<Principal, Error> {
        if token.is_empty() {
            return Err(Error::unauthenticated(
                "service account token must not be empty",
            ));
        }
        if let Some(name) = self.credentials.get(token) {
            Ok(Principal::service(name.clone()))
        } else {
            Err(Error::unauthenticated("unknown service account"))
        }
    }
}

/// Maps claim roles to platform `Role`s.
pub fn map_roles(claim_roles: &[String]) -> Vec<crate::rbac::Role> {
    claim_roles.iter().filter_map(|s| s.parse::<crate::rbac::Role>().ok()).collect()
}

/// Tries HMAC JWT first, then service-account token lookup.
#[derive(Clone)]
pub struct CompositeTokenValidator {
    hmac: Option<HmacValidator>,
    service: Option<ServiceAccountValidator>,
}

impl CompositeTokenValidator {
    pub fn new(hmac: Option<HmacValidator>, service: Option<ServiceAccountValidator>) -> Self {
        Self { hmac, service }
    }

    pub fn is_configured(&self) -> bool {
        self.hmac.is_some() || self.service.is_some()
    }
}

/// Authenticated session with optional role claims from JWT.
#[derive(Debug, Clone)]
pub struct AuthSession {
    pub principal: Principal,
    pub roles: Vec<crate::rbac::Role>,
    pub tenant_ids: Vec<String>,
}

impl CompositeTokenValidator {
    /// Validate and extract role claims when available (HMAC path).
    pub fn validate_session(&self, token: &str) -> Result<AuthSession, Error> {
        if token.trim().is_empty() {
            return Err(Error::unauthenticated("token must not be empty"));
        }
        let mut last_err = Error::unauthenticated("no token validator configured");
        if let Some(hmac) = &self.hmac {
            match hmac.validate_claims(token) {
                Ok((principal, claims)) => {
                    return Ok(AuthSession {
                        principal,
                        roles: map_roles(&claims.roles),
                        tenant_ids: claims.tenant_ids,
                    });
                }
                Err(e) => last_err = e,
            }
        }
        if let Some(service) = &self.service {
            match service.validate(token) {
                Ok(principal) => {
                    return Ok(AuthSession {
                        principal,
                        // Service accounts rely on Authorizer.assign_service_role.
                        roles: Vec::new(),
                        tenant_ids: Vec::new(),
                    });
                }
                Err(e) => last_err = e,
            }
        }
        Err(last_err)
    }
}

impl TokenValidator for CompositeTokenValidator {
    fn validate(&self, token: &str) -> Result<Principal, Error> {
        self.validate_session(token).map(|s| s.principal)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
    use moqentra_types::RandomIdGenerator;

    #[test]
    fn hmac_validator_roundtrip() {
        let gen = RandomIdGenerator;
        let user_id = UserId::new_v7(&gen);
        let claims = TokenClaims {
            sub: user_id.to_string(),
            iss: "https://moqentra.test".to_string(),
            aud: "moqentra".to_string(),
            exp: usize::MAX,
            iat: 0,
            roles: vec!["viewer".to_string()],
            tenant_ids: vec![],
        };

        let token = encode(
            &Header::new(Algorithm::HS256),
            &claims,
            &EncodingKey::from_secret(b"secret"),
        )
        .unwrap();
        let validator = HmacValidator::new("secret", "https://moqentra.test", "moqentra");
        let principal = validator.validate(&token).unwrap();
        assert!(matches!(principal, Principal::User { id } if id == user_id));
    }

    #[test]
    fn expired_token_rejected() {
        let gen = RandomIdGenerator;
        let user_id = UserId::new_v7(&gen);
        let claims = TokenClaims {
            sub: user_id.to_string(),
            iss: "https://moqentra.test".to_string(),
            aud: "moqentra".to_string(),
            exp: 0,
            iat: 0,
            roles: vec![],
            tenant_ids: vec![],
        };

        let token = encode(
            &Header::new(Algorithm::HS256),
            &claims,
            &EncodingKey::from_secret(b"secret"),
        )
        .unwrap();
        let validator = HmacValidator::new("secret", "https://moqentra.test", "moqentra");
        assert!(validator.validate(&token).is_err());
    }

    #[test]
    fn wrong_issuer_rejected() {
        let gen = RandomIdGenerator;
        let user_id = UserId::new_v7(&gen);
        let claims = TokenClaims {
            sub: user_id.to_string(),
            iss: "https://other.test".to_string(),
            aud: "moqentra".to_string(),
            exp: usize::MAX,
            iat: 0,
            roles: vec![],
            tenant_ids: vec![],
        };

        let token = encode(
            &Header::new(Algorithm::HS256),
            &claims,
            &EncodingKey::from_secret(b"secret"),
        )
        .unwrap();
        let validator = HmacValidator::new("secret", "https://moqentra.test", "moqentra");
        assert!(validator.validate(&token).is_err());
    }
}
