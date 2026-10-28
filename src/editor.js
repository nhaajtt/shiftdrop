import { convertFileSrc, invoke } from '@tauri-apps/api/core';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { timedReveal } from './effects/reveal.js';
import { signalPreset } from './effects/timing';
import { initTheme } from './theme.js';

initTheme();

const q = new URLSearchParams(location.search);
const path = q.get('path') || '';
const toolName = q.get('tool') || 'crop';
const $ = (id) => document.getElementById(id);
const stage = $('stage');
const controls = $('controls');
const clamp = (v, a, b) => Math.min(b, Math.max(a, v));

$('file').textContent = path.split(/[\\/]/).pop();
$('tool').textContent = `[${{ crop: 'Crop', addbg: 'Add BG', redact: 'Redact', trim: 'Trim', pages: 'Pages', sign: 'Sign' }[toolName] || toolName}]`;

function h(tag, attrs = {}, ...kids) {
  const e = document.createElement(tag);
  for (const [k, v] of Object.entries(attrs)) {
    if (k.startsWith('on')) e.addEventListener(k.slice(2), v);
    else if (k === 'class') e.className = v;
    else e.setAttribute(k, v);
  }
  e.append(...kids.filter((k) => k != null));
  return e;
}

const group = (label, ...kids) => h('div', { class: 'group' }, h('span', { class: 'label' }, label), ...kids);

/** Row of toggle buttons; returns a function that marks the chosen one. */
function choices(items, current, onPick) {
  const row = h('div', { class: 'row' });
  const mark = (v) => row.querySelectorAll('.btn').forEach((b) => b.setAttribute('aria-pressed', b.dataset.v === String(v)));
  for (const [label, v] of items) {
    row.append(h('button', { class: 'btn', 'data-v': String(v), onclick: () => (mark(v), onPick(v)) }, label));
  }
  mark(current);
  return row;
}

function slider(label, min, max, value, unit, onInput) {
  const out = h('span', { class: 'value' }, `${value}${unit}`);
  const input = h('input', { type: 'range', min, max, value });
  input.addEventListener('input', () => {
    out.textContent = `${input.value}${unit}`;
    onInput(+input.value);
  });
  return group(label, h('div', { class: 'slider' }, input, out));
}

/** Loads the image and keeps a frame sized to fit the stage. */
function imageFrame(onFit) {
  const img = h('img', { src: convertFileSrc(path), alt: '' });
  const frame = h('div', { class: 'frame checker' }, img);
  stage.append(frame);
  const fit = () => {
    if (!img.naturalWidth) return;
    const pad = 48;
    const s = Math.min((stage.clientWidth - pad) / img.naturalWidth, (stage.clientHeight - pad) / img.naturalHeight, 1);
    frame.style.width = `${img.naturalWidth * s}px`;
    frame.style.height = `${img.naturalHeight * s}px`;
    onFit?.();
  };
  img.addEventListener('load', fit);
  new ResizeObserver(fit).observe(stage);
  return { img, frame };
}

/** Pointer position inside the frame as fractions 0–1. */
function frac(frame, e) {
  const r = frame.getBoundingClientRect();
  return [clamp((e.clientX - r.left) / r.width, 0, 1), clamp((e.clientY - r.top) / r.height, 0, 1)];
}

