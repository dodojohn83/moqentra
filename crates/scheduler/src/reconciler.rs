//! Reconciler, lease and leader election primitives for HA and multi-cluster.

use moqentra_types::{TenantId, UtcTimestamp};
use std::collections::{BTreeMap, BTreeSet};

/// A renewable lease with epoch/fencing semantics.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Lease {
    pub resource_id: String,
    pub holder: String,
    pub epoch: u64,
    pub expires_at: UtcTimestamp,
    pub released: bool,
}

impl Lease {
    pub fn new(
        resource_id: impl Into<String>,
        holder: impl Into<String>,
        epoch: u64,
        ttl_seconds: u64,
    ) -> Self {
        Self {
            resource_id: resource_id.into(),
            holder: holder.into(),
            epoch,
            expires_at: UtcTimestamp::now()
                .add_std_duration(std::time::Duration::from_secs(ttl_seconds))
                .unwrap_or_else(UtcTimestamp::now),
            released: false,
        }
    }

    pub fn is_expired(&self, now: UtcTimestamp) -> bool {
        now >= self.expires_at
    }

    pub fn can_take(&self, candidate: &str, epoch: u64, now: UtcTimestamp) -> bool {
        self.released || self.is_expired(now) || (self.holder == candidate && self.epoch < epoch)
    }

    pub fn renew(&mut self, ttl_seconds: u64) -> Result<(), moqentra_types::Error> {
        if self.released {
            return Err(moqentra_types::Error::conflict("lease released"));
        }
        self.expires_at = UtcTimestamp::now()
            .add_std_duration(std::time::Duration::from_secs(ttl_seconds))
            .unwrap_or_else(UtcTimestamp::now);
        Ok(())
    }

    pub fn release(&mut self) {
        self.released = true;
    }
}

/// Leader election state for a control-plane instance.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LeaderElection {
    pub instance_id: String,
    pub leader: Option<String>,
    pub term: u64,
    pub voted_for: Option<String>,
    pub last_heartbeat: Option<UtcTimestamp>,
}

impl LeaderElection {
    pub fn new(instance_id: impl Into<String>) -> Self {
        Self {
            instance_id: instance_id.into(),
            leader: None,
            term: 0,
            voted_for: None,
            last_heartbeat: None,
        }
    }

    pub fn campaign(&mut self, term: u64) -> Result<(), moqentra_types::Error> {
        if term <= self.term {
            return Err(moqentra_types::Error::conflict("stale term"));
        }
        self.term = term;
        self.leader = Some(self.instance_id.clone());
        self.voted_for = Some(self.instance_id.clone());
        self.last_heartbeat = Some(UtcTimestamp::now());
        Ok(())
    }

    pub fn acknowledge(&mut self, leader: &str, term: u64) -> Result<(), moqentra_types::Error> {
        if term < self.term {
            return Err(moqentra_types::Error::conflict("stale leader heartbeat"));
        }
        self.term = term;
        self.leader = Some(leader.to_string());
        self.last_heartbeat = Some(UtcTimestamp::now());
        Ok(())
    }
}

/// Desired and observed state for a reconcilable resource.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DesiredObserved<T: Clone + PartialEq + Eq> {
    pub resource_id: String,
    pub desired: T,
    pub observed: Option<T>,
    pub revision: u64,
}

impl<T: Clone + PartialEq + Eq> DesiredObserved<T> {
    pub fn new(resource_id: impl Into<String>, desired: T, revision: u64) -> Self {
        Self {
            resource_id: resource_id.into(),
            desired,
            observed: None,
            revision,
        }
    }

    pub fn reconcile(
        &mut self,
        observed: T,
        next_revision: u64,
    ) -> Result<bool, moqentra_types::Error> {
        if next_revision <= self.revision {
            return Err(moqentra_types::Error::conflict("stale observed state"));
        }
        let drift = Some(&observed) != self.observed.as_ref();
        self.observed = Some(observed);
        self.revision = next_revision;
        let in_sync = self.observed == Some(self.desired.clone());
        Ok(drift && !in_sync)
    }

    pub fn in_sync(&self) -> bool {
        self.observed.as_ref() == Some(&self.desired)
    }
}

/// Outbound agent registration for multi-cluster federation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClusterAgent {
    pub cluster_id: String,
    pub tenant_id: TenantId,
    pub endpoint: String,
    pub capabilities: BTreeSet<String>,
    pub last_seen: Option<UtcTimestamp>,
    pub offline: bool,
    pub cached_resource_versions: BTreeMap<String, u64>,
}

