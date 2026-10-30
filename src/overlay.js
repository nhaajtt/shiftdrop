import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { ctx, loadContext, menuFor, noteUse } from './menu.js';
import { runTarget } from './run.js';
import { initTheme } from './theme.js';

initTheme();

// Geometry in logical px; the window is 460 × 460 centred on the dragged file.
const C = 230;
const CORE = 52;
const RING1 = [58, 146];
const RING2 = [152, 212];
const GLASS = [150, 216]; // radius of the frosted disc without / with the sub-option ring
const NS = 'http://www.w3.org/2000/svg';
const svg = document.getElementById('wheel');
const glass = document.getElementById('glass');
const disc = document.getElementById('disc');

let paths = [];
let warnings = []; // files pretending to be another type
loadContext(invoke);
let menu = null;
let hot = null; // { ring: 0 } | { ring: 1, i } | { ring: 2, i, j }
let locked = false; // a number key picked the wedge; ignore the pointer until release
let parent = -1; // ring-1 item whose options are shown in ring 2
let ring1 = [];
let ring2 = null;
let coreEl = null;

function el(tag, attrs, into) {
  const e = document.createElementNS(NS, tag);
  for (const [k, v] of Object.entries(attrs)) e.setAttribute(k, v);
  into?.appendChild(e);
  return e;
}

const rad = (deg) => ((deg - 90) * Math.PI) / 180;
const pt = (r, deg) => [C + r * Math.cos(rad(deg)), C + r * Math.sin(rad(deg))];

function sector(r0, r1, a0, a1) {
  const large = a1 - a0 > 180 ? 1 : 0;
  const [x0, y0] = pt(r1, a0);
  const [x1, y1] = pt(r1, a1);
  const [x2, y2] = pt(r0, a1);
  const [x3, y3] = pt(r0, a0);
  return `M${x0} ${y0}A${r1} ${r1} 0 ${large} 1 ${x1} ${y1}L${x2} ${y2}A${r0} ${r0} 0 ${large} 0 ${x3} ${y3}Z`;
}

function text(into, x, y, cls, str) {
  const t = el('text', { x, y, class: cls, 'text-anchor': 'middle', 'dominant-baseline': 'middle' }, into);
  t.textContent = str;
  return t;
}

const step = () => 360 / menu.items.length;

/** Arc of ring-2 options around the parent wedge: [start angle, width of one option]. */
function subArc(i) {
  const m = menu.items[i].sub.length;
  const w = Math.min(34, 320 / m);
  return [i * step() - (w * m) / 2, w];
}

function render() {
  svg.replaceChildren();
  const defs = el('defs', {}, svg);
  const lg = el('linearGradient', { id: 'lg', x1: '0', y1: '0', x2: '1', y2: '1' }, defs);
  el('stop', { offset: '0', 'stop-color': '#4f8bff' }, lg);
  el('stop', { offset: '1', 'stop-color': '#8b5cf6' }, lg);
  ring1 = [];
  ring2 = null;
  parent = -1;
  hot = null;
  el('circle', { cx: C, cy: C, r: GLASS[0] - 1, class: 'plate' }, svg);

  if (menu) {
    menu.items.forEach((item, i) => {
      const s = step();
      const a0 = i * s - s / 2;
      const g = el('g', {}, svg);
      const path = el('path', { d: sector(RING1[0], RING1[1], a0, a0 + s), class: 'seg' }, g);
      const [x, y] = pt((RING1[0] + RING1[1]) / 2, i * s);
      text(g, x, y - 8, 'glyph', item.glyph);
      text(g, x, y + 12, 'name', item.label);
      const [nx, ny] = pt(RING1[1] - 11, i * s);
      text(g, nx, ny, 'num', String(i + 1));
      ring1.push({ g, path });
    });
  }

  coreEl = el('circle', { cx: C, cy: C, r: CORE, class: 'core' }, svg);
  const coreText = el('g', { class: 'core-text' }, svg);
  const n = paths.length;
  if (menu) {
    text(coreText, C, C - 6, 'core-label', `→ ${menu.center.label}`);
    text(coreText, C, C + 14, warnings.length ? 'core-hint warn' : 'core-hint', warnings.length ? '⚠ fake type' : n > 1 ? `${n} files` : 'drop here');
  } else {
    text(coreText, C, C - 4, 'core-hint', 'not');
    text(coreText, C, C + 10, 'core-hint', 'supported');
  }
  requestAnimationFrame(() => {
    svg.classList.add('open');
    glass.classList.add('open');
  });
}

function showRing2(i) {
  if (parent === i) return;
  ring2?.remove();
  ring2 = null;
  parent = i;
  const item = menu.items[i];
  disc.classList.toggle('big', !!item?.sub);
  if (!item?.sub) return;
  ring2 = el('g', { class: 'ring2' }, svg);
  el('circle', { cx: C, cy: C, r: GLASS[1] - 1, class: 'edge' }, ring2);
  const [start, w] = subArc(i);
  item.sub.forEach((sub, j) => {
    const a0 = start + j * w;
    const g = el('g', {}, ring2);
    el('path', { d: sector(RING2[0], RING2[1], a0, a0 + w), class: 'seg' }, g);
    const [x, y] = pt((RING2[0] + RING2[1]) / 2, a0 + w / 2);
    text(g, x, y, 'sub-name', sub.label);
  });
  requestAnimationFrame(() => ring2?.classList.add('show'));
}

