//! Queue policy and priority class application service.

#![allow(missing_docs)]

use async_trait::async_trait;
use moqentra_domain::queue::{PriorityClass, QueuePolicy};
use moqentra_types::{
    Error, Page, PageRequest, PriorityClassId, QueuePolicyId, RequestContext, Revision,
};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::ports::{PriorityClassRepository, QueuePolicyRepository, ResourceListFilter, Versioned};

fn revision_from_u64(value: u64) -> moqentra_types::Revision {
    (0..value).fold(moqentra_types::Revision::initial(), |r, _| r.next())
}

/// Service for managing queue policies and priority classes.
#[derive(Debug, Clone)]
pub struct QueueService<Q: QueuePolicyRepository, P: PriorityClassRepository> {
    queue_repo: Q,
    priority_repo: P,
}

impl<Q: QueuePolicyRepository, P: PriorityClassRepository> QueueService<Q, P> {
    /// Create a service backed by the supplied repositories.
    pub fn new(queue_repo: Q, priority_repo: P) -> Self {
        Self {
            queue_repo,
            priority_repo,
        }
    }

    /// Create a queue policy.
    pub async fn create_queue_policy(
        &self,
        ctx: &RequestContext,
        policy: QueuePolicy,
    ) -> Result<Versioned<QueuePolicy>, Error> {
        self.queue_repo.create_policy(ctx, policy).await
    }

    /// Fetch a queue policy.
    pub async fn get_queue_policy(
        &self,
        ctx: &RequestContext,
        id: QueuePolicyId,
    ) -> Result<Versioned<QueuePolicy>, Error> {
        self.queue_repo.get_policy(ctx, id).await
    }

    /// List queue policies.
    pub async fn list_queue_policies(
        &self,
        ctx: &RequestContext,
        filter: ResourceListFilter,
        page: PageRequest,
    ) -> Result<Page<Versioned<QueuePolicy>>, Error> {
        self.queue_repo.list_policies(ctx, filter, page).await
    }

    /// Update a queue policy.
    pub async fn update_queue_policy(
        &self,
        ctx: &RequestContext,
        id: QueuePolicyId,
        policy: QueuePolicy,
        expected: Revision,
    ) -> Result<Versioned<QueuePolicy>, Error> {
        self.queue_repo.update_policy(ctx, id, policy, expected).await
    }

    /// Delete a queue policy.
    pub async fn delete_queue_policy(
        &self,
        ctx: &RequestContext,
        id: QueuePolicyId,
        expected: Revision,
    ) -> Result<(), Error> {
        self.queue_repo.delete_policy(ctx, id, expected).await
    }

    /// Create a priority class.
    pub async fn create_priority_class(
        &self,
        ctx: &RequestContext,
        class: PriorityClass,
    ) -> Result<Versioned<PriorityClass>, Error> {
        self.priority_repo.create_priority_class(ctx, class).await
    }

    /// Fetch a priority class.
    pub async fn get_priority_class(
        &self,
        ctx: &RequestContext,
        id: PriorityClassId,
    ) -> Result<Versioned<PriorityClass>, Error> {
        self.priority_repo.get_priority_class(ctx, id).await
    }

    /// List priority classes.
    pub async fn list_priority_classes(
        &self,
        ctx: &RequestContext,
        filter: ResourceListFilter,
        page: PageRequest,
    ) -> Result<Page<Versioned<PriorityClass>>, Error> {
        self.priority_repo.list_priority_classes(ctx, filter, page).await
    }

    /// Update a priority class.
    pub async fn update_priority_class(
        &self,
        ctx: &RequestContext,
        id: PriorityClassId,
        class: PriorityClass,
        expected: Revision,
    ) -> Result<Versioned<PriorityClass>, Error> {
        self.priority_repo.update_priority_class(ctx, id, class, expected).await
    }

    /// Delete a priority class.
    pub async fn delete_priority_class(
        &self,
        ctx: &RequestContext,
        id: PriorityClassId,
        expected: Revision,
    ) -> Result<(), Error> {
        self.priority_repo.delete_priority_class(ctx, id, expected).await
    }
}