impl ClusterAgent {
    pub fn heartbeat(&mut self, now: UtcTimestamp) {
        self.last_seen = Some(now);
        self.offline = false;
    }

    pub fn mark_offline_if_stale(&mut self, now: UtcTimestamp, ttl_seconds: u64) {
        if let Some(last) = self.last_seen {
            if let Some(expires) =
                last.add_std_duration(std::time::Duration::from_secs(ttl_seconds))
            {
                if now >= expires {
                    self.offline = true;
                }
            }
        } else {
            self.offline = true;
        }
    }

    pub fn cache_version(&mut self, resource: impl Into<String>, version: u64) {
        self.cached_resource_versions.insert(resource.into(), version);
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reconciler {
    pub instance_id: String,
    pub max_events_per_cycle: u32,
    pub paused: bool,
}

impl Reconciler {
    pub fn new(instance_id: impl Into<String>, max_events_per_cycle: u32) -> Self {
        Self {
            instance_id: instance_id.into(),
            max_events_per_cycle,
            paused: false,
        }
    }

    pub fn run_cycle<T, F>(
        &self,
        resources: &mut BTreeMap<String, DesiredObserved<T>>,
        mut observer: F,
    ) -> u32
    where
        T: Clone + PartialEq + Eq,
        F: FnMut(&str) -> Option<T>,
    {
        if self.paused {
            return 0;
        }
        let mut processed = 0u32;
        for (id, state) in resources.iter_mut() {
            if processed >= self.max_events_per_cycle {
                break;
            }
            if let Some(observed) = observer(id) {
                // Ignore CAS failures during a read-only observation cycle.
                let _ = state.reconcile(observed, state.revision + 1);
                processed += 1;
            }
        }
        processed
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use moqentra_types::RandomIdGenerator;

    #[test]
    fn lease_expires_and_allows_takeover() {
        let lease = Lease::new("r1", "a", 1, 0);
        let now = UtcTimestamp::now();
        assert!(lease.is_expired(now));
        assert!(lease.can_take("b", 2, now));
    }

    #[test]
    fn stale_lease_epoch_rejected() {
        let mut lease = Lease::new("r1", "a", 5, 60);
        assert!(!lease.can_take("a", 4, UtcTimestamp::now()));
        assert!(lease.can_take("a", 6, UtcTimestamp::now()));
        lease.release();
        assert!(lease.renew(60).is_err());
    }

    #[test]
    fn leader_election_term_monotonic() {
        let mut le = LeaderElection::new("i1");
        le.campaign(1).unwrap();
        assert!(le.campaign(1).is_err());
        le.acknowledge("i2", 2).unwrap();
        assert_eq!(le.leader.as_deref(), Some("i2"));
        assert!(le.acknowledge("i2", 1).is_err());
    }

    #[test]
    fn desired_observed_reconcile() {
        let mut state = DesiredObserved::new("r1", "desired".to_string(), 1);
        assert!(!state.in_sync());
        state.reconcile("wrong".to_string(), 2).unwrap();
        assert!(!state.in_sync());
        state.reconcile("desired".to_string(), 3).unwrap();
        assert!(state.in_sync());
        assert!(state.reconcile("desired".to_string(), 3).is_err());
    }

    #[test]
    fn cluster_agent_offline_detection() {
        let gen = RandomIdGenerator;
        let mut agent = ClusterAgent {
            cluster_id: "c1".to_string(),
            tenant_id: TenantId::new_v7(&gen),
            endpoint: "https://c1.example.com".to_string(),
            capabilities: BTreeSet::new(),
            last_seen: None,
            offline: false,
            cached_resource_versions: BTreeMap::new(),
        };
        agent.mark_offline_if_stale(UtcTimestamp::now(), 10);
        assert!(agent.offline);
        agent.heartbeat(UtcTimestamp::now());
        agent.mark_offline_if_stale(UtcTimestamp::now(), 10);
        assert!(!agent.offline);
    }

    #[test]
    fn reconciler_respects_max_events() {
        let mut resources: BTreeMap<String, DesiredObserved<String>> = BTreeMap::new();
        for i in 0..5 {
            resources.insert(
                i.to_string(),
                DesiredObserved::new(i.to_string(), "d".to_string(), 1),
            );
        }
        let r = Reconciler::new("rec-1", 2);
        let count = r.run_cycle(&mut resources, |id| Some(format!("obs-{id}")));
        assert_eq!(count, 2);
    }
}
