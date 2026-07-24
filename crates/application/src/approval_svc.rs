//! Approval request application service.

#![allow(missing_docs)]

use async_trait::async_trait;
use moqentra_domain::approval::{ApprovalKind, ApprovalRequest, ApprovalState};
use moqentra_types::{
    ApprovalRequestId, Error, Page, PageRequest, ProjectId, RequestContext, TenantId, UserId,
    UtcTimestamp,
};
use std::collections::{BTreeMap, HashMap};
use std::sync::{Arc, Mutex};

use crate::ports::{ApprovalRepository, ResourceListFilter, Versioned};

fn revision_from_u64(value: u64) -> moqentra_types::Revision {
    (0..value).fold(moqentra_types::Revision::initial(), |r, _| r.next())
}

/// Service for submitting and deciding approval requests.
#[derive(Debug, Clone)]
pub struct ApprovalService<R: ApprovalRepository> {
    repo: R,
}

impl<R: ApprovalRepository> ApprovalService<R> {
    pub fn new(repo: R) -> Self {
        Self { repo }
    }

    /// Submit a new approval request with the resource snapshot at submission time.
    #[allow(clippy::too_many_arguments)]
    pub async fn request(
        &self,
        ctx: &RequestContext,
        id: ApprovalRequestId,
        tenant_id: TenantId,
        project_id: ProjectId,
        requester_id: UserId,
        kind: ApprovalKind,
        reason: impl Into<String>,
        requested_limits: BTreeMap<String, u64>,
        policy_revision: u64,
        expires_at: UtcTimestamp,
    ) -> Result<Versioned<ApprovalRequest>, Error> {
        let request = ApprovalRequest::new(
            id,
            tenant_id,
            project_id,
            requester_id,
            kind,
            reason.into(),
            requested_limits,
            policy_revision,
            expires_at,
        )?;
        self.repo.create_request(ctx, request).await
    }

    /// Approve a pending request. The approver cannot be the requester.
    pub async fn approve(
        &self,
        ctx: &RequestContext,
        id: ApprovalRequestId,
        approver_id: UserId,
        reason: impl Into<String>,
        valid_until: Option<UtcTimestamp>,
        limit_values: BTreeMap<String, u64>,
    ) -> Result<Versioned<ApprovalRequest>, Error> {
        self.decide(
            ctx,
            id,
            approver_id,
            ApprovalState::Approved,
            reason,
            valid_until,
            limit_values,
        )
        .await
    }

    /// Reject a pending request. The approver cannot be the requester.
    pub async fn reject(
        &self,
        ctx: &RequestContext,
        id: ApprovalRequestId,
        approver_id: UserId,
        reason: impl Into<String>,
    ) -> Result<Versioned<ApprovalRequest>, Error> {
        self.decide(
            ctx,
            id,
            approver_id,
            ApprovalState::Rejected,
            reason,
            None,
            BTreeMap::new(),
        )
        .await
    }

    #[allow(clippy::too_many_arguments)]
    async fn decide(
        &self,
        ctx: &RequestContext,
        id: ApprovalRequestId,
        approver_id: UserId,
        outcome: ApprovalState,
        reason: impl Into<String>,
        valid_until: Option<UtcTimestamp>,
        limit_values: BTreeMap<String, u64>,
    ) -> Result<Versioned<ApprovalRequest>, Error> {
        let current = self.repo.get_request(ctx, id).await?;
        let mut request = current.entity;
        let now = UtcTimestamp::now();
        if request.expires_at < now {
            request.decide(
                approver_id,
                ApprovalState::Expired,
                "request expired".to_string(),
                None,
                BTreeMap::new(),
                current.revision.as_u64().saturating_add(1),
            )?;
            return Err(Error::conflict("approval request expired"));
        }
        request.decide(
            approver_id,
            outcome,
            reason.into(),
            valid_until,
            limit_values,
            current.revision.as_u64().saturating_add(1),
        )?;
        self.repo.update_request(ctx, id, request, current.revision).await
    }
}

/// In-memory approval repository for unit tests and single-process mode.
#[derive(Debug, Default, Clone)]
pub struct InMemoryApprovalRegistry {
    requests: Arc<Mutex<HashMap<ApprovalRequestId, ApprovalRequest>>>,
    revisions: Arc<Mutex<HashMap<ApprovalRequestId, u64>>>,
}

impl InMemoryApprovalRegistry {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl ApprovalRepository for InMemoryApprovalRegistry {
    async fn create_request(
        &self,
        ctx: &RequestContext,
        request: ApprovalRequest,
    ) -> Result<Versioned<ApprovalRequest>, Error> {
        if request.tenant_id != ctx.tenant_id {
            return Err(Error::permission_denied("approval request tenant mismatch"));
        }
        if ctx.project_id.is_some_and(|p| p != request.project_id) {
            return Err(Error::permission_denied(
                "approval request project mismatch",
            ));
        }
        let mut reg = self.requests.lock().map_err(|e| Error::internal(e.to_string()))?;
        if reg.contains_key(&request.id) {
            return Err(Error::conflict("approval request already exists"));
        }
        reg.insert(request.id, request.clone());
        self.revisions
            .lock()
            .map_err(|e| Error::internal(e.to_string()))?
            .insert(request.id, 1);
        Ok(Versioned::new(request, revision_from_u64(1)))
    }

