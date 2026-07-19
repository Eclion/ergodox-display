//! Runtime-imported Oryx layouts.
//!
//! The bundled `ui/layout.json` is the compile-time default; an imported
//! layout (fetched from the Oryx GraphQL API and saved to the app config
//! dir) overrides it without rebuilding the app. The stored JSON has the
//! same shape as the bundled file (`{"data":{"layout":…}}`) so the frontend
//! consumes both identically.

use serde::Serialize;
use serde_json::{json, Value};
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, Manager};

const ORYX_GRAPHQL_URL: &str = "https://oryx.zsa.io/graphql";
const LAYOUT_QUERY: &str = "query getLayout($hashId: String!, $geometry: String, $revisionId: String!) { layout(hashId: $hashId, geometry: $geometry, revisionId: $revisionId) { title geometry revision { hashId title layers { hashId title position keys } } } }";
const EXPECTED_KEYS_PER_LAYER: usize = 76;

pub struct LayoutState(pub Mutex<Option<Value>>);

#[derive(Clone, Serialize)]
pub struct LayoutMeta {
    pub title: String,
    pub hash_id: String,
    pub revision: String,
    pub layers: usize,
}

fn layout_path(app: &AppHandle) -> Option<PathBuf> {
    app.path()
        .app_config_dir()
        .ok()
        .map(|dir| dir.join("layout.json"))
}

pub fn load_stored(app: &AppHandle) -> Option<Value> {
    let raw = fs::read_to_string(layout_path(app)?).ok()?;
    serde_json::from_str(&raw).ok()
}

/// Accepts a full Oryx URL (`…/ergodox-ez/layouts/<hash>/<revision>/0`), a
/// `<hash>/<revision>` pair, or a bare layout hash (revision = latest).
fn parse_source(source: &str) -> Result<(String, String), String> {
    let s = source.trim().trim_matches('"');
    if s.is_empty() {
        return Err("enter an Oryx layout URL or id".into());
    }
    let path = s
        .split_once("/layouts/")
        .map(|(_, rest)| rest)
        .unwrap_or(s);
    let mut parts = path.split('/').filter(|p| !p.is_empty());
    let hash = parts.next().ok_or("no layout id found")?.to_string();
    let revision = parts
        .next()
        .filter(|r| r.chars().any(|c| c.is_alphanumeric()))
        .unwrap_or("latest")
        .to_string();
    if !hash.chars().all(|c| c.is_alphanumeric()) {
        return Err(format!("'{hash}' does not look like an Oryx layout id"));
    }
    Ok((hash, revision))
}

fn fetch_layout(hash: &str, revision: &str) -> Result<Value, String> {
    let body = json!({
        "operationName": "getLayout",
        "variables": {"hashId": hash, "revisionId": revision, "geometry": "ergodox-ez"},
        "query": LAYOUT_QUERY,
    });
    let response = ureq::post(ORYX_GRAPHQL_URL)
        .set("Content-Type", "application/json")
        .send_string(&body.to_string())
        .map_err(|e| format!("Oryx request failed: {e}"))?
        .into_string()
        .map_err(|e| format!("Oryx response unreadable: {e}"))?;
    serde_json::from_str(&response).map_err(|e| format!("Oryx response is not JSON: {e}"))
}

fn validate(doc: &Value) -> Result<LayoutMeta, String> {
    let layout = doc
        .pointer("/data/layout")
        .filter(|l| !l.is_null())
        .ok_or("layout not found on Oryx — check the id/revision")?;
    let geometry = layout["geometry"].as_str().unwrap_or_default();
    if !geometry.starts_with("ergodox") {
        return Err(format!(
            "layout is for '{geometry}', only ergodox-ez layouts can be displayed"
        ));
    }
    let layers = layout
        .pointer("/revision/layers")
        .and_then(Value::as_array)
        .ok_or("layout has no layers")?;
    if layers.is_empty() {
        return Err("layout has no layers".into());
    }
    for layer in layers {
        let keys = layer["keys"].as_array().map(Vec::len).unwrap_or(0);
        if keys != EXPECTED_KEYS_PER_LAYER {
            return Err(format!(
                "layer '{}' has {keys} keys, expected {EXPECTED_KEYS_PER_LAYER}",
                layer["title"].as_str().unwrap_or("?")
            ));
        }
    }
    Ok(LayoutMeta {
        title: layout["title"].as_str().unwrap_or("untitled").to_string(),
        hash_id: String::new(), // filled by the caller, not part of the response
        revision: layout
            .pointer("/revision/hashId")
            .and_then(Value::as_str)
            .unwrap_or("?")
            .to_string(),
        layers: layers.len(),
    })
}

pub fn import(app: &AppHandle, source: &str) -> Result<LayoutMeta, String> {
    let (hash, revision) = parse_source(source)?;
    let doc = fetch_layout(&hash, &revision)?;
    let mut meta = validate(&doc)?;
    meta.hash_id = hash;

    if let Some(path) = layout_path(app) {
        if let Some(dir) = path.parent() {
            let _ = fs::create_dir_all(dir);
        }
        fs::write(&path, doc.to_string()).map_err(|e| format!("cannot save layout: {e}"))?;
    }
    let state = app.state::<LayoutState>();
    *state.0.lock().unwrap() = Some(doc.clone());
    let _ = app.emit("layout-changed", &doc);
    Ok(meta)
}

pub fn reset(app: &AppHandle) {
    if let Some(path) = layout_path(app) {
        let _ = fs::remove_file(path);
    }
    let state = app.state::<LayoutState>();
    *state.0.lock().unwrap() = None;
    let _ = app.emit("layout-changed", &Value::Null);
}