/* ---------------------------------------------------------------- crop */
function crop() {
  const { img, frame } = imageFrame(() => draw());
  let r = { x: 0.08, y: 0.08, w: 0.84, h: 0.84 };
  let ratio = 0; // 0 = free, otherwise width / height in pixels
  const box = h('div', { class: 'crop-box' }, ...['nw', 'ne', 'sw', 'se'].map((k) => h('i', { class: 'handle', 'data-h': k })));
  frame.append(box);
  const size = h('span', { class: 'value' }, '');

  function draw() {
    Object.assign(box.style, { left: `${r.x * 100}%`, top: `${r.y * 100}%`, width: `${r.w * 100}%`, height: `${r.h * 100}%` });
    if (img.naturalWidth) size.textContent = `${Math.round(r.w * img.naturalWidth)} × ${Math.round(r.h * img.naturalHeight)} px`;
  }

  // Fits a box with corner `a` fixed and the other corner at `p` to the chosen ratio.
  function fromCorners(a, p) {
    let w = Math.abs(p[0] - a[0]);
    let hh = Math.abs(p[1] - a[1]);
    if (ratio) {
      const k = img.naturalWidth / (ratio * img.naturalHeight); // h = w * k in fractions
      hh = w * k;
      const maxH = p[1] >= a[1] ? 1 - a[1] : a[1];
      if (hh > maxH) [hh, w] = [maxH, maxH / k];
    }
    const x = p[0] >= a[0] ? a[0] : a[0] - w;
    const y = p[1] >= a[1] ? a[1] : a[1] - hh;
    return { x, y, w: Math.max(w, 0.01), h: Math.max(hh, 0.01) };
  }

  function applyRatio(v) {
    ratio = v;
    if (!ratio || !img.naturalWidth) return draw();
    const c = [r.x + r.w / 2, r.y + r.h / 2];
    const k = img.naturalWidth / (ratio * img.naturalHeight);
    let w = Math.min(r.w, 1);
    let hh = w * k;
    if (hh > 1) [hh, w] = [1, 1 / k];
    r = { x: clamp(c[0] - w / 2, 0, 1 - w), y: clamp(c[1] - hh / 2, 0, 1 - hh), w, h: hh };
    draw();
  }

  frame.addEventListener('pointerdown', (e) => {
    frame.setPointerCapture(e.pointerId);
    const p0 = frac(frame, e);
    const handle = e.target.dataset?.h;
    const start = { ...r };
    let mode = handle ? 'resize' : e.target === box ? 'move' : 'draw';
    const anchor = handle
      ? [handle.includes('w') ? r.x + r.w : r.x, handle.includes('n') ? r.y + r.h : r.y]
      : p0;
    const move = (ev) => {
      const p = frac(frame, ev);
      if (mode === 'move') {
        r.x = clamp(start.x + p[0] - p0[0], 0, 1 - r.w);
        r.y = clamp(start.y + p[1] - p0[1], 0, 1 - r.h);
      } else {
        r = fromCorners(anchor, p);
      }
      draw();
    };
    const up = () => {
      frame.removeEventListener('pointermove', move);
      frame.removeEventListener('pointerup', up);
    };
    frame.addEventListener('pointermove', move);
    frame.addEventListener('pointerup', up);
  });

  const ratios = [['Free', 0], ['1:1', 1], ['4:3', 4 / 3], ['3:2', 3 / 2], ['16:9', 16 / 9], ['9:16', 9 / 16]];
  controls.append(
    group('Aspect', choices(ratios, 0, applyRatio)),
    group('Size', size),
    group('Selection', h('div', { class: 'row' }, h('button', { class: 'btn', onclick: () => ((r = { x: 0, y: 0, w: 1, h: 1 }), applyRatio(ratio)) }, 'Select all'))),
  );
  draw();
  return () => ({ action: { op: 'crop', rect: [r.x, r.y, r.w, r.h] }, title: 'Cropping' });
}