    async fn get_request(
        &self,
        ctx: &RequestContext,
        id: ApprovalRequestId,
    ) -> Result<Versioned<ApprovalRequest>, Error> {
        let reg = self.requests.lock().map_err(|e| Error::internal(e.to_string()))?;
        let req = reg.get(&id).ok_or_else(|| Error::not_found("approval request"))?;
        if req.tenant_id != ctx.tenant_id {
            return Err(Error::not_found("approval request"));
        }
        if ctx.project_id.is_some_and(|p| p != req.project_id) {
            return Err(Error::not_found("approval request"));
        }
        let rev = self
            .revisions
            .lock()
            .map_err(|e| Error::internal(e.to_string()))?
            .get(&id)
            .copied()
            .unwrap_or(1);
        Ok(Versioned::new(req.clone(), revision_from_u64(rev)))
    }

    async fn list_requests(
        &self,
        ctx: &RequestContext,
        _filter: ResourceListFilter,
        _page: PageRequest,
    ) -> Result<Page<Versioned<ApprovalRequest>>, Error> {
        let reg = self.requests.lock().map_err(|e| Error::internal(e.to_string()))?;
        let revs = self.revisions.lock().map_err(|e| Error::internal(e.to_string()))?;
        let items: Vec<_> = reg
            .values()
            .filter(|r| r.tenant_id == ctx.tenant_id)
            .filter(|r| ctx.project_id.is_none_or(|p| p == r.project_id))
            .map(|r| {
                let rev = revs.get(&r.id).copied().unwrap_or(1);
                Versioned::new(r.clone(), revision_from_u64(rev))
            })
            .collect();
        let total = u64::try_from(items.len()).unwrap_or(u64::MAX);
        Ok(Page::new(items, total, _page))
    }

    async fn update_request(
        &self,
        ctx: &RequestContext,
        id: ApprovalRequestId,
        request: ApprovalRequest,
        expected: moqentra_types::Revision,
    ) -> Result<Versioned<ApprovalRequest>, Error> {
        if request.tenant_id != ctx.tenant_id {
            return Err(Error::permission_denied("approval request tenant mismatch"));
        }
        if ctx.project_id.is_some_and(|p| p != request.project_id) {
            return Err(Error::permission_denied(
                "approval request project mismatch",
            ));
        }
        let mut reg = self.requests.lock().map_err(|e| Error::internal(e.to_string()))?;
        let mut revs = self.revisions.lock().map_err(|e| Error::internal(e.to_string()))?;
        let existing = reg.get(&id).ok_or_else(|| Error::not_found("approval request"))?;
        if existing.tenant_id != ctx.tenant_id {
            return Err(Error::not_found("approval request"));
        }
        let current = revs.get(&id).copied().unwrap_or(1);
        if expected.as_u64() != current {
            return Err(Error::conflict("approval request revision mismatch"));
        }
        let next = current + 1;
        revs.insert(id, next);
        reg.insert(id, request.clone());
        Ok(Versioned::new(request, revision_from_u64(next)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use moqentra_types::RandomIdGenerator;

    #[tokio::test]
    async fn requester_cannot_self_approve() {
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
        let repo = InMemoryApprovalRegistry::new();
        let svc = ApprovalService::new(repo);
        let requester = UserId::new_v7(&g);
        let id = ApprovalRequestId::new_v7(&g);

        svc.request(
            &ctx,
            id,
            tenant,
            project,
            requester,
            ApprovalKind::QuotaOverride,
            "need more GPUs",
            BTreeMap::new(),
            1,
            expires_at,
        )
        .await
        .unwrap();

        assert!(svc.approve(&ctx, id, requester, "ok", None, BTreeMap::new()).await.is_err());
    }

    #[tokio::test]
    async fn approve_then_no_reapprove() {
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
        let repo = InMemoryApprovalRegistry::new();
        let svc = ApprovalService::new(repo);
        let requester = UserId::new_v7(&g);
        let approver = UserId::new_v7(&g);
        let id = ApprovalRequestId::new_v7(&g);

        svc.request(
            &ctx,
            id,
            tenant,
            project,
            requester,
            ApprovalKind::ModelPromotion,
            "promote",
            BTreeMap::new(),
            1,
            expires_at,
        )
        .await
        .unwrap();

        svc.approve(&ctx, id, approver, "approved", None, BTreeMap::new())
            .await
            .unwrap();

        assert!(svc.reject(&ctx, id, approver, "no").await.is_err());
    }
}
