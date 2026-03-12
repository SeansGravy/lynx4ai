use chromiumoxide::Page;

use crate::browser::instance::RefMap;
use crate::error::LynxError;
use crate::types::SnapshotNode;

/// JavaScript that walks the DOM and extracts accessibility info.
/// Uses computedRole/computedName (Chrome 90+) with aria-* fallbacks.
/// Stamps each matched element with data-lynx-ref="eN" for click/type resolution.
///
/// Refs are **persistent and monotonic**: once an element is stamped with eN, it keeps
/// that ref across subsequent snapshots. New elements get the next available ref from
/// `window.__lynx_next_ref`. Removed elements naturally lose their stamp.
///
/// The `__LYNX_SELECTOR__` placeholder is replaced by Rust with a CSS selector string
/// or `null` to scope the snapshot to a subtree.
const JS_SNAPSHOT_TEMPLATE: &str = r#"
(function(rootSelector) {
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

    // Persistent monotonic counter — never resets across snapshots
    if (typeof window.__lynx_next_ref === 'undefined') {
        window.__lynx_next_ref = 0;
    }

    // Scope to selector subtree or full body
    var root = rootSelector ? document.querySelector(rootSelector) : document.body;
    if (!root) root = document.body;

    var nodes = [];
    var walker = document.createTreeWalker(root, NodeFilter.SHOW_ELEMENT, null);
    var el = walker.currentNode;
    while (el) {
        if (!SKIP_TAGS.has(el.tagName)) {
            var role = getRole(el);
            if (role) {
                var name = getName(el);
                var interactive = INTERACTIVE_TAGS.has(el.tagName) || INTERACTIVE_ROLES.has(role);
                var value = el.value !== undefined && el.value !== '' ? String(el.value) : undefined;
                var desc = el.getAttribute('aria-description') || undefined;

                // Persistent ref: keep existing stamp, only assign new refs to unstamped elements
                var existingRef = el.getAttribute('data-lynx-ref');
                var refIdx;
                if (existingRef) {
                    refIdx = parseInt(existingRef.substring(1), 10);
                } else {
                    refIdx = window.__lynx_next_ref++;
                    el.setAttribute('data-lynx-ref', 'e' + refIdx);
                }

                nodes.push({ role: role, name: name, interactive: interactive, value: value, desc: desc, tag: el.tagName, r: refIdx });
            }
        }
        el = walker.nextNode();
    }
    return JSON.stringify({ nodes: nodes, nextRef: window.__lynx_next_ref });
})(__LYNX_SELECTOR__)
"#;

/// Build a snapshot of the page's accessibility tree.
/// Uses JS-based DOM walk that stamps elements with data-lynx-ref attributes.
/// The JS walk is the sole source of truth for ref numbering — this guarantees
/// snapshot refs match the DOM stamps that click/type_text/press use via querySelector.
///
/// Refs are persistent: elements keep their ref across snapshots. New elements get
/// monotonically increasing refs. Returns (nodes, ref_map, snapshot_version).
///
/// Previous architecture used CDP getFullAXTree as primary path, but the AX tree
/// traversal order differs from DOM document order, causing ref index mismatches
/// when JS_TAG_ELEMENTS stamped elements in DOM order. Eliminated entirely.
pub async fn build_snapshot(
    page: &Page,
    interactive_only: bool,
    selector: Option<&str>,
) -> Result<(Vec<SnapshotNode>, RefMap, u64), LynxError> {
    // Inject the selector into the JS template
    let js = if let Some(sel) = selector {
        let escaped = sel.replace('\\', "\\\\").replace('\'', "\\'");
        JS_SNAPSHOT_TEMPLATE.replace("__LYNX_SELECTOR__", &format!("'{escaped}'"))
    } else {
        JS_SNAPSHOT_TEMPLATE.replace("__LYNX_SELECTOR__", "null")
    };

    let json_str: String = page
        .evaluate(js)
        .await
        .map_err(|e| LynxError::Snapshot(format!("JS snapshot failed: {e}")))?
        .into_value()
        .map_err(|e| LynxError::Snapshot(format!("JS snapshot parse failed: {e:?}")))?;

    let result: JsSnapshotResult = serde_json::from_str(&json_str)
        .map_err(|e| LynxError::Snapshot(format!("JS snapshot JSON parse failed: {e}")))?;

    let mut ref_map = RefMap::new();
    let mut snapshot_nodes = Vec::new();

    for node in result.nodes {
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

    Ok((snapshot_nodes, ref_map, result.next_ref))
}

#[derive(serde::Deserialize)]
struct JsSnapshotResult {
    nodes: Vec<JsNode>,
    #[serde(rename = "nextRef")]
    next_ref: u64,
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
