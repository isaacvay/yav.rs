//! # YAV Resumable Chunked Upload Server (Tus Protocol v1.0.0)
//!
//! Provides chunked and resumable file uploads with offset validation,
//! metadata decoding, and zero-exhaustion chunk streaming.

use std::collections::HashMap;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use std::time::{SystemTime, UNIX_EPOCH};

/// Standard Tus protocol version supported.
pub const TUS_RESUMABLE_VERSION: &str = "1.0.0";

/// Maximum allowed buffer size (64 MB) for in-memory upload sessions to prevent memory exhaustion (DoS).
/// For uploads exceeding this threshold, specify a dedicated spool directory with [`ChunkedUploader::with_spool_dir`].
pub const MAX_IN_MEMORY_UPLOAD_BYTES: u64 = 64 * 1024 * 1024;

/// Error outcomes for Tus protocol operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TusError {
    SessionNotFound(String),
    OffsetMismatch { expected: u64, received: u64 },
    ExceedsTotalSize { total: u64, attempted: u64 },
    ExceedsMemoryLimit { max_allowed: u64 },
    SessionAlreadyCompleted(String),
    InvalidMetadata,
    IoError(String),
}

impl std::fmt::Display for TusError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SessionNotFound(id) => write!(f, "Tus upload session '{id}' not found"),
            Self::OffsetMismatch { expected, received } => {
                write!(
                    f,
                    "Offset mismatch: expected {expected}, received {received}"
                )
            }
            Self::ExceedsTotalSize { total, attempted } => write!(
                f,
                "Uploaded data exceeds declared total size: {attempted} > {total}"
            ),
            Self::ExceedsMemoryLimit { max_allowed } => write!(
                f,
                "In-memory upload size limit exceeded: {max_allowed} bytes (use spool_dir for larger uploads)"
            ),
            Self::SessionAlreadyCompleted(id) => {
                write!(f, "Upload session '{id}' is already completed")
            }
            Self::InvalidMetadata => write!(f, "Malformed Upload-Metadata header"),
            Self::IoError(err) => write!(f, "I/O error during chunk persistence: {err}"),
        }
    }
}

impl std::error::Error for TusError {}

/// Represents an ongoing or completed resumable upload session.
#[derive(Debug, Clone, PartialEq)]
pub struct UploadSession {
    pub id: String,
    pub total_size: u64,
    pub current_offset: u64,
    pub metadata: HashMap<String, String>,
    pub created_at_secs: u64,
    pub is_completed: bool,
    pub data: Vec<u8>,
}

impl UploadSession {
    #[must_use]
    pub fn new(id: impl Into<String>, total_size: u64, metadata: HashMap<String, String>) -> Self {
        let created_at_secs = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        Self {
            id: id.into(),
            total_size,
            current_offset: 0,
            metadata,
            created_at_secs,
            is_completed: false,
            data: Vec::with_capacity(total_size.min(1024 * 1024) as usize),
        }
    }

    /// Check if the session is fully uploaded.
    #[must_use]
    pub fn progress_pct(&self) -> f64 {
        if self.total_size == 0 {
            100.0
        } else {
            (self.current_offset as f64 / self.total_size as f64) * 100.0
        }
    }
}

/// Central coordinator managing Tus upload sessions in memory or spooled to disk.
#[derive(Debug, Clone, Default)]
pub struct ChunkedUploader {
    sessions: Arc<RwLock<HashMap<String, UploadSession>>>,
    spool_dir: Option<PathBuf>,
}

