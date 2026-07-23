//! Approval request and decision domain model.

use moqentra_types::{ApprovalRequestId, ProjectId, TenantId, UserId, UtcTimestamp};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// What the approval is for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalKind {
    QuotaOverride,
    ModelPromotion,
    ProductionDeployment,
}

/// Current state of an approval request.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalState {
    Pending,
    Approved,
    Rejected,
    Cancelled,
    Expired,
}

/// An approval request captures the resource/policy snapshot at submission time.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApprovalRequest {
    pub id: ApprovalRequestId,
    pub tenant_id: TenantId,
    pub project_id: ProjectId,
    pub requester_id: UserId,
    pub kind: ApprovalKind,
    pub reason: String,
    pub requested_limits: BTreeMap<String, u64>,
    pub policy_revision: u64,
    pub state: ApprovalState,
    pub expires_at: UtcTimestamp,
    pub created_at: UtcTimestamp,
    pub decided_at: Option<UtcTimestamp>,
    pub decision: Option<ApprovalDecision>,
}

/// Immutable approval decision.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApprovalDecision {
    pub approver_id: UserId,
    pub outcome: ApprovalState,
    pub reason: String,
    pub valid_until: Option<UtcTimestamp>,
    pub limit_values: BTreeMap<String, u64>,
    pub decision_revision: u64,
    pub created_at: UtcTimestamp,
}

impl ApprovalRequest {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: ApprovalRequestId,
        tenant_id: TenantId,
        project_id: ProjectId,
        requester_id: UserId,
        kind: ApprovalKind,
        reason: String,
        requested_limits: BTreeMap<String, u64>,
        policy_revision: u64,
        expires_at: UtcTimestamp,
    ) -> Result<Self, moqentra_types::Error> {
        if reason.trim().is_empty() {
            return Err(moqentra_types::Error::invalid_argument(
                "reason is required",
            ));
        }
        if policy_revision == 0 {
            return Err(moqentra_types::Error::invalid_argument(
                "policy revision must be > 0",
            ));
        }
        Ok(Self {
            id,
            tenant_id,
            project_id,
            requester_id,
            kind,
            reason,
            requested_limits,
            policy_revision,
            state: ApprovalState::Pending,
            expires_at,
            created_at: UtcTimestamp::now(),
            decided_at: None,
            decision: None,
        })
    }

    pub fn decide(
        &mut self,
        approver_id: UserId,
        outcome: ApprovalState,
        reason: String,
        valid_until: Option<UtcTimestamp>,
        limit_values: BTreeMap<String, u64>,
        decision_revision: u64,
    ) -> Result<(), moqentra_types::Error> {
        if self.requester_id == approver_id {
            return Err(moqentra_types::Error::permission_denied(
                "requester cannot approve their own request",
            ));
        }
        if !matches!(self.state, ApprovalState::Pending) {
            return Err(moqentra_types::Error::conflict("request is not pending"));
        }
        if decision_revision == 0 {
            return Err(moqentra_types::Error::invalid_argument(
                "decision revision must be > 0",
            ));
        }
        if reason.trim().is_empty() {
            return Err(moqentra_types::Error::invalid_argument(
                "reason is required",
            ));
        }
        self.decision = Some(ApprovalDecision {
            approver_id,
            outcome,
            reason: reason.clone(),
            valid_until,
            limit_values,
            decision_revision,
            created_at: UtcTimestamp::now(),
        });
        self.state = outcome;
        self.decided_at = Some(UtcTimestamp::now());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use moqentra_types::RandomIdGenerator;

    #[test]
    fn requester_cannot_self_approve() {
        let g = RandomIdGenerator;
        let requester = UserId::new_v7(&g);
        let mut req = ApprovalRequest::new(
            ApprovalRequestId::new_v7(&g),
            TenantId::new_v7(&g),
            ProjectId::new_v7(&g),
            requester,
            ApprovalKind::QuotaOverride,
            "need more GPUs".to_string(),
            BTreeMap::new(),
            1,
            UtcTimestamp::now(),
        )
        .unwrap();
        assert!(req
            .decide(
                requester,
                ApprovalState::Approved,
                "ok".to_string(),
                None,
                BTreeMap::new(),
                1
            )
            .is_err());
    }

    #[test]
    fn pending_only_decision() {
        let g = RandomIdGenerator;
        let requester = UserId::new_v7(&g);
        let approver = UserId::new_v7(&g);
        let mut req = ApprovalRequest::new(
            ApprovalRequestId::new_v7(&g),
            TenantId::new_v7(&g),
            ProjectId::new_v7(&g),
            requester,
            ApprovalKind::ModelPromotion,
            "promote".to_string(),
            BTreeMap::new(),
            1,
            UtcTimestamp::now(),
        )
        .unwrap();
        req.decide(
            approver,
            ApprovalState::Approved,
            "approved".to_string(),
            None,
            BTreeMap::new(),
            1,
        )
        .unwrap();
        assert!(matches!(req.state, ApprovalState::Approved));
        assert!(req
            .decide(
                approver,
                ApprovalState::Rejected,
                "no".to_string(),
                None,
                BTreeMap::new(),
                2
            )
            .is_err());
    }
}
