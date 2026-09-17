//! Object storage abstraction for S3-compatible backends (AWS S3, MinIO, R2, ...).
//!
//! PostgreSQL stores document *metadata* and the storage key; binaries never go
//! into the database. Clients upload/download directly with short-lived
//! presigned URLs so large files never pass through the API process.
//!
//! TODO: implement `S3ObjectStorage` (crate `aws-sdk-s3` with a custom endpoint
//! for MinIO) when the documents module is built, and add it to `AppState`.

use std::time::Duration;

use async_trait::async_trait;

/// Opaque key inside the bucket, e.g. `suppliers/{supplier_id}/verification/{document_id}`.
/// Keys are generated server-side; never derive them from user-supplied filenames.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StorageKey(String);

impl StorageKey {
    pub fn new(key: impl Into<String>) -> Self {
        Self(key.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone)]
pub struct PresignedUrl {
    pub url: String,
    pub expires_in: Duration,
}

#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("object not found")]
    NotFound,

    #[error("storage backend error: {0}")]
    Backend(#[source] anyhow::Error),
}

#[async_trait]
pub trait ObjectStorage: Send + Sync + 'static {
    /// Server-side upload for generated files (invoice PDFs, reports).
    async fn put_object(
        &self,
        key: &StorageKey,
        bytes: Vec<u8>,
        content_type: &str,
    ) -> Result<(), StorageError>;

    async fn delete_object(&self, key: &StorageKey) -> Result<(), StorageError>;

    /// Direct browser upload. The documents module must validate size and MIME
    /// type after upload (magic bytes) before marking the document as available.
    async fn presigned_upload_url(
        &self,
        key: &StorageKey,
        content_type: &str,
        expires_in: Duration,
    ) -> Result<PresignedUrl, StorageError>;

    /// Issued only after the documents module has checked the caller's permission.
    async fn presigned_download_url(
        &self,
        key: &StorageKey,
        expires_in: Duration,
    ) -> Result<PresignedUrl, StorageError>;
}
