//! Resource class application service.

#![allow(missing_docs)]

use async_trait::async_trait;
use moqentra_domain::resource_class::ResourceClass;
use moqentra_types::{Error, Page, PageRequest, RequestContext, ResourceClassId, Revision};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::ports::{ResourceClassRepository, ResourceListFilter, Versioned};

fn revision_from_u64(value: u64) -> moqentra_types::Revision {
    (0..value).fold(moqentra_types::Revision::initial(), |r, _| r.next())
}

/// Service for managing hardware resource classes.
#[derive(Debug, Clone)]
pub struct ResourceClassService<R: ResourceClassRepository> {
    repo: R,
}

impl<R: ResourceClassRepository> ResourceClassService<R> {
    /// Create a service backed by the supplied repository.
    pub fn new(repo: R) -> Self {
        Self { repo }
    }

    /// Register a new resource class.
    pub async fn create(
        &self,
        ctx: &RequestContext,
        class: ResourceClass,
    ) -> Result<Versioned<ResourceClass>, Error> {
        self.repo.create_resource_class(ctx, class).await
    }

    /// Fetch a resource class by id.
    pub async fn get(
        &self,
        ctx: &RequestContext,
        id: ResourceClassId,
    ) -> Result<Versioned<ResourceClass>, Error> {
        self.repo.get_resource_class(ctx, id).await
    }

    /// List resource classes.
    pub async fn list(
        &self,
        ctx: &RequestContext,
        filter: ResourceListFilter,
        page: PageRequest,
    ) -> Result<Page<Versioned<ResourceClass>>, Error> {
        self.repo.list_resource_classes(ctx, filter, page).await
    }

    /// Replace an existing resource class.
    pub async fn update(
        &self,
        ctx: &RequestContext,
        id: ResourceClassId,
        class: ResourceClass,
        expected: Revision,
    ) -> Result<Versioned<ResourceClass>, Error> {
        self.repo.update_resource_class(ctx, id, class, expected).await
    }

    /// Remove a resource class.
    pub async fn delete(
        &self,
        ctx: &RequestContext,
        id: ResourceClassId,
        expected: Revision,
    ) -> Result<(), Error> {
        self.repo.delete_resource_class(ctx, id, expected).await
    }
}

/// In-memory resource class registry for unit tests and single-process mode.
#[derive(Debug, Default, Clone)]
pub struct InMemoryResourceClassRegistry {
    classes: Arc<Mutex<HashMap<ResourceClassId, ResourceClass>>>,
    revisions: Arc<Mutex<HashMap<ResourceClassId, u64>>>,
}

impl InMemoryResourceClassRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl ResourceClassRepository for InMemoryResourceClassRegistry {
    async fn create_resource_class(
        &self,
        _ctx: &RequestContext,
        class: ResourceClass,
    ) -> Result<Versioned<ResourceClass>, Error> {
        let mut reg = self.classes.lock().map_err(|e| Error::internal(e.to_string()))?;
        if reg.contains_key(&class.id) {
            return Err(Error::conflict("resource class already exists"));
        }
        let mut revs = self.revisions.lock().map_err(|e| Error::internal(e.to_string()))?;
        revs.insert(class.id, 1);
        reg.insert(class.id, class.clone());
        Ok(Versioned::new(class, revision_from_u64(1)))
    }

    async fn get_resource_class(
        &self,
        _ctx: &RequestContext,
        id: ResourceClassId,
    ) -> Result<Versioned<ResourceClass>, Error> {
        let reg = self.classes.lock().map_err(|e| Error::internal(e.to_string()))?;
        let class = reg.get(&id).ok_or_else(|| Error::not_found("resource class"))?;
        let rev = self
            .revisions
            .lock()
            .map_err(|e| Error::internal(e.to_string()))?
            .get(&id)
            .copied()
            .unwrap_or(1);
        Ok(Versioned::new(class.clone(), revision_from_u64(rev)))
    }

