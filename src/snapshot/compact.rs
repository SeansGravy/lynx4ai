use crate::types::SnapshotNode;

/// Render snapshot nodes in compact one-line-per-node format
/// Format: `e5 [button] "Submit" interactive`
/// This is 56-64% fewer tokens than full JSON
pub fn render_compact(nodes: &[SnapshotNode], lines: &mut Vec<String>) {
    for node in nodes {
        let mut parts = vec![node.ref_id.clone(), format!("[{}]", node.role)];

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

#[cfg(test)]
mod tests {
    use super::*;

    fn node(ref_id: &str, role: &str, name: &str, interactive: bool) -> SnapshotNode {
        SnapshotNode {
            ref_id: ref_id.to_string(),
            role: role.to_string(),
            name: name.to_string(),
            description: None,
            value: None,
            interactive,
            children: Vec::new(),
        }
    }

    #[test]
    fn basic_render() {
        let nodes = vec![
            node("e0", "button", "Submit", true),
            node("e1", "heading", "Welcome", false),
        ];
        let mut lines = Vec::new();
        render_compact(&nodes, &mut lines);
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0], r#"e0 [button] "Submit" interactive"#);
        assert_eq!(lines[1], r#"e1 [heading] "Welcome""#);
    }

    #[test]
    fn with_value() {
        let mut n = node("e0", "textbox", "Email", true);
        n.value = Some("test@example.com".to_string());
        let mut lines = Vec::new();
        render_compact(&[n], &mut lines);
        assert!(lines[0].contains(r#"val="test@example.com""#));
    }

    #[test]
    fn empty_name_omitted() {
        let n = node("e0", "generic", "", false);
        let mut lines = Vec::new();
        render_compact(&[n], &mut lines);
        assert_eq!(lines[0], "e0 [generic]");
    }

    #[test]
    fn children_rendered() {
        let mut parent = node("e0", "list", "Nav", false);
        parent.children = vec![
            node("e1", "listitem", "Home", false),
            node("e2", "listitem", "About", false),
        ];
        let mut lines = Vec::new();
        render_compact(&[parent], &mut lines);
        assert_eq!(lines.len(), 3);
        assert!(lines[1].starts_with("e1"));
        assert!(lines[2].starts_with("e2"));
    }

    #[test]
    fn empty_nodes() {
        let mut lines = Vec::new();
        render_compact(&[], &mut lines);
        assert!(lines.is_empty());
    }
}
