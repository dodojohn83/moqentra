//! Local executor and node-agent resource management.

use moqentra_types::{AttemptId, NodeId, UtcTimestamp};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::{Path, PathBuf};

/// Kind of accelerator.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub enum AcceleratorKind {
    Nvidia,
    Amd,
    Ascend,
    Cpu,
}

/// A compute device attached to a node.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Device {
    pub uuid: String,
    pub kind: AcceleratorKind,
    pub memory_mib: u64,
    pub driver_version: String,
    pub runtime: String,
    pub healthy: bool,
}

/// Node capabilities discovered at startup.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeCapabilities {
    pub node_id: NodeId,
    pub cpu_cores: u32,
    pub memory_mib: u64,
    pub disk_mib: u64,
    pub container_runtime: String,
    pub devices: Vec<Device>,
    pub healthy: bool,
    pub reported_at: UtcTimestamp,
}

/// A persistent resource allocation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Allocation {
    pub id: String,
    pub attempt_id: AttemptId,
    pub node_id: NodeId,
    pub cpu_cores: u32,
    pub memory_mib: u64,
    pub device_uuids: BTreeSet<String>,
    pub fencing_token: u64,
    pub created_at: UtcTimestamp,
    pub released_at: Option<UtcTimestamp>,
}

/// Container security profile.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContainerSecurityProfile {
    pub run_as_root: bool,
    pub read_only_rootfs: bool,
    pub dropped_capabilities: Vec<String>,
    pub seccomp_profile: Option<String>,
}

impl Default for ContainerSecurityProfile {
    fn default() -> Self {
        Self {
            run_as_root: false,
            read_only_rootfs: true,
            dropped_capabilities: vec!["ALL".to_string()],
            seccomp_profile: None,
        }
    }
}

/// Container launch config.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContainerConfig {
    pub image_digest: String,
    pub entrypoint: Vec<String>,
    pub env: BTreeMap<String, String>,
    pub bind_mounts: BTreeMap<String, String>,
    pub security: ContainerSecurityProfile,
}

fn is_allowed_bind_mount_path(path: &str, base: &Path) -> bool {
    let base = match base.canonicalize() {
        Ok(p) => p,
        Err(_) => return false,
    };
    let abs = match std::fs::canonicalize(path) {
        Ok(p) => p,
        Err(_) => return false,
    };
    abs.starts_with(base)
}

/// Local executor managing node resources and containers.
#[derive(Debug, Clone)]
pub struct LocalExecutor {
    allocations: HashMap<String, Allocation>,
    device_usage: BTreeMap<String, String>,
    allocated_cpu_cores: u64,
    allocated_memory_mib: u64,
    workspace_root: PathBuf,
}

impl Default for LocalExecutor {
    fn default() -> Self {
        Self {
            allocations: HashMap::new(),
            device_usage: BTreeMap::new(),
            allocated_cpu_cores: 0,
            allocated_memory_mib: 0,
            workspace_root: PathBuf::from("/tmp/moqentra"),
        }
    }
}

impl LocalExecutor {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_workspace_root(mut self, root: impl Into<PathBuf>) -> Self {
        self.workspace_root = root.into();
        self
    }

