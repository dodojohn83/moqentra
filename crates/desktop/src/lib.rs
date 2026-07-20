//! Desktop shell IPC allowlist, offline cache and file transfer primitives.

#![allow(missing_docs)]

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

/// Allowed IPC command patterns.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IpcAllowlist {
    pub allowed_commands: BTreeSet<String>,
    pub allowed_paths: Vec<String>,
    pub allowed_schemes: BTreeSet<String>,
}

impl IpcAllowlist {
    pub fn default_safe() -> Self {
        Self {
            allowed_commands: BTreeSet::from([
                "open_file_dialog".to_string(),
                "save_file_dialog".to_string(),
                "read_local_cache".to_string(),
                "write_local_cache".to_string(),
                "start_agent".to_string(),
                "stop_agent".to_string(),
            ]),
            allowed_paths: vec!["/home/*/moqentra/**".to_string()],
            allowed_schemes: BTreeSet::from(["https".to_string()]),
        }
    }

    pub fn validate_command(&self, command: &str) -> Result<(), moqentra_types::Error> {
        if self.allowed_commands.contains(command) {
            Ok(())
        } else {
            Err(moqentra_types::Error::permission_denied(
                "ipc command not allowed",
            ))
        }
    }

    pub fn validate_path(&self, path: &str) -> Result<(), moqentra_types::Error> {
        if path.is_empty() || path.contains('\0') {
            return Err(moqentra_types::Error::invalid_argument("invalid path"));
        }
        if !path.starts_with('/') {
            return Err(moqentra_types::Error::permission_denied(
                "path must be absolute",
            ));
        }

        let components: Vec<&str> = path.split('/').filter(|c| !c.is_empty()).collect();
        if components.contains(&"..") {
            return Err(moqentra_types::Error::permission_denied(
                "path traversal not allowed",
            ));
        }

        let allowed =
            self.allowed_paths.iter().any(|pat| Self::path_matches_glob(&components, pat));
        if !allowed {
            return Err(moqentra_types::Error::permission_denied(
                "path not in allowlist",
            ));
        }

        if Self::path_contains_symlink(Path::new(path)) {
            return Err(moqentra_types::Error::permission_denied(
                "symbolic links not allowed",
            ));
        }

        Ok(())
    }

    fn path_contains_symlink(path: &Path) -> bool {
        let mut current = Some(path);
        while let Some(p) = current {
            if p.is_symlink() {
                return true;
            }
            current = p.parent();
        }
        false
    }

    fn path_matches_glob(path_components: &[&str], pattern: &str) -> bool {
        let pat_components: Vec<&str> = pattern.split('/').filter(|c| !c.is_empty()).collect();
        Self::match_glob(path_components, &pat_components)
    }

    fn match_glob(path: &[&str], pat: &[&str]) -> bool {
        if pat.is_empty() {
            return path.is_empty();
        }
        if pat[0] == "**" {
            // ** may match zero or more path components.
            for i in 0..=path.len() {
                if Self::match_glob(&path[i..], &pat[1..]) {
                    return true;
                }
            }
            return false;
        }
        if path.is_empty() {
            return false;
        }
        if pat[0] == "*" || pat[0] == path[0] {
            return Self::match_glob(&path[1..], &pat[1..]);
        }
        false
    }
}

/// File chunk descriptor for large upload resume.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileChunk {
    pub chunk_index: u64,
    pub offset: u64,
    pub size: u64,
    pub sha256: String,
    pub etag: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileUpload {
    pub file_id: String,
    pub file_path: String,
    pub total_size: u64,
    pub chunk_size: u64,
    pub chunks: Vec<FileChunk>,
    pub bandwidth_bps: Option<u64>,
    pub completed_chunks: BTreeSet<u64>,
}

impl FileUpload {
    const MAX_CHUNKS: u64 = 1_000_000;

    pub fn new(
        file_id: impl Into<String>,
        file_path: impl Into<String>,
        total_size: u64,
        chunk_size: u64,
    ) -> Result<Self, moqentra_types::Error> {
        if chunk_size == 0 {
            return Err(moqentra_types::Error::invalid_argument(
                "chunk size must be greater than zero",
            ));
        }
        let chunk_count = total_size.div_ceil(chunk_size);
        if chunk_count > Self::MAX_CHUNKS {
            return Err(moqentra_types::Error::invalid_argument(
                "file too large for configured chunk size",
            ));
        }
        let mut chunks = Vec::with_capacity(chunk_count as usize);
        for i in 0..chunk_count {
            chunks.push(FileChunk {
                chunk_index: i,
                offset: i * chunk_size,
                size: chunk_size.min(total_size - i * chunk_size),
                sha256: String::new(),
                etag: None,
            });
        }
        Ok(Self {
            file_id: file_id.into(),
            file_path: file_path.into(),
            total_size,
            chunk_size,
            chunks,
            bandwidth_bps: None,
            completed_chunks: BTreeSet::new(),
        })
    }

