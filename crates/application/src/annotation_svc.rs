//! Annotation project application services.

use moqentra_domain::annotation::{AnnotationProject, AnnotationProjectState};
use moqentra_types::{AnnotationProjectId, Error, ProjectId, TenantId};
use std::collections::BTreeMap;

/// In-memory annotation project registry.
#[derive(Debug, Default)]
pub struct InMemoryAnnotationRegistry {
    projects: BTreeMap<AnnotationProjectId, AnnotationProject>,
}

impl InMemoryAnnotationRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create an annotation project.
    pub fn create_project(
        &mut self,
        project: AnnotationProject,
    ) -> Result<AnnotationProject, Error> {
        if self.projects.contains_key(&project.id) {
            return Err(Error::conflict("annotation project already exists"));
        }
        self.projects.insert(project.id, project.clone());
        Ok(project)
    }

    /// Get project with tenant isolation.
    pub fn get_project(
        &self,
        tenant_id: TenantId,
        id: AnnotationProjectId,
    ) -> Result<&AnnotationProject, Error> {
        let p = self.projects.get(&id).ok_or_else(|| Error::not_found("annotation project"))?;
        if p.tenant_id != tenant_id {
            return Err(Error::not_found("annotation project"));
        }
        Ok(p)
    }

    /// List projects for a tenant.
    pub fn list_projects(
        &self,
        tenant_id: TenantId,
        project_id: Option<ProjectId>,
    ) -> Vec<&AnnotationProject> {
        self.projects
            .values()
            .filter(|p| p.tenant_id == tenant_id)
            .filter(|p| project_id.is_none_or(|pid| p.project_id == pid))
            .collect()
    }

    /// Activate a draft/archived annotation project.
    pub fn activate(
        &mut self,
        tenant_id: TenantId,
        id: AnnotationProjectId,
    ) -> Result<&AnnotationProject, Error> {
        let p = self
            .projects
            .get_mut(&id)
            .ok_or_else(|| Error::not_found("annotation project"))?;
        if p.tenant_id != tenant_id {
            return Err(Error::not_found("annotation project"));
        }
        p.activate()?;
        Ok(p)
    }

    /// Archive an active annotation project.
    pub fn archive(
        &mut self,
        tenant_id: TenantId,
        id: AnnotationProjectId,
    ) -> Result<&AnnotationProject, Error> {
        let p = self
            .projects
            .get_mut(&id)
            .ok_or_else(|| Error::not_found("annotation project"))?;
        if p.tenant_id != tenant_id {
            return Err(Error::not_found("annotation project"));
        }
        p.archive()?;
        Ok(p)
    }

    /// Active project count (ops metric).
    pub fn active_count(&self, tenant_id: TenantId) -> usize {
        self.projects
            .values()
            .filter(|p| {
                p.tenant_id == tenant_id && matches!(p.state, AnnotationProjectState::Active)
            })
            .count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use moqentra_domain::annotation::{Ontology, TaskType};
    use moqentra_types::{DatasetVersionId, RandomIdGenerator};

    #[test]
    fn activate_annotation_project() {
        let gen = RandomIdGenerator;
        let mut reg = InMemoryAnnotationRegistry::new();
        let tenant = TenantId::new_v7(&gen);
        let project = ProjectId::new_v7(&gen);
        let ap = AnnotationProject::new(
            AnnotationProjectId::new_v7(&gen),
            tenant,
            project,
            DatasetVersionId::new_v7(&gen),
            "labels",
            TaskType::BoundingBox,
            Ontology::new(vec![], vec!["rectTool".to_string()]),
        )
        .unwrap();
        let id = ap.id;
        reg.create_project(ap).unwrap();
        reg.activate(tenant, id).unwrap();
        assert_eq!(reg.active_count(tenant), 1);
    }
}
