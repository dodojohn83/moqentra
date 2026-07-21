//! Moqentra authentication, authorization, and audit crate.

#![allow(missing_docs)]

pub mod audit;
pub mod jwt;
pub mod rbac;
pub mod secrets;

pub use audit::{AuditCategory, AuditEvent, AuditLog, AuditOutcome, InMemoryAuditLog};
pub use jwt::{
    map_roles, AuthSession, CompositeTokenValidator, HmacValidator, ServiceAccountValidator,
    TokenClaims, TokenValidator,
};
pub use rbac::{
    Action, AuthorizationError, Authorizer, Decision, Permission, Resource, Role, Scope,
};
pub use secrets::{Certificate, SecretProvider, SecretRedactor, SecurityLimits, SignedArtifact};
