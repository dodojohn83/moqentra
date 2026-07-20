//! Application graph orchestration and compiler.

use moqentra_types::{
    ApplicationId, ApplicationVersionId, DatasetVersionId, ModelVersionId, ProjectId, TenantId,
    UtcTimestamp,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// Port definition on a graph node.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Port {
    pub name: String,
    pub port_type: String,
    pub schema: String,
}

/// Resource reference used instead of inline secrets.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ResourceRef {
    Model(ModelVersionId),
    Dataset(DatasetVersionId),
    Stream { topic: String },
    Secret { name: String },
    Device { kind: String, count: u32 },
}

/// A node in the application graph.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApplicationNode {
    pub id: String,
    pub node_type: String,
    pub version: String,
    pub deprecated: bool,
    pub inputs: Vec<Port>,
    pub outputs: Vec<Port>,
    pub parameters: BTreeMap<String, serde_json::Value>,
    pub resource_request: String,
    pub capabilities: Vec<String>,
    pub bindings: BTreeMap<String, ResourceRef>,
}

/// Application graph spec.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApplicationSpec {
    pub nodes: BTreeMap<String, ApplicationNode>,
    pub edges: Vec<(String, String, String, String)>,
}

impl ApplicationSpec {
    pub fn validate(&self) -> Result<(), moqentra_types::Error> {
        let mut visited = BTreeSet::new();
        let mut stack = BTreeSet::new();

        // Detect cycles and missing nodes.
        let all_node_ids: BTreeSet<String> = self.nodes.keys().cloned().collect();
        for (src, _, dst, _) in &self.edges {
            if !all_node_ids.contains(src) || !all_node_ids.contains(dst) {
                return Err(moqentra_types::Error::invalid_argument(
                    "edge references missing node",
                ));
            }
        }

        for id in self.nodes.keys() {
            self.visit(id, &mut visited, &mut stack)?;
        }

        // Validate port types on edges.
        for (src, src_port, dst, dst_port) in &self.edges {
            let s = self
                .nodes
                .get(src)
                .ok_or_else(|| moqentra_types::Error::internal("missing source node"))?;
            let d = self
                .nodes
                .get(dst)
                .ok_or_else(|| moqentra_types::Error::internal("missing destination node"))?;
            let s_type = s
                .outputs
                .iter()
                .find(|p| &p.name == src_port)
                .map(|p| &p.port_type)
                .ok_or_else(|| moqentra_types::Error::invalid_argument("source port not found"))?;
            let d_type =
                d.inputs.iter().find(|p| &p.name == dst_port).map(|p| &p.port_type).ok_or_else(
                    || moqentra_types::Error::invalid_argument("destination port not found"),
                )?;
            if s_type != d_type {
                return Err(moqentra_types::Error::invalid_argument(
                    "port type mismatch",
                ));
            }
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
                "application graph cycle",
            ));
        }
        if visited.contains(id) {
            return Ok(());
        }
        stack.insert(id.to_string());
        for (src, _, dst, _) in &self.edges {
            if src == id {
                self.visit(dst, visited, stack)?;
            }
        }
        stack.remove(id);
        visited.insert(id.to_string());
        Ok(())
    }
}

/// Application version state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ApplicationVersionState {
    Draft,
    Published,
    Deprecated,
}

/// A published application graph version.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApplicationVersion {
    pub id: ApplicationVersionId,
    pub application_id: ApplicationId,
    pub tenant_id: TenantId,
    pub project_id: ProjectId,
    pub version: String,
    pub spec: ApplicationSpec,
    pub digest: String,
    pub state: ApplicationVersionState,
    pub created_at: UtcTimestamp,
    pub published_at: Option<UtcTimestamp>,
}

impl ApplicationVersion {
    pub fn new(
        id: ApplicationVersionId,
        application_id: ApplicationId,
        tenant_id: TenantId,
        project_id: ProjectId,
        version: impl Into<String>,
        spec: ApplicationSpec,
    ) -> Result<Self, moqentra_types::Error> {
        let version = version.into();
        if version.trim().is_empty() || version.len() > 64 {
            return Err(moqentra_types::Error::invalid_argument(
                "application version must be non-empty and at most 64 characters",
            ));
        }
        spec.validate()?;
        let digest = Self::canonical_digest(&spec)?;
        let now = UtcTimestamp::now();
        Ok(Self {
            id,
            application_id,
            tenant_id,
            project_id,
            version,
            spec,
            digest,
            state: ApplicationVersionState::Draft,
            created_at: now,
            published_at: None,
        })
    }

    pub fn publish(&mut self) -> Result<(), moqentra_types::Error> {
        if !matches!(self.state, ApplicationVersionState::Draft) {
            return Err(moqentra_types::Error::conflict("not draft"));
        }
        self.state = ApplicationVersionState::Published;
        self.published_at = Some(UtcTimestamp::now());
        Ok(())
    }

    fn canonical_digest(spec: &ApplicationSpec) -> Result<String, moqentra_types::Error> {
        use sha2::{Digest, Sha256};
        let json = serde_json::to_string(spec).map_err(|e| {
            moqentra_types::Error::internal(format!("canonical serialization failed: {e}"))
        })?;
        let hash = Sha256::digest(json.as_bytes());
        Ok(format!("sha256:{:x}", hash))
    }
}

