//! Relationship-Based Access Control (ReBAC) / Google Zanzibar Engine.
//!
//! Provides granular authorization graph traversal, permission inheritance,
//! and multi-tenant resource access checks (`subject#relation@object`).

use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, RwLock},
};

use serde::{Deserialize, Serialize};

/// A stored relationship assertion in the Zanzibar authorization graph.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RelationshipTuple {
    /// Subject entity (e.g. `"user:alice"` or `"team:engineering#member"`).
    pub subject: String,
    /// Relationship name (e.g. `"owner"`, `"editor"`, `"viewer"`, `"member"`).
    pub relation: String,
    /// Object entity (e.g. `"doc:roadmap"`, `"folder:finance"`, `"workspace:acme"`).
    pub object: String,
}

impl RelationshipTuple {
    #[must_use]
    pub fn new(
        subject: impl Into<String>,
        relation: impl Into<String>,
        object: impl Into<String>,
    ) -> Self {
        Self {
            subject: subject.into(),
            relation: relation.into(),
            object: object.into(),
        }
    }
}

/// Authorization engine evaluating relationships, roles, and hierarchical inheritance.
#[derive(Debug, Clone, Default)]
pub struct RebacGraph {
    tuples: Arc<RwLock<HashSet<RelationshipTuple>>>,
    /// Implication rules: relation -> list of implied permissions (e.g. "owner" -> ["editor", "viewer"]).
    implications: Arc<RwLock<HashMap<String, Vec<String>>>>,
    /// Parent link relation name (e.g. `"parent"` or `"folder"`).
    parent_relation: String,
}

impl RebacGraph {
    /// Create a new ReBAC authorization graph.
    #[must_use]
    pub fn new() -> Self {
        let mut implications = HashMap::new();
        implications.insert(
            "owner".to_string(),
            vec!["editor".to_string(), "viewer".to_string()],
        );
        implications.insert("editor".to_string(), vec!["viewer".to_string()]);

        Self {
            tuples: Arc::new(RwLock::new(HashSet::new())),
            implications: Arc::new(RwLock::new(implications)),
            parent_relation: "parent".to_string(),
        }
    }

    /// Add an implication rule: having `base_relation` also grants `implied_relation`.
    pub fn define_implication(&self, base_relation: &str, implied_relation: &str) {
        let mut imp = self.implications.write().expect("Lock poisoned");
        imp.entry(base_relation.to_string())
            .or_default()
            .push(implied_relation.to_string());
    }

    /// Add a relationship tuple to the graph.
    pub fn write_tuple(
        &self,
        subject: impl Into<String>,
        relation: impl Into<String>,
        object: impl Into<String>,
    ) {
        let mut set = self.tuples.write().expect("Lock poisoned");
        set.insert(RelationshipTuple::new(subject, relation, object));
    }

    /// Delete a relationship tuple from the graph.
    pub fn delete_tuple(&self, subject: &str, relation: &str, object: &str) -> bool {
        let mut set = self.tuples.write().expect("Lock poisoned");
        set.remove(&RelationshipTuple::new(subject, relation, object))
    }

    /// Check if a subject has the requested relation/permission on the given object.
    #[must_use]
    pub fn check(&self, subject: &str, relation: &str, object: &str) -> bool {
        let tuples = self.tuples.read().expect("Lock poisoned");
        let implications = self.implications.read().expect("Lock poisoned");
        let mut visited = HashSet::new();

        self.check_recursive(
            &tuples,
            &implications,
            subject,
            relation,
            object,
            &mut visited,
        )
    }

    fn check_recursive(
        &self,
        tuples: &HashSet<RelationshipTuple>,
        implications: &HashMap<String, Vec<String>>,
        subject: &str,
        relation: &str,
        object: &str,
        visited: &mut HashSet<(String, String, String)>,
    ) -> bool {
        let visit_key = (
            subject.to_string(),
            relation.to_string(),
            object.to_string(),
        );
        if !visited.insert(visit_key) {
            return false; // Cycle detection
        }

        // 1. Direct tuple match: subject#relation@object
        if tuples.contains(&RelationshipTuple::new(subject, relation, object)) {
            return true;
        }

        // 2. Implied relations (e.g. if checking "viewer", user might have "editor" or "owner")
        for (higher_rel, implied_list) in implications {
            if implied_list.iter().any(|r| r == relation)
                && self.check_recursive(tuples, implications, subject, higher_rel, object, visited)
            {
                return true;
            }
        }

        // 3. Subject-set delegation: (e.g. subject is a member of a group that has the permission)
        // Find all groups where subject is member: subject#member@group
        for t in tuples.iter() {
            if t.subject == subject && (t.relation == "member" || t.relation == "member_of") {
                let group_subject = format!("{}#member", t.object);
                if self.check_recursive(
                    tuples,
                    implications,
                    &group_subject,
                    relation,
                    object,
                    visited,
                ) {
                    return true;
                }
            }
        }

        // 4. Hierarchical parent inheritance (e.g. object belongs to a folder/workspace that user can access)
        for t in tuples.iter() {
            if t.object == object && t.relation == self.parent_relation {
                let parent_object = &t.subject;
                if self.check_recursive(
                    tuples,
                    implications,
                    subject,
                    relation,
                    parent_object,
                    visited,
                ) {
                    return true;
                }
            }
        }

        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rebac_direct_and_implied_permissions() {
        let rebac = RebacGraph::new();

        rebac.write_tuple("user:alice", "owner", "doc:q4_strategy");
        rebac.write_tuple("user:bob", "viewer", "doc:q4_strategy");

        // Alice is owner, which implies editor and viewer
        assert!(rebac.check("user:alice", "owner", "doc:q4_strategy"));
        assert!(rebac.check("user:alice", "editor", "doc:q4_strategy"));
        assert!(rebac.check("user:alice", "viewer", "doc:q4_strategy"));

        // Bob is only viewer
        assert!(rebac.check("user:bob", "viewer", "doc:q4_strategy"));
        assert!(!rebac.check("user:bob", "editor", "doc:q4_strategy"));
        assert!(!rebac.check("user:bob", "owner", "doc:q4_strategy"));
    }

    #[test]
    fn test_rebac_hierarchical_folder_inheritance() {
        let rebac = RebacGraph::new();

        // Folder belongs to Workspace, Doc belongs to Folder
        rebac.write_tuple("folder:tech", "parent", "doc:architecture");
        rebac.write_tuple("user:carol", "editor", "folder:tech");

        // Carol has editor permission on folder, which inherits down to doc
        assert!(rebac.check("user:carol", "editor", "doc:architecture"));
        assert!(rebac.check("user:carol", "viewer", "doc:architecture"));
        assert!(!rebac.check("user:carol", "owner", "doc:architecture"));
    }
}
