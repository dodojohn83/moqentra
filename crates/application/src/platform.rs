//! Moqentra platform intermediate annotation format.
//!
//! This is a lossless JSON representation of annotation tasks and their
//! annotation histories, used for round-trip backup/restore between Moqentra
//! instances.

use moqentra_domain::annotation::{Annotation, AnnotationTask};
use moqentra_types::{AnnotationProjectId, TenantId};
use serde::{Deserialize, Serialize};

/// Platform intermediate dataset for a single annotation project.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct PlatformAnnotationDataset {
    pub project_id: String,
    pub tasks: Vec<AnnotationTask>,
    pub annotations: Vec<Annotation>,
}

impl PlatformAnnotationDataset {
    /// Export tasks and all of their annotations.
    pub fn export(
        project_id: AnnotationProjectId,
        tasks: &[AnnotationTask],
        annotations_by_task: &std::collections::BTreeMap<
            moqentra_types::AnnotationTaskId,
            Vec<Annotation>,
        >,
    ) -> Self {
        let project_tasks: Vec<AnnotationTask> =
            tasks.iter().filter(|t| t.project_id == project_id).cloned().collect();
        let mut annotations = Vec::new();
        for task in &project_tasks {
            if let Some(list) = annotations_by_task.get(&task.id) {
                annotations.extend(list.iter().cloned());
            }
        }
        Self {
            project_id: project_id.to_string(),
            tasks: project_tasks,
            annotations,
        }
    }

    /// Import all tasks and annotations into a registry.
    pub fn import(
        &self,
        project_id: AnnotationProjectId,
        tenant_id: TenantId,
    ) -> Result<Vec<(AnnotationTask, Vec<Annotation>)>, moqentra_types::Error> {
        if self.project_id != project_id.to_string() {
            return Err(moqentra_types::Error::invalid_argument(
                "project id mismatch",
            ));
        }
        let mut pairs = Vec::with_capacity(self.tasks.len());
        for task in &self.tasks {
            let mut owned = task.clone();
            owned.tenant_id = tenant_id;
            owned.project_id = project_id;
            let list: Vec<Annotation> =
                self.annotations.iter().filter(|a| a.task_id == owned.id).cloned().collect();
            pairs.push((owned, list));
        }
        Ok(pairs)
    }
}
