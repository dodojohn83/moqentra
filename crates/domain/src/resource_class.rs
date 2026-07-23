//! Resource class and hardware capability domain model.

use moqentra_types::{ResourceClassId, UtcTimestamp};
use serde::{Deserialize, Serialize};

/// Tier of support for a resource class.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SupportTier {
    Supported,
    Preview,
    CompileOnly,
    #[default]
    Mock,
}

/// Sharing mode for an accelerator class.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SharingMode {
    #[default]
    WholeCard,
    Shareable,
    TimeSliced,
}

/// A resource class describes a homogeneous pool of hardware.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResourceClass {
    pub id: ResourceClassId,
    pub name: String,
    pub vendor: String,
    pub family: String,
    pub memory_mib: u64,
    pub driver_version: String,
    pub runtime: String,
    pub collective_backend: String,
    pub topology: String,
    pub sharing_mode: SharingMode,
    pub support_tier: SupportTier,
    pub evidence_date: Option<UtcTimestamp>,
    pub revision: u64,
    pub created_at: UtcTimestamp,
}

impl ResourceClass {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: ResourceClassId,
        name: String,
        vendor: String,
        family: String,
        memory_mib: u64,
        driver_version: String,
        runtime: String,
        collective_backend: String,
        topology: String,
        sharing_mode: SharingMode,
        support_tier: SupportTier,
        revision: u64,
    ) -> Result<Self, moqentra_types::Error> {
        if name.trim().is_empty() || vendor.trim().is_empty() || family.trim().is_empty() {
            return Err(moqentra_types::Error::invalid_argument(
                "name, vendor and family are required",
            ));
        }
        if revision == 0 {
            return Err(moqentra_types::Error::invalid_argument(
                "revision must be > 0",
            ));
        }
        Ok(Self {
            id,
            name,
            vendor,
            family,
            memory_mib,
            driver_version,
            runtime,
            collective_backend,
            topology,
            sharing_mode,
            support_tier,
            evidence_date: None,
            revision,
            created_at: UtcTimestamp::now(),
        })
    }

    pub fn supports_ddp(&self) -> bool {
        matches!(self.sharing_mode, SharingMode::WholeCard)
            && !matches!(
                self.support_tier,
                SupportTier::Mock | SupportTier::CompileOnly
            )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use moqentra_types::RandomIdGenerator;

    #[test]
    fn whole_card_supported_allows_ddp() {
        let g = RandomIdGenerator;
        let rc = ResourceClass::new(
            ResourceClassId::new_v7(&g),
            "nvidia-a100-80g".to_string(),
            "nvidia".to_string(),
            "a100".to_string(),
            81920,
            "550.90".to_string(),
            "cuda-12.8".to_string(),
            "nccl".to_string(),
            "nvlink".to_string(),
            SharingMode::WholeCard,
            SupportTier::Supported,
            1,
        )
        .unwrap();
        assert!(rc.supports_ddp());
    }

    #[test]
    fn shareable_disallows_ddp() {
        let g = RandomIdGenerator;
        let rc = ResourceClass::new(
            ResourceClassId::new_v7(&g),
            "nvidia-a10-shareable".to_string(),
            "nvidia".to_string(),
            "a10".to_string(),
            10240,
            "550.90".to_string(),
            "cuda-12.8".to_string(),
            "nccl".to_string(),
            "pcie".to_string(),
            SharingMode::Shareable,
            SupportTier::Preview,
            1,
        )
        .unwrap();
        assert!(!rc.supports_ddp());
    }
}