/// Application family.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Application {
    pub id: ApplicationId,
    pub tenant_id: TenantId,
    pub project_id: ProjectId,
    pub name: String,
    pub version_ids: Vec<ApplicationVersionId>,
    pub latest_published: Option<ApplicationVersionId>,
    pub created_at: UtcTimestamp,
    pub updated_at: UtcTimestamp,
}

impl Application {
    pub fn new(
        id: ApplicationId,
        tenant_id: TenantId,
        project_id: ProjectId,
        name: impl Into<String>,
    ) -> Result<Self, moqentra_types::Error> {
        let name = name.into();
        if name.trim().is_empty() || name.len() > 128 {
            return Err(moqentra_types::Error::invalid_argument(
                "application name must be non-empty and at most 128 characters",
            ));
        }
        let now = UtcTimestamp::now();
        Ok(Self {
            id,
            tenant_id,
            project_id,
            name,
            version_ids: Vec::new(),
            latest_published: None,
            created_at: now,
            updated_at: now,
        })
    }

    pub fn add_version(&mut self, version_id: ApplicationVersionId) {
        self.version_ids.push(version_id);
        self.updated_at = UtcTimestamp::now();
    }

    pub fn set_latest_published(&mut self, version_id: ApplicationVersionId) {
        self.latest_published = Some(version_id);
        self.updated_at = UtcTimestamp::now();
    }
}

/// Deployment-time binding for resources referenced by an application.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Binding {
    pub node_id: String,
    pub slot: String,
    pub resource: ResourceRef,
}

#[cfg(test)]
mod tests {
    use super::*;
    use moqentra_types::RandomIdGenerator;

    fn port(name: &str, port_type: &str) -> Port {
        Port {
            name: name.to_string(),
            port_type: port_type.to_string(),
            schema: "any".to_string(),
        }
    }

    fn make_node(id: &str, inputs: Vec<Port>, outputs: Vec<Port>) -> ApplicationNode {
        ApplicationNode {
            id: id.to_string(),
            node_type: "infer".to_string(),
            version: "1".to_string(),
            deprecated: false,
            inputs,
            outputs,
            parameters: BTreeMap::new(),
            resource_request: "small".to_string(),
            capabilities: vec![],
            bindings: BTreeMap::new(),
        }
    }

    #[test]
    fn cycle_rejected() {
        let gen = RandomIdGenerator;
        let a = make_node("a", vec![port("in", "image")], vec![port("out", "image")]);
        let b = make_node("b", vec![port("in", "image")], vec![port("out", "image")]);
        let mut nodes = BTreeMap::new();
        nodes.insert("a".to_string(), a);
        nodes.insert("b".to_string(), b);
        let spec = ApplicationSpec {
            nodes,
            edges: vec![
                (
                    "a".to_string(),
                    "out".to_string(),
                    "b".to_string(),
                    "in".to_string(),
                ),
                (
                    "b".to_string(),
                    "out".to_string(),
                    "a".to_string(),
                    "in".to_string(),
                ),
            ],
        };
        assert!(spec.validate().is_err());

        let err = ApplicationVersion::new(
            ApplicationVersionId::new_v7(&gen),
            ApplicationId::new_v7(&gen),
            TenantId::new_v7(&gen),
            ProjectId::new_v7(&gen),
            "v1",
            spec,
        );
        assert!(err.is_err());
    }

    #[test]
    fn port_type_mismatch_rejected() {
        let gen = RandomIdGenerator;
        let a = make_node("a", vec![], vec![port("out", "image")]);
        let b = make_node("b", vec![port("in", "tensor")], vec![]);
        let mut nodes = BTreeMap::new();
        nodes.insert("a".to_string(), a);
        nodes.insert("b".to_string(), b);
        let spec = ApplicationSpec {
            nodes,
            edges: vec![(
                "a".to_string(),
                "out".to_string(),
                "b".to_string(),
                "in".to_string(),
            )],
        };
        assert!(ApplicationVersion::new(
            ApplicationVersionId::new_v7(&gen),
            ApplicationId::new_v7(&gen),
            TenantId::new_v7(&gen),
            ProjectId::new_v7(&gen),
            "v1",
            spec,
        )
        .is_err());
    }

    #[test]
    fn publish_freezes_version() {
        let gen = RandomIdGenerator;
        let a = make_node("a", vec![], vec![port("out", "image")]);
        let mut nodes = BTreeMap::new();
        nodes.insert("a".to_string(), a);
        let spec = ApplicationSpec {
            nodes,
            edges: vec![],
        };
        let mut app = ApplicationVersion::new(
            ApplicationVersionId::new_v7(&gen),
            ApplicationId::new_v7(&gen),
            TenantId::new_v7(&gen),
            ProjectId::new_v7(&gen),
            "v1",
            spec,
        )
        .unwrap();
        app.publish().unwrap();
        assert!(matches!(app.state, ApplicationVersionState::Published));
    }
}
