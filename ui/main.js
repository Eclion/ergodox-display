// Renders the stacked layer boards and highlights live keypresses coming
// from the Rust backend ("kb-event"). Also runs standalone in a plain
// browser (no window.__TAURI__) for quick visual checks.

const SCALE = 30; // px per key unit
const DISPLAYED_LAYERS = [1, 0]; // top-down: Symbols, then Base

const TAURI = window.__TAURI__;

const LABELS = {
  KC_EQUAL: "=", KC_MINUS: "-", KC_PLUS: "+", KC_UNDS: "_",
  KC_GRAVE: "`", KC_TILD: "~", KC_QUOTE: "'", KC_DQUO: '"',
  KC_SCLN: ";", KC_COLN: ":", KC_COMMA: ",", KC_DOT: ".",
  KC_SLASH: "/", KC_QUES: "?", KC_BSLS: "\\", KC_PIPE: "|",
  KC_LBRC: "[", KC_RBRC: "]", KC_LCBR: "{", KC_RCBR: "}",
  KC_LPRN: "(", KC_RPRN: ")", KC_LABK: "<", KC_RABK: ">",
  KC_EXLM: "!", KC_AT: "@", KC_HASH: "#", KC_DLR: "$", KC_PERC: "%",
  KC_CIRC: "^", KC_AMPR: "&", KC_ASTR: "*",
  KC_SPACE: "Space", KC_ENTER: "Enter", KC_ESCAPE: "Esc", KC_BSPC: "Bksp",
  KC_TAB: "Tab", KC_DELETE: "Del", KC_INSERT: "Ins",
  KC_HOME: "Home", KC_END: "End",
  KC_PAGE_UP: "PgUp", KC_PGUP: "PgUp", KC_PGDN: "PgDn", KC_PAGE_DOWN: "PgDn",
  KC_LEFT: "←", KC_RIGHT: "→", KC_UP: "↑", KC_DOWN: "↓",
  KC_APPLICATION: "Menu", KC_CAPS_LOCK: "Caps", KC_PSCR: "PrtSc",
  KC_LEFT_SHIFT: "Shift", KC_RIGHT_SHIFT: "Shift",
  KC_LEFT_CTRL: "Ctrl", KC_RIGHT_CTRL: "Ctrl",
  KC_LEFT_ALT: "Alt", KC_RIGHT_ALT: "AltGr",
  KC_LEFT_GUI: "Super", KC_RIGHT_GUI: "Super",
  MEH_T: "Meh", MEH: "Meh", ALL_T: "Hyper", HYPR: "Hyper",
  KC_MS_UP: "Ms↑", KC_MS_DOWN: "Ms↓", KC_MS_LEFT: "Ms←", KC_MS_RIGHT: "Ms→",
  KC_MS_BTN1: "LClick", KC_MS_BTN2: "RClick",
  KC_MEDIA_PLAY_PAUSE: "⏯", KC_MEDIA_PREV_TRACK: "⏮", KC_MEDIA_NEXT_TRACK: "⏭",
  KC_AUDIO_VOL_UP: "Vol+", KC_AUDIO_VOL_DOWN: "Vol-", KC_AUDIO_MUTE: "Mute",
  KC_WWW_BACK: "Web←", QK_BOOT: "Boot", RESET: "Boot",
};

const LAYER_OPS = new Set(["TG", "MO", "TT", "TO", "OSL", "OSM", "LT", "DF"]);

let layers = [];
let layerShort = [];
const boardByLayer = new Map(); // layer position -> { root, keyEls }
let matrixToIndex = new Map(); // "row,col" -> key index
let activeLayer = 0;
let mode = "movable";
let geometryData = null;
// KC_* -> character under the OS's active keyboard layout ("layout-map"
// events from the backend); null until known, then labels are re-rendered.
let osKeyMap = null;
// KC_* -> character Shift produces on that key, where it differs from the
// main label (keycap-style upper legend). Same lifecycle as osKeyMap.
let osShifts = null;