/// In-memory queue policy registry for unit tests and single-process mode.
#[derive(Debug, Default, Clone)]
pub struct InMemoryQueuePolicyRegistry {
    policies: Arc<Mutex<HashMap<QueuePolicyId, QueuePolicy>>>,
    revisions: Arc<Mutex<HashMap<QueuePolicyId, u64>>>,
}

impl InMemoryQueuePolicyRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl QueuePolicyRepository for InMemoryQueuePolicyRegistry {
    async fn create_policy(
        &self,
        ctx: &RequestContext,
        policy: QueuePolicy,
    ) -> Result<Versioned<QueuePolicy>, Error> {
        if policy.tenant_id != ctx.tenant_id {
            return Err(Error::permission_denied("queue policy tenant mismatch"));
        }
        if let (Some(cp), Some(pp)) = (ctx.project_id, policy.project_id) {
            if cp != pp {
                return Err(Error::permission_denied("queue policy project mismatch"));
            }
        }
        let mut reg = self.policies.lock().map_err(|e| Error::internal(e.to_string()))?;
        if reg.contains_key(&policy.id) {
            return Err(Error::conflict("queue policy already exists"));
        }
        let mut revs = self.revisions.lock().map_err(|e| Error::internal(e.to_string()))?;
        revs.insert(policy.id, 1);
        reg.insert(policy.id, policy.clone());
        Ok(Versioned::new(policy, revision_from_u64(1)))
    }

    async fn get_policy(
        &self,
        ctx: &RequestContext,
        id: QueuePolicyId,
    ) -> Result<Versioned<QueuePolicy>, Error> {
        let reg = self.policies.lock().map_err(|e| Error::internal(e.to_string()))?;
        let policy = reg.get(&id).ok_or_else(|| Error::not_found("queue policy"))?;
        if policy.tenant_id != ctx.tenant_id {
            return Err(Error::not_found("queue policy"));
        }
        if let (Some(cp), Some(pp)) = (ctx.project_id, policy.project_id) {
            if cp != pp {
                return Err(Error::not_found("queue policy"));
            }
        }
        let rev = self
            .revisions
            .lock()
            .map_err(|e| Error::internal(e.to_string()))?
            .get(&id)
            .copied()
            .unwrap_or(1);
        Ok(Versioned::new(policy.clone(), revision_from_u64(rev)))
    }

    async fn list_policies(
        &self,
        ctx: &RequestContext,
        filter: ResourceListFilter,
        page: PageRequest,
    ) -> Result<Page<Versioned<QueuePolicy>>, Error> {
        let reg = self.policies.lock().map_err(|e| Error::internal(e.to_string()))?;
        let revs = self.revisions.lock().map_err(|e| Error::internal(e.to_string()))?;
        let mut items: Vec<_> = reg
            .values()
            .filter(|p| p.tenant_id == ctx.tenant_id)
            .filter(|p| p.project_id.is_none_or(|pp| ctx.project_id.is_none_or(|cp| cp == pp)))
            .filter(|p| filter.name_prefix.as_ref().is_none_or(|prefix| p.name.starts_with(prefix)))
            .map(|p| {
                let rev = revs.get(&p.id).copied().unwrap_or(1);
                Versioned::new(p.clone(), revision_from_u64(rev))
            })
            .collect();
        items.sort_by(|a, b| a.entity.name.cmp(&b.entity.name));
        let total = u64::try_from(items.len()).unwrap_or(u64::MAX);
        Ok(Page::new(items, total, page))
    }

    async fn update_policy(
        &self,
        ctx: &RequestContext,
        id: QueuePolicyId,
        policy: QueuePolicy,
        expected: Revision,
    ) -> Result<Versioned<QueuePolicy>, Error> {
        if policy.tenant_id != ctx.tenant_id {
            return Err(Error::permission_denied("queue policy tenant mismatch"));
        }
        if let (Some(cp), Some(pp)) = (ctx.project_id, policy.project_id) {
            if cp != pp {
                return Err(Error::permission_denied("queue policy project mismatch"));
            }
        }
        let mut reg = self.policies.lock().map_err(|e| Error::internal(e.to_string()))?;
        let mut revs = self.revisions.lock().map_err(|e| Error::internal(e.to_string()))?;
        let existing = reg.get(&id).ok_or_else(|| Error::not_found("queue policy"))?;
        if existing.tenant_id != ctx.tenant_id {
            return Err(Error::not_found("queue policy"));
        }
        let current = revs.get(&id).copied().unwrap_or(1);
        if expected.as_u64() != current {
            return Err(Error::conflict("queue policy revision mismatch"));
        }
        let next = current + 1;
        revs.insert(id, next);
        reg.insert(id, policy.clone());
        Ok(Versioned::new(policy, revision_from_u64(next)))
    }