/* ------------------------------------------------------- add background */
function addBg() {
  const img = h('img', { src: convertFileSrc(path), alt: '' });
  const bg = h('div', { class: 'bg-preview' }, img);
  stage.append(bg);
  const o = { color: '#F3F1EC', padding: 8, radius: 3, shadow: 50 };

  function draw() {
    const W = img.naturalWidth;
    const H = img.naturalHeight;
    if (!W) return;
    const pad = (o.padding / 100) * Math.max(W, H);
    const s = Math.min((stage.clientWidth - 48) / (W + 2 * pad), (stage.clientHeight - 48) / (H + 2 * pad), 1);
    Object.assign(bg.style, { background: o.color, padding: `${pad * s}px` });
    Object.assign(img.style, {
      width: `${W * s}px`,
      height: `${H * s}px`,
      borderRadius: `${(o.radius / 100) * Math.min(W, H) * s}px`,
      boxShadow: pad ? `0 ${pad * 0.25 * s}px ${pad * 0.9 * s}px rgb(0 0 0 / ${(o.shadow / 100) * 0.45})` : 'none',
    });
  }
  img.addEventListener('load', draw);
  new ResizeObserver(draw).observe(stage);

  const palette = ['#FFFFFF', '#F3F1EC', '#EBE8DE', '#161512', '#000000', '#D9B84A', '#765E0E', '#3E5378', '#8FA3C8', '#A3385C', '#E07A9A', '#2F7D4F'];
  const sw = h('div', { class: 'swatches' });
  const mark = () => sw.querySelectorAll('button').forEach((b) => b.setAttribute('aria-pressed', b.dataset.c === o.color));
  for (const c of palette) {
    sw.append(h('button', { class: 'swatch', 'data-c': c, title: c, style: `background:${c}`, onclick: () => ((o.color = c), mark(), draw()) }));
  }
  const custom = h('input', { type: 'color', class: 'swatch', value: '#d9b84a', title: 'Custom colour' });
  custom.addEventListener('input', () => ((o.color = custom.value.toUpperCase()), mark(), draw()));
  sw.append(custom);
  mark();

  controls.append(
    group('Background', sw),
    slider('Padding', 0, 30, o.padding, '%', (v) => ((o.padding = v), draw())),
    slider('Corner radius', 0, 20, o.radius, '%', (v) => ((o.radius = v), draw())),
    slider('Shadow', 0, 100, o.shadow, '%', (v) => ((o.shadow = v), draw())),
  );
  return () => ({
    action: { op: 'add_bg', color: o.color, padding: o.padding / 100, radius: o.radius / 100, shadow: o.shadow / 100 },
    title: 'Adding background',
  });
}

/* ---------------------------------------------------------------- redact */
function redact() {
  const { frame } = imageFrame();
  frame.classList.add('drawing');
  const boxes = [];
  const count = h('span', { class: 'value' }, '0 areas');
  const place = (el, b) => Object.assign(el.style, { left: `${b[0] * 100}%`, top: `${b[1] * 100}%`, width: `${b[2] * 100}%`, height: `${b[3] * 100}%` });
  const refresh = () => (count.textContent = `${boxes.length} area${boxes.length === 1 ? '' : 's'}`);

  frame.addEventListener('pointerdown', (e) => {
    frame.setPointerCapture(e.pointerId);
    const a = frac(frame, e);
    const el = h('div', { class: 'redact-box' });
    frame.append(el);
    let b = [a[0], a[1], 0, 0];
    const move = (ev) => {
      const p = frac(frame, ev);
      b = [Math.min(a[0], p[0]), Math.min(a[1], p[1]), Math.abs(p[0] - a[0]), Math.abs(p[1] - a[1])];
      place(el, b);
    };
    const up = () => {
      frame.removeEventListener('pointermove', move);
      frame.removeEventListener('pointerup', up);
      if (b[2] < 0.005 || b[3] < 0.005) return el.remove();
      boxes.push({ b, el });
      refresh();
    };
    frame.addEventListener('pointermove', move);
    frame.addEventListener('pointerup', up);
  });

  const undo = () => (boxes.pop()?.el.remove(), refresh());
  addEventListener('keydown', (e) => (e.ctrlKey || e.metaKey) && e.key === 'z' && undo());
  controls.append(
    group('Areas', count, h('p', { class: 'label' }, 'Drag over what to black out.')),
    group(
      'Edit',
      h(
        'div',
        { class: 'row' },
        h('button', { class: 'btn', onclick: undo }, 'Undo'),
        h('button', { class: 'btn', onclick: () => (boxes.splice(0).forEach((x) => x.el.remove()), refresh()) }, 'Clear'),
      ),
    ),
  );
  return () => {
    if (!boxes.length) throw new Error('Draw at least one area first.');
    return { action: { op: 'redact', boxes: boxes.map((x) => x.b) }, title: 'Redacting' };
  };
}

/* ------------------------------------------------------------------ trim */
function fmt(t) {
  if (!isFinite(t)) return '0:00.0';
  const m = Math.floor(t / 60);
  return `${m}:${(t - m * 60).toFixed(1).padStart(4, '0')}`;
}

function parseTime(s) {
  const parts = s.trim().split(':').map(Number);
  if (parts.some(isNaN)) return NaN;
  return parts.reduce((acc, v) => acc * 60 + v, 0);
}