impl ChunkedUploader {
    #[must_use]
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            spool_dir: None,
        }
    }

    /// Create an uploader with a dedicated disk spool directory to prevent memory exhaustion (OOM).
    pub fn with_spool_dir(spool_dir: impl Into<PathBuf>) -> Self {
        let dir = spool_dir.into();
        let _ = std::fs::create_dir_all(&dir);
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            spool_dir: Some(dir),
        }
    }

    /// Initiate a new upload session (Tus POST).
    pub fn create_session(
        &self,
        session_id: impl Into<String>,
        total_size: u64,
        metadata: HashMap<String, String>,
    ) -> UploadSession {
        let session = UploadSession::new(session_id, total_size, metadata);
        let mut map = self.sessions.write().unwrap_or_else(|e| e.into_inner());
        map.insert(session.id.clone(), session.clone());
        session
    }

    /// Retrieve the current upload offset (Tus HEAD).
    pub fn get_offset(&self, session_id: &str) -> Option<u64> {
        let map = self.sessions.read().unwrap_or_else(|e| e.into_inner());
        map.get(session_id).map(|s| s.current_offset)
    }

    /// Append a chunk of bytes to an active session (Tus PATCH).
    pub fn append_chunk(
        &self,
        session_id: &str,
        upload_offset: u64,
        chunk: &[u8],
    ) -> Result<u64, TusError> {
        let (spool_dir, total_size) = {
            let mut map = self.sessions.write().unwrap_or_else(|e| e.into_inner());
            let session = map
                .get_mut(session_id)
                .ok_or_else(|| TusError::SessionNotFound(session_id.to_string()))?;

            if session.is_completed {
                return Err(TusError::SessionAlreadyCompleted(session_id.to_string()));
            }
            if upload_offset != session.current_offset {
                return Err(TusError::OffsetMismatch {
                    expected: session.current_offset,
                    received: upload_offset,
                });
            }
            let new_offset = session
                .current_offset
                .checked_add(chunk.len() as u64)
                .ok_or(TusError::ExceedsTotalSize {
                    total: session.total_size,
                    attempted: u64::MAX,
                })?;

            if self.spool_dir.is_none() && new_offset > MAX_IN_MEMORY_UPLOAD_BYTES {
                return Err(TusError::ExceedsMemoryLimit {
                    max_allowed: MAX_IN_MEMORY_UPLOAD_BYTES,
                });
            }
            if new_offset > session.total_size {
                return Err(TusError::ExceedsTotalSize {
                    total: session.total_size,
                    attempted: new_offset,
                });
            }

            if self.spool_dir.is_none() {
                session.data.extend_from_slice(chunk);
                session.current_offset = new_offset;
                if session.current_offset == session.total_size {
                    session.is_completed = true;
                }
                return Ok(session.current_offset);
            }
            (self.spool_dir.clone(), session.total_size)
        };

        // Perform blocking file I/O WITHOUT holding the sessions RwLock write guard
        if let Some(ref dir) = spool_dir {
            let file_path = safe_spool_path(dir, session_id)?;
            let mut file = OpenOptions::new()
                .create(true)
                .append(true)
                .open(&file_path)
                .map_err(|e| TusError::IoError(e.to_string()))?;
            file.write_all(chunk)
                .map_err(|e| TusError::IoError(e.to_string()))?;
        }

        let mut map = self.sessions.write().unwrap_or_else(|e| e.into_inner());
        let session = map
            .get_mut(session_id)
            .ok_or_else(|| TusError::SessionNotFound(session_id.to_string()))?;
        let new_offset = upload_offset.saturating_add(chunk.len() as u64);
        session.current_offset = new_offset.min(total_size);
        if session.current_offset == session.total_size {
            session.is_completed = true;
        }
        Ok(session.current_offset)
    }

    /// Terminate and remove an upload session (Tus DELETE).
    pub fn delete_session(&self, session_id: &str) -> bool {
        if let Some(ref dir) = self.spool_dir {
            if let Ok(file_path) = safe_spool_path(dir, session_id) {
                let _ = std::fs::remove_file(file_path);
            }
        }
        let mut map = self.sessions.write().unwrap_or_else(|e| e.into_inner());
        map.remove(session_id).is_some()
    }

    /// Consume completed uploaded data.
    pub fn take_completed_data(&self, session_id: &str) -> Option<Vec<u8>> {
        let is_spooled_done = {
            let map = self.sessions.read().unwrap_or_else(|e| e.into_inner());
            map.get(session_id).is_some_and(|s| s.is_completed)
        };
        if !is_spooled_done {
            return None;
        }

        if let Some(ref dir) = self.spool_dir {
            if let Ok(file_path) = safe_spool_path(dir, session_id) {
                if let Ok(bytes) = std::fs::read(&file_path) {
                    let _ = std::fs::remove_file(file_path);
                    return Some(bytes);
                }
            }
        }
        let mut map = self.sessions.write().unwrap_or_else(|e| e.into_inner());
        let session = map.get_mut(session_id)?;
        if session.is_completed {
            Some(std::mem::take(&mut session.data))
        } else {
            None
        }
    }
}

