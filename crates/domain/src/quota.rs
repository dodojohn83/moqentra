//! Quota policy, reservation, usage ledger and roll-up domain model.

use moqentra_types::{
    ProjectId, QuotaPolicyId, QuotaReservationId, TenantId, UsageLedgerId, UtcTimestamp,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Scope at which a quota policy applies.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QuotaScope {
    Tenant,
    Project,
}

/// A resource dimension tracked by quota.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResourceDimension {
    ConcurrentTrainingJobs,
    ConcurrentConversionJobs,
    AcceleratorCount,
    AcceleratorHours,
    StorageBytes,
    QueueDepth,
}

/// Limit value for a dimension. `None` means unlimited within the enclosing scope.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct DimensionLimit {
    pub dimension: ResourceDimension,
    pub hard_limit: Option<u64>,
}

/// A quota policy with immutable limits and a revision.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QuotaPolicy {
    pub id: QuotaPolicyId,
    pub tenant_id: TenantId,
    pub project_id: Option<ProjectId>,
    pub scope: QuotaScope,
    pub revision: u64,
    pub effective_from: UtcTimestamp,
    pub effective_until: Option<UtcTimestamp>,
    pub limits: Vec<DimensionLimit>,
    pub created_at: UtcTimestamp,
}

impl QuotaPolicy {
    pub fn new(
        id: QuotaPolicyId,
        tenant_id: TenantId,
        project_id: Option<ProjectId>,
        scope: QuotaScope,
        revision: u64,
        effective_from: UtcTimestamp,
        limits: Vec<DimensionLimit>,
    ) -> Result<Self, moqentra_types::Error> {
        if revision == 0 {
            return Err(moqentra_types::Error::invalid_argument(
                "revision must be > 0",
            ));
        }
        Ok(Self {
            id,
            tenant_id,
            project_id,
            scope,
            revision,
            effective_from,
            effective_until: None,
            limits,
            created_at: UtcTimestamp::now(),
        })
    }

    pub fn limit(&self, dimension: ResourceDimension) -> Option<u64> {
        self.limits.iter().find(|l| l.dimension == dimension).and_then(|l| l.hard_limit)
    }
}

/// State of a quota reservation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReservationState {
    Active,
    Consumed,
    Expired,
    Cancelled,
}

/// Reserved capacity for a workload before it starts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QuotaReservation {
    pub id: QuotaReservationId,
    pub tenant_id: TenantId,
    pub project_id: ProjectId,
    pub policy_id: QuotaPolicyId,
    pub policy_revision: u64,
    pub resource_type: String,
    pub resource_dimension: ResourceDimension,
    pub requested: u64,
    pub state: ReservationState,
    pub expires_at: UtcTimestamp,
    pub created_at: UtcTimestamp,
}

impl QuotaReservation {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: QuotaReservationId,
        tenant_id: TenantId,
        project_id: ProjectId,
        policy_id: QuotaPolicyId,
        policy_revision: u64,
        resource_type: String,
        resource_dimension: ResourceDimension,
        requested: u64,
        expires_at: UtcTimestamp,
    ) -> Result<Self, moqentra_types::Error> {
        if requested == 0 {
            return Err(moqentra_types::Error::invalid_argument(
                "reservation must request a positive amount",
            ));
        }
        Ok(Self {
            id,
            tenant_id,
            project_id,
            policy_id,
            policy_revision,
            resource_type,
            resource_dimension,
            requested,
            state: ReservationState::Active,
            expires_at,
            created_at: UtcTimestamp::now(),
        })
    }
}

/// Immutable usage ledger entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UsageLedgerEntry {
    pub id: UsageLedgerId,
    pub tenant_id: TenantId,
    pub project_id: ProjectId,
    pub reservation_id: QuotaReservationId,
    pub source_event_id: String,
    pub resource_dimension: ResourceDimension,
    pub amount: u64,
    pub recorded_at: UtcTimestamp,
    pub metadata: BTreeMap<String, String>,
}

/// Pre-aggregated usage roll-up.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UsageRollup {
    pub tenant_id: TenantId,
    pub project_id: Option<ProjectId>,
    pub period_start: UtcTimestamp,
    pub period_end: UtcTimestamp,
    pub dimension: ResourceDimension,
    pub total_amount: u64,
    pub entry_count: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use moqentra_types::RandomIdGenerator;

    #[test]
    fn quota_policy_limits_lookup() {
        let g = RandomIdGenerator;
        let policy = QuotaPolicy::new(
            QuotaPolicyId::new_v7(&g),
            TenantId::new_v7(&g),
            None,
            QuotaScope::Tenant,
            1,
            UtcTimestamp::now(),
            vec![DimensionLimit {
                dimension: ResourceDimension::ConcurrentTrainingJobs,
                hard_limit: Some(5),
            }],
        )
        .unwrap();
        assert_eq!(
            policy.limit(ResourceDimension::ConcurrentTrainingJobs),
            Some(5)
        );
        assert_eq!(policy.limit(ResourceDimension::QueueDepth), None);
    }

    #[test]
    fn reservation_requires_positive_amount() {
        let g = RandomIdGenerator;
        let res = QuotaReservation::new(
            QuotaReservationId::new_v7(&g),
            TenantId::new_v7(&g),
            ProjectId::new_v7(&g),
            QuotaPolicyId::new_v7(&g),
            1,
            "nvidia.com/gpu".to_string(),
            ResourceDimension::AcceleratorCount,
            0,
            UtcTimestamp::now(),
        );
        assert!(res.is_err());
    }
}
