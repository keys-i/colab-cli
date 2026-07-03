//! File manifest, diff, and chunk planning for Colab file sync.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use cocli_protocol::FileEntry;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Debug, thiserror::Error)]
pub enum FsError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("path is outside sync root: {0}")]
    OutsideRoot(String),
}

pub type Result<T> = std::result::Result<T, FsError>;

pub const DEFAULT_EXCLUDES: &[&str] = &[
    ".git",
    "target",
    "__pycache__",
    ".ipynb_checkpoints",
    "node_modules",
    ".venv",
    ".env",
    "checkpoints",
    "checkpoint",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HashMode {
    Never,
    Always,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManifestOptions {
    pub include: Vec<String>,
    pub exclude: Vec<String>,
    pub hash: HashMode,
}

impl Default for ManifestOptions {
    fn default() -> Self {
        Self {
            include: Vec::new(),
            exclude: DEFAULT_EXCLUDES.iter().map(|s| s.to_string()).collect(),
            hash: HashMode::Never,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct FileManifest {
    pub entries: Vec<FileEntry>,
}

impl FileManifest {
    pub fn build(root: &Path, options: &ManifestOptions) -> Result<Self> {
        let mut entries = Vec::new();
        visit(root, root, options, &mut entries)?;
        entries.sort_by(|a, b| a.path.cmp(&b.path));
        Ok(Self { entries })
    }

    pub fn by_path(&self) -> BTreeMap<&str, &FileEntry> {
        self.entries.iter().map(|e| (e.path.as_str(), e)).collect()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct SyncPlan {
    pub upload: Vec<String>,
    pub delete_remote: Vec<String>,
    pub unchanged: usize,
}

pub fn diff(local: &FileManifest, remote: &FileManifest, delete: bool) -> SyncPlan {
    let local_by_path = local.by_path();
    let remote_by_path = remote.by_path();
    let mut plan = SyncPlan::default();

    for entry in &local.entries {
        match remote_by_path.get(entry.path.as_str()) {
            Some(remote) if same_file(entry, remote) => plan.unchanged += 1,
            _ => plan.upload.push(entry.path.clone()),
        }
    }

    if delete {
        for path in remote_by_path.keys() {
            if !local_by_path.contains_key(path) {
                plan.delete_remote.push((*path).to_string());
            }
        }
    }

    plan
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Chunk {
    pub index: u64,
    pub offset: u64,
    pub len: u64,
}

pub fn chunk_plan(size: u64, chunk_size: u64) -> Vec<Chunk> {
    if size == 0 || chunk_size == 0 {
        return Vec::new();
    }
    let mut chunks = Vec::with_capacity(size.div_ceil(chunk_size) as usize);
    let mut offset = 0;
    while offset < size {
        let len = (size - offset).min(chunk_size);
        chunks.push(Chunk {
            index: chunks.len() as u64,
            offset,
            len,
        });
        offset += len;
    }
    chunks
}

fn same_file(a: &FileEntry, b: &FileEntry) -> bool {
    if a.size != b.size {
        return false;
    }
    match (&a.hash, &b.hash) {
        (Some(ah), Some(bh)) => ah == bh,
        _ => a.mtime_unix == b.mtime_unix && a.executable == b.executable,
    }
}

fn visit(
    root: &Path,
    dir: &Path,
    options: &ManifestOptions,
    out: &mut Vec<FileEntry>,
) -> Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let rel = relative_slash(root, &path)?;
        if should_skip(&rel, options) {
            continue;
        }
        let meta = entry.metadata()?;
        if meta.is_dir() {
            visit(root, &path, options, out)?;
        } else if meta.is_file() {
            out.push(file_entry(rel, &path, &meta, options.hash)?);
        }
    }
    Ok(())
}

fn relative_slash(root: &Path, path: &Path) -> Result<String> {
    let rel = path
        .strip_prefix(root)
        .map_err(|_| FsError::OutsideRoot(path.display().to_string()))?;
    Ok(rel
        .components()
        .map(|c| c.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/"))
}

fn should_skip(path: &str, options: &ManifestOptions) -> bool {
    if options.include.iter().any(|p| pattern_matches(path, p)) {
        return false;
    }
    options.exclude.iter().any(|p| pattern_matches(path, p))
}

fn pattern_matches(path: &str, pattern: &str) -> bool {
    if pattern == "*" {
        return true;
    }
    if !pattern.contains('*') {
        return path == pattern || path.split('/').any(|part| part == pattern);
    }

    let mut rest = path;
    let anchored_start = !pattern.starts_with('*');
    let anchored_end = !pattern.ends_with('*');
    for (i, part) in pattern.split('*').filter(|s| !s.is_empty()).enumerate() {
        let Some(pos) = rest.find(part) else {
            return false;
        };
        if i == 0 && anchored_start && pos != 0 {
            return false;
        }
        rest = &rest[pos + part.len()..];
    }
    !anchored_end || rest.is_empty()
}

fn file_entry(
    rel: String,
    path: &Path,
    meta: &std::fs::Metadata,
    hash: HashMode,
) -> Result<FileEntry> {
    let mtime_unix = meta
        .modified()
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map_or(0, |d| d.as_secs());

    Ok(FileEntry {
        path: rel,
        size: meta.len(),
        mtime_unix,
        executable: executable(meta),
        hash: match hash {
            HashMode::Never => None,
            HashMode::Always => Some(hash_file(path)?),
        },
    })
}

fn hash_file(path: &Path) -> Result<String> {
    let mut file = std::fs::File::open(path)?;
    let mut hasher = Sha256::new();
    std::io::copy(&mut file, &mut hasher)?;
    Ok(format!("{:x}", hasher.finalize()))
}

#[cfg(unix)]
fn executable(meta: &std::fs::Metadata) -> bool {
    use std::os::unix::fs::PermissionsExt;
    meta.permissions().mode() & 0o111 != 0
}

#[cfg(not(unix))]
fn executable(_meta: &std::fs::Metadata) -> bool {
    false
}

pub fn safe_local_join(root: &Path, remote_path: &str) -> Result<PathBuf> {
    let mut out = root.to_path_buf();
    for part in remote_path.split('/') {
        if part.is_empty() || part == "." {
            continue;
        }
        if part == ".." {
            return Err(FsError::OutsideRoot(remote_path.into()));
        }
        out.push(part);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manifest_skips_defaults_and_hashes_when_asked() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("a.py"), "print(1)").unwrap();
        std::fs::create_dir(dir.path().join(".git")).unwrap();
        std::fs::write(dir.path().join(".git/config"), "x").unwrap();

        let opts = ManifestOptions {
            hash: HashMode::Always,
            ..ManifestOptions::default()
        };
        let manifest = FileManifest::build(dir.path(), &opts).unwrap();
        assert_eq!(manifest.entries.len(), 1);
        assert_eq!(manifest.entries[0].path, "a.py");
        assert!(manifest.entries[0].hash.is_some());
    }

    #[test]
    fn diff_uploads_changed_and_deletes_remote_when_requested() {
        let local = FileManifest {
            entries: vec![FileEntry {
                path: "a".into(),
                size: 2,
                mtime_unix: 1,
                executable: false,
                hash: None,
            }],
        };
        let remote = FileManifest {
            entries: vec![
                FileEntry {
                    path: "a".into(),
                    size: 3,
                    mtime_unix: 1,
                    executable: false,
                    hash: None,
                },
                FileEntry {
                    path: "old".into(),
                    size: 1,
                    mtime_unix: 1,
                    executable: false,
                    hash: None,
                },
            ],
        };
        let plan = diff(&local, &remote, true);
        assert_eq!(plan.upload, vec!["a"]);
        assert_eq!(plan.delete_remote, vec!["old"]);
    }

    #[test]
    fn chunks_cover_file() {
        let chunks = chunk_plan(10, 4);
        assert_eq!(
            chunks,
            vec![
                Chunk {
                    index: 0,
                    offset: 0,
                    len: 4
                },
                Chunk {
                    index: 1,
                    offset: 4,
                    len: 4
                },
                Chunk {
                    index: 2,
                    offset: 8,
                    len: 2
                },
            ]
        );
    }

    #[test]
    fn safe_join_rejects_parent_escape() {
        assert!(safe_local_join(Path::new("/tmp/out"), "../secret").is_err());
        assert_eq!(
            safe_local_join(Path::new("/tmp/out"), "/content/a.txt").unwrap(),
            Path::new("/tmp/out").join("content/a.txt")
        );
    }
}
