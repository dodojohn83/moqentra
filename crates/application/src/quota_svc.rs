//! Quota and usage ledger application service.

#![allow(missing_docs)]

use async_trait::async_trait;
use moqentra_domain::quota::{
    DimensionLimit, QuotaPolicy, QuotaReservation, QuotaScope, ReservationState, ResourceDimension,
};
use moqentra_types::{
    Error, Page, PageRequest, ProjectId, QuotaPolicyId, QuotaReservationId, RequestContext,
    TenantId, UtcTimestamp,
};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::ports::{QuotaRepository, ResourceListFilter, Versioned};

fn revision_from_u64(value: u64) -> moqentra_types::Revision {
    (0..value).fold(moqentra_types::Revision::initial(), |r, _| r.next())
}

/// Service enforcing quota admission and recording usage ledger entries.
#[derive(Debug, Clone)]
pub struct QuotaService<R: QuotaRepository> {
    repo: R,
}

impl<R: QuotaRepository> QuotaService<R> {
    pub fn new(repo: R) -> Self {
        Self { repo }
    }

    /// Create a quota policy. A tenant may have one active tenant-level policy;
    /// project-level policies override for that project.
    pub async fn create_policy(
        &self,
        ctx: &RequestContext,
        id: QuotaPolicyId,
        tenant_id: TenantId,
        project_id: Option<ProjectId>,
        scope: QuotaScope,
        limits: Vec<DimensionLimit>,
    ) -> Result<Versioned<QuotaPolicy>, Error> {
        let policy = QuotaPolicy::new(
            id,
            tenant_id,
            project_id,
            scope,
            1,
            UtcTimestamp::now(),
            limits,
        )?;
        self.repo.create_policy(ctx, policy).await
    }

    /// Admit a workload by atomically checking the effective policy and active
    /// reservations. If room remains, creates an `Active` reservation.
    #[allow(clippy::too_many_arguments)]
    pub async fn admit(
        &self,
        ctx: &RequestContext,
        reservation_id: QuotaReservationId,
        tenant_id: TenantId,
        project_id: ProjectId,
        policy_id: QuotaPolicyId,
        resource_type: impl Into<String>,
        dimension: ResourceDimension,
        requested: u64,
        expires_at: UtcTimestamp,
    ) -> Result<Versioned<QuotaReservation>, Error> {
        if requested == 0 {
            return Err(Error::invalid_argument("requested quota must be > 0"));
        }
        if ctx.tenant_id != tenant_id {
            return Err(Error::permission_denied(
                "tenant mismatch in quota admission",
            ));
        }
        if ctx.project_id.is_some_and(|p| p != project_id) {
            return Err(Error::permission_denied(
                "project mismatch in quota admission",
            ));
        }

        let policy = self.repo.get_policy(ctx, policy_id).await?;
        let active = self.repo.active_reservations(ctx, dimension).await?;

        let effective = self.effective_limit(&policy.entity, tenant_id, project_id, dimension)?;
        let now = UtcTimestamp::now();
        let used: u64 = active
            .iter()
            .filter(|r| {
                r.tenant_id == tenant_id
                    && (policy.entity.project_id.is_none() || r.project_id == project_id)
                    && r.expires_at >= now
                    && matches!(r.state, ReservationState::Active)
            })
            .map(|r| r.requested)
            .fold(0u64, |a, b| a.saturating_add(b));

        if let Some(limit) = effective {
            if used.saturating_add(requested) > limit {
                return Err(Error::conflict(format!(
                    "quota exceeded for {dimension:?}: requested {requested}, used {used}, limit {limit}"
                )));
            }
        }

        let reservation = QuotaReservation::new(
            reservation_id,
            tenant_id,
            project_id,
            policy_id,
            policy.entity.revision,
            resource_type.into(),
            dimension,
            requested,
            expires_at,
        )?;
        self.repo.create_reservation(ctx, reservation).await
    }

