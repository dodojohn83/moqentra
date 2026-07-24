//! Moqentra application services and ports.
//!
//! Bridges pure domain aggregates with adapter-facing use cases. Domain types
//! stay free of transport/storage; this crate owns orchestration workflows such
//! as compile, version diff, and publish.

#![warn(missing_docs)]

mod annotation_svc;
mod approval_svc;
mod conversion_svc;
mod dispatch;
mod hardware_svc;
mod model_svc;
mod quota_svc;
mod training_svc;

/// Repository ports used by application services.
pub mod coco;
pub mod labelu;
pub mod platform;
pub mod ports;

pub use annotation_svc::InMemoryAnnotationRegistry;
pub use approval_svc::{ApprovalService, InMemoryApprovalRegistry};
pub use coco::{CocoAnnotation, CocoCategory, CocoDataset, CocoImage};
pub use conversion_svc::{InMemoryConversionRegistry, InMemoryEvaluationRegistry};
pub use dispatch::{plan_dispatch, DispatchAction};
pub use hardware_svc::{HardwareAdmission, HardwareCompatibilityService};
pub use labelu::{LabelUAnnotation, LabelUDataset, LabelUProjectConfig, LabelUToolConfig};
pub use model_svc::{ArtifactReconciler, InMemoryModelRegistry};
pub use platform::PlatformAnnotationDataset;
pub use ports::{
    AnnotationRepository, ApplicationRepository, ApprovalRepository, ConversionRepository,
    DatasetRepository, DeploymentRepository, EvaluationRepository, ModelRepository,
    QuotaRepository, ResourceListFilter, TrainingJobRepository, Versioned,
};
pub use quota_svc::{InMemoryQuotaRegistry, QuotaService};
pub use training_svc::InMemoryTrainingRegistry;

use moqentra_domain::application::{
    Application, ApplicationSpec, ApplicationVersion, ApplicationVersionState, Binding,
    ComponentCatalog, GraphConnection, GraphElement, GraphSpec,
};
use moqentra_object_store::ObjectKey;
use moqentra_types::{ApplicationId, ApplicationVersionId, Error, ProjectId, TenantId};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// Result of compiling (validating + canonicalizing) an application graph.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompileResult {
    /// Canonical content digest of the validated spec.
    pub digest: String,
    /// Normalized edges (sorted) used for the digest.
    pub edge_count: usize,
    /// Node count in the graph.
    pub node_count: usize,
    /// Capability requirements unioned from all nodes.
    pub capabilities: Vec<String>,
    /// Runtime profiles required by the selected components.
    pub runtime_profiles: Vec<String>,
    /// Resolved model/resource bindings keyed by `node_id:slot`.
    pub artifact_bindings: BTreeMap<String, moqentra_domain::application::ResourceRef>,
    /// Concrete object-key/digest bindings for artifact slots.
    #[serde(default)]
    pub resolved_artifact_bindings: BTreeMap<String, moqentra_domain::application::ArtifactBinding>,
    /// Canonical dg/v1 graph representation.
    pub graph_spec: GraphSpec,
    /// Canonical digest of the graph spec.
    pub graph_spec_digest: String,
}

/// Structured diff between two application specs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct SpecDiff {
    /// Node ids present only in the left (from) version.
    pub removed_nodes: Vec<String>,
    /// Node ids present only in the right (to) version.
    pub added_nodes: Vec<String>,
    /// Node ids present in both but with different content.
    pub changed_nodes: Vec<String>,
    /// Edges present only in the left version.
    pub removed_edges: Vec<(String, String, String, String)>,
    /// Edges present only in the right version.
    pub added_edges: Vec<(String, String, String, String)>,
}

impl SpecDiff {
    /// Returns true when no structural differences exist.
    pub fn is_empty(&self) -> bool {
        self.removed_nodes.is_empty()
            && self.added_nodes.is_empty()
            && self.changed_nodes.is_empty()
            && self.removed_edges.is_empty()
            && self.added_edges.is_empty()
    }
}

/// Application graph compiler service (server-side authority).
#[derive(Debug, Clone)]
pub struct ApplicationCompiler {
    catalog: ComponentCatalog,
}

impl Default for ApplicationCompiler {
    fn default() -> Self {
        Self::new()
    }
}

