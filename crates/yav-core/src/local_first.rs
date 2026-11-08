//! Local-First Architecture & Offline-Sync Engine.
//!
//! Provides client IndexedDB mutation queues, conflict resolution protocols,
//! and server-side synchronization primitives for offline-capable web applications.

/// Type of mutation performed while in an offline or local-first state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MutationType {
    /// Create a new record or entity.
    Insert,
    /// Update an existing record.
    Update,
    /// Delete an existing record.
    Delete,
    /// Custom domain mutation action (e.g. `"archive"` or `"reorder"`).
    Custom(String),
}

impl MutationType {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Insert => "insert",
            Self::Update => "update",
            Self::Delete => "delete",
            Self::Custom(s) => s.as_str(),
        }
    }
}

/// A recorded mutation queued locally when the user operates offline or with instant local optimism.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OfflineMutation {
    /// Unique client mutation UUID.
    pub id: String,
    /// Target entity / table name (e.g. `"tasks"` or `"drafts"`).
    pub entity: String,
    /// Operation type.
    pub mutation_type: MutationType,
    /// JSON serialized mutation payload.
    pub payload: String,
    /// Client-side timestamp in milliseconds.
    pub timestamp_ms: u64,
    /// Client-side logical clock sequence number.
    pub sequence: u64,
    /// Whether mutation has been acknowledged by server.
    pub synced: bool,
}

fn escape_json_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 8);
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            _ => out.push(c),
        }
    }
    out
}

impl OfflineMutation {
    pub fn new(
        id: impl Into<String>,
        entity: impl Into<String>,
        mutation_type: MutationType,
        payload: impl Into<String>,
        timestamp_ms: u64,
        sequence: u64,
    ) -> Self {
        Self {
            id: id.into(),
            entity: entity.into(),
            mutation_type,
            payload: payload.into(),
            timestamp_ms,
            sequence,
            synced: false,
        }
    }

    /// Serialize mutation to compact JSON string.
    pub fn to_json(&self) -> String {
        let payload_json = if self.payload.starts_with('{') || self.payload.starts_with('[') {
            self.payload.clone()
        } else {
            format!("\"{}\"", escape_json_string(&self.payload))
        };
        format!(
            r#"{{"id":"{}","entity":"{}","type":"{}","payload":{},"timestamp_ms":{},"sequence":{},"synced":{}}}"#,
            escape_json_string(&self.id),
            escape_json_string(&self.entity),
            escape_json_string(self.mutation_type.as_str()),
            payload_json,
            self.timestamp_ms,
            self.sequence,
            self.synced
        )
    }
}

/// Status of an individual mutation reconciliation on the server.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncStatus {
    /// Mutation applied successfully without conflicts.
    Applied,
    /// Mutation encountered a concurrent edit and was resolved via LWW / CRDT.
    ConflictResolved,
    /// Mutation was rejected (e.g. validation failure or authorization).
    Rejected,
}

impl SyncStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Applied => "applied",
            Self::ConflictResolved => "conflict_resolved",
            Self::Rejected => "rejected",
        }
    }
}

/// Server acknowledgement and resolution for a single mutation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyncResolution {
    pub mutation_id: String,
    pub status: SyncStatus,
    pub server_version: Option<u64>,
    pub message: Option<String>,
}

/// Batch of offline mutations submitted from client to server.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyncBatch {
    pub client_id: String,
    pub last_sync_timestamp: u64,
    pub mutations: Vec<OfflineMutation>,
}

/// Server response containing resolution status for all submitted mutations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyncResponse {
    pub resolutions: Vec<SyncResolution>,
    pub server_timestamp: u64,
}

impl SyncResponse {
    pub fn new(server_timestamp: u64) -> Self {
        Self {
            resolutions: Vec::new(),
            server_timestamp,
        }
    }

    pub fn add_resolution(
        &mut self,
        mutation_id: impl Into<String>,
        status: SyncStatus,
        server_version: Option<u64>,
        message: Option<String>,
    ) {
        self.resolutions.push(SyncResolution {
            mutation_id: mutation_id.into(),
            status,
            server_version,
            message,
        });
    }

    /// Format response as JSON string.
    pub fn to_json(&self) -> String {
        let mut parts = Vec::new();
        for r in &self.resolutions {
            let ver = r
                .server_version
                .map(|v| v.to_string())
                .unwrap_or_else(|| "null".to_string());
            let msg = r
                .message
                .as_ref()
                .map(|m| format!("\"{}\"", escape_json_string(m)))
                .unwrap_or_else(|| "null".to_string());
            parts.push(format!(
                r#"{{"mutation_id":"{}","status":"{}","server_version":{},"message":{}}}"#,
                escape_json_string(&r.mutation_id),
                escape_json_string(r.status.as_str()),
                ver,
                msg
            ));
        }

        format!(
            r#"{{"server_timestamp":{},"resolutions":[{}]}}"#,
            self.server_timestamp,
            parts.join(",")
        )
    }
}

/// Configuration for the client IndexedDB store and sync worker.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalStoreConfig {
    pub db_name: String,
    pub store_name: String,
    pub version: u32,
    pub sync_url: String,
}