function trim() {
  const isVideo = /\.(mp4|mov|mkv|webm|avi|m4v|wmv|flv|mts|m2ts|3gp)$/i.test(path);
  const media = h(isVideo ? 'video' : 'audio', { src: convertFileSrc(path), controls: '', preload: 'metadata' });
  const sel = h('div', { class: 'sel' });
  const head = h('div', { class: 'head' });
  const line = h('div', { class: 'timeline' }, sel, head);
  stage.append(h('div', { class: 'media' }, media, line));
  let d = 0;
  let a = 0;
  let b = 0;
  const startIn = h('input', { type: 'text', value: '0:00.0' });
  const endIn = h('input', { type: 'text', value: '0:00.0' });
  const length = h('span', { class: 'value' }, '');

  function draw() {
    if (!d) return;
    Object.assign(sel.style, { left: `${(a / d) * 100}%`, width: `${((b - a) / d) * 100}%` });
    head.style.left = `${(media.currentTime / d) * 100}%`;
    if (document.activeElement !== startIn) startIn.value = fmt(a);
    if (document.activeElement !== endIn) endIn.value = fmt(b);
    length.textContent = `${fmt(b - a)} kept of ${fmt(d)}`;
  }
  const set = (na, nb) => {
    a = clamp(na, 0, d);
    b = clamp(nb, 0, d);
    if (b - a < 0.1) b = Math.min(d, a + 0.1);
    draw();
  };

  media.addEventListener('loadedmetadata', () => {
    d = media.duration || 0;
    set(0, d);
  });
  media.addEventListener('timeupdate', () => {
    if (looping && media.currentTime >= b) media.pause();
    draw();
  });

  line.addEventListener('pointerdown', (e) => {
    if (!d) return;
    line.setPointerCapture(e.pointerId);
    const at = (ev) => {
      const r = line.getBoundingClientRect();
      return clamp((ev.clientX - r.left) / r.width, 0, 1) * d;
    };
    const t0 = at(e);
    const edgePx = (6 / line.clientWidth) * d * 2;
    const which = Math.abs(t0 - a) < edgePx ? 'a' : Math.abs(t0 - b) < edgePx ? 'b' : 'seek';
    const move = (ev) => {
      const t = at(ev);
      if (which === 'a') set(Math.min(t, b - 0.1), b);
      else if (which === 'b') set(a, Math.max(t, a + 0.1));
      else media.currentTime = t;
      draw();
    };
    move(e);
    const up = () => {
      line.removeEventListener('pointermove', move);
      line.removeEventListener('pointerup', up);
    };
    line.addEventListener('pointermove', move);
    line.addEventListener('pointerup', up);
  });

  startIn.addEventListener('change', () => set(parseTime(startIn.value) || 0, b));
  endIn.addEventListener('change', () => set(a, parseTime(endIn.value) || d));

  let looping = false;
  const preview = () => {
    looping = true;
    media.currentTime = a;
    media.play();
  };
  addEventListener('keydown', (e) => {
    if (e.target instanceof HTMLInputElement) return;
    if (e.key === 'i') set(media.currentTime, b);
    if (e.key === 'o') set(a, media.currentTime);
  });

  controls.append(
    group('Start', startIn, h('button', { class: 'btn', onclick: () => set(media.currentTime, b) }, 'Set to playhead ', h('span', { class: 'kbd' }, 'I'))),
    group('End', endIn, h('button', { class: 'btn', onclick: () => set(a, media.currentTime) }, 'Set to playhead ', h('span', { class: 'kbd' }, 'O'))),
    group('Length', length, h('div', { class: 'row' }, h('button', { class: 'btn', onclick: preview }, 'Preview ▶'))),
  );
  return () => {
    if (!d) throw new Error('The file is still loading.');
    return { action: { op: 'trim', start: a, end: b }, title: 'Trimming' };
  };
}

/* ----------------------------------------------------------------- pdf pages */
async function thumbs(width) {
  const note = h('p', { class: 'label' }, 'Loading pages…');
  stage.append(note);
  const list = await invoke('pdf_thumbs', { path, width });
  note.remove();
  return list;
}

