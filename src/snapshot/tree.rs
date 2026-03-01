use chromiumoxide::cdp::browser_protocol::accessibility::{
    AxValue, GetFullAxTreeParams, GetFullAxTreeReturns,
};
use chromiumoxide::Page;

use crate::browser::instance::RefMap;
use crate::error::LynxError;
use crate::types::SnapshotNode;

/// Interactive accessibility roles
const INTERACTIVE_ROLES: &[&str] = &[
    "button",
    "link",
    "textbox",
    "checkbox",
    "radio",
    "combobox",
    "listbox",
    "menuitem",
    "menuitemcheckbox",
    "menuitemradio",
    "option",
    "searchbox",
    "slider",
    "spinbutton",
    "switch",
    "tab",
    "treeitem",
];

fn is_interactive(role: &str) -> bool {
    INTERACTIVE_ROLES.contains(&role.to_lowercase().as_str())
}

fn ax_value_to_string(value: &Option<AxValue>) -> String {
    match value {
        Some(v) => {
            if let Some(val) = &v.value {
                match val {
                    serde_json::Value::String(s) => s.clone(),
                    other => other.to_string(),
                }
            } else {
                String::new()
            }
        }
        None => String::new(),
    }
}

/// Build a snapshot of the page's accessibility tree
pub async fn build_snapshot(
    page: &Page,
    interactive_only: bool,
) -> Result<(Vec<SnapshotNode>, RefMap), LynxError> {
    // Fetch the full accessibility tree via CDP
    let params = GetFullAxTreeParams::default();
    let response: GetFullAxTreeReturns = page
        .execute(params)
        .await
        .map_err(|e| LynxError::Snapshot(format!("getFullAXTree failed: {e}")))?
        .result;

    let ax_nodes = &response.nodes;

    let mut ref_map = RefMap::new();
    let mut snapshot_nodes = Vec::new();

    for node in ax_nodes {
        // Skip ignored nodes
        if node.ignored {
            continue;
        }

        let role = ax_value_to_string(&node.role);
        if role.is_empty() || role == "none" || role == "generic" {
            continue;
        }

        let name = ax_value_to_string(&node.name);
        let description = {
            let d = ax_value_to_string(&node.description);
            if d.is_empty() { None } else { Some(d) }
        };
        let value = {
            let v = ax_value_to_string(&node.value);
            if v.is_empty() { None } else { Some(v) }
        };

        let interactive = is_interactive(&role);

        if interactive_only && !interactive {
            continue;
        }

        // Assign a stable ref for this node
        let backend_node_id = node.backend_dom_node_id.unwrap_or_default();
        let ref_id = ref_map.assign(backend_node_id);

        snapshot_nodes.push(SnapshotNode {
            ref_id,
            role,
            name,
            description,
            value,
            interactive,
            children: Vec::new(), // Flat list, no nesting for now
        });
    }

    Ok((snapshot_nodes, ref_map))
}
