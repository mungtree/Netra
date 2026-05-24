use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::sync::RwLock;

use crate::error::ChromaError;
use crate::server::Collection;

/// Thin async wrapper around the ChromaDB v2 HTTP API.
///
/// The base URL is held behind an `RwLock` so it can be updated when the
/// caller changes `host`/`port` in settings without rebuilding the client.
pub struct ChromaClient {
    base_url: RwLock<String>,
    http: reqwest::Client,
}

#[derive(Debug, Serialize, Deserialize)]
struct CreateCollectionBody<'a> {
    name: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    metadata: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    get_or_create: Option<bool>,
}

impl ChromaClient {
    #[must_use]
    pub fn new(base_url: String) -> Self {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .expect("reqwest client builds with default config");
        Self {
            base_url: RwLock::new(base_url),
            http,
        }
    }

    pub async fn set_base_url(&self, url: String) {
        *self.base_url.write().await = url;
    }

    async fn url(&self, path: &str) -> String {
        format!("{}{}", self.base_url.read().await, path)
    }

    /// Hit the server's heartbeat endpoint. Returns `Ok(true)` on 2xx.
    pub async fn heartbeat(&self) -> Result<bool, ChromaError> {
        let url = self.url("/api/v2/heartbeat").await;
        let resp = self.http.get(url).send().await?;
        Ok(resp.status().is_success())
    }

    /// List collections in the default tenant/database.
    pub async fn list_collections(&self) -> Result<Vec<Collection>, ChromaError> {
        let url = self
            .url("/api/v2/tenants/default_tenant/databases/default_database/collections")
            .await;
        let resp = self.http.get(url).send().await?;
        if !resp.status().is_success() {
            return Err(ChromaError::other(format!(
                "list_collections failed: HTTP {}",
                resp.status()
            )));
        }
        let raw: serde_json::Value = resp.json().await?;
        let arr = raw.as_array().cloned().unwrap_or_default();
        let mut out = Vec::with_capacity(arr.len());
        for entry in arr {
            let name = entry
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            let id = entry
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            out.push(Collection { id, name });
        }
        Ok(out)
    }

    /// Create a collection if missing. Idempotent via `get_or_create`.
    pub async fn ensure_collection(&self, name: &str) -> Result<Collection, ChromaError> {
        let url = self
            .url("/api/v2/tenants/default_tenant/databases/default_database/collections")
            .await;
        let body = CreateCollectionBody {
            name,
            metadata: None,
            get_or_create: Some(true),
        };
        let resp = self.http.post(url).json(&body).send().await?;
        if !resp.status().is_success() {
            return Err(ChromaError::other(format!(
                "ensure_collection failed: HTTP {}",
                resp.status()
            )));
        }
        let raw: serde_json::Value = resp.json().await?;
        Ok(Collection {
            id: raw
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string(),
            name: name.to_string(),
        })
    }

    pub async fn delete_collection(&self, name: &str) -> Result<(), ChromaError> {
        let url = self
            .url(&format!(
                "/api/v2/tenants/default_tenant/databases/default_database/collections/{name}"
            ))
            .await;
        let resp = self.http.delete(url).send().await?;
        if !resp.status().is_success() && resp.status().as_u16() != 404 {
            return Err(ChromaError::other(format!(
                "delete_collection failed: HTTP {}",
                resp.status()
            )));
        }
        Ok(())
    }

    /// Upsert a batch of documents into a collection.
    ///
    /// `ids`, `documents`, and `metadatas` must have equal length.
    pub async fn upsert(
        &self,
        collection_id: &str,
        ids: &[String],
        documents: &[String],
        metadatas: &[serde_json::Value],
    ) -> Result<(), ChromaError> {
        let url = self
            .url(&format!(
                "/api/v2/tenants/default_tenant/databases/default_database/collections/{collection_id}/upsert"
            ))
            .await;
        let body = json!({
            "ids": ids,
            "documents": documents,
            "metadatas": metadatas,
        });
        let resp = self.http.post(url).json(&body).send().await?;
        if !resp.status().is_success() {
            return Err(ChromaError::other(format!(
                "upsert failed: HTTP {}",
                resp.status()
            )));
        }
        Ok(())
    }

    /// Get all distinct file paths recorded in a collection (read by paging
    /// through `metadatas`). Best-effort — used for the "indexed files" view.
    pub async fn collection_files(
        &self,
        collection_id: &str,
    ) -> Result<Vec<crate::indexer::IndexedFile>, ChromaError> {
        let url = self
            .url(&format!(
                "/api/v2/tenants/default_tenant/databases/default_database/collections/{collection_id}/get"
            ))
            .await;
        let body = json!({
            "include": ["metadatas"],
            "limit": 100_000,
        });
        let resp = self.http.post(url).json(&body).send().await?;
        if !resp.status().is_success() {
            return Err(ChromaError::other(format!(
                "collection_files failed: HTTP {}",
                resp.status()
            )));
        }
        let raw: serde_json::Value = resp.json().await?;
        let metas = raw
            .get("metadatas")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();
        use std::collections::HashMap;
        let mut files: HashMap<String, crate::indexer::IndexedFile> = HashMap::new();
        for m in metas {
            let path = m
                .get("path")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            if path.is_empty() {
                continue;
            }
            let size = m.get("size").and_then(|v| v.as_u64()).unwrap_or(0);
            files
                .entry(path.clone())
                .and_modify(|f| f.chunk_count += 1)
                .or_insert(crate::indexer::IndexedFile {
                    path,
                    chunk_count: 1,
                    size_bytes: size,
                });
        }
        let mut out: Vec<_> = files.into_values().collect();
        out.sort_by(|a, b| a.path.cmp(&b.path));
        Ok(out)
    }
}