function pages() {
  const grid = h('div', { class: 'pages' });
  stage.append(grid);
  let items = []; // { i: original index, rot, src }
  const count = h('span', { class: 'value' }, '');
  const draw = () => {
    grid.replaceChildren(
      ...items.map((p, k) =>
        h(
          'figure',
          { class: 'page' },
          h('img', { src: p.src, alt: '', style: `transform: rotate(${p.rot}deg)` }),
          h(
            'figcaption',
            {},
            h('span', { class: 'label' }, `${p.i + 1}`),
            h('button', { class: 'icon', title: 'Move left', onclick: () => k > 0 && ([items[k - 1], items[k]] = [items[k], items[k - 1]], draw()) }, '‹'),
            h('button', { class: 'icon', title: 'Move right', onclick: () => k < items.length - 1 && ([items[k + 1], items[k]] = [items[k], items[k + 1]], draw()) }, '›'),
            h('button', { class: 'icon', title: 'Rotate', onclick: () => ((p.rot = (p.rot + 90) % 360), draw()) }, '↻'),
            h('button', { class: 'icon', title: 'Remove', onclick: () => (items.splice(k, 1), draw()) }, '✕'),
          ),
        ),
      ),
    );
    count.textContent = `${items.length} page${items.length === 1 ? '' : 's'}`;
  };
  thumbs(180).then((list) => {
    items = list.map((src, i) => ({ i, rot: 0, src }));
    draw();
  });
  controls.append(
    group('Pages', count, h('p', { class: 'label' }, 'Move, turn or remove pages. A new PDF is saved; the original stays as it is.')),
    group('Order', h('div', { class: 'row' }, h('button', { class: 'btn', onclick: () => (items.reverse(), draw()) }, 'Reverse'))),
  );
  return () => {
    if (!items.length) throw new Error('Keep at least one page.');
    return { action: { op: 'pages', order: items.map((p) => p.i), rotate: items.map((p) => p.rot) }, title: 'Saving pages' };
  };
}