impl ApplicationCompiler {
    /// Create a new compiler instance with the R1 baseline component catalog.
    pub fn new() -> Self {
        Self {
            catalog: ComponentCatalog::baseline(),
        }
    }

    /// Create a compiler with a custom catalog (useful for testing).
    pub fn with_catalog(catalog: ComponentCatalog) -> Self {
        Self { catalog }
    }

    /// Validate, canonicalize, and produce a digest for `spec`.
    ///
    /// This is the only path that should produce deployable digests; browser
    /// clients must not fabricate digests independently.
    pub fn compile(&self, mut spec: ApplicationSpec) -> Result<CompileResult, Error> {
        spec.validate()?;
        for node in spec.nodes.values() {
            if node.id.trim().is_empty() {
                return Err(Error::invalid_argument("node id must be non-empty"));
            }
            if node.node_type.trim().is_empty() {
                return Err(Error::invalid_argument("node_type must be non-empty"));
            }
            let component = self.catalog.get(&node.node_type, &node.version).ok_or_else(|| {
                Error::invalid_argument(format!(
                    "unknown component '{}/{}' for node '{}'",
                    node.node_type, node.version, node.id
                ))
            })?;
            if component.deprecated || node.deprecated {
                return Err(Error::invalid_argument(format!(
                    "node '{}' uses a deprecated type/version",
                    node.id
                )));
            }
            for required in &component.required_parameters {
                if !node.parameters.contains_key(required) {
                    return Err(Error::invalid_argument(format!(
                        "node '{}' missing required parameter '{}'",
                        node.id, required
                    )));
                }
            }
            for binding in node.bindings.values() {
                validate_resource_ref_name(binding)?;
            }
        }
        let mut component_lookup: BTreeMap<String, &moqentra_domain::application::Component> =
            BTreeMap::new();
        for node in spec.nodes.values() {
            let component = self.catalog.get(&node.node_type, &node.version).ok_or_else(|| {
                Error::invalid_argument(format!(
                    "unknown component '{}/{}' for node '{}'",
                    node.node_type, node.version, node.id
                ))
            })?;
            component_lookup.insert(format!("{}/{}", node.node_type, node.version), component);
        }
        spec.edges.sort();
        // Domain constructor is the single authority for digest computation.
        // Ephemeral ids are fine; digests are pure functions of the spec.
        let gen = moqentra_types::RandomIdGenerator;
        let digest = ApplicationVersion::new(
            ApplicationVersionId::new_v7(&gen),
            ApplicationId::new_v7(&gen),
            TenantId::new_v7(&gen),
            ProjectId::new_v7(&gen),
            "compile",
            spec.clone(),
        )?
        .digest;

        // Capabilities and runtime profiles are derived from the canonical
        // catalog, not from client supplied node metadata, so clients cannot
        // inflate requirements.
        let mut capabilities: BTreeSet<String> = BTreeSet::new();
        let mut runtime_profiles: BTreeSet<String> = BTreeSet::new();
        let mut artifact_bindings = BTreeMap::new();
        for (node_id, node) in &spec.nodes {
            if let Some(component) = self.catalog.get(&node.node_type, &node.version) {
                for cap in &component.capabilities {
                    capabilities.insert(cap.clone());
                }
                runtime_profiles.insert(component.runtime_profile.clone());
            }
            for (slot, resource) in &node.bindings {
                if matches!(
                    resource,
                    moqentra_domain::application::ResourceRef::Model(_)
                ) || matches!(
                    resource,
                    moqentra_domain::application::ResourceRef::Dataset(_)
                ) || matches!(
                    resource,
                    moqentra_domain::application::ResourceRef::Secret { .. }
                ) {
                    artifact_bindings.insert(format!("{node_id}:{slot}"), resource.clone());
                }
            }
        }

        let graph_spec = self.build_graph(&spec, &component_lookup)?;
        let graph_spec_digest = graph_spec.canonical_digest()?;

        Ok(CompileResult {
            digest,
            edge_count: spec.edges.len(),
            node_count: spec.nodes.len(),
            capabilities: capabilities.into_iter().collect(),
            runtime_profiles: runtime_profiles.into_iter().collect(),
            artifact_bindings,
            resolved_artifact_bindings: BTreeMap::new(),
            graph_spec,
            graph_spec_digest,
        })
    }

