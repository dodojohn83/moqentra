//! Pipeline, HPO and Notebook domain aggregates.

use moqentra_types::{
    AssetId, HpoRunId, NotebookId, PipelineId, PipelineRunId, ProjectId, TenantId, TrainingJobId,
    UtcTimestamp,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// A task node in a pipeline DAG.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PipelineNode {
    pub id: String,
    pub task_template: String,
    pub dependencies: BTreeSet<String>,
    pub parameters: BTreeMap<String, String>,
    pub cache_key_inputs: Vec<String>,
}

/// Pipeline spec as an immutable DAG.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PipelineSpec {
    pub pipeline_id: PipelineId,
    pub name: String,
    pub nodes: BTreeMap<String, PipelineNode>,
    pub artifacts: BTreeMap<String, AssetId>,
}

impl PipelineSpec {
    pub fn validate_dag(&self) -> Result<(), moqentra_types::Error> {
        let mut visited = BTreeSet::new();
        let mut stack = BTreeSet::new();
        for id in self.nodes.keys() {
            self.visit(id, &mut visited, &mut stack)?;
        }
        Ok(())
    }

    fn visit(
        &self,
        id: &str,
        visited: &mut BTreeSet<String>,
        stack: &mut BTreeSet<String>,
    ) -> Result<(), moqentra_types::Error> {
        if stack.contains(id) {
            return Err(moqentra_types::Error::invalid_argument(
                "pipeline cycle detected",
            ));
        }
        if visited.contains(id) {
            return Ok(());
        }
        let node = self.nodes.get(id).ok_or_else(|| {
            moqentra_types::Error::invalid_argument(format!("missing dependency: {id}"))
        })?;
        stack.insert(id.to_string());
        for dep in &node.dependencies {
            if !self.nodes.contains_key(dep) {
                return Err(moqentra_types::Error::invalid_argument(
                    "missing dependency",
                ));
            }
            self.visit(dep, visited, stack)?;
        }
        stack.remove(id);
        visited.insert(id.to_string());
        Ok(())
    }
}

/// Pipeline run state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PipelineRunState {
    Pending,
    Running,
    Succeeded,
    Failed,
    Cancelled,
}

/// Pipeline run with node status.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PipelineRun {
    pub id: PipelineRunId,
    pub tenant_id: TenantId,
    pub project_id: ProjectId,
    pub spec: PipelineSpec,
    pub state: PipelineRunState,
    pub node_states: BTreeMap<String, NodeRunState>,
    pub created_at: UtcTimestamp,
    pub updated_at: UtcTimestamp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NodeRunState {
    Pending,
    Running,
    Succeeded,
    Failed,
    Cancelled,
    Cached,
}

impl PipelineRun {
    pub fn new(
        id: PipelineRunId,
        tenant_id: TenantId,
        project_id: ProjectId,
        spec: PipelineSpec,
    ) -> Self {
        let now = UtcTimestamp::now();
        Self {
            id,
            tenant_id,
            project_id,
            spec,
            state: PipelineRunState::Pending,
            node_states: BTreeMap::new(),
            created_at: now,
            updated_at: now,
        }
    }

    pub fn start(&mut self) -> Result<(), moqentra_types::Error> {
        self.spec.validate_dag()?;
        if !matches!(self.state, PipelineRunState::Pending) {
            return Err(moqentra_types::Error::conflict("pipeline run not pending"));
        }
        self.node_states =
            self.spec.nodes.keys().map(|k| (k.clone(), NodeRunState::Pending)).collect();
        self.state = PipelineRunState::Running;
        self.updated_at = UtcTimestamp::now();
        Ok(())
    }

    pub fn update_node(
        &mut self,
        node_id: &str,
        state: NodeRunState,
    ) -> Result<(), moqentra_types::Error> {
        if !matches!(self.state, PipelineRunState::Running) {
            return Err(moqentra_types::Error::conflict("pipeline not running"));
        }
        if let Some(node) = self.spec.nodes.get(node_id) {
            for dep in &node.dependencies {
                let dep_state = self.node_states.get(dep).copied().unwrap_or(NodeRunState::Pending);
                if !matches!(dep_state, NodeRunState::Succeeded | NodeRunState::Cached) {
                    return Err(moqentra_types::Error::conflict("dependency not satisfied"));
                }
            }
        } else {
            return Err(moqentra_types::Error::not_found("node"));
        }
        self.node_states.insert(node_id.to_string(), state);
        self.updated_at = UtcTimestamp::now();
        self.recompute_state();
        Ok(())
    }

    pub fn cancel(&mut self) -> Result<(), moqentra_types::Error> {
        if matches!(
            self.state,
            PipelineRunState::Succeeded | PipelineRunState::Failed | PipelineRunState::Cancelled
        ) {
            return Err(moqentra_types::Error::conflict("already terminal"));
        }
        self.state = PipelineRunState::Cancelled;
        self.updated_at = UtcTimestamp::now();
        Ok(())
    }