fn safe_spool_path(dir: &std::path::Path, session_id: &str) -> Result<PathBuf, TusError> {
    if session_id.is_empty()
        || session_id.contains("..")
        || session_id.contains('/')
        || session_id.contains('\\')
    {
        return Err(TusError::SessionNotFound("Invalid session ID".into()));
    }
    // Collision-free lowercase hex encoding preserves distinct session IDs without stripping characters
    let mut hex_name = String::with_capacity(session_id.len() * 2 + 4);
    hex_name.push_str("tus_");
    for &b in session_id.as_bytes() {
        use std::fmt::Write;
        let _ = write!(&mut hex_name, "{b:02x}");
    }
    Ok(dir.join(hex_name))
}

/// Helper parsing standard Tus `Upload-Metadata` header (decoding base64 values per Tus 1.0).
#[must_use]
pub fn parse_tus_metadata(header: &str) -> HashMap<String, String> {
    use base64::Engine;
    let mut map = HashMap::new();
    for pair in header.split(',') {
        let mut parts = pair.trim().splitn(2, ' ');
        if let Some(key) = parts.next() {
            if !key.is_empty() {
                let raw_val = parts.next().unwrap_or("").trim();
                let decoded = if raw_val.is_empty() {
                    String::new()
                } else {
                    base64::engine::general_purpose::STANDARD
                        .decode(raw_val.as_bytes())
                        .ok()
                        .and_then(|bytes| String::from_utf8(bytes).ok())
                        .unwrap_or_else(|| raw_val.to_string())
                };
                map.insert(key.to_string(), decoded);
            }
        }
    }
    map
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chunked_uploader_session_lifecycle() {
        let uploader = ChunkedUploader::new();

        let mut meta = HashMap::new();
        meta.insert("filename".to_string(), "invoice.pdf".to_string());

        let session = uploader.create_session("sess_001", 10, meta);
        assert_eq!(session.current_offset, 0);
        assert_eq!(session.total_size, 10);
        assert_eq!(uploader.get_offset("sess_001"), Some(0));

        // Append 4 bytes at offset 0
        let off1 = uploader
            .append_chunk("sess_001", 0, b"1234")
            .expect("First chunk should succeed");
        assert_eq!(off1, 4);

        // Attempting to append at wrong offset 2 -> OffsetMismatch
        let err = uploader.append_chunk("sess_001", 2, b"5678").unwrap_err();
        assert_eq!(
            err,
            TusError::OffsetMismatch {
                expected: 4,
                received: 2
            }
        );

        // Append remaining 6 bytes at offset 4
        let off2 = uploader
            .append_chunk("sess_001", 4, b"567890")
            .expect("Second chunk should succeed");
        assert_eq!(off2, 10);

        let data = uploader
            .take_completed_data("sess_001")
            .expect("Data should be completed");
        assert_eq!(data, b"1234567890");

        assert!(uploader.delete_session("sess_001"));
        assert_eq!(uploader.get_offset("sess_001"), None);
    }

    #[test]
    fn test_chunked_uploader_disk_spooling() {
        let temp_dir = std::env::temp_dir().join(format!("yav_test_tus_{}", std::process::id()));
        let uploader = ChunkedUploader::with_spool_dir(&temp_dir);

        let session = uploader.create_session("sess_spool", 6, HashMap::new());
        assert_eq!(session.current_offset, 0);

        let off1 = uploader.append_chunk("sess_spool", 0, b"abc").unwrap();
        assert_eq!(off1, 3);

        let off2 = uploader.append_chunk("sess_spool", 3, b"def").unwrap();
        assert_eq!(off2, 6);

        let data = uploader.take_completed_data("sess_spool").unwrap();
        assert_eq!(data, b"abcdef");

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_chunked_uploader_in_memory_size_limit() {
        let uploader = ChunkedUploader::new();
        let oversized = MAX_IN_MEMORY_UPLOAD_BYTES + 4096;
        let _ = uploader.create_session("sess_large", oversized, HashMap::new());

        // In-memory appending exceeding MAX_IN_MEMORY_UPLOAD_BYTES is rejected
        let chunk = vec![0u8; 1024];
        // Pretend session already at MAX_IN_MEMORY_UPLOAD_BYTES
        {
            let mut map = uploader.sessions.write().unwrap();
            let sess = map.get_mut("sess_large").unwrap();
            sess.current_offset = MAX_IN_MEMORY_UPLOAD_BYTES;
        }

        let err = uploader
            .append_chunk("sess_large", MAX_IN_MEMORY_UPLOAD_BYTES, &chunk)
            .unwrap_err();
        assert_eq!(
            err,
            TusError::ExceedsMemoryLimit {
                max_allowed: MAX_IN_MEMORY_UPLOAD_BYTES
            }
        );
    }
}