    /// Build a canonical dg/v1 `GraphSpec` from a validated `ApplicationSpec`.
    fn build_graph(
        &self,
        spec: &ApplicationSpec,
        component_lookup: &BTreeMap<String, &moqentra_domain::application::Component>,
    ) -> Result<GraphSpec, Error> {
        let mut elements: BTreeMap<String, GraphElement> = BTreeMap::new();
        for (id, node) in &spec.nodes {
            let component = component_lookup
                .get(&format!("{}/{}", node.node_type, node.version))
                .copied()
                .ok_or_else(|| Error::invalid_argument(format!("unknown component for {id}")))?;
            // Start with component defaults; node parameters override.
            let mut parameters = component.parameters.clone();
            for (k, v) in &node.parameters {
                parameters.insert(k.clone(), v.clone());
            }
            elements.insert(
                id.clone(),
                GraphElement {
                    element_type: node.node_type.clone(),
                    runtime_profile: component.runtime_profile.clone(),
                    parameters,
                    inputs: BTreeMap::new(),
                    outputs: BTreeMap::new(),
                },
            );
        }

        let mut connections = Vec::new();
        for (from, from_port, to, to_port) in &spec.edges {
            connections.push(GraphConnection {
                from: from.clone(),
                from_port: from_port.clone(),
                to: to.clone(),
                to_port: to_port.clone(),
            });
            if let Some(el) = elements.get_mut(to) {
                el.inputs.entry(from_port.clone()).or_default().push(from.clone());
            }
            if let Some(el) = elements.get_mut(from) {
                el.outputs.entry(to_port.clone()).or_default().push(to.clone());
            }
        }
        connections.sort();

        Ok(GraphSpec {
            version: "dg/v1".to_string(),
            elements,
            connections,
        })
    }

    /// Diff two specs for version comparison / UI change lists.
    pub fn diff(&self, from: &ApplicationSpec, to: &ApplicationSpec) -> SpecDiff {
        let from_ids: BTreeSet<&String> = from.nodes.keys().collect();
        let to_ids: BTreeSet<&String> = to.nodes.keys().collect();

        let removed_nodes: Vec<String> =
            from_ids.difference(&to_ids).map(|s| (*s).clone()).collect();
        let added_nodes: Vec<String> = to_ids.difference(&from_ids).map(|s| (*s).clone()).collect();

        let mut changed_nodes = Vec::new();
        for id in from_ids.intersection(&to_ids) {
            if from.nodes.get(*id) != to.nodes.get(*id) {
                changed_nodes.push((*id).clone());
            }
        }
        changed_nodes.sort();

        let from_edges: BTreeSet<_> = from.edges.iter().cloned().collect();
        let to_edges: BTreeSet<_> = to.edges.iter().cloned().collect();
        let removed_edges: Vec<_> = from_edges.difference(&to_edges).cloned().collect();
        let added_edges: Vec<_> = to_edges.difference(&from_edges).cloned().collect();

        SpecDiff {
            removed_nodes,
            added_nodes,
            changed_nodes,
            removed_edges,
            added_edges,
        }
    }
}

fn validate_resource_ref_name(r: &moqentra_domain::application::ResourceRef) -> Result<(), Error> {
    use moqentra_domain::application::ResourceRef;
    match r {
        ResourceRef::Secret { name } if name.trim().is_empty() => Err(Error::invalid_argument(
            "secret binding name must be non-empty",
        )),
        ResourceRef::Stream { topic } if topic.trim().is_empty() => {
            Err(Error::invalid_argument("stream topic must be non-empty"))
        }
        ResourceRef::Device { kind, count } => {
            if kind.trim().is_empty() {
                return Err(Error::invalid_argument("device kind must be non-empty"));
            }
            if *count == 0 {
                return Err(Error::invalid_argument("device count must be > 0"));
            }
            Ok(())
        }
        _ => Ok(()),
    }
}

/// In-memory application registry used by unit tests and single-process mode.
#[derive(Debug, Default)]
pub struct InMemoryApplicationRegistry {
    applications: BTreeMap<ApplicationId, Application>,
    versions: BTreeMap<ApplicationVersionId, ApplicationVersion>,
}

