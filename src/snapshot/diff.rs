use std::collections::HashMap;

use crate::types::SnapshotNode;

/// Compute a human-readable diff between two snapshots
pub fn compute_diff(previous: &[SnapshotNode], current: &[SnapshotNode]) -> String {
    let prev_map: HashMap<&str, &SnapshotNode> =
        previous.iter().map(|n| (n.ref_id.as_str(), n)).collect();
    let curr_map: HashMap<&str, &SnapshotNode> =
        current.iter().map(|n| (n.ref_id.as_str(), n)).collect();

    let mut added = Vec::new();
    let mut removed = Vec::new();
    let mut changed = Vec::new();

    // Find added and changed
    for node in current {
        match prev_map.get(node.ref_id.as_str()) {
            None => added.push(&node.ref_id),
            Some(prev) => {
                if prev.name != node.name || prev.value != node.value || prev.role != node.role {
                    changed.push(&node.ref_id);
                }
            }
        }
    }

    // Find removed
    for node in previous {
        if !curr_map.contains_key(node.ref_id.as_str()) {
            removed.push(&node.ref_id);
        }
    }

    let mut lines = Vec::new();
    if !added.is_empty() {
        lines.push(format!("Added: {}", added.iter().map(|r| r.as_str()).collect::<Vec<_>>().join(", ")));
    }
    if !removed.is_empty() {
        lines.push(format!("Removed: {}", removed.iter().map(|r| r.as_str()).collect::<Vec<_>>().join(", ")));
    }
    if !changed.is_empty() {
        lines.push(format!("Changed: {}", changed.iter().map(|r| r.as_str()).collect::<Vec<_>>().join(", ")));
    }
    if lines.is_empty() {
        lines.push("No changes detected".to_string());
    }

    lines.join("\n")
}