    async fn delete_policy(
        &self,
        ctx: &RequestContext,
        id: QueuePolicyId,
        expected: Revision,
    ) -> Result<(), Error> {
        let mut reg = self.policies.lock().map_err(|e| Error::internal(e.to_string()))?;
        let mut revs = self.revisions.lock().map_err(|e| Error::internal(e.to_string()))?;
        let existing = reg.get(&id).ok_or_else(|| Error::not_found("queue policy"))?;
        if existing.tenant_id != ctx.tenant_id {
            return Err(Error::not_found("queue policy"));
        }
        if let (Some(cp), Some(pp)) = (ctx.project_id, existing.project_id) {
            if cp != pp {
                return Err(Error::not_found("queue policy"));
            }
        }
        let current = revs.get(&id).copied().unwrap_or(1);
        if expected.as_u64() != current {
            return Err(Error::conflict("queue policy revision mismatch"));
        }
        reg.remove(&id);
        revs.remove(&id);
        Ok(())
    }
}

/// In-memory priority class registry for unit tests and single-process mode.
#[derive(Debug, Default, Clone)]
pub struct InMemoryPriorityClassRegistry {
    classes: Arc<Mutex<HashMap<PriorityClassId, PriorityClass>>>,
    revisions: Arc<Mutex<HashMap<PriorityClassId, u64>>>,
}

impl InMemoryPriorityClassRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl PriorityClassRepository for InMemoryPriorityClassRegistry {
    async fn create_priority_class(
        &self,
        ctx: &RequestContext,
        class: PriorityClass,
    ) -> Result<Versioned<PriorityClass>, Error> {
        if class.tenant_id != ctx.tenant_id {
            return Err(Error::permission_denied("priority class tenant mismatch"));
        }
        if let (Some(cp), Some(pp)) = (ctx.project_id, class.project_id) {
            if cp != pp {
                return Err(Error::permission_denied("priority class project mismatch"));
            }
        }
        let mut reg = self.classes.lock().map_err(|e| Error::internal(e.to_string()))?;
        if reg.contains_key(&class.id) {
            return Err(Error::conflict("priority class already exists"));
        }
        let mut revs = self.revisions.lock().map_err(|e| Error::internal(e.to_string()))?;
        revs.insert(class.id, 1);
        reg.insert(class.id, class.clone());
        Ok(Versioned::new(class, revision_from_u64(1)))
    }

    async fn get_priority_class(
        &self,
        ctx: &RequestContext,
        id: PriorityClassId,
    ) -> Result<Versioned<PriorityClass>, Error> {
        let reg = self.classes.lock().map_err(|e| Error::internal(e.to_string()))?;
        let class = reg.get(&id).ok_or_else(|| Error::not_found("priority class"))?;
        if class.tenant_id != ctx.tenant_id {
            return Err(Error::not_found("priority class"));
        }
        if let (Some(cp), Some(pp)) = (ctx.project_id, class.project_id) {
            if cp != pp {
                return Err(Error::not_found("priority class"));
            }
        }
        let rev = self
            .revisions
            .lock()
            .map_err(|e| Error::internal(e.to_string()))?
            .get(&id)
            .copied()
            .unwrap_or(1);
        Ok(Versioned::new(class.clone(), revision_from_u64(rev)))
    }

    async fn list_priority_classes(
        &self,
        ctx: &RequestContext,
        filter: ResourceListFilter,
        page: PageRequest,
    ) -> Result<Page<Versioned<PriorityClass>>, Error> {
        let reg = self.classes.lock().map_err(|e| Error::internal(e.to_string()))?;
        let revs = self.revisions.lock().map_err(|e| Error::internal(e.to_string()))?;
        let mut items: Vec<_> = reg
            .values()
            .filter(|c| c.tenant_id == ctx.tenant_id)
            .filter(|c| c.project_id.is_none_or(|pp| ctx.project_id.is_none_or(|cp| cp == pp)))
            .filter(|c| filter.name_prefix.as_ref().is_none_or(|p| c.name.starts_with(p)))
            .map(|c| {
                let rev = revs.get(&c.id).copied().unwrap_or(1);
                Versioned::new(c.clone(), revision_from_u64(rev))
            })
            .collect();
        items.sort_by(|a, b| a.entity.priority.cmp(&b.entity.priority).reverse());
        let total = u64::try_from(items.len()).unwrap_or(u64::MAX);
        Ok(Page::new(items, total, page))
    }

