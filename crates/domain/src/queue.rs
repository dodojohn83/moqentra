//! Fair queue, priority class and queue decision domain model.

use moqentra_types::{PriorityClassId, ProjectId, QueuePolicyId, TenantId, UtcTimestamp};
use serde::{Deserialize, Serialize};

/// Admission policy for a queue.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QueueAdmissionPolicy {
    #[default]
    AllowAny,
    PriorityAllowlist,
    QuotaRequired,
}

/// Weighted fair queue policy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QueuePolicy {
    pub id: QueuePolicyId,
    pub tenant_id: TenantId,
    pub project_id: Option<ProjectId>,
    pub name: String,
    pub weight: u32,
    pub capacity: u32,
    pub max_running: u32,
    pub priority_allowlist: Vec<String>,
    pub admission_policy: QueueAdmissionPolicy,
    pub revision: u64,
    pub created_at: UtcTimestamp,
}

impl QueuePolicy {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: QueuePolicyId,
        tenant_id: TenantId,
        project_id: Option<ProjectId>,
        name: String,
        weight: u32,
        capacity: u32,
        max_running: u32,
        revision: u64,
    ) -> Result<Self, moqentra_types::Error> {
        if name.trim().is_empty() {
            return Err(moqentra_types::Error::invalid_argument("name is required"));
        }
        if weight == 0 {
            return Err(moqentra_types::Error::invalid_argument(
                "weight must be > 0",
            ));
        }
        if capacity == 0 {
            return Err(moqentra_types::Error::invalid_argument(
                "capacity must be > 0",
            ));
        }
        if max_running == 0 {
            return Err(moqentra_types::Error::invalid_argument(
                "max_running must be > 0",
            ));
        }
        Ok(Self {
            id,
            tenant_id,
            project_id,
            name,
            weight,
            capacity,
            max_running,
            priority_allowlist: Vec::new(),
            admission_policy: QueueAdmissionPolicy::AllowAny,
            revision,
            created_at: UtcTimestamp::now(),
        })
    }
}

/// Priority class used for preemption and aging decisions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PriorityClass {
    pub id: PriorityClassId,
    pub tenant_id: TenantId,
    pub project_id: Option<ProjectId>,
    pub name: String,
    pub priority: i32,
    pub preemptible: bool,
    pub max_wait_seconds: u32,
    pub revision: u64,
    pub created_at: UtcTimestamp,
}

impl PriorityClass {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: PriorityClassId,
        tenant_id: TenantId,
        project_id: Option<ProjectId>,
        name: String,
        priority: i32,
        preemptible: bool,
        max_wait_seconds: u32,
        revision: u64,
    ) -> Result<Self, moqentra_types::Error> {
        if name.trim().is_empty() {
            return Err(moqentra_types::Error::invalid_argument("name is required"));
        }
        if max_wait_seconds == 0 {
            return Err(moqentra_types::Error::invalid_argument(
                "max_wait_seconds must be > 0",
            ));
        }
        Ok(Self {
            id,
            tenant_id,
            project_id,
            name,
            priority,
            preemptible,
            max_wait_seconds,
            revision,
            created_at: UtcTimestamp::now(),
        })
    }
}

/// Recorded queue decision for audit and replay.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QueueDecision {
    pub queue_id: QueuePolicyId,
    pub policy_revision: u64,
    pub resource_snapshot: String,
    pub candidate_ids: Vec<String>,
    pub selected_id: String,
    pub explanation: String,
    pub created_at: UtcTimestamp,
}

#[cfg(test)]
mod tests {
    use super::*;
    use moqentra_types::RandomIdGenerator;

    #[test]
    fn queue_policy_validates_weight() {
        let g = RandomIdGenerator;
        assert!(QueuePolicy::new(
            QueuePolicyId::new_v7(&g),
            TenantId::new_v7(&g),
            None,
            "default".to_string(),
            0,
            100,
            10,
            1,
        )
        .is_err());
    }
}