    pub fn next_missing_chunk(&self) -> Option<&FileChunk> {
        self.chunks.iter().find(|c| !self.completed_chunks.contains(&c.chunk_index))
    }

    pub fn complete_chunk(
        &mut self,
        index: u64,
        etag: impl Into<String>,
    ) -> Result<(), moqentra_types::Error> {
        let chunk = self
            .chunks
            .get_mut(index as usize)
            .ok_or_else(|| moqentra_types::Error::not_found("chunk"))?;
        chunk.etag = Some(etag.into());
        self.completed_chunks.insert(index);
        Ok(())
    }

    pub fn is_complete(&self) -> bool {
        self.completed_chunks.len() == self.chunks.len()
    }
}

/// Local encrypted cache entry keyed by tenant.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocalDraft {
    pub tenant_id: String,
    pub key: String,
    pub encrypted_payload: Vec<u8>,
    pub nonce: Vec<u8>,
    pub revision: u64,
    pub expires_at: moqentra_types::UtcTimestamp,
}

#[derive(Debug, Clone, Default)]
pub struct LocalDraftStore {
    drafts: BTreeMap<String, LocalDraft>,
}

impl LocalDraftStore {
    pub fn insert(&mut self, draft: LocalDraft) {
        self.drafts.insert(format!("{}:{}", draft.tenant_id, draft.key), draft);
    }

    pub fn get(&self, tenant_id: &str, key: &str) -> Option<&LocalDraft> {
        self.drafts.get(&format!("{tenant_id}:{key}"))
    }

    pub fn clear_tenant(&mut self, tenant_id: &str) {
        self.drafts
            .retain(|k, _| k.split(':').next().is_none_or(|prefix| prefix != tenant_id));
    }

    pub fn remove_expired(&mut self, now: moqentra_types::UtcTimestamp) {
        self.drafts.retain(|_, d| d.expires_at > now);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ipc_rejects_unknown_command() {
        let list = IpcAllowlist::default_safe();
        assert!(list.validate_command("open_file_dialog").is_ok());
        assert!(list.validate_command("exec_shell").is_err());
    }

    #[test]
    fn ipc_rejects_path_traversal_and_symlink() {
        let list = IpcAllowlist::default_safe();
        assert!(list.validate_path("/home/user/moqentra/file.txt").is_ok());
        assert!(list.validate_path("/home/user/moqentra/../etc/passwd").is_err());
        // Bypass patterns that used to match a substring anywhere in the path.
        assert!(list.validate_path("/home/attacker/evil/moqentra/backdoor").is_err());
    }

    #[test]
    fn file_upload_rejects_zero_chunk_size() {
        assert!(FileUpload::new("f1", "/tmp/f1.bin", 11, 0).is_err());
    }

    #[test]
    fn file_upload_resume() {
        let mut upload = FileUpload::new("f1", "/tmp/f1.bin", 11, 5).unwrap();
        assert_eq!(upload.chunks.len(), 3);
        assert_eq!(upload.next_missing_chunk().unwrap().chunk_index, 0);
        upload.complete_chunk(0, "etag0").unwrap();
        upload.complete_chunk(2, "etag2").unwrap();
        assert!(!upload.is_complete());
        upload.complete_chunk(1, "etag1").unwrap();
        assert!(upload.is_complete());
    }

    #[test]
    fn draft_store_isolates_tenants() {
        let mut store = LocalDraftStore::default();
        let now = moqentra_types::UtcTimestamp::now();
        let draft_t1 = LocalDraft {
            tenant_id: "t1".to_string(),
            key: "k1".to_string(),
            encrypted_payload: vec![1, 2, 3],
            nonce: vec![0],
            revision: 1,
            expires_at: now.add_std_duration(std::time::Duration::from_secs(3600)).unwrap(),
        };
        let draft_t10 = LocalDraft {
            tenant_id: "t10".to_string(),
            key: "k1".to_string(),
            encrypted_payload: vec![4],
            nonce: vec![0],
            revision: 1,
            expires_at: now.add_std_duration(std::time::Duration::from_secs(3600)).unwrap(),
        };
        store.insert(draft_t1);
        store.insert(draft_t10);
        assert!(store.get("t1", "k1").is_some());
        assert!(store.get("t10", "k1").is_some());
        store.clear_tenant("t1");
        assert!(store.get("t1", "k1").is_none());
        assert!(store.get("t10", "k1").is_some());
    }

    #[test]
    fn draft_store_removes_expired() {
        let mut store = LocalDraftStore::default();
        let now = moqentra_types::UtcTimestamp::now();
        let draft = LocalDraft {
            tenant_id: "t1".to_string(),
            key: "k1".to_string(),
            encrypted_payload: vec![1],
            nonce: vec![0],
            revision: 1,
            expires_at: now,
        };
        store.insert(draft);
        store.remove_expired(now);
        assert!(store.get("t1", "k1").is_none());
    }
}