    /// Release or consume a reservation.
    pub async fn settle_reservation(
        &self,
        ctx: &RequestContext,
        id: QuotaReservationId,
        state: ReservationState,
    ) -> Result<Versioned<QuotaReservation>, Error> {
        let current = self.repo.get_reservation(ctx, id).await?;
        let mut reservation = current.entity;
        match state {
            ReservationState::Consumed => reservation.state = ReservationState::Consumed,
            ReservationState::Cancelled => reservation.state = ReservationState::Cancelled,
            ReservationState::Expired => reservation.state = ReservationState::Expired,
            _ => return Err(Error::invalid_argument("invalid settle state")),
        }
        self.repo.update_reservation(ctx, id, reservation, current.revision).await
    }

    fn effective_limit(
        &self,
        policy: &QuotaPolicy,
        tenant_id: TenantId,
        project_id: ProjectId,
        dimension: ResourceDimension,
    ) -> Result<Option<u64>, Error> {
        if policy.tenant_id != tenant_id {
            return Err(Error::not_found("quota policy not found"));
        }
        match policy.scope {
            QuotaScope::Tenant => Ok(policy.limit(dimension)),
            QuotaScope::Project => {
                if policy.project_id != Some(project_id) {
                    return Err(Error::permission_denied("project quota policy mismatch"));
                }
                Ok(policy.limit(dimension))
            }
        }
    }
}

/// In-memory quota repository for unit tests and single-process mode.
#[derive(Debug, Default, Clone)]
pub struct InMemoryQuotaRegistry {
    policies: Arc<Mutex<HashMap<QuotaPolicyId, QuotaPolicy>>>,
    reservations: Arc<Mutex<HashMap<QuotaReservationId, QuotaReservation>>>,
    revisions: Arc<Mutex<HashMap<QuotaPolicyId, u64>>>,
    reservation_revisions: Arc<Mutex<HashMap<QuotaReservationId, u64>>>,
}

impl InMemoryQuotaRegistry {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl QuotaRepository for InMemoryQuotaRegistry {
    async fn create_policy(
        &self,
        ctx: &RequestContext,
        policy: QuotaPolicy,
    ) -> Result<Versioned<QuotaPolicy>, Error> {
        if policy.tenant_id != ctx.tenant_id {
            return Err(Error::permission_denied("quota policy tenant mismatch"));
        }
        if let (Some(cp), Some(pp)) = (ctx.project_id, policy.project_id) {
            if cp != pp {
                return Err(Error::permission_denied("quota policy project mismatch"));
            }
        }
        let mut reg = self.policies.lock().map_err(|e| Error::internal(e.to_string()))?;
        if reg.contains_key(&policy.id) {
            return Err(Error::conflict("quota policy already exists"));
        }
        let mut revs = self.revisions.lock().map_err(|e| Error::internal(e.to_string()))?;
        let rev = revs.entry(policy.id).or_insert(1);
        reg.insert(policy.id, policy.clone());
        Ok(Versioned::new(policy, revision_from_u64(*rev)))
    }

    async fn get_policy(
        &self,
        ctx: &RequestContext,
        id: QuotaPolicyId,
    ) -> Result<Versioned<QuotaPolicy>, Error> {
        let reg = self.policies.lock().map_err(|e| Error::internal(e.to_string()))?;
        let policy = reg.get(&id).ok_or_else(|| Error::not_found("quota policy"))?;
        if policy.tenant_id != ctx.tenant_id {
            return Err(Error::not_found("quota policy"));
        }
        if let (Some(cp), Some(pp)) = (ctx.project_id, policy.project_id) {
            if cp != pp {
                return Err(Error::not_found("quota policy"));
            }
        }
        let revs = self.revisions.lock().map_err(|e| Error::internal(e.to_string()))?;
        let rev = revs.get(&id).copied().unwrap_or(1);
        Ok(Versioned::new(policy.clone(), revision_from_u64(rev)))
    }

