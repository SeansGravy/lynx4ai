use serde::{Deserialize, Serialize};

/// Unique identifier for a browser instance
pub type InstanceId = String;

/// A node in the compacted accessibility tree
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotNode {
    /// Stable element reference (e.g., "e0", "e5", "e42")
    pub ref_id: String,
    /// Accessibility role (button, textbox, link, heading, etc.)
    pub role: String,
    /// Accessible name
    pub name: String,
    /// Accessible description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Current value (for inputs, selects, etc.)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    /// Whether this element is interactive (clickable, typeable)
    #[serde(skip_serializing_if = "is_false")]
    pub interactive: bool,
    /// Child nodes
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub children: Vec<SnapshotNode>,
}

fn is_false(v: &bool) -> bool {
    !v
}

/// Snapshot result returned to the agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotResult {
    pub url: String,
    pub title: String,
    pub nodes: Vec<SnapshotNode>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diff_summary: Option<String>,
    pub total_refs: usize,
    pub interactive_refs: usize,
}

/// Instance metadata for list_instances
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstanceInfo {
    pub id: InstanceId,
    pub profile: String,
    pub url: String,
    pub created_at: String,
    pub headless: bool,
    /// "alive" or "dead" — dead means Chrome process exited
    pub status: String,
}
