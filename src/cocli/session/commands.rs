use std::collections::HashSet;

use chrono::{Duration, Utc};
use uuid::Uuid;

use crate::cocli::config::ColabConfig;
use crate::cocli::error::{ColabError, Result};
use crate::cocli::session::client::ColabClient;
use crate::cocli::session::model::{Assignment, Shape, Variant};
use crate::cocli::session::store::{ServerStorage, StoredServer};

pub struct ServerManager {
    client: ColabClient,
    storage: ServerStorage,
}

pub struct AssignOutcome {
    pub server: StoredServer,
    pub requested_shape: Shape,
    pub reported_shape: Option<Shape>,
    pub shape_mismatch: bool,
}

impl ServerManager {
    pub fn new(client: ColabClient, config: &ColabConfig) -> Self {
        Self {
            client,
            storage: ServerStorage::new(config.servers_file()),
        }
    }

    pub async fn list(&self) -> Result<(Vec<StoredServer>, usize)> {
        let live = self.client.list_assignments().await?;
        let live_endpoints: HashSet<String> = live.iter().map(|a| a.endpoint.clone()).collect();
        let removed = self.storage.reconcile(&live_endpoints)?;

        for assignment in &live {
            if let Some(proxy) = &assignment.runtime_proxy_info
                && let Ok(Some(stored)) = self.storage.get_by_endpoint(&assignment.endpoint)
            {
                let updated = StoredServer {
                    proxy_url: proxy.url.clone(),
                    proxy_token: proxy.token.clone(),
                    token_expires_at: Utc::now()
                        + Duration::seconds(proxy.token_expires_in_seconds),
                    ..stored
                };
                let _ = self.storage.upsert(updated);
            }
        }

        let servers = self.storage.list()?;
        Ok((servers, removed.len()))
    }

    pub fn list_local(&self) -> Result<Vec<StoredServer>> {
        self.storage.list()
    }

    pub fn save_local(&self, server: StoredServer) -> Result<()> {
        self.storage.upsert(server)
    }

    /// Borrow the inner Colab API client. Lets handlers reuse the
    /// already-built `reqwest::Client` (rustls + http2 + connection
    /// pool) instead of constructing a fresh one per command, which
    /// was previously costing ~20-40 ms of cold-handshake setup on
    /// every short-lived invocation.
    pub fn client(&self) -> &ColabClient {
        &self.client
    }

    pub async fn assign(
        &self,
        label: String,
        variant: Variant,
        accelerator: Option<String>,
        shape: Shape,
    ) -> Result<AssignOutcome> {
        let notebook_hash = Uuid::new_v4();
        let (assignment, _is_new) = self
            .client
            .assign(notebook_hash, variant, accelerator.as_deref(), shape)
            .await?;

        let reported = assignment.machine_shape;
        let stored_shape = reported.unwrap_or(shape);
        let shape_mismatch = matches!(reported, Some(r) if r != shape);

        let server = self.assignment_to_stored(Uuid::new_v4(), label, &assignment, stored_shape);
        self.storage.upsert(server.clone())?;
        Ok(AssignOutcome {
            server,
            requested_shape: shape,
            reported_shape: reported,
            shape_mismatch,
        })
    }

    pub async fn reconfigure(
        &self,
        id: Uuid,
        variant: Variant,
        accelerator: Option<String>,
        shape: Shape,
    ) -> Result<AssignOutcome> {
        let existing = self
            .storage
            .get(id)?
            .ok_or_else(|| ColabError::ServerNotFound {
                endpoint: id.to_string(),
            })?;
        let label = existing.label.clone();
        self.remove(id).await?;
        self.assign(label, variant, accelerator, shape).await
    }

    pub async fn remove(&self, id: Uuid) -> Result<()> {
        let server = self
            .storage
            .get(id)?
            .ok_or_else(|| ColabError::ServerNotFound {
                endpoint: id.to_string(),
            })?;

        self.storage.remove(id)?;

        if let Ok(sessions) = self.client.list_sessions_via_tunnel(&server.endpoint).await {
            for session in sessions {
                let _ = self
                    .client
                    .delete_session(&server.proxy_url, &server.proxy_token, &session.id)
                    .await;
            }
        }

        self.client.unassign(&server.endpoint).await
    }

    pub async fn refresh(&self, id: Uuid) -> Result<StoredServer> {
        let server = self
            .storage
            .get(id)?
            .ok_or_else(|| ColabError::ServerNotFound {
                endpoint: id.to_string(),
            })?;

        let proxy_info = self.client.refresh_connection(&server.endpoint).await?;
        let updated = StoredServer {
            proxy_url: proxy_info.url.clone(),
            proxy_token: proxy_info.token.clone(),
            token_expires_at: Utc::now() + Duration::seconds(proxy_info.token_expires_in_seconds),
            ..server
        };
        self.storage.upsert(updated.clone())?;
        Ok(updated)
    }

    fn assignment_to_stored(
        &self,
        id: Uuid,
        label: String,
        assignment: &Assignment,
        shape: Shape,
    ) -> StoredServer {
        let proxy = &assignment.runtime_proxy_info;
        StoredServer {
            id,
            label,
            variant: assignment.variant,
            accelerator: assignment.accelerator.clone(),
            shape,
            endpoint: assignment.endpoint.clone(),
            proxy_url: proxy.url.clone(),
            proxy_token: proxy.token.clone(),
            token_expires_at: Utc::now() + Duration::seconds(proxy.token_expires_in_seconds),
            date_assigned: Utc::now(),
            selected_kernel_id: None,
            selected_kernel_name: None,
            kernel_language: None,
            kernel_language_version: None,
            kernel_cache_stale: false,
        }
    }
}