    pub fn allocate(
        &mut self,
        request: &AllocationRequest,
        node_id: NodeId,
        capabilities: &NodeCapabilities,
        fencing_token: u64,
    ) -> Result<Allocation, moqentra_types::Error> {
        if request.cpu_cores == 0 || request.memory_mib == 0 {
            return Err(moqentra_types::Error::invalid_argument(
                "cpu_cores and memory_mib must be greater than zero",
            ));
        }
        let available_cpu =
            u64::from(capabilities.cpu_cores).saturating_sub(self.allocated_cpu_cores);
        if u64::from(request.cpu_cores) > available_cpu {
            return Err(moqentra_types::Error::unavailable("insufficient cpu"));
        }
        let available_memory = capabilities.memory_mib.saturating_sub(self.allocated_memory_mib);
        if request.memory_mib > available_memory {
            return Err(moqentra_types::Error::unavailable("insufficient memory"));
        }
        if request.device_count == 0 && !request.devices.is_empty() {
            return Err(moqentra_types::Error::invalid_argument(
                "device_count must be greater than zero",
            ));
        }
        if request.devices.is_empty() && request.device_count > 0 {
            return Err(moqentra_types::Error::invalid_argument(
                "device_count set but no device kinds requested",
            ));
        }
        let unique_kinds: BTreeSet<_> = request.devices.iter().collect();
        if unique_kinds.len() != request.devices.len() {
            return Err(moqentra_types::Error::invalid_argument(
                "duplicate device kinds in request",
            ));
        }

        let device_count = usize::try_from(request.device_count)
            .map_err(|_| moqentra_types::Error::invalid_argument("device_count too large"))?;
        let mut assigned = BTreeSet::new();
        for needed in &request.devices {
            let available = capabilities
                .devices
                .iter()
                .filter(|d| {
                    d.kind == *needed && d.healthy && !self.device_usage.contains_key(&d.uuid)
                })
                .map(|d| d.uuid.clone())
                .take(device_count)
                .collect::<BTreeSet<_>>();
            if available.len() < device_count {
                return Err(moqentra_types::Error::unavailable("insufficient devices"));
            }
            for uuid in available {
                self.device_usage.insert(uuid.clone(), request.attempt_id.to_string());
                assigned.insert(uuid);
            }
        }

        let allocation = Allocation {
            id: format!("alloc-{}", request.attempt_id),
            attempt_id: request.attempt_id,
            node_id,
            cpu_cores: request.cpu_cores,
            memory_mib: request.memory_mib,
            device_uuids: assigned,
            fencing_token,
            created_at: UtcTimestamp::now(),
            released_at: None,
        };
        self.allocated_cpu_cores = self
            .allocated_cpu_cores
            .checked_add(u64::from(request.cpu_cores))
            .ok_or_else(|| moqentra_types::Error::unavailable("cpu allocation overflow"))?;
        self.allocated_memory_mib = self
            .allocated_memory_mib
            .checked_add(request.memory_mib)
            .ok_or_else(|| moqentra_types::Error::unavailable("memory allocation overflow"))?;
        self.allocations.insert(allocation.id.clone(), allocation.clone());
        Ok(allocation)
    }

    pub fn release(&mut self, allocation_id: &str) -> Result<(), moqentra_types::Error> {
        let allocation = self
            .allocations
            .get_mut(allocation_id)
            .ok_or_else(|| moqentra_types::Error::not_found("allocation"))?;
        if allocation.released_at.is_none() {
            for uuid in &allocation.device_uuids {
                self.device_usage.remove(uuid);
            }
            self.allocated_cpu_cores =
                self.allocated_cpu_cores.saturating_sub(u64::from(allocation.cpu_cores));
            self.allocated_memory_mib =
                self.allocated_memory_mib.saturating_sub(allocation.memory_mib);
            allocation.released_at = Some(UtcTimestamp::now());
        }
        Ok(())
    }

    pub fn reconcile_orphans(&mut self, active_attempts: &[AttemptId]) {
        let active: BTreeSet<_> = active_attempts.iter().map(|a| a.to_string()).collect();
        let stale: Vec<_> = self
            .allocations
            .values()
            .filter(|a| a.released_at.is_none() && !active.contains(&a.attempt_id.to_string()))
            .map(|a| a.id.clone())
            .collect();
        for id in stale {
            let _ = self.release(&id);
        }
    }

