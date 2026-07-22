//! Application graph orchestration and compiler.

use moqentra_types::{
    ApplicationId, ApplicationVersionId, DatasetVersionId, ModelVersionId, ProjectId, TenantId,
    UtcTimestamp,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::str::FromStr;

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

/// A component in the versioned component catalog.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Component {
    pub node_type: String,
    pub version: String,
    pub deprecated: bool,
    pub inputs: Vec<Port>,
    pub outputs: Vec<Port>,
    /// Parameter names with JSON schema default values.
    pub parameters: BTreeMap<String, serde_json::Value>,
    /// Required parameter names.
    pub required_parameters: BTreeSet<String>,
    pub capabilities: Vec<String>,
    pub runtime_profile: String,
}

/// Versioned component catalog used by the application compiler.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ComponentCatalog {
    pub components: BTreeMap<String, Component>,
}

impl ComponentCatalog {
    /// Catalog key used to lookup a component by `node_type` and `version`.
    fn key(node_type: &str, version: &str) -> String {
        format!("{}/{}", node_type, version)
    }

    /// Lookup a component by type and version.
    pub fn get(&self, node_type: &str, version: &str) -> Option<&Component> {
        self.components.get(&Self::key(node_type, version))
    }

    /// Baseline R1 component catalog. Each component declares its ports,
    /// required/default parameters, capabilities and runtime profile.
    pub fn baseline() -> Self {
        use serde_json::json;
        let mut components = BTreeMap::new();
        components.insert(
            Self::key("rtsp-source", "1"),
            Component {
                node_type: "rtsp-source".to_string(),
                version: "1".to_string(),
                deprecated: false,
                inputs: vec![],
                outputs: vec![Port {
                    name: "video".to_string(),
                    port_type: "stream".to_string(),
                    schema: "h264".to_string(),
                }],
                parameters: BTreeMap::new(),
                required_parameters: ["url".to_string()].iter().cloned().collect(),
                capabilities: vec!["network".to_string()],
                runtime_profile: "gstreamer".to_string(),
            },
        );
        components.insert(
            Self::key("decode", "1"),
            Component {
                node_type: "decode".to_string(),
                version: "1".to_string(),
                deprecated: false,
                inputs: vec![Port {
                    name: "video".to_string(),
                    port_type: "stream".to_string(),
                    schema: "h264".to_string(),
                }],
                outputs: vec![Port {
                    name: "frames".to_string(),
                    port_type: "image".to_string(),
                    schema: "raw".to_string(),
                }],
                parameters: BTreeMap::new(),
                required_parameters: BTreeSet::new(),
                capabilities: vec!["decode".to_string()],
                runtime_profile: "ffmpeg".to_string(),
            },
        );
        components.insert(
            Self::key("preprocess", "1"),
            Component {
                node_type: "preprocess".to_string(),
                version: "1".to_string(),
                deprecated: false,
                inputs: vec![Port {
                    name: "frames".to_string(),
                    port_type: "image".to_string(),
                    schema: "raw".to_string(),
                }],
                outputs: vec![Port {
                    name: "tensors".to_string(),
                    port_type: "tensor".to_string(),
                    schema: "float32".to_string(),
                }],
                parameters: BTreeMap::from([
                    ("resize".to_string(), json!([224, 224])),
                    ("mean".to_string(), json!([0.485, 0.456, 0.406])),
                    ("std".to_string(), json!([0.229, 0.224, 0.225])),
                ]),
                required_parameters: BTreeSet::new(),
                capabilities: vec!["preprocess".to_string()],
                runtime_profile: "native".to_string(),
            },
        );
        components.insert(
            Self::key("inference", "1"),
            Component {
                node_type: "inference".to_string(),
                version: "1".to_string(),
                deprecated: false,
                inputs: vec![Port {
                    name: "tensors".to_string(),
                    port_type: "tensor".to_string(),
                    schema: "float32".to_string(),
                }],
                outputs: vec![Port {
                    name: "features".to_string(),
                    port_type: "tensor".to_string(),
                    schema: "float32".to_string(),
                }],
                parameters: BTreeMap::new(),
                required_parameters: ["model_version_id".to_string()].iter().cloned().collect(),
                capabilities: vec!["onnxruntime".to_string()],
                runtime_profile: "onnxruntime".to_string(),
            },
        );
        components.insert(
            Self::key("postprocess", "1"),
            Component {
                node_type: "postprocess".to_string(),
                version: "1".to_string(),
                deprecated: false,
                inputs: vec![Port {
                    name: "features".to_string(),
                    port_type: "tensor".to_string(),
                    schema: "float32".to_string(),
                }],
                outputs: vec![Port {
                    name: "boxes".to_string(),
                    port_type: "detection".to_string(),
                    schema: "coco".to_string(),
                }],
                parameters: BTreeMap::from([
                    ("confidence".to_string(), json!(0.5)),
                    ("nms_threshold".to_string(), json!(0.5)),
                ]),
                required_parameters: BTreeSet::new(),
                capabilities: vec!["nms".to_string()],
                runtime_profile: "native".to_string(),
            },
        );
        components.insert(
            Self::key("tracker", "1"),
            Component {
                node_type: "tracker".to_string(),
                version: "1".to_string(),
                deprecated: false,
                inputs: vec![Port {
                    name: "boxes".to_string(),
                    port_type: "detection".to_string(),
                    schema: "coco".to_string(),
                }],
                outputs: vec![Port {
                    name: "tracks".to_string(),
                    port_type: "track".to_string(),
                    schema: "id".to_string(),
                }],
                parameters: BTreeMap::new(),
                required_parameters: BTreeSet::new(),
                capabilities: vec!["track".to_string()],
                runtime_profile: "native".to_string(),
            },
        );
        components.insert(
            Self::key("osd", "1"),
            Component {
                node_type: "osd".to_string(),
                version: "1".to_string(),
                deprecated: false,
                inputs: vec![Port {
                    name: "tracks".to_string(),
                    port_type: "track".to_string(),
                    schema: "id".to_string(),
                }],
                outputs: vec![Port {
                    name: "annotated".to_string(),
                    port_type: "image".to_string(),
                    schema: "raw".to_string(),
                }],
                parameters: BTreeMap::new(),
                required_parameters: BTreeSet::new(),
                capabilities: vec!["draw".to_string()],
                runtime_profile: "native".to_string(),
            },
        );
        components.insert(
            Self::key("encode", "1"),
            Component {
                node_type: "encode".to_string(),
                version: "1".to_string(),
                deprecated: false,
                inputs: vec![Port {
                    name: "annotated".to_string(),
                    port_type: "image".to_string(),
                    schema: "raw".to_string(),
                }],
                outputs: vec![Port {
                    name: "encoded".to_string(),
                    port_type: "stream".to_string(),
                    schema: "h264".to_string(),
                }],
                parameters: BTreeMap::new(),
                required_parameters: BTreeSet::new(),
                capabilities: vec!["encode".to_string()],
                runtime_profile: "ffmpeg".to_string(),
            },
        );
        components.insert(
            Self::key("rtmp-sink", "1"),
            Component {
                node_type: "rtmp-sink".to_string(),
                version: "1".to_string(),
                deprecated: false,
                inputs: vec![Port {
                    name: "encoded".to_string(),
                    port_type: "stream".to_string(),
                    schema: "h264".to_string(),
                }],
                outputs: vec![],
                parameters: BTreeMap::new(),
                required_parameters: ["endpoint".to_string()].iter().cloned().collect(),
                capabilities: vec!["network".to_string()],
                runtime_profile: "gstreamer".to_string(),
            },
        );
        // Generic `infer` alias retained for test graphs; production graphs
        // should prefer the typed `inference` component.
        components.insert(
            Self::key("infer", "1"),
            Component {
                node_type: "infer".to_string(),
                version: "1".to_string(),
                deprecated: false,
                inputs: vec![Port {
                    name: "in".to_string(),
                    port_type: "image".to_string(),
                    schema: "any".to_string(),
                }],
                outputs: vec![Port {
                    name: "out".to_string(),
                    port_type: "image".to_string(),
                    schema: "any".to_string(),
                }],
                parameters: BTreeMap::new(),
                required_parameters: BTreeSet::new(),
                capabilities: vec!["gpu".to_string()],
                runtime_profile: "native".to_string(),
            },
        );
        Self { components }
    }
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

impl FromStr for ApplicationVersionState {
    type Err = moqentra_types::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Draft" => Ok(Self::Draft),
            "Published" => Ok(Self::Published),
            "Deprecated" => Ok(Self::Deprecated),
            _ => Err(moqentra_types::Error::invalid_argument(format!(
                "unknown application version state: {s}"
            ))),
        }
    }
}

