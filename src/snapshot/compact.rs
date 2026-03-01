use crate::types::SnapshotNode;

/// Render snapshot nodes in compact one-line-per-node format
/// Format: `e5 [button] "Submit" interactive`
/// This is 56-64% fewer tokens than full JSON
pub fn render_compact(nodes: &[SnapshotNode], lines: &mut Vec<String>) {
    for node in nodes {
        let mut parts = vec![
            node.ref_id.clone(),
            format!("[{}]", node.role),
        ];

        if !node.name.is_empty() {
            parts.push(format!("\"{}\"", node.name));
        }

        if let Some(ref val) = node.value {
            parts.push(format!("val=\"{}\"", val));
        }

        if node.interactive {
            parts.push("interactive".to_string());
        }

        lines.push(parts.join(" "));

        // Recurse into children
        if !node.children.is_empty() {
            render_compact(&node.children, lines);
        }
    }
}