    pub fn launch_container(
        &self,
        _config: &ContainerConfig,
    ) -> Result<String, moqentra_types::Error> {
        if _config.security.run_as_root {
            return Err(moqentra_types::Error::permission_denied(
                "root execution not allowed",
            ));
        }
        if !moqentra_types::valid_content_digest(&_config.image_digest) {
            return Err(moqentra_types::Error::invalid_argument(
                "image digest must be a valid content digest",
            ));
        }
        for (src, target) in &_config.bind_mounts {
            for path in [src.as_str(), target.as_str()] {
                if path.is_empty()
                    || path.contains('\0')
                    || !path.starts_with('/')
                    || path.split('/').any(|c| c == "..")
                {
                    return Err(moqentra_types::Error::permission_denied(
                        "bind mount path traversal not allowed",
                    ));
                }
            }
            if !is_allowed_bind_mount_path(src, &self.workspace_root) {
                return Err(moqentra_types::Error::permission_denied(
                    "bind mount source outside workspace",
                ));
            }
        }
        Ok(format!(
            "container-{}",
            _config.image_digest.replace(':', "-")
        ))
    }
}

/// Allocation request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AllocationRequest {
    pub attempt_id: AttemptId,
    pub cpu_cores: u32,
    pub memory_mib: u64,
    pub devices: Vec<AcceleratorKind>,
    pub device_count: u32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use moqentra_types::{AttemptId, NodeId, RandomIdGenerator};

    fn make_capabilities() -> NodeCapabilities {
        let gen = RandomIdGenerator;
        NodeCapabilities {
            node_id: NodeId::new_v7(&gen),
            cpu_cores: 8,
            memory_mib: 32768,
            disk_mib: 1024000,
            container_runtime: "podman".to_string(),
            devices: vec![Device {
                uuid: "gpu-0".to_string(),
                kind: AcceleratorKind::Nvidia,
                memory_mib: 24576,
                driver_version: "535".to_string(),
                runtime: "cuda".to_string(),
                healthy: true,
            }],
            healthy: true,
            reported_at: UtcTimestamp::now(),
        }
    }

    #[test]
    fn allocate_and_release_device() {
        let gen = RandomIdGenerator;
        let caps = make_capabilities();
        let mut executor = LocalExecutor::new();
        let request = AllocationRequest {
            attempt_id: AttemptId::new_v7(&gen),
            cpu_cores: 2,
            memory_mib: 8192,
            devices: vec![AcceleratorKind::Nvidia],
            device_count: 1,
        };
        let alloc = executor.allocate(&request, caps.node_id, &caps, 1).unwrap();
        assert_eq!(alloc.device_uuids.len(), 1);

        // Second allocation on the same device should fail.
        let request2 = AllocationRequest {
            attempt_id: AttemptId::new_v7(&gen),
            cpu_cores: 2,
            memory_mib: 8192,
            devices: vec![AcceleratorKind::Nvidia],
            device_count: 1,
        };
        assert!(executor.allocate(&request2, caps.node_id, &caps, 2).is_err());

        executor.release(&alloc.id).unwrap();
        assert!(executor.allocate(&request2, caps.node_id, &caps, 3).is_ok());
    }

    #[test]
    fn root_container_rejected() {
        let config = ContainerConfig {
            image_digest: "sha256:ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
                .to_string(),
            entrypoint: vec!["train".to_string()],
            env: BTreeMap::new(),
            bind_mounts: BTreeMap::new(),
            security: ContainerSecurityProfile {
                run_as_root: true,
                read_only_rootfs: true,
                dropped_capabilities: vec![],
                seccomp_profile: None,
            },
        };
        let executor = LocalExecutor::new();
        assert!(executor.launch_container(&config).is_err());
    }

    #[test]
    fn reconcile_orphans() {
        let gen = RandomIdGenerator;
        let caps = make_capabilities();
        let mut executor = LocalExecutor::new();
        let attempt = AttemptId::new_v7(&gen);
        let request = AllocationRequest {
            attempt_id: attempt,
            cpu_cores: 1,
            memory_mib: 1024,
            devices: vec![],
            device_count: 0,
        };
        let alloc = executor.allocate(&request, caps.node_id, &caps, 1).unwrap();
        executor.reconcile_orphans(&[]);
        assert!(executor.allocations[&alloc.id].released_at.is_some());
    }
}