    async fn list_policies(
        &self,
        ctx: &RequestContext,
        _filter: ResourceListFilter,
        _page: PageRequest,
    ) -> Result<Page<Versioned<QuotaPolicy>>, Error> {
        let reg = self.policies.lock().map_err(|e| Error::internal(e.to_string()))?;
        let revs = self.revisions.lock().map_err(|e| Error::internal(e.to_string()))?;
        let items: Vec<_> = reg
            .values()
            .filter(|p| p.tenant_id == ctx.tenant_id)
            .filter(|p| p.project_id.is_none_or(|pp| ctx.project_id.is_none_or(|cp| cp == pp)))
            .map(|p| {
                let rev = revs.get(&p.id).copied().unwrap_or(1);
                Versioned::new(p.clone(), revision_from_u64(rev))
            })
            .collect();
        let total = u64::try_from(items.len()).unwrap_or(u64::MAX);
        Ok(Page::new(items, total, _page))
    }

    async fn create_reservation(
        &self,
        ctx: &RequestContext,
        reservation: QuotaReservation,
    ) -> Result<Versioned<QuotaReservation>, Error> {
        if reservation.tenant_id != ctx.tenant_id {
            return Err(Error::permission_denied("reservation tenant mismatch"));
        }
        if ctx.project_id.is_some_and(|p| p != reservation.project_id) {
            return Err(Error::permission_denied("reservation project mismatch"));
        }
        let mut reg = self.reservations.lock().map_err(|e| Error::internal(e.to_string()))?;
        if reg.contains_key(&reservation.id) {
            return Err(Error::conflict("reservation already exists"));
        }
        reg.insert(reservation.id, reservation.clone());
        self.reservation_revisions
            .lock()
            .map_err(|e| Error::internal(e.to_string()))?
            .insert(reservation.id, 1);
        Ok(Versioned::new(reservation, revision_from_u64(1)))
    }

    async fn get_reservation(
        &self,
        ctx: &RequestContext,
        id: QuotaReservationId,
    ) -> Result<Versioned<QuotaReservation>, Error> {
        let reg = self.reservations.lock().map_err(|e| Error::internal(e.to_string()))?;
        let r = reg.get(&id).ok_or_else(|| Error::not_found("quota reservation"))?;
        if r.tenant_id != ctx.tenant_id {
            return Err(Error::not_found("quota reservation"));
        }
        if ctx.project_id.is_some_and(|p| p != r.project_id) {
            return Err(Error::not_found("quota reservation"));
        }
        let rev = self
            .reservation_revisions
            .lock()
            .map_err(|e| Error::internal(e.to_string()))?
            .get(&id)
            .copied()
            .unwrap_or(1);
        Ok(Versioned::new(r.clone(), revision_from_u64(rev)))
    }

    async fn active_reservations(
        &self,
        ctx: &RequestContext,
        dimension: ResourceDimension,
    ) -> Result<Vec<QuotaReservation>, Error> {
        let reg = self.reservations.lock().map_err(|e| Error::internal(e.to_string()))?;
        let now = UtcTimestamp::now();
        Ok(reg
            .values()
            .filter(|r| {
                r.tenant_id == ctx.tenant_id
                    && r.resource_dimension == dimension
                    && matches!(r.state, ReservationState::Active)
                    && r.expires_at >= now
            })
            .cloned()
            .collect())
    }

