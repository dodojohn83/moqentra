//! Pure outbox event routing decisions (no I/O).

#![allow(missing_docs)]

use serde_json::Value;

/// Downstream action derived from an outbox event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DispatchAction {
    /// Enqueue a training job on the scheduler.
    EnqueueTrainingJob {
        job_id: String,
        tenant_id: String,
        project_id: String,
        priority: u32,
    },
    /// Admit a training job in the control plane (after scheduler pop).
    AdmitTrainingJob { job_id: String, tenant_id: String },
    /// Allocate node resources for an admitted job.
    AllocateNode {
        attempt_id: String,
        cpu_cores: u32,
        memory_mib: u64,
    },
    /// No side effect — mark complete.
    Noop,
    /// Unrecognized payload — fail the event.
    Reject(String),
}

/// Map an outbox event type + payload to a dispatch action.
pub fn plan_dispatch(event_type: &str, payload: &Value) -> DispatchAction {
    match event_type {
        "training_job.queued" => {
            let job_id = payload.get("job_id").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let tenant_id =
                payload.get("tenant_id").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let project_id =
                payload.get("project_id").and_then(|v| v.as_str()).unwrap_or("").to_string();
            if job_id.is_empty() || tenant_id.is_empty() {
                return DispatchAction::Reject(
                    "training_job.queued requires job_id and tenant_id".into(),
                );
            }
            let priority = payload
                .get("priority")
                .and_then(|v| v.as_u64())
                .and_then(|v| u32::try_from(v).ok())
                .unwrap_or(0);
            DispatchAction::EnqueueTrainingJob {
                job_id,
                tenant_id,
                project_id,
                priority,
            }
        }
        "training_job.scheduled" => {
            let job_id = payload.get("job_id").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let tenant_id =
                payload.get("tenant_id").and_then(|v| v.as_str()).unwrap_or("").to_string();
            if job_id.is_empty() || tenant_id.is_empty() {
                return DispatchAction::Reject(
                    "training_job.scheduled requires job_id and tenant_id".into(),
                );
            }
            DispatchAction::AdmitTrainingJob { job_id, tenant_id }
        }
        "training_job.admitted" => {
            let attempt_id =
                payload.get("attempt_id").and_then(|v| v.as_str()).unwrap_or("").to_string();
            if attempt_id.is_empty() {
                // Fall back to job_id as attempt placeholder for single-node.
                let job_id =
                    payload.get("job_id").and_then(|v| v.as_str()).unwrap_or("").to_string();
                if job_id.is_empty() {
                    return DispatchAction::Reject(
                        "training_job.admitted requires attempt_id or job_id".into(),
                    );
                }
                return DispatchAction::AllocateNode {
                    attempt_id: job_id,
                    cpu_cores: payload
                        .get("cpu_cores")
                        .and_then(|v| v.as_u64())
                        .and_then(|v| u32::try_from(v).ok())
                        .unwrap_or(2),
                    memory_mib: payload.get("memory_mib").and_then(|v| v.as_u64()).unwrap_or(4096),
                };
            }
            DispatchAction::AllocateNode {
                attempt_id,
                cpu_cores: payload
                    .get("cpu_cores")
                    .and_then(|v| v.as_u64())
                    .and_then(|v| u32::try_from(v).ok())
                    .unwrap_or(2),
                memory_mib: payload.get("memory_mib").and_then(|v| v.as_u64()).unwrap_or(4096),
            }
        }
        "dataset.created" | "annotation_project.activated" => DispatchAction::Noop,
        _ => DispatchAction::Noop,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn plans_enqueue_from_queued_event() {
        let action = plan_dispatch(
            "training_job.queued",
            &json!({
                "job_id": "j1",
                "tenant_id": "t1",
                "project_id": "p1",
                "priority": 5
            }),
        );
        assert_eq!(
            action,
            DispatchAction::EnqueueTrainingJob {
                job_id: "j1".into(),
                tenant_id: "t1".into(),
                project_id: "p1".into(),
                priority: 5,
            }
        );
    }

    #[test]
    fn rejects_incomplete_queued_event() {
        let action = plan_dispatch("training_job.queued", &json!({}));
        assert!(matches!(action, DispatchAction::Reject(_)));
    }

    #[test]
    fn plans_allocate_from_admitted() {
        let action = plan_dispatch(
            "training_job.admitted",
            &json!({"job_id": "j1", "cpu_cores": 4, "memory_mib": 8192}),
        );
        assert_eq!(
            action,
            DispatchAction::AllocateNode {
                attempt_id: "j1".into(),
                cpu_cores: 4,
                memory_mib: 8192,
            }
        );
    }
}
