use chromiumoxide::cdp::browser_protocol::accessibility::{
    AxValue, EnableParams as AxEnableParams, GetFullAxTreeParams, GetFullAxTreeReturns,
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

/// JavaScript that walks the DOM and extracts accessibility info.
/// Uses computedRole/computedName (Chrome 90+) with aria-* fallbacks.
/// Returns JSON array of {role, name, tag, interactive} objects.
const JS_SNAPSHOT: &str = r#"
(function() {
    const INTERACTIVE_TAGS = new Set(['A','BUTTON','INPUT','SELECT','TEXTAREA','DETAILS','SUMMARY']);
    const INTERACTIVE_ROLES = new Set([
        'button','link','textbox','checkbox','radio','combobox','listbox',
        'menuitem','menuitemcheckbox','menuitemradio','option','searchbox',
        'slider','spinbutton','switch','tab','treeitem'
    ]);
    const SKIP_TAGS = new Set(['SCRIPT','STYLE','NOSCRIPT','SVG','PATH','META','LINK','BR','HR']);

    function getRole(el) {
        if (el.computedRole && el.computedRole !== 'generic' && el.computedRole !== 'none') {
            return el.computedRole;
        }
        const ariaRole = el.getAttribute('role');
        if (ariaRole && ariaRole !== 'none' && ariaRole !== 'presentation') return ariaRole;
        const tag = el.tagName;
        if (tag === 'A' && el.href) return 'link';
        if (tag === 'BUTTON') return 'button';
        if (tag === 'INPUT') {
            const t = (el.type || 'text').toLowerCase();
            if (t === 'checkbox') return 'checkbox';
            if (t === 'radio') return 'radio';
            if (t === 'submit' || t === 'button') return 'button';
            if (t === 'search') return 'searchbox';
            return 'textbox';
        }
        if (tag === 'TEXTAREA') return 'textbox';
        if (tag === 'SELECT') return 'combobox';
        if (tag === 'IMG') return 'img';
        if (tag === 'H1' || tag === 'H2' || tag === 'H3' || tag === 'H4' || tag === 'H5' || tag === 'H6') return 'heading';
        if (tag === 'NAV') return 'navigation';
        if (tag === 'MAIN') return 'main';
        if (tag === 'HEADER') return 'banner';
        if (tag === 'FOOTER') return 'contentinfo';
        if (tag === 'ASIDE') return 'complementary';
        if (tag === 'FORM') return 'form';
        if (tag === 'TABLE') return 'table';
        if (tag === 'UL' || tag === 'OL') return 'list';
        if (tag === 'LI') return 'listitem';
        return '';
    }

    function getName(el) {
        if (el.computedName) return el.computedName;
        const ariaLabel = el.getAttribute('aria-label');
        if (ariaLabel) return ariaLabel;
        const ariaLabelledBy = el.getAttribute('aria-labelledby');
        if (ariaLabelledBy) {
            const ref = document.getElementById(ariaLabelledBy);
            if (ref) return ref.textContent.trim().substring(0, 100);
        }
        if (el.tagName === 'IMG') return el.alt || '';
        if (el.tagName === 'INPUT' || el.tagName === 'TEXTAREA' || el.tagName === 'SELECT') {
            if (el.labels && el.labels.length > 0) return el.labels[0].textContent.trim().substring(0, 100);
            return el.placeholder || el.name || '';
        }
        const text = el.textContent || '';
        return text.trim().substring(0, 100);
    }

    // Clean up any previous ref tags
    document.querySelectorAll('[data-lynx-ref]').forEach(function(e) {
        e.removeAttribute('data-lynx-ref');
    });

    const nodes = [];
    let refIdx = 0;
    const walker = document.createTreeWalker(document.body, NodeFilter.SHOW_ELEMENT, null);
    let el = walker.currentNode;
    while (el) {
        if (!SKIP_TAGS.has(el.tagName)) {
            const role = getRole(el);
            if (role) {
                const name = getName(el);
                const interactive = INTERACTIVE_TAGS.has(el.tagName) || INTERACTIVE_ROLES.has(role);
                const value = el.value !== undefined && el.value !== '' ? String(el.value) : undefined;
                const desc = el.getAttribute('aria-description') || undefined;
                // Stamp the DOM element so click/type_text can find it by ref
                el.setAttribute('data-lynx-ref', 'e' + refIdx);
                refIdx++;
                nodes.push({ role, name, interactive, value, desc, tag: el.tagName });
            }
        }
        el = walker.nextNode();
    }
    return JSON.stringify(nodes);
})()
"#;

/// JavaScript to stamp DOM elements with data-lynx-ref attributes.
/// Must use identical role-detection logic as JS_SNAPSHOT.
/// Used after CDP snapshots so click/type_text can find elements by ref.
const JS_TAG_ELEMENTS: &str = r#"
(function() {
    const INTERACTIVE_TAGS = new Set(['A','BUTTON','INPUT','SELECT','TEXTAREA','DETAILS','SUMMARY']);
    const SKIP_TAGS = new Set(['SCRIPT','STYLE','NOSCRIPT','SVG','PATH','META','LINK','BR','HR']);

    function getRole(el) {
        if (el.computedRole && el.computedRole !== 'generic' && el.computedRole !== 'none') {
            return el.computedRole;
        }
        const ariaRole = el.getAttribute('role');
        if (ariaRole && ariaRole !== 'none' && ariaRole !== 'presentation') return ariaRole;
        const tag = el.tagName;
        if (tag === 'A' && el.href) return 'link';
        if (tag === 'BUTTON') return 'button';
        if (tag === 'INPUT') {
            const t = (el.type || 'text').toLowerCase();
            if (t === 'checkbox') return 'checkbox';
            if (t === 'radio') return 'radio';
            if (t === 'submit' || t === 'button') return 'button';
            if (t === 'search') return 'searchbox';
            return 'textbox';
        }
        if (tag === 'TEXTAREA') return 'textbox';
        if (tag === 'SELECT') return 'combobox';
        if (tag === 'IMG') return 'img';
        if (tag === 'H1' || tag === 'H2' || tag === 'H3' || tag === 'H4' || tag === 'H5' || tag === 'H6') return 'heading';
        if (tag === 'NAV') return 'navigation';
        if (tag === 'MAIN') return 'main';
        if (tag === 'HEADER') return 'banner';
        if (tag === 'FOOTER') return 'contentinfo';
        if (tag === 'ASIDE') return 'complementary';
        if (tag === 'FORM') return 'form';
        if (tag === 'TABLE') return 'table';
        if (tag === 'UL' || tag === 'OL') return 'list';
        if (tag === 'LI') return 'listitem';
        return '';
    }

    document.querySelectorAll('[data-lynx-ref]').forEach(function(e) {
        e.removeAttribute('data-lynx-ref');
    });

    let refIdx = 0;
    const walker = document.createTreeWalker(document.body, NodeFilter.SHOW_ELEMENT, null);
    let el = walker.currentNode;
    while (el) {
        if (!SKIP_TAGS.has(el.tagName)) {
            if (getRole(el)) {
                el.setAttribute('data-lynx-ref', 'e' + refIdx);
                refIdx++;
            }
        }
        el = walker.nextNode();
    }
    return refIdx;
})()
"#;

/// Build a snapshot of the page's accessibility tree.
/// Tries CDP Accessibility.getFullAXTree first, falls back to JS-based DOM walk.
/// Always stamps DOM elements with data-lynx-ref for click/type resolution.
pub async fn build_snapshot(
    page: &Page,
    interactive_only: bool,
) -> Result<(Vec<SnapshotNode>, RefMap), LynxError> {
    // Try CDP accessibility tree first
    match build_snapshot_cdp(page, interactive_only).await {
        Ok(result) => {
            // CDP path doesn't stamp DOM elements — run the tagger
            let _ = page.evaluate(JS_TAG_ELEMENTS).await;
            return Ok(result);
        }
        Err(e) => {
            tracing::warn!("CDP getFullAXTree failed ({e}), falling back to JS snapshot");
        }
    }

    // Fallback: JS-based DOM walk (stamps elements during walk)
    build_snapshot_js(page, interactive_only).await
}

/// CDP-based snapshot via Accessibility.getFullAXTree
async fn build_snapshot_cdp(
    page: &Page,
    interactive_only: bool,
) -> Result<(Vec<SnapshotNode>, RefMap), LynxError> {
    // Enable the accessibility domain first
    page.execute(AxEnableParams::default())
        .await
        .map_err(|e| LynxError::Snapshot(format!("Accessibility.enable failed: {e}")))?;

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

        let backend_node_id = node.backend_dom_node_id.unwrap_or_default();
        let ref_id = ref_map.assign(backend_node_id);

        snapshot_nodes.push(SnapshotNode {
            ref_id,
            role,
            name,
            description,
            value,
            interactive,
            children: Vec::new(),
        });
    }

    Ok((snapshot_nodes, ref_map))
}