    async fn update_reservation(
        &self,
        ctx: &RequestContext,
        id: QuotaReservationId,
        reservation: QuotaReservation,
        expected: moqentra_types::Revision,
    ) -> Result<Versioned<QuotaReservation>, Error> {
        if reservation.tenant_id != ctx.tenant_id {
            return Err(Error::permission_denied("reservation tenant mismatch"));
        }
        if ctx.project_id.is_some_and(|p| p != reservation.project_id) {
            return Err(Error::permission_denied("reservation project mismatch"));
        }
        let mut reg = self.reservations.lock().map_err(|e| Error::internal(e.to_string()))?;
        let mut revs =
            self.reservation_revisions.lock().map_err(|e| Error::internal(e.to_string()))?;
        let existing = reg.get(&id).ok_or_else(|| Error::not_found("quota reservation"))?;
        if existing.tenant_id != ctx.tenant_id {
            return Err(Error::not_found("quota reservation"));
        }
        let current = revs.get(&id).copied().unwrap_or(1);
        if expected.as_u64() != current {
            return Err(Error::conflict("reservation revision mismatch"));
        }
        let next = current.checked_add(1).ok_or_else(|| Error::internal("revision overflow"))?;
        revs.insert(id, next);
        reg.insert(id, reservation.clone());
        Ok(Versioned::new(reservation, revision_from_u64(next)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use moqentra_types::RandomIdGenerator;

    #[tokio::test]
    async fn admit_respects_limit() {
        let g = RandomIdGenerator;
        let expires_at = UtcTimestamp::now()
            .add_std_duration(std::time::Duration::from_secs(3600))
            .unwrap();
        let tenant = TenantId::new_v7(&g);
        let project = ProjectId::new_v7(&g);
        let ctx = RequestContext {
            tenant_id: tenant,
            project_id: Some(project),
            roles: vec![],
            project_ids: vec![],
            principal: moqentra_types::Principal::Anonymous,
            request_id: "test".to_string(),
            correlation_id: None,
            deadline: None,
        };
        let repo = InMemoryQuotaRegistry::new();
        let svc = QuotaService::new(repo);
        let policy_id = QuotaPolicyId::new_v7(&g);

        svc.create_policy(
            &ctx,
            policy_id,
            tenant,
            None,
            QuotaScope::Tenant,
            vec![DimensionLimit {
                dimension: ResourceDimension::ConcurrentTrainingJobs,
                hard_limit: Some(2),
            }],
        )
        .await
        .unwrap();

        for _ in 0..2 {
            let rid = QuotaReservationId::new_v7(&g);
            svc.admit(
                &ctx,
                rid,
                tenant,
                project,
                policy_id,
                "training",
                ResourceDimension::ConcurrentTrainingJobs,
                1,
                expires_at,
            )
            .await
            .unwrap();
        }

        let rid = QuotaReservationId::new_v7(&g);
        assert!(svc
            .admit(
                &ctx,
                rid,
                tenant,
                project,
                policy_id,
                "training",
                ResourceDimension::ConcurrentTrainingJobs,
                1,
                expires_at,
            )
            .await
            .is_err());
    }

    #[tokio::test]
    async fn settle_releases_capacity() {
        let g = RandomIdGenerator;
        let expires_at = UtcTimestamp::now()
            .add_std_duration(std::time::Duration::from_secs(3600))
            .unwrap();
        let tenant = TenantId::new_v7(&g);
        let project = ProjectId::new_v7(&g);
        let ctx = RequestContext {
            tenant_id: tenant,
            project_id: Some(project),
            roles: vec![],
            project_ids: vec![],
            principal: moqentra_types::Principal::Anonymous,
            request_id: "test".to_string(),
            correlation_id: None,
            deadline: None,
        };
        let repo = InMemoryQuotaRegistry::new();
        let svc = QuotaService::new(repo);
        let policy_id = QuotaPolicyId::new_v7(&g);

        svc.create_policy(
            &ctx,
            policy_id,
            tenant,
            None,
            QuotaScope::Tenant,
            vec![DimensionLimit {
                dimension: ResourceDimension::ConcurrentTrainingJobs,
                hard_limit: Some(1),
            }],
        )
        .await
        .unwrap();

        let rid = QuotaReservationId::new_v7(&g);
        svc.admit(
            &ctx,
            rid,
            tenant,
            project,
            policy_id,
            "training",
            ResourceDimension::ConcurrentTrainingJobs,
            1,
            expires_at,
        )
        .await
        .unwrap();

        svc.settle_reservation(&ctx, rid, ReservationState::Cancelled).await.unwrap();

        let rid2 = QuotaReservationId::new_v7(&g);
        svc.admit(
            &ctx,
            rid2,
            tenant,
            project,
            policy_id,
            "training",
            ResourceDimension::ConcurrentTrainingJobs,
            1,
            expires_at,
        )
        .await
        .unwrap();
    }
}