    async fn update_priority_class(
        &self,
        ctx: &RequestContext,
        id: PriorityClassId,
        class: PriorityClass,
        expected: Revision,
    ) -> Result<Versioned<PriorityClass>, Error> {
        if class.tenant_id != ctx.tenant_id {
            return Err(Error::permission_denied("priority class tenant mismatch"));
        }
        if let (Some(cp), Some(pp)) = (ctx.project_id, class.project_id) {
            if cp != pp {
                return Err(Error::permission_denied("priority class project mismatch"));
            }
        }
        let mut reg = self.classes.lock().map_err(|e| Error::internal(e.to_string()))?;
        let mut revs = self.revisions.lock().map_err(|e| Error::internal(e.to_string()))?;
        let existing = reg.get(&id).ok_or_else(|| Error::not_found("priority class"))?;
        if existing.tenant_id != ctx.tenant_id {
            return Err(Error::not_found("priority class"));
        }
        let current = revs.get(&id).copied().unwrap_or(1);
        if expected.as_u64() != current {
            return Err(Error::conflict("priority class revision mismatch"));
        }
        let next = current + 1;
        revs.insert(id, next);
        reg.insert(id, class.clone());
        Ok(Versioned::new(class, revision_from_u64(next)))
    }

    async fn delete_priority_class(
        &self,
        ctx: &RequestContext,
        id: PriorityClassId,
        expected: Revision,
    ) -> Result<(), Error> {
        let mut reg = self.classes.lock().map_err(|e| Error::internal(e.to_string()))?;
        let mut revs = self.revisions.lock().map_err(|e| Error::internal(e.to_string()))?;
        let existing = reg.get(&id).ok_or_else(|| Error::not_found("priority class"))?;
        if existing.tenant_id != ctx.tenant_id {
            return Err(Error::not_found("priority class"));
        }
        if let (Some(cp), Some(pp)) = (ctx.project_id, existing.project_id) {
            if cp != pp {
                return Err(Error::not_found("priority class"));
            }
        }
        let current = revs.get(&id).copied().unwrap_or(1);
        if expected.as_u64() != current {
            return Err(Error::conflict("priority class revision mismatch"));
        }
        reg.remove(&id);
        revs.remove(&id);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use moqentra_types::{RandomIdGenerator, TenantId};

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
    async fn queue_policy_crud() {
        let g = RandomIdGenerator;
        let svc = QueueService::new(
            InMemoryQueuePolicyRegistry::new(),
            InMemoryPriorityClassRegistry::new(),
        );
        let ctx = ctx();
        let policy = QueuePolicy::new(
            QueuePolicyId::new_v7(&g),
            ctx.tenant_id,
            None,
            "default".to_string(),
            10,
            100,
            10,
            1,
        )
        .unwrap();
        let created = svc.create_queue_policy(&ctx, policy.clone()).await.unwrap();
        assert_eq!(created.entity.weight, 10);
        let got = svc.get_queue_policy(&ctx, policy.id).await.unwrap();
        assert_eq!(got.entity.name, "default");
    }

    #[tokio::test]
    async fn priority_class_preemptible() {
        let g = RandomIdGenerator;
        let svc = QueueService::new(
            InMemoryQueuePolicyRegistry::new(),
            InMemoryPriorityClassRegistry::new(),
        );
        let ctx = ctx();
        let class = PriorityClass::new(
            PriorityClassId::new_v7(&g),
            ctx.tenant_id,
            None,
            "spot".to_string(),
            100,
            true,
            3600,
            1,
        )
        .unwrap();
        let created = svc.create_priority_class(&ctx, class.clone()).await.unwrap();
        assert!(created.entity.preemptible);
    }
}
