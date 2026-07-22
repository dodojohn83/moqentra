//! Controlled object-key builder.
//!
//! Callers cannot pass arbitrary bucket keys. Every object key is prefixed with
//! `tenants/{tenant_id}/projects/{project_id}` and follows a fixed resource/
//! version/asset hierarchy. The database stores the resulting key together with
//! the content digest.

use moqentra_types::{ProjectId, TenantId};

/// A validated, tenant-scoped object storage key.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ObjectKey(String);

impl ObjectKey {
    /// Construct a key for an asset belonging to a resource version.
    ///
    /// `resource` is a short type name (`datasets`, `models`, `training-jobs`,
    /// `annotation-projects`, ...). `resource_id` and `version_id` identify the
    /// owning version. `name` is the asset filename within that version.
    pub fn asset(
        tenant_id: TenantId,
        project_id: ProjectId,
        resource: &str,
        resource_id: &str,
        version_id: &str,
        name: &str,
    ) -> Result<Self, moqentra_types::Error> {
        validate_segment(resource)?;
        validate_id_segment(resource_id)?;
        validate_id_segment(version_id)?;
        validate_name(name)?;
        Ok(Self(format!(
            "tenants/{}/projects/{}/{}/{}/versions/{}/{}",
            tenant_id, project_id, resource, resource_id, version_id, name
        )))
    }

    /// Construct a key for a multipart upload part.
    pub fn upload_part(
        tenant_id: TenantId,
        upload_id: &str,
        part_number: i32,
    ) -> Result<Self, moqentra_types::Error> {
        if part_number <= 0 {
            return Err(moqentra_types::Error::invalid_argument(
                "part number must be positive",
            ));
        }
        validate_id_segment(upload_id)?;
        Ok(Self(format!(
            "tenants/{}/uploads/{}/parts/{}",
            tenant_id, upload_id, part_number
        )))
    }

    /// Construct a key for a temporary object awaiting assembly.
    pub fn upload_temp(
        tenant_id: TenantId,
        upload_id: &str,
    ) -> Result<Self, moqentra_types::Error> {
        validate_id_segment(upload_id)?;
        Ok(Self(format!(
            "tenants/{}/uploads/{}/temp",
            tenant_id, upload_id
        )))
    }

    /// Validate that a raw key is a controlled tenant-scoped key.
    pub fn from_str(
        tenant_id: TenantId,
        project_id: ProjectId,
        key: &str,
    ) -> Result<Self, moqentra_types::Error> {
        let expected_prefix = format!("tenants/{}/projects/{}/", tenant_id, project_id);
        if !key.starts_with(&expected_prefix) {
            return Err(moqentra_types::Error::invalid_argument(
                "object key is not scoped to the tenant/project",
            ));
        }
        validate_key(key)?;
        Ok(Self(key.to_string()))
    }

    /// Borrow the key as a string.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for ObjectKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

fn validate_segment(s: &str) -> Result<(), moqentra_types::Error> {
    if s.is_empty() || s.contains('/') || s.contains('\0') || s.contains("..") {
        return Err(moqentra_types::Error::invalid_argument(
            "invalid object key segment",
        ));
    }
    Ok(())
}

fn validate_id_segment(s: &str) -> Result<(), moqentra_types::Error> {
    if s.is_empty() || s.contains('/') || s.contains('\0') || s.contains("..") {
        return Err(moqentra_types::Error::invalid_argument(
            "invalid object key id segment",
        ));
    }
    Ok(())
}

fn validate_name(s: &str) -> Result<(), moqentra_types::Error> {
    if s.is_empty() || s.contains('\0') || s.contains("..") {
        return Err(moqentra_types::Error::invalid_argument(
            "invalid object key name",
        ));
    }
    // Strip a leading slash to avoid double slashes, but reject absolute paths.
    if s.starts_with('/') {
        return Err(moqentra_types::Error::invalid_argument(
            "asset name must be relative",
        ));
    }
    Ok(())
}

fn validate_key(key: &str) -> Result<(), moqentra_types::Error> {
    if key.is_empty() || key.contains('\0') || key.contains("..") {
        return Err(moqentra_types::Error::invalid_argument(
            "object key contains invalid characters",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use moqentra_types::{ProjectId, RandomIdGenerator, TenantId};

    #[test]
    fn asset_key_format() {
        let gen = RandomIdGenerator;
        let tenant = TenantId::new_v7(&gen);
        let project = ProjectId::new_v7(&gen);
        let key =
            ObjectKey::asset(tenant, project, "datasets", "ds-1", "v-1", "train.parquet").unwrap();
        assert!(key.as_str().starts_with("tenants/"));
        assert!(key.as_str().contains("/projects/"));
        assert!(key.as_str().contains("/datasets/ds-1/versions/v-1/train.parquet"));
    }

    #[test]
    fn rejects_path_traversal() {
        let gen = RandomIdGenerator;
        let tenant = TenantId::new_v7(&gen);
        let project = ProjectId::new_v7(&gen);
        assert!(ObjectKey::asset(tenant, project, "datasets", "ds-1", "v-1", "../x").is_err());
        assert!(ObjectKey::asset(tenant, project, "datasets", "../ds-1", "v-1", "x").is_err());
        assert!(ObjectKey::asset(tenant, project, "../datasets", "ds-1", "v-1", "x").is_err());
    }

    #[test]
    fn from_str_enforces_tenant_project_prefix() {
        let gen = RandomIdGenerator;
        let tenant = TenantId::new_v7(&gen);
        let project = ProjectId::new_v7(&gen);
        let other_tenant = TenantId::new_v7(&gen);
        let key = ObjectKey::asset(tenant, project, "datasets", "ds-1", "v-1", "x").unwrap();
        assert!(ObjectKey::from_str(other_tenant, project, key.as_str()).is_err());
        assert!(ObjectKey::from_str(tenant, project, key.as_str()).is_ok());
    }
}