    fn recompute_state(&mut self) {
        if self
            .node_states
            .values()
            .all(|s| matches!(s, NodeRunState::Succeeded | NodeRunState::Cached))
        {
            self.state = PipelineRunState::Succeeded;
        } else if self
            .node_states
            .values()
            .any(|s| matches!(s, NodeRunState::Failed | NodeRunState::Cancelled))
        {
            self.state = PipelineRunState::Failed;
        }
    }
}

/// Search space parameter.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SearchParam {
    Choice(Vec<String>),
    Range { min: f64, max: f64, log_scale: bool },
    Int { min: i64, max: i64 },
}

/// HPO run with trial budget and early stop.
#[derive(Debug, Clone, PartialEq)]
pub struct HpoRun {
    pub id: HpoRunId,
    pub tenant_id: TenantId,
    pub project_id: ProjectId,
    pub base_training_template: String,
    pub search_space: BTreeMap<String, SearchParam>,
    pub trial_budget: u32,
    pub max_parallel: u32,
    pub trials: Vec<Trial>,
    pub best_trial: Option<u32>,
    pub early_stop_enabled: bool,
    pub state: HpoRunState,
    pub created_at: UtcTimestamp,
    pub updated_at: UtcTimestamp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HpoRunState {
    Pending,
    Running,
    Succeeded,
    Failed,
    Stopped,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Trial {
    pub number: u32,
    pub parameters: BTreeMap<String, String>,
    pub metric: Option<f64>,
    pub training_job_id: Option<TrainingJobId>,
    pub state: TrialState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TrialState {
    Pending,
    Running,
    Completed,
    Pruned,
    Failed,
}

impl HpoRun {
    pub fn new(
        id: HpoRunId,
        tenant_id: TenantId,
        project_id: ProjectId,
        base_training_template: impl Into<String>,
        search_space: BTreeMap<String, SearchParam>,
        trial_budget: u32,
        max_parallel: u32,
    ) -> Self {
        let now = UtcTimestamp::now();
        Self {
            id,
            tenant_id,
            project_id,
            base_training_template: base_training_template.into(),
            search_space,
            trial_budget,
            max_parallel,
            trials: Vec::new(),
            best_trial: None,
            early_stop_enabled: true,
            state: HpoRunState::Pending,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn suggest_trial(
        &mut self,
        parameters: BTreeMap<String, String>,
    ) -> Result<u32, moqentra_types::Error> {
        let number = u32::try_from(self.trials.len())
            .map_err(|_| moqentra_types::Error::internal("trial count overflow"))?;
        if number >= self.trial_budget {
            return Err(moqentra_types::Error::unavailable("trial budget exhausted"));
        }
        self.trials.push(Trial {
            number,
            parameters,
            metric: None,
            training_job_id: None,
            state: TrialState::Pending,
        });
        self.updated_at = UtcTimestamp::now();
        Ok(number)
    }

    pub fn report_trial(
        &mut self,
        number: u32,
        metric: f64,
        job_id: TrainingJobId,
    ) -> Result<(), moqentra_types::Error> {
        if !metric.is_finite() {
            return Err(moqentra_types::Error::invalid_argument(
                "metric must be finite",
            ));
        }
        let trial = self
            .trials
            .get_mut(number as usize)
            .ok_or_else(|| moqentra_types::Error::not_found("trial"))?;
        trial.metric = Some(metric);
        trial.training_job_id = Some(job_id);
        trial.state = TrialState::Completed;
        if self.best_trial.is_none_or(|best| {
            self.trials[best as usize].metric.unwrap_or(f64::NEG_INFINITY) < metric
        }) {
            self.best_trial = Some(number);
        }
        self.updated_at = UtcTimestamp::now();
        Ok(())
    }

    pub fn should_stop_early(&self) -> bool {
        if !self.early_stop_enabled {
            return false;
        }
        // Median-pruning placeholder: stop if last 3 trials all worse than best.
        let completed: Vec<f64> = self
            .trials
            .iter()
            .filter(|t| matches!(t.state, TrialState::Completed))
            .filter_map(|t| t.metric)
            .collect();
        if completed.len() < 5 {
            return false;
        }
        let best = completed.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        let recent = &completed[completed.len().saturating_sub(3)..];
        recent.iter().all(|m| *m < best * 0.95)
    }
}

/// Interactive notebook environment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Notebook {
    pub id: NotebookId,
    pub tenant_id: TenantId,
    pub project_id: ProjectId,
    pub image_digest: String,
    pub resource_profile: String,
    pub allowed_egress: Vec<String>,
    pub allow_privileged: bool,
    pub allow_host_path: bool,
    pub idle_timeout_seconds: u64,
    pub expires_at: UtcTimestamp,
    pub state: NotebookState,
    pub last_activity: Option<UtcTimestamp>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NotebookState {
    Pending,
    Running,
    Idle,
    Expired,
    Stopped,
}

impl Notebook {
    pub fn validate_security(&self) -> Result<(), moqentra_types::Error> {
        if self.allow_privileged {
            return Err(moqentra_types::Error::permission_denied(
                "privileged notebook not allowed",
            ));
        }
        if self.allow_host_path {
            return Err(moqentra_types::Error::permission_denied(
                "hostPath not allowed",
            ));
        }
        if self.image_digest.is_empty() {
            return Err(moqentra_types::Error::invalid_argument(
                "missing image digest",
            ));
        }
        Ok(())
    }

    pub fn mark_idle(&mut self) {
        self.state = NotebookState::Idle;
    }

    pub fn expire_if_timeout(&mut self, now: UtcTimestamp) {
        if now >= self.expires_at {
            self.state = NotebookState::Expired;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use moqentra_types::RandomIdGenerator;

    #[test]
    fn pipeline_dag_cycle_rejected() {
        let gen = RandomIdGenerator;
        let mut nodes = BTreeMap::new();
        nodes.insert(
            "a".to_string(),
            PipelineNode {
                id: "a".to_string(),
                task_template: "t1".to_string(),
                dependencies: ["b".to_string()].iter().cloned().collect(),
                parameters: BTreeMap::new(),
                cache_key_inputs: vec![],
            },
        );
        nodes.insert(
            "b".to_string(),
            PipelineNode {
                id: "b".to_string(),
                task_template: "t2".to_string(),
                dependencies: ["a".to_string()].iter().cloned().collect(),
                parameters: BTreeMap::new(),
                cache_key_inputs: vec![],
            },
        );
        let spec = PipelineSpec {
            pipeline_id: PipelineId::new_v7(&gen),
            name: "cycle".to_string(),
            nodes,
            artifacts: BTreeMap::new(),
        };
        assert!(spec.validate_dag().is_err());
    }

    #[test]
    fn pipeline_node_requires_dependency_success() {
        let gen = RandomIdGenerator;
        let mut nodes = BTreeMap::new();
        nodes.insert(
            "a".to_string(),
            PipelineNode {
                id: "a".to_string(),
                task_template: "t1".to_string(),
                dependencies: BTreeSet::new(),
                parameters: BTreeMap::new(),
                cache_key_inputs: vec![],
            },
        );
        nodes.insert(
            "b".to_string(),
            PipelineNode {
                id: "b".to_string(),
                task_template: "t2".to_string(),
                dependencies: ["a".to_string()].iter().cloned().collect(),
                parameters: BTreeMap::new(),
                cache_key_inputs: vec![],
            },
        );
        let spec = PipelineSpec {
            pipeline_id: PipelineId::new_v7(&gen),
            name: "linear".to_string(),
            nodes,
            artifacts: BTreeMap::new(),
        };
        let mut run = PipelineRun::new(
            PipelineRunId::new_v7(&gen),
            TenantId::new_v7(&gen),
            ProjectId::new_v7(&gen),
            spec,
        );
        run.start().unwrap();
        assert!(run.update_node("b", NodeRunState::Running).is_err());
        run.update_node("a", NodeRunState::Succeeded).unwrap();
        run.update_node("b", NodeRunState::Succeeded).unwrap();
        assert!(matches!(run.state, PipelineRunState::Succeeded));
    }

    #[test]
    fn hpo_best_trial_tracked() {
        let gen = RandomIdGenerator;
        let mut hpo = HpoRun::new(
            HpoRunId::new_v7(&gen),
            TenantId::new_v7(&gen),
            ProjectId::new_v7(&gen),
            "train-template",
            BTreeMap::new(),
            5,
            2,
        );
        let params = BTreeMap::new();
        let n0 = hpo.suggest_trial(params.clone()).unwrap();
        let n1 = hpo.suggest_trial(params.clone()).unwrap();
        let job0 = TrainingJobId::new_v7(&gen);
        let job1 = TrainingJobId::new_v7(&gen);
        hpo.report_trial(n0, 0.8, job0).unwrap();
        hpo.report_trial(n1, 0.9, job1).unwrap();
        assert_eq!(hpo.best_trial, Some(n1));
    }

    #[test]
    fn notebook_rejects_privileged() {
        let gen = RandomIdGenerator;
        let nb = Notebook {
            id: NotebookId::new_v7(&gen),
            tenant_id: TenantId::new_v7(&gen),
            project_id: ProjectId::new_v7(&gen),
            image_digest: "sha256:nb".to_string(),
            resource_profile: "small".to_string(),
            allowed_egress: vec![],
            allow_privileged: true,
            allow_host_path: false,
            idle_timeout_seconds: 600,
            expires_at: UtcTimestamp::now()
                .add_std_duration(std::time::Duration::from_secs(3600))
                .unwrap(),
            state: NotebookState::Pending,
            last_activity: None,
        };
        assert!(nb.validate_security().is_err());
    }
}
