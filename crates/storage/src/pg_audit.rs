//! PostgreSQL-backed structured audit log.

use moqentra_auth::{AuditEvent, AuditLog};
use moqentra_types::{Error, TenantId};
use sqlx::PgPool;

/// Writes audit events to the `audit_logs` table under row-level security.
#[derive(Debug, Clone)]
pub struct PgAuditLog {
    pool: PgPool,
}

impl PgAuditLog {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    fn details(event: &AuditEvent) -> serde_json::Value {
        // Redact sensitive fields from persistence; keep a stable shape.
        serde_json::json!({
            "reason": event.reason,
            "resource": event.resource,
        })
    }

    /// Write an audit event using an existing connection/transaction.
    pub async fn record_with_conn(
        &self,
        conn: &mut sqlx::PgConnection,
        event: AuditEvent,
    ) -> Result<(), Error> {
        let tenant_id = TenantId::try_from(event.tenant_id.to_string().as_str())?;
        let _ = sqlx::query("SELECT set_config('app.current_tenant', $1, true)")
            .bind(tenant_id.to_string())
            .execute(&mut *conn)
            .await
            .map_err(|e| Error::internal(format!("failed to set tenant context: {e}")))?;
        let _ = sqlx::query(
            "INSERT INTO audit_logs \
             (event_id, category, actor_type, actor_id, tenant_id, project_id, \
              action, resource_type, resource_id, outcome, reason, details, correlation_id, occurred_at) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)",
        )
        .bind(&event.event_id)
        .bind(event.category.as_str())
        .bind(event.actor.actor_type())
        .bind(event.actor.actor_id())
        .bind(tenant_id.as_uuid())
        .bind(event.project_id.map(|p| p.as_uuid()))
        .bind(&event.action)
        .bind(resource_type(&event.resource))
        .bind(resource_id(&event.resource))
        .bind(outcome_str(event.outcome))
        .bind(event.reason.as_deref().unwrap_or("") as &str)
        .bind(PgAuditLog::details(&event))
        .bind(&event.correlation_id)
        .bind(event.occurred_at.as_offset())
        .execute(&mut *conn)
        .await
        .map_err(|e| Error::internal(format!("failed to write audit log: {e}")))?;
        Ok(())
    }
}

#[async_trait::async_trait]
impl AuditLog for PgAuditLog {
    async fn record(&self, event: AuditEvent) -> Result<(), Error> {
        let mut conn = self
            .pool
            .acquire()
            .await
            .map_err(|e| Error::internal(format!("audit acquire failed: {e}")))?;
        self.record_with_conn(&mut *conn, event).await
    }
}

fn resource_type(resource: &str) -> String {
    resource.split('/').next().unwrap_or("unknown").to_string()
}

fn resource_id(resource: &str) -> String {
    resource.split('/').nth(1).unwrap_or(resource).to_string()
}

fn outcome_str(outcome: moqentra_auth::AuditOutcome) -> &'static str {
    match outcome {
        moqentra_auth::AuditOutcome::Success => "success",
        moqentra_auth::AuditOutcome::Denied => "denied",
        moqentra_auth::AuditOutcome::Failure => "failure",
    }
}
