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
        lines.push(format!(
            "Added: {}",
            added
                .iter()
                .map(|r| r.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    if !removed.is_empty() {
        lines.push(format!(
            "Removed: {}",
            removed
                .iter()
                .map(|r| r.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    if !changed.is_empty() {
        lines.push(format!(
            "Changed: {}",
            changed
                .iter()
                .map(|r| r.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    if lines.is_empty() {
        lines.push("No changes detected".to_string());
    }

    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node(ref_id: &str, role: &str, name: &str) -> SnapshotNode {
        SnapshotNode {
            ref_id: ref_id.to_string(),
            role: role.to_string(),
            name: name.to_string(),
            description: None,
            value: None,
            interactive: false,
            children: Vec::new(),
        }
    }

    #[test]
    fn no_changes() {
        let a = vec![node("e0", "button", "OK")];
        let b = vec![node("e0", "button", "OK")];
        assert_eq!(compute_diff(&a, &b), "No changes detected");
    }

    #[test]
    fn added_node() {
        let a = vec![node("e0", "button", "OK")];
        let b = vec![node("e0", "button", "OK"), node("e5", "link", "Help")];
        let diff = compute_diff(&a, &b);
        assert!(diff.contains("Added: e5"));
        assert!(!diff.contains("Removed"));
        assert!(!diff.contains("Changed"));
    }

    #[test]
    fn removed_node() {
        let a = vec![node("e0", "button", "OK"), node("e3", "link", "Help")];
        let b = vec![node("e0", "button", "OK")];
        let diff = compute_diff(&a, &b);
        assert!(diff.contains("Removed: e3"));
    }

    #[test]
    fn changed_name() {
        let a = vec![node("e0", "button", "Submit")];
        let b = vec![node("e0", "button", "Loading...")];
        let diff = compute_diff(&a, &b);
        assert!(diff.contains("Changed: e0"));
    }

    #[test]
    fn changed_role() {
        let a = vec![node("e0", "button", "X")];
        let b = vec![node("e0", "link", "X")];
        let diff = compute_diff(&a, &b);
        assert!(diff.contains("Changed: e0"));
    }

    #[test]
    fn changed_value() {
        let mut a = node("e0", "textbox", "Email");
        a.value = Some("old@test.com".to_string());
        let mut b = node("e0", "textbox", "Email");
        b.value = Some("new@test.com".to_string());
        let diff = compute_diff(&[a], &[b]);
        assert!(diff.contains("Changed: e0"));
    }

    #[test]
    fn sparse_refs_stable() {
        // Simulates persistent refs: e0 stays, e3 stays, e7 is new
        let a = vec![node("e0", "button", "OK"), node("e3", "link", "Home")];
        let b = vec![
            node("e0", "button", "OK"),
            node("e3", "link", "Home"),
            node("e7", "textbox", "Search"),
        ];
        let diff = compute_diff(&a, &b);
        assert!(diff.contains("Added: e7"));
        assert!(!diff.contains("Changed"));
        assert!(!diff.contains("Removed"));
    }

    #[test]
    fn both_empty() {
        assert_eq!(compute_diff(&[], &[]), "No changes detected");
    }
}
