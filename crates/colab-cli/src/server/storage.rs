use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::client::api::{Shape, Variant};
use crate::error::{ColabError, Result};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredServer {
    pub id: Uuid,
    pub label: String,
    pub variant: Variant,
    pub accelerator: Option<String>,
    #[serde(default)]
    pub shape: Shape,
    pub endpoint: String,
    pub proxy_url: String,
    pub proxy_token: String,
    pub token_expires_at: DateTime<Utc>,
    pub date_assigned: DateTime<Utc>,
}

pub struct ServerStorage {
    path: PathBuf,
    // memo so a single command's list() calls don't re-parse the file
    cache: Mutex<Option<Vec<StoredServer>>>,
}

impl ServerStorage {
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            cache: Mutex::new(None),
        }
    }

    pub fn list(&self) -> Result<Vec<StoredServer>> {
        if let Some(cached) = self.cache_lock()?.as_ref() {
            return Ok(cached.clone());
        }
        let fresh = self.read_from_disk()?;
        *self.cache_lock()? = Some(fresh.clone());
        Ok(fresh)
    }

    fn read_from_disk(&self) -> Result<Vec<StoredServer>> {
        match std::fs::read_to_string(&self.path) {
            Ok(json) => Ok(serde_json::from_str(&json)?),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(vec![]),
            Err(e) => Err(ColabError::Io(e)),
        }
    }

    pub fn get(&self, id: Uuid) -> Result<Option<StoredServer>> {
        Ok(self.list()?.into_iter().find(|s| s.id == id))
    }

    pub fn get_by_endpoint(&self, endpoint: &str) -> Result<Option<StoredServer>> {
        Ok(self.list()?.into_iter().find(|s| s.endpoint == endpoint))
    }

    pub fn upsert(&self, server: StoredServer) -> Result<()> {
        let mut servers = self.list()?;
        let pos = servers.iter().position(|s| s.id == server.id);
        match pos {
            Some(i) => {
                let original_date = servers[i].date_assigned;
                servers[i] = StoredServer {
                    date_assigned: original_date,
                    ..server
                };
            }
            None => servers.push(server),
        }
        self.write(&servers)
    }

    pub fn remove(&self, id: Uuid) -> Result<bool> {
        let mut servers = self.list()?;
        let len_before = servers.len();
        servers.retain(|s| s.id != id);
        if servers.len() == len_before {
            return Ok(false);
        }
        self.write(&servers)?;
        Ok(true)
    }

    pub fn reconcile(
        &self,
        live_endpoints: &std::collections::HashSet<String>,
    ) -> Result<Vec<StoredServer>> {
        let servers = self.list()?;
        let (keep, removed): (Vec<_>, Vec<_>) = servers
            .into_iter()
            .partition(|s| live_endpoints.contains(&s.endpoint));
        if !removed.is_empty() {
            self.write(&keep)?;
        }
        Ok(removed)
    }

    fn write(&self, servers: &[StoredServer]) -> Result<()> {
        let mut sorted = servers.to_vec();
        sorted.sort_by_key(|s| s.id);
        let json = serde_json::to_string_pretty(&sorted)?;
        let tmp = self.path.with_extension("json.tmp");
        std::fs::write(&tmp, &json)?;
        std::fs::rename(&tmp, &self.path)?;
        // keep the cache in sync with what we just wrote
        *self.cache_lock()? = Some(sorted);
        Ok(())
    }

    fn cache_lock(&self) -> Result<MutexGuard<'_, Option<Vec<StoredServer>>>> {
        self.cache
            .lock()
            .map_err(|_| ColabError::config("server storage cache poisoned"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn sample(id: Uuid, label: &str, endpoint: &str) -> StoredServer {
        StoredServer {
            id,
            label: label.into(),
            variant: Variant::Gpu,
            accelerator: Some("T4".into()),
            shape: Shape::HighMem,
            endpoint: endpoint.into(),
            proxy_url: "https://p.example".into(),
            proxy_token: "tok".into(),
            token_expires_at: Utc::now(),
            date_assigned: Utc::now(),
        }
    }

    #[test]
    fn upsert_insert_then_update_preserves_date_assigned() {
        let dir = tempdir().unwrap();
        let storage = ServerStorage::new(dir.path().join("servers.json"));
        let id = Uuid::new_v4();

        let first = sample(id, "a", "ep-1");
        let original_date = first.date_assigned;
        storage.upsert(first).unwrap();

        let mut second = sample(id, "renamed", "ep-1");
        second.date_assigned = Utc::now() + chrono::Duration::hours(1);
        storage.upsert(second).unwrap();

        let listed = storage.list().unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].label, "renamed");
        assert_eq!(listed[0].date_assigned, original_date);
    }

    #[test]
    fn remove_reports_existence() {
        let dir = tempdir().unwrap();
        let storage = ServerStorage::new(dir.path().join("servers.json"));
        let id = Uuid::new_v4();
        storage.upsert(sample(id, "a", "ep")).unwrap();
        assert!(storage.remove(id).unwrap());
        assert!(!storage.remove(id).unwrap());
    }

    #[test]
    fn reconcile_drops_stale_servers() {
        let dir = tempdir().unwrap();
        let storage = ServerStorage::new(dir.path().join("servers.json"));
        storage
            .upsert(sample(Uuid::new_v4(), "alive", "live-ep"))
            .unwrap();
        storage
            .upsert(sample(Uuid::new_v4(), "stale", "dead-ep"))
            .unwrap();

        let mut live = std::collections::HashSet::new();
        live.insert("live-ep".to_string());
        let removed = storage.reconcile(&live).unwrap();
        assert_eq!(removed.len(), 1);
        assert_eq!(removed[0].endpoint, "dead-ep");

        let remaining = storage.list().unwrap();
        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0].endpoint, "live-ep");
    }

    #[test]
    fn list_returns_empty_when_file_missing() {
        let dir = tempdir().unwrap();
        let storage = ServerStorage::new(dir.path().join("missing.json"));
        assert!(storage.list().unwrap().is_empty());
    }

    #[test]
    fn shape_round_trips_through_json() {
        let dir = tempdir().unwrap();
        let storage = ServerStorage::new(dir.path().join("servers.json"));
        let id = Uuid::new_v4();
        let mut s = sample(id, "hm", "ep");
        s.shape = Shape::HighMem;
        storage.upsert(s).unwrap();
        let loaded = storage.get(id).unwrap().unwrap();
        assert_eq!(loaded.shape, Shape::HighMem);
    }
}