function hitTest(x, y) {
  const dx = x - C;
  const dy = y - C;
  const r = Math.hypot(dx, dy);
  if (r <= CORE + 3) return { ring: 0 };
  if (!menu?.items.length) return null;
  const deg = ((Math.atan2(dy, dx) * 180) / Math.PI + 90 + 360) % 360;
  if (r <= RING1[1] + 2) {
    const s = step();
    return { ring: 1, i: Math.floor(((deg + s / 2) % 360) / s) };
  }
  if (r <= RING2[1] + 8 && parent >= 0 && menu.items[parent].sub) {
    const [start, w] = subArc(parent);
    const d = (((deg - start) % 360) + 360) % 360;
    const j = Math.floor(d / w);
    if (j < menu.items[parent].sub.length) return { ring: 2, i: parent, j };
  }
  return null;
}

function setHot(h) {
  if (JSON.stringify(h) === JSON.stringify(hot)) return;
  hot = h;
  coreEl?.classList.toggle('hot', h?.ring === 0);
  ring1.forEach(({ g, path }, i) => {
    const on = h?.ring === 1 && h.i === i;
    path.classList.toggle('hot', on);
    path.classList.toggle('parent', h?.ring === 2 && h.i === i);
    g.classList.toggle('hot-label', on || (h?.ring === 2 && h.i === i));
  });
  if (h?.ring === 1) showRing2(menu.items[h.i].sub ? h.i : -1);
  if (h?.ring === 0 && parent >= 0) showRing2(-1);
  if (ring2) {
    [...ring2.querySelectorAll(':scope > g')].forEach((g, j) => {
      const on = h?.ring === 2 && h.j === j;
      g.firstChild.classList.toggle('hot', on);
      g.classList.toggle('hot-label', on);
    });
  }
}

function hover(position) {
  if (locked) return;
  const k = window.devicePixelRatio || 1;
  setHot(hitTest(position.x / k, position.y / k));
}

/** What a drop at the current hover point does. */
function target() {
  if (!menu || !hot) return null;
  if (hot.ring === 0) return menu.center;
  const item = menu.items[hot.i];
  if (hot.ring === 2) return item.sub[hot.j];
  if (item.editor || item.action || item.app || item.more || item.shelf || item.share) return item;
  return item.sub?.[item.def ?? 0] ?? null;
}

async function drop() {
  const t = target();
  const files = paths;
  // an action picked from around More counts for that action, so it can earn a wedge
  if (t && hot?.ring > 0) noteUse(menu.kind, t.from || menu.items[hot.i].id);
  // close first: opening a window can take a moment, and a new Shift-drag in that time
  // must not have its fresh wheel closed by this one
  close();
  try {
    if (t?.more) await invoke('open_hub', { paths: files, view: 'more', item: null });
    else if (t?.ask) await invoke('open_hub', { paths: files, view: 'ask', item: t });
    else await runTarget(t, files);
  } catch (e) {
    invoke('ui_log', { msg: `drop failed: ${e}` });
  }
}

function close() {
  svg.classList.remove('open');
  glass.classList.remove('open');
  menu = null;
  invoke('overlay_hide');
}

// Explorer keeps the drag to itself, so the app reads the dragged files and follows the cursor.
// apps installed (or add-ons downloaded) since start show up on the wheel without a restart
let contextAt = Date.now();

listen('overlay:show', ({ payload }) => {
  if (Date.now() - contextAt > 60000) {
    contextAt = Date.now();
    setTimeout(() => loadContext(invoke), 1500);
  }
  svg.classList.remove('open');
  glass.classList.remove('open');
  paths = payload.paths || [];
  locked = false;
  warnings = payload.warnings || [];
  document.body.classList.toggle('warn', warnings.length > 0);
  // the screen behind the menu, frosted when it was captured
  disc.firstChild.style.backgroundImage = payload.backdrop ? `url(${payload.backdrop})` : 'none';
  if (payload.backdrop) shade(payload.backdrop);
  disc.classList.remove('big');
  menu = paths.length ? menuFor(paths, ctx.apps) : null;
  render();
});

/** Light text over dark backdrops, dark text over light ones. */
function shade(src) {
  const img = new Image();
  img.onload = () => {
    const c = document.createElement('canvas');
    c.width = c.height = 16;
    const g = c.getContext('2d', { willReadFrequently: true });
    g.drawImage(img, 0, 0, 16, 16);
    const d = g.getImageData(0, 0, 16, 16).data;
    // only the part under the disc counts
    let sum = 0;
    let n = 0;
    for (let i = 0; i < d.length; i += 4) {
      const x = (i / 4) % 16;
      const y = Math.floor(i / 64);
      if (Math.hypot(x - 7.5, y - 7.5) > 5.5) continue;
      sum += 0.2126 * d[i] + 0.7152 * d[i + 1] + 0.0722 * d[i + 2];
      n++;
    }
    document.body.classList.toggle('on-dark', sum / n < 128);
  };
  img.src = src;
}

listen('overlay:key', ({ payload: n }) => {
  if (!menu) return;
  if (n === 0) return close();
  if (n <= menu.items.length) {
    locked = false;
    setHot({ ring: 1, i: n - 1 });
    locked = true;
  }
});

// mice report up to 1000 moves a second; handle only the latest one per frame
let pending = null;
listen('overlay:move', ({ payload: [x, y] }) => {
  if (!svg.childElementCount) return;
  if (!pending) requestAnimationFrame(() => (pending && hover(pending), (pending = null)));
  pending = { x, y };
});

listen('overlay:drop', ({ payload: [x, y] }) => {
  pending = null;
  hover({ x, y });
  drop();
});

addEventListener('keydown', (e) => e.key === 'Escape' && close());
