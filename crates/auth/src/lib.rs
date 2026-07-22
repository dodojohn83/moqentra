//! Moqentra authentication, authorization, and audit crate.

#![allow(missing_docs)]

pub mod audit;
pub mod jwt;
pub mod oidc;
pub mod rbac;
pub mod role_store;
pub mod secrets;

pub use audit::{AuditCategory, AuditEvent, AuditLog, AuditOutcome, InMemoryAuditLog};
pub use jwt::{
    map_roles, AuthSession, CompositeTokenValidator, HmacValidator, ServiceAccountValidator,
    TokenClaims, TokenValidator,
};
pub use oidc::{JwkSetValidator, OidcConfig};
pub use rbac::{
    Action, AuthorizationError, Authorizer, Decision, Permission, Resource, Role, Scope,
};
pub use role_store::RoleStore;
pub use secrets::{Certificate, SecretProvider, SecretRedactor, SecurityLimits, SignedArtifact};