    async fn list_resource_classes(
        &self,
        _ctx: &RequestContext,
        filter: ResourceListFilter,
        page: PageRequest,
    ) -> Result<Page<Versioned<ResourceClass>>, Error> {
        let reg = self.classes.lock().map_err(|e| Error::internal(e.to_string()))?;
        let revs = self.revisions.lock().map_err(|e| Error::internal(e.to_string()))?;
        let mut items: Vec<_> = reg
            .values()
            .filter(|c| filter.name_prefix.as_ref().is_none_or(|p| c.name.starts_with(p)))
            .map(|c| {
                let rev = revs.get(&c.id).copied().unwrap_or(1);
                Versioned::new(c.clone(), revision_from_u64(rev))
            })
            .collect();
        items.sort_by(|a, b| a.entity.name.cmp(&b.entity.name));
        let total = u64::try_from(items.len()).unwrap_or(u64::MAX);
        Ok(Page::new(items, total, page))
    }

    async fn update_resource_class(
        &self,
        _ctx: &RequestContext,
        id: ResourceClassId,
        class: ResourceClass,
        expected: Revision,
    ) -> Result<Versioned<ResourceClass>, Error> {
        let mut reg = self.classes.lock().map_err(|e| Error::internal(e.to_string()))?;
        let mut revs = self.revisions.lock().map_err(|e| Error::internal(e.to_string()))?;
        let current = revs.get(&id).copied().unwrap_or(1);
        if expected.as_u64() != current {
            return Err(Error::conflict("resource class revision mismatch"));
        }
        let next = current + 1;
        revs.insert(id, next);
        reg.insert(id, class.clone());
        Ok(Versioned::new(class, revision_from_u64(next)))
    }

    async fn delete_resource_class(
        &self,
        _ctx: &RequestContext,
        id: ResourceClassId,
        expected: Revision,
    ) -> Result<(), Error> {
        let mut reg = self.classes.lock().map_err(|e| Error::internal(e.to_string()))?;
        let mut revs = self.revisions.lock().map_err(|e| Error::internal(e.to_string()))?;
        let current = revs.get(&id).copied().unwrap_or(1);
        if expected.as_u64() != current {
            return Err(Error::conflict("resource class revision mismatch"));
        }
        reg.remove(&id);
        revs.remove(&id);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use moqentra_domain::resource_class::{SharingMode, SupportTier};
    use moqentra_types::{RandomIdGenerator, TenantId};

    fn make_class(name: &str, vendor: &str, sharing: SharingMode) -> ResourceClass {
        let g = RandomIdGenerator;
        ResourceClass::new(
            ResourceClassId::new_v7(&g),
            name.to_string(),
            vendor.to_string(),
            "a100".to_string(),
            81920,
            "550.90".to_string(),
            "nvidia".to_string(),
            "nccl".to_string(),
            "nvlink".to_string(),
            sharing,
            SupportTier::Supported,
            1,
        )
        .unwrap()
    }

    fn ctx() -> RequestContext {
        let g = RandomIdGenerator;
        RequestContext {
            tenant_id: TenantId::new_v7(&g),
            project_id: None,
            roles: vec![],
            project_ids: vec![],
            principal: moqentra_types::Principal::Anonymous,
            request_id: "test".to_string(),
            correlation_id: None,
            deadline: None,
        }
    }

    #[tokio::test]
    async fn create_and_get_resource_class() {
        let svc = ResourceClassService::new(InMemoryResourceClassRegistry::new());
        let ctx = ctx();
        let class = make_class("nvidia-a100", "nvidia", SharingMode::WholeCard);
        let created = svc.create(&ctx, class.clone()).await.unwrap();
        assert_eq!(created.entity.name, "nvidia-a100");
        let got = svc.get(&ctx, class.id).await.unwrap();
        assert_eq!(got.entity.device_resource_name(), "nvidia.com/gpu");
    }

    #[tokio::test]
    async fn shareable_resource_class_maps_to_hami() {
        let svc = ResourceClassService::new(InMemoryResourceClassRegistry::new());
        let ctx = ctx();
        let class = make_class("nvidia-a10-share", "nvidia", SharingMode::Shareable);
        let created = svc.create(&ctx, class.clone()).await.unwrap();
        assert_eq!(created.entity.device_resource_name(), "hami.sh.io/vgpu");
    }
}
