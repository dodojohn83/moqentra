//! PostgreSQL connection pool with tenant-scoped RLS.

use moqentra_types::{Error, TenantId};
use sqlx::PgPool;

/// A PostgreSQL connection pool.
#[derive(Debug, Clone)]
pub struct ConnectionPool {
    inner: PgPool,
}

impl ConnectionPool {
    pub fn new(pool: PgPool) -> Self {
        Self { inner: pool }
    }

    pub async fn acquire(&self) -> Result<ScopedConnection, Error> {
        let conn = self
            .inner
            .acquire()
            .await
            .map_err(|e| Error::unavailable(format!("database unavailable: {}", e)))?;
        Ok(ScopedConnection { conn })
    }

    /// Runs the sqlx migrator against the pool.
    pub async fn migrate(&self) -> Result<(), Error> {
        sqlx::migrate!("./migrations")
            .run(&self.inner)
            .await
            .map_err(|e| Error::internal(format!("migration failed: {}", e)))
    }
}

/// A connection with an optional tenant context.
pub struct ScopedConnection {
    conn: sqlx::pool::PoolConnection<sqlx::Postgres>,
}

impl ScopedConnection {
    /// Sets the PostgreSQL `app.current_tenant` variable for RLS policies.
    /// The variable is session-local and cleared when the session ends.
    pub async fn set_tenant(&mut self, tenant_id: TenantId) -> Result<(), Error> {
        let value = tenant_id.to_string();
        sqlx::query("SET LOCAL app.current_tenant = $1")
            .bind(&value)
            .execute(&mut *self.conn)
            .await
            .map_err(|e| Error::internal(format!("failed to set tenant context: {}", e)))?;
        Ok(())
    }

    /// Clears the tenant context before the connection is returned to the pool.
    pub async fn clear_tenant(&mut self) -> Result<(), Error> {
        sqlx::query("SET LOCAL app.current_tenant = ''")
            .execute(&mut *self.conn)
            .await
            .map_err(|e| Error::internal(format!("failed to clear tenant context: {}", e)))?;
        Ok(())
    }

    /// Returns the underlying connection for query execution.
    pub fn connection(&mut self) -> &mut sqlx::PgConnection {
        &mut self.conn
    }
}