impl InMemoryApplicationRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a new application family.
    pub fn create_application(&mut self, app: Application) -> Result<(), Error> {
        if self.applications.contains_key(&app.id) {
            return Err(Error::conflict("application already exists"));
        }
        self.applications.insert(app.id, app);
        Ok(())
    }

    /// Create a draft version after server-side compile.
    pub fn create_version(
        &mut self,
        mut version: ApplicationVersion,
        compiler: &ApplicationCompiler,
    ) -> Result<ApplicationVersion, Error> {
        let app = self
            .applications
            .get_mut(&version.application_id)
            .ok_or_else(|| Error::not_found("application"))?;
        if app.tenant_id != version.tenant_id || app.project_id != version.project_id {
            return Err(Error::permission_denied(
                "version tenant/project does not match application",
            ));
        }
        let compiled = compiler.compile(version.spec.clone())?;
        if compiled.digest != version.digest {
            // Re-bind digest to compiler authority.
            version = ApplicationVersion::new(
                version.id,
                version.application_id,
                version.tenant_id,
                version.project_id,
                version.version.clone(),
                version.spec,
            )?;
        }
        app.add_version(version.id)?;
        self.versions.insert(version.id, version.clone());
        Ok(version)
    }

    /// Publish a draft version and mark it as the latest published.
    pub fn publish_version(
        &mut self,
        version_id: ApplicationVersionId,
    ) -> Result<&ApplicationVersion, Error> {
        let version = self
            .versions
            .get_mut(&version_id)
            .ok_or_else(|| Error::not_found("application version"))?;
        version.publish()?;
        let app_id = version.application_id;
        let app = self
            .applications
            .get_mut(&app_id)
            .ok_or_else(|| Error::not_found("application"))?;
        app.set_latest_published(version_id)?;
        self.versions
            .get(&version_id)
            .ok_or_else(|| Error::internal("version missing after publish"))
    }

    /// Resolve deployment bindings against a published version.
    pub fn resolve_bindings(
        &self,
        version_id: ApplicationVersionId,
        bindings: &[Binding],
    ) -> Result<BTreeMap<String, Binding>, Error> {
        let version = self
            .versions
            .get(&version_id)
            .ok_or_else(|| Error::not_found("application version"))?;
        if !matches!(version.state, ApplicationVersionState::Published) {
            return Err(Error::conflict("only published versions can be deployed"));
        }
        let mut resolved = BTreeMap::new();
        for binding in bindings {
            if !version.spec.nodes.contains_key(&binding.node_id) {
                return Err(Error::invalid_argument(format!(
                    "binding references unknown node '{}'",
                    binding.node_id
                )));
            }
            let key = format!("{}:{}", binding.node_id, binding.slot);
            if resolved.contains_key(&key) {
                return Err(Error::conflict(format!("duplicate binding for {key}")));
            }
            resolved.insert(key, binding.clone());
        }
        Ok(resolved)
    }

    /// Fetch a version by id.
    pub fn get_version(&self, id: ApplicationVersionId) -> Option<&ApplicationVersion> {
        self.versions.get(&id)
    }
}

/// In-memory dataset registry for single-process / unit-test control plane.
#[derive(Debug, Default)]
pub struct InMemoryDatasetRegistry {
    datasets: BTreeMap<moqentra_types::DatasetId, moqentra_domain::dataset::Dataset>,
    versions: BTreeMap<moqentra_types::DatasetVersionId, moqentra_domain::dataset::DatasetVersion>,
}