impl Default for LocalStoreConfig {
    fn default() -> Self {
        Self {
            db_name: "yav_offline_db".to_string(),
            store_name: "mutation_queue".to_string(),
            version: 1,
            sync_url: "/api/sync".to_string(),
        }
    }
}

/// Client runtime script providing transparent IndexedDB offline queues and auto-sync on reconnect.
pub const YAV_LOCAL_FIRST_SCRIPT: &str = r#"<script>(function(){
  var DB_NAME = 'yav_local_first_db';
  var STORE_NAME = 'mutations';
  var seq = 0;

  function openDb() {
    return new Promise(function(resolve, reject) {
      if (!window.indexedDB) { return resolve(null); }
      var req = indexedDB.open(DB_NAME, 1);
      req.onupgradeneeded = function(e) {
        var db = e.target.result;
        if (!db.objectStoreNames.contains(STORE_NAME)) {
          db.createObjectStore(STORE_NAME, { keyPath: 'id' });
        }
      };
      req.onsuccess = function(e) { resolve(e.target.result); };
      req.onerror = function(e) { resolve(null); };
    });
  }

  function queueMutation(entity, type, payload) {
    seq++;
    var mut = {
      id: 'mut_' + Date.now() + '_' + Math.random().toString(36).substr(2, 9),
      entity: entity,
      mutation_type: type,
      payload: JSON.stringify(payload),
      timestamp_ms: Date.now(),
      sequence: seq,
      synced: false
    };

    openDb().then(function(db) {
      if (!db) return;
      var tx = db.transaction([STORE_NAME], 'readwrite');
      tx.objectStore(STORE_NAME).add(mut);
      if (navigator.onLine) {
        triggerSync();
      }
    });
  }

  function triggerSync() {
    openDb().then(function(db) {
      if (!db) return;
      var tx = db.transaction([STORE_NAME], 'readonly');
      var store = tx.objectStore(STORE_NAME);
      var req = store.getAll();
      req.onsuccess = function() {
        var items = req.result || [];
        var pending = items.filter(function(m){ return !m.synced; });
        if (pending.length === 0) return;

        var syncUrl = document.body.getAttribute('yav:offline-sync') || '/api/sync';
        fetch(syncUrl, {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({
            client_id: localStorage.getItem('yav_client_id') || 'c_' + Math.random().toString(36).substr(2, 9),
            last_sync_timestamp: Date.now(),
            mutations: pending
          })
        }).then(function(res) {
          if (res.ok) {
            return res.json();
          }
        }).then(function(data) {
          if (!data || !data.resolutions) return;
          var rwTx = db.transaction([STORE_NAME], 'readwrite');
          var rwStore = rwTx.objectStore(STORE_NAME);
          data.resolutions.forEach(function(r) {
            if (r.status === 'applied' || r.status === 'conflict_resolved') {
              rwStore.delete(r.mutation_id);
            }
          });
        }).catch(function(err){});
      };
    });
  }

  window.addEventListener('online', triggerSync);

  document.addEventListener('submit', function(e) {
    var form = e.target;
    var localStore = form.getAttribute('yav:local-store');
    if (!localStore) return;

    if (!navigator.onLine) {
      e.preventDefault();
      var formData = new FormData(form);
      var obj = {};
      formData.forEach(function(val, key){ obj[key] = val; });
      queueMutation(localStore, 'insert', obj);

      var toast = document.createElement('div');
      toast.className = 'fixed bottom-4 right-4 bg-amber-600 text-white px-4 py-2 rounded-lg shadow-xl text-sm z-50';
      toast.textContent = 'Enregistré hors ligne. Synchronisation dès reconnexion.';
      document.body.appendChild(toast);
      setTimeout(function(){ toast.remove(); }, 3500);
    }
  });

  window.__yavQueueOfflineMutation = queueMutation;
  window.__yavTriggerSync = triggerSync;
})();</script>"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_offline_mutation_lifecycle() {
        let mut mutation = OfflineMutation::new(
            "m_1",
            "todos",
            MutationType::Insert,
            r#"{"title":"Acheter du pain"}"#,
            1700000000,
            1,
        );
        assert_eq!(mutation.entity, "todos");
        assert!(!mutation.synced);
        mutation.synced = true;
        assert!(mutation.synced);
        let json = mutation.to_json();
        assert!(json.contains("todos"));
        assert!(json.contains("insert"));
    }

    #[test]
    fn test_sync_response_building() {
        let mut response = SyncResponse::new(1700000050);
        response.add_resolution("m_1", SyncStatus::Applied, Some(2), None);
        response.add_resolution(
            "m_2",
            SyncStatus::ConflictResolved,
            Some(3),
            Some("LWW rule applied".to_string()),
        );

        assert_eq!(response.resolutions.len(), 2);
        assert_eq!(response.resolutions[0].status, SyncStatus::Applied);
        assert_eq!(response.resolutions[1].status, SyncStatus::ConflictResolved);
        let json = response.to_json();
        assert!(json.contains("applied"));
        assert!(json.contains("conflict_resolved"));
    }

    #[test]
    fn test_script_structure() {
        assert!(YAV_LOCAL_FIRST_SCRIPT.contains("indexedDB"));
        assert!(YAV_LOCAL_FIRST_SCRIPT.contains("window.addEventListener('online'"));
    }
}
