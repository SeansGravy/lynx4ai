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

#[allow(dead_code)]
fn is_interactive(role: &str) -> bool {
    INTERACTIVE_ROLES.contains(&role.to_lowercase().as_str())
}

/// JavaScript that walks the DOM and extracts accessibility info.
/// Uses computedRole/computedName (Chrome 90+) with aria-* fallbacks.
/// Stamps each matched element with data-lynx-ref="eN" for click/type resolution.
/// Returns JSON array including the ref index so Rust uses the SAME indices.
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
        var ariaRole = el.getAttribute('role');
        if (ariaRole && ariaRole !== 'none' && ariaRole !== 'presentation') return ariaRole;
        var tag = el.tagName;
        if (tag === 'A' && el.href) return 'link';
        if (tag === 'BUTTON') return 'button';
        if (tag === 'INPUT') {
            var t = (el.type || 'text').toLowerCase();
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
        var ariaLabel = el.getAttribute('aria-label');
        if (ariaLabel) return ariaLabel;
        var ariaLabelledBy = el.getAttribute('aria-labelledby');
        if (ariaLabelledBy) {
            var labelEl = document.getElementById(ariaLabelledBy);
            if (labelEl) return labelEl.textContent.trim().substring(0, 100);
        }
        if (el.tagName === 'IMG') return el.alt || '';
        if (el.tagName === 'INPUT' || el.tagName === 'TEXTAREA' || el.tagName === 'SELECT') {
            if (el.labels && el.labels.length > 0) return el.labels[0].textContent.trim().substring(0, 100);
            return el.placeholder || el.name || '';
        }
        var text = el.textContent || '';
        return text.trim().substring(0, 100);
    }

    // Clean up any previous ref tags
    document.querySelectorAll('[data-lynx-ref]').forEach(function(e) {
        e.removeAttribute('data-lynx-ref');
    });

    var nodes = [];
    var refIdx = 0;
    var walker = document.createTreeWalker(document.body, NodeFilter.SHOW_ELEMENT, null);
    var el = walker.currentNode;
    while (el) {
        if (!SKIP_TAGS.has(el.tagName)) {
            var role = getRole(el);
            if (role) {
                var name = getName(el);
                var interactive = INTERACTIVE_TAGS.has(el.tagName) || INTERACTIVE_ROLES.has(role);
                var value = el.value !== undefined && el.value !== '' ? String(el.value) : undefined;
                var desc = el.getAttribute('aria-description') || undefined;
                // Stamp the DOM element so click/type_text can find it by ref
                el.setAttribute('data-lynx-ref', 'e' + refIdx);
                nodes.push({ role: role, name: name, interactive: interactive, value: value, desc: desc, tag: el.tagName, r: refIdx });
                refIdx++;
            }
        }
        el = walker.nextNode();
    }
    return JSON.stringify(nodes);
})()
"#;

/// Build a snapshot of the page's accessibility tree.
/// Uses JS-based DOM walk that stamps elements with data-lynx-ref attributes.
/// The JS walk is the sole source of truth for ref numbering — this guarantees
/// snapshot refs match the DOM stamps that click/type_text/press use via querySelector.
///
/// Previous architecture used CDP getFullAXTree as primary path, but the AX tree
/// traversal order differs from DOM document order, causing ref index mismatches
/// when JS_TAG_ELEMENTS stamped elements in DOM order. Eliminated entirely.
pub async fn build_snapshot(
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

        // Use the SAME ref index that JS stamped on the DOM element.
        // Critical: when interactive_only filters out nodes, the remaining
        // nodes keep their original ref indices (e.g. e0, e3, e7) which
        // match what's stamped in the DOM. No sequential re-numbering.
        let ref_id = format!("e{}", node.ref_idx);
        ref_map.insert(ref_id.clone(), Default::default());

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
    /// The ref index stamped on the DOM element (data-lynx-ref="eN")
    #[serde(rename = "r")]
    ref_idx: usize,
}