/// A published application graph version.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
        mut spec: ApplicationSpec,
    ) -> Result<Self, moqentra_types::Error> {
        let version = version.into();
        if version.trim().is_empty() || version.len() > 64 {
            return Err(moqentra_types::Error::invalid_argument(
                "application version must be non-empty and at most 64 characters",
            ));
        }
        spec.validate()?;
        // Canonicalize edge order so semantically identical graphs produce the
        // same digest regardless of how edges were constructed.
        spec.edges.sort();
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

    pub fn add_version(
        &mut self,
        version_id: ApplicationVersionId,
    ) -> Result<(), moqentra_types::Error> {
        if self.version_ids.contains(&version_id) {
            return Err(moqentra_types::Error::conflict(
                "version already in application",
            ));
        }
        self.version_ids.push(version_id);
        self.updated_at = UtcTimestamp::now();
        Ok(())
    }

    pub fn set_latest_published(
        &mut self,
        version_id: ApplicationVersionId,
    ) -> Result<(), moqentra_types::Error> {
        if !self.version_ids.contains(&version_id) {
            return Err(moqentra_types::Error::not_found(
                "version not in application",
            ));
        }
        self.latest_published = Some(version_id);
        self.updated_at = UtcTimestamp::now();
        Ok(())
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
    fn digest_ignores_edge_order() {
        let gen = RandomIdGenerator;
        let a = make_node("a", vec![], vec![port("out", "image")]);
        let b = make_node("b", vec![port("in", "image")], vec![]);
        let c = make_node("c", vec![port("in", "image")], vec![]);
        let mut nodes = BTreeMap::new();
        nodes.insert("a".to_string(), a);
        nodes.insert("b".to_string(), b);
        nodes.insert("c".to_string(), c);
        let spec1 = ApplicationSpec {
            nodes: nodes.clone(),
            edges: vec![
                (
                    "a".to_string(),
                    "out".to_string(),
                    "b".to_string(),
                    "in".to_string(),
                ),
                (
                    "a".to_string(),
                    "out".to_string(),
                    "c".to_string(),
                    "in".to_string(),
                ),
            ],
        };
        let spec2 = ApplicationSpec {
            nodes,
            edges: vec![
                (
                    "a".to_string(),
                    "out".to_string(),
                    "c".to_string(),
                    "in".to_string(),
                ),
                (
                    "a".to_string(),
                    "out".to_string(),
                    "b".to_string(),
                    "in".to_string(),
                ),
            ],
        };
        let app1 = ApplicationVersion::new(
            ApplicationVersionId::new_v7(&gen),
            ApplicationId::new_v7(&gen),
            TenantId::new_v7(&gen),
            ProjectId::new_v7(&gen),
            "v1",
            spec1,
        )
        .unwrap();
        let app2 = ApplicationVersion::new(
            ApplicationVersionId::new_v7(&gen),
            ApplicationId::new_v7(&gen),
            TenantId::new_v7(&gen),
            ProjectId::new_v7(&gen),
            "v1",
            spec2,
        )
        .unwrap();
        assert_eq!(app1.digest, app2.digest);
        assert_eq!(app1.spec, app2.spec);
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