impl InMemoryDatasetRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Replace in-memory state with rows loaded from durable storage (restart recovery).
    pub fn hydrate(
        &mut self,
        datasets: Vec<moqentra_domain::dataset::Dataset>,
        versions: Vec<moqentra_domain::dataset::DatasetVersion>,
    ) {
        self.datasets.clear();
        self.versions.clear();
        for ds in datasets {
            self.datasets.insert(ds.id, ds);
        }
        for v in versions {
            self.versions.insert(v.id, v);
        }
    }

    /// Find an asset by id across all dataset versions.
    pub fn find_asset(
        &self,
        tenant_id: TenantId,
        asset_id: moqentra_types::AssetId,
    ) -> Option<(
        moqentra_types::DatasetVersionId,
        moqentra_domain::dataset::AssetRef,
    )> {
        for v in self.versions.values() {
            if v.tenant_id != tenant_id {
                continue;
            }
            for a in &v.assets {
                if a.id == asset_id {
                    return Some((v.id, a.clone()));
                }
            }
        }
        None
    }

    /// Return all object keys referenced by dataset version assets.
    pub fn referenced_object_keys(&self) -> std::collections::BTreeSet<String> {
        let mut keys = std::collections::BTreeSet::new();
        for v in self.versions.values() {
            for a in &v.assets {
                keys.insert(a.object_key.clone());
            }
        }
        keys
    }

    /// Return pending (tenant_id, version_id, asset_id, asset_name, object_key, media_type)
    /// tuples that need media validation.
    pub fn pending_validations(
        &self,
    ) -> Vec<(
        moqentra_types::TenantId,
        moqentra_types::DatasetVersionId,
        String,
        String,
        String,
        String,
    )> {
        let mut out = Vec::new();
        for v in self.versions.values() {
            if matches!(
                v.state,
                moqentra_domain::dataset::DatasetVersionState::Published
                    | moqentra_domain::dataset::DatasetVersionState::Deprecated
            ) {
                continue;
            }
            for a in &v.assets {
                let key = a.id.to_string();
                match v.asset_validations.get(&key) {
                    None | Some(moqentra_domain::dataset::AssetValidation::Pending) => {
                        out.push((
                            v.tenant_id,
                            v.id,
                            key,
                            a.name.clone(),
                            a.object_key.clone(),
                            a.media_type.clone(),
                        ));
                    }
                    _ => {}
                }
            }
        }
        out
    }

    /// Create a dataset, enforcing tenant isolation on later reads.
    pub fn create_dataset(
        &mut self,
        dataset: moqentra_domain::dataset::Dataset,
    ) -> Result<moqentra_domain::dataset::Dataset, Error> {
        if self.datasets.contains_key(&dataset.id) {
            return Err(Error::conflict("dataset already exists"));
        }
        self.datasets.insert(dataset.id, dataset.clone());
        Ok(dataset)
    }

    /// Get a dataset only when it belongs to `tenant_id`.
    pub fn get_dataset(
        &self,
        tenant_id: TenantId,
        id: moqentra_types::DatasetId,
    ) -> Result<&moqentra_domain::dataset::Dataset, Error> {
        let ds = self.datasets.get(&id).ok_or_else(|| Error::not_found("dataset"))?;
        if ds.tenant_id != tenant_id {
            return Err(Error::not_found("dataset"));
        }
        Ok(ds)
    }

    /// List datasets for a tenant, optionally filtered by project.
    pub fn list_datasets(
        &self,
        tenant_id: TenantId,
        project_id: Option<ProjectId>,
    ) -> Vec<&moqentra_domain::dataset::Dataset> {
        self.datasets
            .values()
            .filter(|d| d.tenant_id == tenant_id)
            .filter(|d| project_id.is_none_or(|p| d.project_id == p))
            .collect()
    }

    /// Attach a draft version to a dataset under the same tenant/project.
    pub fn create_version(
        &mut self,
        version: moqentra_domain::dataset::DatasetVersion,
    ) -> Result<moqentra_domain::dataset::DatasetVersion, Error> {
        let ds = self
            .datasets
            .get_mut(&version.dataset_id)
            .ok_or_else(|| Error::not_found("dataset"))?;
        if ds.tenant_id != version.tenant_id || ds.project_id != version.project_id {
            return Err(Error::permission_denied(
                "version tenant/project does not match dataset",
            ));
        }
        if self.versions.contains_key(&version.id) {
            return Err(Error::conflict("dataset version already exists"));
        }
        for asset in &version.assets {
            ObjectKey::from_str(version.tenant_id, version.project_id, &asset.object_key)?;
        }
        ds.add_version(version.id)?;
        self.versions.insert(version.id, version.clone());
        Ok(version)
    }

    /// Add an asset to a draft version and validate its object key.
    pub fn add_asset(
        &mut self,
        tenant_id: TenantId,
        id: moqentra_types::DatasetVersionId,
        asset: moqentra_domain::dataset::AssetRef,
    ) -> Result<moqentra_domain::dataset::DatasetVersion, Error> {
        let v = self.versions.get_mut(&id).ok_or_else(|| Error::not_found("dataset version"))?;
        if v.tenant_id != tenant_id {
            return Err(Error::not_found("dataset version"));
        }
        ObjectKey::from_str(v.tenant_id, v.project_id, &asset.object_key)?;
        v.add_asset(asset)?;
        Ok(v.clone())
    }

    /// Generate deterministic train/val/test splits for a draft version.
    pub fn generate_splits(
        &mut self,
        tenant_id: TenantId,
        id: moqentra_types::DatasetVersionId,
        seed: u64,
        train: f64,
        val: f64,
        test: f64,
    ) -> Result<moqentra_domain::dataset::DatasetVersion, Error> {
        let v = self.versions.get_mut(&id).ok_or_else(|| Error::not_found("dataset version"))?;
        if v.tenant_id != tenant_id {
            return Err(Error::not_found("dataset version"));
        }
        v.generate_splits(seed, train, val, test)?;
        Ok(v.clone())
    }

    /// Publish a dataset version after validating its assets and computing the
    /// canonical manifest digest.
    pub fn publish_version(
        &mut self,
        tenant_id: TenantId,
        id: moqentra_types::DatasetVersionId,
    ) -> Result<moqentra_domain::dataset::DatasetVersion, Error> {
        let v = self.versions.get_mut(&id).ok_or_else(|| Error::not_found("dataset version"))?;
        if v.tenant_id != tenant_id {
            return Err(Error::not_found("dataset version"));
        }
        v.publish()?;
        let dataset = self
            .datasets
            .get_mut(&v.dataset_id)
            .ok_or_else(|| Error::internal("dataset missing for version"))?;
        dataset.set_latest_published(v.id)?;
        Ok(v.clone())
    }

    /// Mark an asset within a draft version as successfully validated.
    pub fn mark_asset_valid(
        &mut self,
        tenant_id: TenantId,
        id: moqentra_types::DatasetVersionId,
        asset_name: &str,
    ) -> Result<moqentra_domain::dataset::DatasetVersion, Error> {
        let v = self.versions.get_mut(&id).ok_or_else(|| Error::not_found("dataset version"))?;
        if v.tenant_id != tenant_id {
            return Err(Error::not_found("dataset version"));
        }
        v.mark_asset_valid(asset_name)?;
        Ok(v.clone())
    }

    /// Mark an asset within a draft version as failed validation.
    pub fn mark_asset_failed(
        &mut self,
        tenant_id: TenantId,
        id: moqentra_types::DatasetVersionId,
        asset_name: &str,
        reason: impl Into<String>,
    ) -> Result<moqentra_domain::dataset::DatasetVersion, Error> {
        let v = self.versions.get_mut(&id).ok_or_else(|| Error::not_found("dataset version"))?;
        if v.tenant_id != tenant_id {
            return Err(Error::not_found("dataset version"));
        }
        v.mark_asset_failed(asset_name, reason)?;
        Ok(v.clone())
    }

    /// Get a version only when it belongs to `tenant_id`.
    pub fn get_version(
        &self,
        tenant_id: TenantId,
        id: moqentra_types::DatasetVersionId,
    ) -> Result<&moqentra_domain::dataset::DatasetVersion, Error> {
        let v = self.versions.get(&id).ok_or_else(|| Error::not_found("dataset version"))?;
        if v.tenant_id != tenant_id {
            return Err(Error::not_found("dataset version"));
        }
        Ok(v)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use moqentra_domain::application::{ApplicationNode, Port};
    use moqentra_types::RandomIdGenerator;

    fn port(name: &str, port_type: &str) -> Port {
        Port {
            name: name.to_string(),
            port_type: port_type.to_string(),
            schema: "any".to_string(),
        }
    }

    fn node(id: &str) -> ApplicationNode {
        ApplicationNode {
            id: id.to_string(),
            node_type: "infer".to_string(),
            version: "1".to_string(),
            deprecated: false,
            inputs: vec![port("in", "image")],
            outputs: vec![port("out", "image")],
            parameters: BTreeMap::new(),
            resource_request: "small".to_string(),
            capabilities: vec!["gpu".to_string()],
            bindings: BTreeMap::new(),
        }
    }

    fn linear_spec() -> ApplicationSpec {
        let mut nodes = BTreeMap::new();
        nodes.insert("a".to_string(), node("a"));
        nodes.insert("b".to_string(), node("b"));
        ApplicationSpec {
            nodes,
            edges: vec![(
                "a".to_string(),
                "out".to_string(),
                "b".to_string(),
                "in".to_string(),
            )],
        }
    }

    #[test]
    fn compile_rejects_cycle() {
        let mut nodes = BTreeMap::new();
        nodes.insert("a".to_string(), node("a"));
        nodes.insert("b".to_string(), node("b"));
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
        let compiler = ApplicationCompiler::new();
        assert!(compiler.compile(spec).is_err());
    }

    #[test]
    fn compile_and_diff_roundtrip() {
        let compiler = ApplicationCompiler::new();
        let from = linear_spec();
        let compiled = compiler.compile(from.clone()).unwrap();
        assert_eq!(compiled.node_count, 2);
        assert_eq!(compiled.edge_count, 1);
        assert!(compiled.digest.starts_with("sha256:"));
        assert_eq!(compiled.capabilities, vec!["gpu".to_string()]);
        assert_eq!(compiled.runtime_profiles, vec!["native".to_string()]);
        assert!(compiled.artifact_bindings.is_empty());

        let mut to = from.clone();
        to.nodes.insert("c".to_string(), node("c"));
        to.edges.push((
            "b".to_string(),
            "out".to_string(),
            "c".to_string(),
            "in".to_string(),
        ));
        let diff = compiler.diff(&from, &to);
        assert_eq!(diff.added_nodes, vec!["c".to_string()]);
        assert!(diff.removed_nodes.is_empty());
        assert_eq!(diff.added_edges.len(), 1);
    }

    #[test]
    fn publish_flow_and_bindings() {
        let gen = RandomIdGenerator;
        let compiler = ApplicationCompiler::new();
        let mut registry = InMemoryApplicationRegistry::new();

        let app_id = ApplicationId::new_v7(&gen);
        let tenant = TenantId::new_v7(&gen);
        let project = ProjectId::new_v7(&gen);
        let app = Application::new(app_id, tenant, project, "demo").unwrap();
        registry.create_application(app).unwrap();

        let version_id = ApplicationVersionId::new_v7(&gen);
        let version =
            ApplicationVersion::new(version_id, app_id, tenant, project, "v1", linear_spec())
                .unwrap();
        let version = registry.create_version(version, &compiler).unwrap();
        assert!(matches!(version.state, ApplicationVersionState::Draft));

        registry.publish_version(version_id).unwrap();
        let published = registry.get_version(version_id).unwrap();
        assert!(matches!(
            published.state,
            ApplicationVersionState::Published
        ));

        let bindings = vec![Binding {
            node_id: "a".to_string(),
            slot: "model".to_string(),
            resource: moqentra_domain::application::ResourceRef::Secret {
                name: "model-key".to_string(),
            },
        }];
        let resolved = registry.resolve_bindings(version_id, &bindings).unwrap();
        assert_eq!(resolved.len(), 1);

        // Compile an equivalent spec with the binding inline; artifact_bindings
        // captures the resolved model resource keyed by node_id:slot.
        let mut spec_with_binding = linear_spec();
        spec_with_binding.nodes.get_mut("a").unwrap().bindings.insert(
            "model".to_string(),
            moqentra_domain::application::ResourceRef::Model(
                moqentra_types::ModelVersionId::new_v7(&RandomIdGenerator),
            ),
        );
        let compiled = compiler.compile(spec_with_binding).unwrap();
        assert!(compiled.artifact_bindings.contains_key("a:model"));

        // Unknown node rejected.
        let bad = vec![Binding {
            node_id: "missing".to_string(),
            slot: "x".to_string(),
            resource: moqentra_domain::application::ResourceRef::Stream {
                topic: "t".to_string(),
            },
        }];
        assert!(registry.resolve_bindings(version_id, &bad).is_err());
    }

    #[test]
    fn dataset_registry_tenant_isolation() {
        let gen = RandomIdGenerator;
        let mut reg = InMemoryDatasetRegistry::new();
        let tenant_a = TenantId::new_v7(&gen);
        let tenant_b = TenantId::new_v7(&gen);
        let project = ProjectId::new_v7(&gen);
        let id = moqentra_types::DatasetId::new_v7(&gen);
        let ds = moqentra_domain::dataset::Dataset::new(id, tenant_a, project, "ds-a").unwrap();
        reg.create_dataset(ds).unwrap();

        assert!(reg.get_dataset(tenant_a, id).is_ok());
        assert!(reg.get_dataset(tenant_b, id).is_err());
        assert_eq!(reg.list_datasets(tenant_a, None).len(), 1);
        assert!(reg.list_datasets(tenant_b, None).is_empty());
    }
}