/// JS-based snapshot fallback using computedRole/computedName + DOM walk
async fn build_snapshot_js(
    page: &Page,
    interactive_only: bool,
) -> Result<(Vec<SnapshotNode>, RefMap), LynxError> {
    let json_str: String = page
        .evaluate(JS_SNAPSHOT)
        .await
        .map_err(|e| LynxError::Snapshot(format!("JS snapshot failed: {e}")))?
        .into_value()
        .map_err(|e| LynxError::Snapshot(format!("JS snapshot parse failed: {e:?}")))?;

    let raw_nodes: Vec<JsNode> = serde_json::from_str(&json_str)
        .map_err(|e| LynxError::Snapshot(format!("JS snapshot JSON parse failed: {e}")))?;

    let mut ref_map = RefMap::new();
    let mut snapshot_nodes = Vec::new();

    for node in raw_nodes {
        let interactive = node.interactive;

        if interactive_only && !interactive {
            continue;
        }

        // Use a dummy BackendNodeId(0) for JS-based refs — actions will use JS selectors
        let ref_id = ref_map.assign(Default::default());

        snapshot_nodes.push(SnapshotNode {
            ref_id,
            role: node.role,
            name: node.name,
            description: node.desc,
            value: node.value,
            interactive,
            children: Vec::new(),
        });
    }

    Ok((snapshot_nodes, ref_map))
}

#[derive(serde::Deserialize)]
struct JsNode {
    role: String,
    name: String,
    interactive: bool,
    #[serde(default)]
    value: Option<String>,
    #[serde(default)]
    desc: Option<String>,
    #[serde(default)]
    #[allow(dead_code)]
    tag: Option<String>,
}