function codeLabel(action, layerNames) {
  if (!action || !action.code || action.code === "KC_NO") return "";
  const { code, layer } = action;
  if (code === "KC_TRANSPARENT" || code === "KC_TRNS") return null; // resolve from base
  if (LAYER_OPS.has(code) && layer !== null && layer !== undefined) {
    return `${code} ${layerNames[layer] ?? layer}`;
  }
  if (osKeyMap && osKeyMap[code] !== undefined) return osKeyMap[code];
  if (LABELS[code] !== undefined) return LABELS[code];
  let m = /^KC_([A-Z])$/.exec(code);
  if (m) return m[1];
  m = /^KC_(\d)$/.exec(code);
  if (m) return m[1];
  m = /^KC_(F\d{1,2})$/.exec(code);
  if (m) return m[1];
  return code.replace(/^KC_/, "").replace(/_/g, " ");
}

function shiftFor(action) {
  return (osShifts && action?.code && osShifts[action.code]) || "";
}

function keyContent(key, baseKey, layerNames) {
  const custom = key.customLabel;
  let tap = custom || codeLabel(key.tap, layerNames);
  let shift = custom ? "" : shiftFor(key.tap);
  let transparent = false;
  if (tap === null) {
    transparent = true;
    tap = baseKey ? (baseKey.customLabel || codeLabel(baseKey.tap, layerNames) || "") : "";
    shift = baseKey && !baseKey.customLabel ? shiftFor(baseKey.tap) : "";
    if (tap === null) tap = "";
  }
  let hold = codeLabel(key.hold, layerNames);
  if (hold === null) hold = "";
  return { tap: tap || "", hold: hold || "", shift, transparent };
}

function buildBoard(geometry, layer, baseLayer, layerNames) {
  const root = document.createElement("section");
  root.className = "board";
  root.dataset.layer = layer.position;

  const title = document.createElement("div");
  title.className = "board-title";
  title.textContent = layer.title;
  root.appendChild(title);

  const keysBox = document.createElement("div");
  keysBox.className = "keys";
  const maxX = Math.max(...geometry.map((k) => k.x + k.w));
  const maxY = Math.max(...geometry.map((k) => k.y + k.h));
  keysBox.style.width = `${maxX * SCALE}px`;
  keysBox.style.height = `${maxY * SCALE}px`;

  const keyEls = [];
  geometry.forEach((geo, i) => {
    const el = document.createElement("div");
    el.className = "key";
    el.style.left = `${geo.x * SCALE + 1}px`;
    el.style.top = `${geo.y * SCALE + 1}px`;
    el.style.width = `${geo.w * SCALE - 2}px`;
    el.style.height = `${geo.h * SCALE - 2}px`;

    const { tap, hold, shift, transparent } = keyContent(
      layer.keys[i],
      baseLayer.keys[i],
      layerNames
    );
    if (transparent) el.classList.add("trns");
    if (!tap && !hold) el.classList.add("empty");

    if (shift) {
      const shiftEl = document.createElement("span");
      shiftEl.className = "shift";
      shiftEl.textContent = shift;
      el.appendChild(shiftEl);
    }
    const tapEl = document.createElement("span");
    tapEl.className = "tap";
    if (tap.length >= 6) tapEl.classList.add("tiny");
    else if (tap.length >= 4) tapEl.classList.add("small");
    tapEl.textContent = tap;
    el.appendChild(tapEl);
    if (hold) {
      const holdEl = document.createElement("span");
      holdEl.className = "hold";
      holdEl.textContent = hold;
      el.appendChild(holdEl);
    }
    keysBox.appendChild(el);
    keyEls.push(el);
  });
  root.appendChild(keysBox);
  return { root, keyEls };
}

function setActiveLayer(layer) {
  activeLayer = layer;
  for (const [pos, board] of boardByLayer) {
    board.root.classList.toggle("active", pos === layer);
  }
  const name = layers[layer]?.title ?? `Layer ${layer}`;
  document.getElementById("layer-name").textContent = name;
}

function setStatus(connected, detail) {
  document.getElementById("status").classList.toggle("connected", connected);
  document.getElementById("status-text").textContent = connected
    ? "connected"
    : detail || "disconnected";
}

// The keyboard sends key events best-effort; a keyup can get lost under
// fast typing, which would leave a key highlighted forever. Clear stale
// highlights after a fallback delay.
const RELEASE_FALLBACK_MS = 500;
const releaseTimers = new Map();
// Physical key -> element currently highlighted for it. The release must
// clear the element the press lit, even if the active layer (and therefore
// the board a key maps to) changed between press and release.
const litByKey = new Map();

function releaseKey(mapKey) {
  const el = litByKey.get(mapKey);
  litByKey.delete(mapKey);
  clearTimeout(releaseTimers.get(mapKey));
  releaseTimers.delete(mapKey);
  if (el) el.classList.remove("pressed", "foreign");
}