/* ------------------------------------------------------------------ sign */
function sign() {
  let list = [];
  let page = 0;
  let box = null; // [x, y, w, h] fractions on the page
  const view = h('div', { class: 'frame' });
  const img = h('img', { alt: '' });
  const mark = h('div', { class: 'sign-box', hidden: true });
  view.append(img, mark);
  stage.append(view);
  const fit = () => {
    if (!img.naturalWidth) return;
    const s = Math.min((stage.clientWidth - 48) / img.naturalWidth, (stage.clientHeight - 48) / img.naturalHeight);
    view.style.width = `${img.naturalWidth * s}px`;
    view.style.height = `${img.naturalHeight * s}px`;
  };
  img.addEventListener('load', fit);
  new ResizeObserver(fit).observe(stage);

  // signature pad
  const pad = h('canvas', { class: 'pad', width: 560, height: 200 });
  const g = pad.getContext('2d');
  let inked = false;
  const clear = () => (g.clearRect(0, 0, pad.width, pad.height), (inked = false));
  pad.addEventListener('pointerdown', (e) => {
    pad.setPointerCapture(e.pointerId);
    const r = pad.getBoundingClientRect();
    const at = (ev) => [((ev.clientX - r.left) / r.width) * pad.width, ((ev.clientY - r.top) / r.height) * pad.height];
    g.lineWidth = 3.2;
    g.lineCap = g.lineJoin = 'round';
    g.strokeStyle = '#13235b';
    g.beginPath();
    g.moveTo(...at(e));
    const move = (ev) => (g.lineTo(...at(ev)), g.stroke(), (inked = true));
    const up = () => (pad.removeEventListener('pointermove', move), pad.removeEventListener('pointerup', up));
    pad.addEventListener('pointermove', move);
    pad.addEventListener('pointerup', up);
  });
  const typed = h('input', { type: 'text', placeholder: 'or type your name', class: 'text' });
  typed.addEventListener('input', () => {
    clear();
    g.font = '64px "Segoe Script", "Lucida Handwriting", cursive';
    g.fillStyle = '#13235b';
    g.textBaseline = 'middle';
    g.fillText(typed.value, 16, pad.height / 2, pad.width - 32);
    inked = !!typed.value.trim();
  });

  // place by dragging on the page
  view.addEventListener('pointerdown', (e) => {
    view.setPointerCapture(e.pointerId);
    const a = frac(view, e);
    const move = (ev) => {
      const p = frac(view, ev);
      box = [Math.min(a[0], p[0]), Math.min(a[1], p[1]), Math.abs(p[0] - a[0]), Math.abs(p[1] - a[1])];
      Object.assign(mark.style, { left: `${box[0] * 100}%`, top: `${box[1] * 100}%`, width: `${box[2] * 100}%`, height: `${box[3] * 100}%` });
      mark.hidden = false;
    };
    const up = () => (view.removeEventListener('pointermove', move), view.removeEventListener('pointerup', up));
    view.addEventListener('pointermove', move);
    view.addEventListener('pointerup', up);
  });

  const pageSel = h('select', { onchange: (e) => ((page = +e.target.value), (img.src = list[page]), (box = null), (mark.hidden = true)) });
  thumbs(900).then((l) => {
    list = l;
    l.forEach((_, i) => pageSel.append(h('option', { value: i }, `Page ${i + 1}`)));
    img.src = l[0];
  });
  controls.append(
    group('Signature', pad, typed, h('div', { class: 'row' }, h('button', { class: 'btn', onclick: () => ((typed.value = ''), clear()) }, 'Clear'))),
    group('Place', pageSel, h('p', { class: 'label' }, 'Drag a box on the page where the signature goes.')),
  );

  /** The ink only, cropped to its bounds, as a transparent PNG. */
  const ink = () => {
    const d = g.getImageData(0, 0, pad.width, pad.height).data;
    let [x0, y0, x1, y1] = [pad.width, pad.height, 0, 0];
    for (let y = 0; y < pad.height; y++)
      for (let x = 0; x < pad.width; x++)
        if (d[(y * pad.width + x) * 4 + 3] > 8) [x0, y0, x1, y1] = [Math.min(x0, x), Math.min(y0, y), Math.max(x1, x), Math.max(y1, y)];
    const c = h('canvas', { width: x1 - x0 + 8, height: y1 - y0 + 8 });
    c.getContext('2d').drawImage(pad, x0 - 4, y0 - 4, c.width, c.height, 0, 0, c.width, c.height);
    return { png: c.toDataURL('image/png'), ratio: c.width / c.height };
  };

  return () => {
    if (!inked) throw new Error('Draw or type a signature first.');
    if (!box || box[2] < 0.01) throw new Error('Drag a box on the page to place it.');
    const { png, ratio } = ink();
    // keep the signature's own shape inside the box
    const pageRatio = img.naturalWidth / img.naturalHeight;
    let [x, y, w, hh] = box;
    if (w / hh * pageRatio > ratio) {
      const nw = (hh * ratio) / pageRatio;
      x += (w - nw) / 2;
      w = nw;
    } else {
      const nh = (w * pageRatio) / ratio;
      y += (hh - nh) / 2;
      hh = nh;
    }
    return { action: { op: 'stamp', page, rect: [x, y, w, hh], png }, title: 'Signing' };
  };
}

/* ----------------------------------------------------------------- wiring */
const tools = { crop, addbg: addBg, redact, trim, pages, sign };
const getResult = (tools[toolName] || crop)();
const win = getCurrentWindow();

async function save() {
  try {
    const { action, title } = getResult();
    await invoke('run_action', { paths: [path], action, title });
    await win.close();
  } catch (e) {
    $('error').hidden = false;
    $('error').textContent = e?.message || String(e);
  }
}

$('save').addEventListener('click', save);
$('cancel').addEventListener('click', () => win.close());
addEventListener('keydown', (e) => {
  if (e.key === 'Escape') win.close();
  if (e.key === 'Enter' && !(e.target instanceof HTMLButtonElement)) save();
});

// First appearance: a different long reveal per region (header 4 s, panel 4 s, stage 5 s).
requestAnimationFrame(() => {
  timedReveal($('top'), 'decode', 4000, { x: 0, y: 0 });
  timedReveal($('panel'), signalPreset, 4000);
  timedReveal(stage, 'dither', 5000);
});