function onKey(down, row, col) {
  const idx = matrixToIndex.get(`${row},${col}`);
  if (idx === undefined) return;
  const mapKey = `${row},${col}`;
  releaseKey(mapKey);
  if (!down) return;

  const board = boardByLayer.get(activeLayer) ?? boardByLayer.get(0);
  if (!board) return;
  const el = board.keyEls[idx];
  if (!el) return;
  el.classList.add("pressed");
  el.classList.toggle("foreign", !boardByLayer.has(activeLayer));
  litByKey.set(mapKey, el);
  releaseTimers.set(mapKey, setTimeout(() => releaseKey(mapKey), RELEASE_FALLBACK_MS));
}

function setMode(newMode) {
  mode = newMode;
  document.body.classList.toggle("movable", mode === "movable");
}

function renderBoards() {
  // Full rebuild: also used when the OS keyboard layout changes. Any live
  // highlight state refers to elements being thrown away, so reset it.
  for (const timer of releaseTimers.values()) clearTimeout(timer);
  releaseTimers.clear();
  litByKey.clear();
  boardByLayer.clear();

  const boardsBox = document.getElementById("boards");
  boardsBox.replaceChildren();
  const baseLayer = layers[0];
  for (const pos of DISPLAYED_LAYERS) {
    const layer = layers[pos];
    if (!layer) continue;
    const board = buildBoard(geometryData, layer, baseLayer, layerShort);
    boardByLayer.set(pos, board);
    boardsBox.appendChild(board.root);
  }
  setActiveLayer(activeLayer);
}

function applyLayoutMap(payload) {
  if (
    !payload ||
    (JSON.stringify(payload.map) === JSON.stringify(osKeyMap) &&
      JSON.stringify(payload.shifts) === JSON.stringify(osShifts))
  ) {
    return;
  }
  osKeyMap = payload.map;
  osShifts = payload.shifts ?? null;
  document.getElementById("layer-name").title = `OS layout: ${payload.name}`;
  renderBoards();
}

// The bundled layout.json is the default; an imported layout (from the
// settings window) has the same shape and replaces it at runtime.
let bundledLayout = null;

function setLayoutData(layoutData) {
  layers = layoutData.data.layout.revision.layers;
  layerShort = layers.map((l) => l.title.slice(0, 3));
  renderBoards();
}

async function init() {
  const [geometry, layoutData] = await Promise.all([
    fetch("geometry.json").then((r) => r.json()),
    fetch("layout.json").then((r) => r.json()),
  ]);
  geometryData = geometry;
  bundledLayout = layoutData;
  matrixToIndex = new Map(geometry.map((k, i) => [`${k.row},${k.col}`, i]));

  let layout = bundledLayout;
  if (TAURI) {
    try {
      layout = (await TAURI.core.invoke("get_layout")) ?? bundledLayout;
    } catch {}
  }
  setLayoutData(layout);

  if (!TAURI) {
    // Plain-browser preview: simulate a few events.
    setStatus(false, "browser preview");
    return;
  }

  try {
    const settings = await TAURI.core.invoke("get_settings");
    setMode(settings.mode);
  } catch {
    setMode("movable");
  }

  await TAURI.event.listen("kb-event", ({ payload }) => {
    if (payload.type === "status") setStatus(payload.connected, payload.detail);
    else if (payload.type === "layer") setActiveLayer(payload.layer);
    else if (payload.type === "key") onKey(payload.down, payload.row, payload.col);
  });
  await TAURI.event.listen("mode-changed", ({ payload }) => setMode(payload));
  await TAURI.event.listen("layout-map", ({ payload }) => applyLayoutMap(payload));
  await TAURI.event.listen("layout-changed", ({ payload }) =>
    setLayoutData(payload ?? bundledLayout)
  );

  // Catch up on events emitted before the listeners attached.
  try {
    const [status, layer] = await TAURI.core.invoke("get_kb_state");
    if (status.type === "status") setStatus(status.connected, status.detail);
    setActiveLayer(layer);
    applyLayoutMap(await TAURI.core.invoke("get_layout_map"));
  } catch {}

  document.addEventListener("mousedown", (e) => {
    if (mode === "movable" && e.button === 0) {
      TAURI.window.getCurrentWindow().startDragging();
    }
  });
}

init();
