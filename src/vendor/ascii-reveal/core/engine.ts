import { resolvePreset, type Preset, type PresetName } from './presets';
import { clamp, dpr as getDpr, ease, easeIO, freeCanvas, hash, isBrowser, parseColor, reducedMotion } from './util';
import { imageMascot, mascots, type MascotDraw, type MascotName } from './mascots';

/* ------------------------------------------------------------------ */
/* Public types                                                        */
/* ------------------------------------------------------------------ */

export type Origin = 'center' | 'top-left' | 'pointer' | { x: number; y: number };
/** `fast` (default) keeps data readable within ~400 ms; `cinematic` is the slow showcase wave. */
export type Duration = 'fast' | 'cinematic';

export interface AsciiRevealOptions {
  /** Built-in preset name or a custom preset object. Default `decode`. */
  preset?: PresetName | Preset;
  /** Playback speed multiplier. 2 is twice as fast. Default 1. */
  speed?: number;
  /** `fast` (default) for data, `cinematic` for heroes and demos. */
  duration?: Duration;
  /** Where the reveal wave starts. Default `center` of the element. */
  origin?: Origin;
  /** Only animate content inside elements matching this selector. */
  selector?: string;
  /** Elements that get the dot-field and corner frame before fading in. Default `[data-ascii="surface"]`. */
  surfaces?: string;
  /** Elements that simply fade in on the wave (buttons, chips). Default `[data-ascii="fade"]`. */
  fades?: string;
  /** Elements rendered through the glyph grid. Default `img, canvas, video, [data-ascii="media"]`. */
  media?: string;
  /** Element the effect draws into. Created automatically when omitted. */
  overlay?: HTMLElement;
  /**
   * Show an animated ASCII mascot while loading instead of the plain shimmer:
   * `spider`, `ghost`, `pumpkin`, `bat`, an SVG URL, SVG markup, or your own draw function.
   */
  mascot?: MascotName | string | MascotDraw | null;
  /**
   * Reveal sections below the fold as they scroll into view instead of all at once.
   * Applies to full reveals (no `within`). Default true.
   */
  lazy?: boolean;
  /** If the device can't keep up (frames over ~45 ms), finish instantly instead of stuttering. Default true. */
  fallback?: boolean;
}

export interface RunOptions {
  origin?: Origin;
  /** Limit this run to a part of the tree, for example one card. */
  within?: Element;
  speed?: number;
  duration?: Duration;
  preset?: PresetName | Preset;
  /** Hide only: skip the animation and hide immediately. */
  instant?: boolean;
}

export interface AsciiRevealController {
  /** Decode the content into place. Resolves when every target has landed. */
  reveal(options?: RunOptions): Promise<void>;
  /** Scramble the content back out. It stays hidden until the next `reveal()`. */
  hide(options?: RunOptions): Promise<void>;
  /** Show a glyph shimmer (or the mascot) where hidden content was, for loading states. */
  shimmer(on: boolean): void;
  update(options: Partial<AsciiRevealOptions>): void;
  destroy(): void;
}

/* ------------------------------------------------------------------ */
/* Timing                                                              */
/* ------------------------------------------------------------------ */

interface Timing {
  vel: number; // wave speed, px per ms
  maxWave: number; // cap on how long the wave may take to reach a target
  spread: (n: number) => number; // how long a text node takes to start all its characters
  char: [number, number]; // per-character scramble duration: base + random range
  media: number;
  surface: number;
  fade: number;
  onSurface: number; // content on a surface starts this much later
}
const TIMING: Record<Duration, Timing> = {
  fast: { vel: 3.2, maxWave: 220, spread: (n) => Math.min(200, 40 + n * 4), char: [90, 110], media: 450, surface: 300, fade: 220, onSurface: 60 },
  cinematic: { vel: 1.15, maxWave: Infinity, spread: (n) => Math.min(720, 90 + n * 13), char: [200, 240], media: 1150, surface: 720, fade: 420, onSurface: 140 },
};

/* ------------------------------------------------------------------ */
/* Global stylesheet, pointer tracking, scheduler                      */
/* ------------------------------------------------------------------ */

const T = 'data-ar-t'; // text parent hidden
const M = 'data-ar-m'; // media hidden / fading
const TC = 'data-ar-tc'; // gradient text being hidden
const S = 'data-ar-s'; // surface fading
const F = 'data-ar-f'; // fade element
const DEFAULT_FONT = 'ui-monospace, "SF Mono", "Geist Mono", "JetBrains Mono", Menlo, Consolas, monospace';

let styled = false;
function ensureStyles() {
  if (styled || !isBrowser()) return;
  styled = true;
  const el = document.createElement('style');
  el.setAttribute('data-ascii-reveal', '');
  // text is hidden with text-fill-color, not color, so icons drawn with currentColor stay visible
  el.textContent = `
[${T}]{-webkit-text-fill-color:transparent!important;text-shadow:none!important;text-decoration-color:transparent!important}
[${TC}]{background-image:none!important}
[${M}],[${S}],[${F}]{opacity:var(--ar-o,0)!important}
[data-ar-pending]{visibility:hidden}
.ar-overlay{position:absolute;inset:0;pointer-events:none;z-index:2;overflow:visible}
.ar-overlay>canvas{position:absolute;pointer-events:none;display:block}`;
  document.head.appendChild(el);
}

let pointer: { x: number; y: number } | null = null;
let tracking = false;
function trackPointer() {
  if (tracking || !isBrowser()) return;
  tracking = true;
  document.addEventListener('pointermove', (e) => (pointer = { x: e.clientX, y: e.clientY }), { passive: true });
  document.addEventListener('pointerdown', (e) => (pointer = { x: e.clientX, y: e.clientY }), { passive: true });
}

/** `object-position` as two functions: free space (device px) → offset. Percentages and keywords scale the free space; lengths are fixed. */
function objectPos(v: string): [(free: number, dp: number) => number, (free: number, dp: number) => number] {
  const KW: Record<string, number> = { left: 0, top: 0, center: 50, right: 100, bottom: 100 };
  const parts = (v || '50% 50%').trim().split(/\s+/);
  const one = (t: string | undefined) => (free: number, dp: number) => {
    if (!t) return free / 2;
    if (t in KW) return (free * KW[t]) / 100;
    if (t.endsWith('%')) return (free * parseFloat(t)) / 100;
    if (t.endsWith('px')) return parseFloat(t) * dp;
    return free / 2;
  };
  // keyword order can be swapped ("bottom right"); computed values are usually "x y" already
  let [a, b] = parts;
  if (a === 'top' || a === 'bottom' || b === 'left' || b === 'right') [a, b] = [b, a];
  return [one(a), one(b)];
}

interface Target {
  key: object;
  holdEl: Element;
  holdName: string;
  cx: number;
  cy: number;
  offset: number;
  dur: number;
  text?: boolean;
  insideSurface?: boolean;
  init(dir: 1 | -1): void;
  /** Still loading (an image being decoded): not drawn yet, the run waits for it. */
  pending?: boolean;
  prepare?(): void;
  ready?(): Promise<void>;
  /** `paint` is true on frames where the shared canvas was cleared and must be redrawn (~30 fps). */
  render(lt: number, dir: 1 | -1, paint: boolean): void;
  dispose(): void;
  rect?: { x: number; y: number; w: number; h: number };
}

/** Two canvases per run: `live` is redrawn each frame, `bake` receives text once it has landed. */
interface Layer {
  live: HTMLCanvasElement;
  lctx: CanvasRenderingContext2D;
  bake: HTMLCanvasElement;
  bctx: CanvasRenderingContext2D;
  x: number; // overlay coords of the layer's top-left (css px)
  y: number;
  w: number;
  h: number;
  dpr: number;
}

interface Run {
  ctrl: Controller;
  dir: 1 | -1;
  start: number;
  targets: Target[];
  byKey: Map<object, Target>;
  baked: Set<Target>;
  layer: Layer | null;
  lastPaint: number;
  done: Promise<void>;
  resolve: () => void;
  live: boolean;
}

interface Loop {
  render(now: number): boolean; // return false to stop
}

const runs = new Set<Run>();
const loops = new Set<Loop>();
const owner = new WeakMap<object, Run>();
const holds = new WeakMap<Element, Map<string, number>>();
let raf = 0;
// frame-budget watchdog
let lastTick = 0;
let slowFrames = 0;
let degradedUntil = 0; // after the watchdog fires, reveals fade instead of decoding for a while
const isDegraded = () => performance.now() < degradedUntil;

function hold(el: Element, name: string) {
  let m = holds.get(el);
  if (!m) holds.set(el, (m = new Map()));
  const n = (m.get(name) || 0) + 1;
  m.set(name, n);
  if (n === 1) {
    el.setAttribute(name, '');
    // gradient text (background-clip: text) paints through a transparent fill, so hide its background too
    if (name === T) {
      const cs = getComputedStyle(el);
      if ((cs.backgroundClip || (cs as unknown as Record<string, string>).webkitBackgroundClip || '').includes('text')) el.setAttribute(TC, '');
    }
  }
}
function release(el: Element, name: string) {
  const m = holds.get(el);
  if (!m) return;
  const n = (m.get(name) || 0) - 1;
  if (n > 0) return void m.set(name, n);
  m.delete(name);
  el.removeAttribute(name);
  if (name === T) el.removeAttribute(TC);
  if (name !== T && !el.hasAttribute(M) && !el.hasAttribute(S) && !el.hasAttribute(F)) {
    (el as HTMLElement).style.removeProperty('--ar-o');
  }
}

function kick() {
  if (!raf) {
    lastTick = 0;
    raf = requestAnimationFrame(tick);
  }
}

// Mobile browsers pause or slow animation frames while the page is being scrolled; those gaps say nothing
// about whether the effect itself is too heavy, so the watchdog ignores frames around scrolling.
let lastScroll = -Infinity;
if (isBrowser()) {
  const mark = () => (lastScroll = performance.now());
  for (const ev of ['scroll', 'touchmove', 'wheel']) window.addEventListener(ev, mark, { passive: true, capture: true });
}

function tick() {
  // schedule first: an error while drawing must never stop the loop for every later reveal on the page
  raf = requestAnimationFrame(tick);
  try {
    step();
  } catch (err) {
    // drop whatever was animating (its content is shown as-is) and keep going
    for (const run of Array.from(runs)) {
      try { finishRun(run); } catch { runs.delete(run); run.resolve(); }
    }
    for (const loop of Array.from(loops)) freeCanvas((loop as { canvas?: HTMLCanvasElement }).canvas);
    loops.clear();
    if (typeof console !== 'undefined') console.error('[ascii-reveal]', err);
  }
  if (!runs.size && !loops.size) {
    cancelAnimationFrame(raf);
    raf = 0;
  }
}

function step() {
  const now = performance.now();
  // watchdog: if frames keep blowing the budget while runs are animating, finish them instead of stuttering
  if (lastTick && runs.size) {
    const dt = now - lastTick;
    // a very long gap is a paused tab or a phone waking up, not a slow frame
    const counts = dt < 400 && now - lastScroll > 250;
    slowFrames = counts && dt > 45 ? slowFrames + 1 : Math.max(0, slowFrames - 1);
    if (slowFrames >= 8) {
      let fired = false;
      for (const run of Array.from(runs)) if (run.ctrl.opts.fallback !== false) { finishRun(run); fired = true; }
      if (fired) degradedUntil = performance.now() + 4_000;
      slowFrames = 0;
    }
  }
  lastTick = now;
  for (const run of Array.from(runs)) {
    if (!run.live) continue;
    const local = now - run.start;
    if (local < 0) continue;
    const L = run.layer;
    // the shared canvas is repainted at ~30 fps (glyphs only change every 60-90 ms); opacities update every frame
    const paint = now - run.lastPaint >= 30;
    if (paint) {
      run.lastPaint = now;
      if (L) L.lctx.clearRect(0, 0, L.live.width, L.live.height);
    }
    let finished = true;
    for (const t of run.targets) {
      if (owner.get(t.key) !== run || run.baked.has(t)) continue;
      if (t.pending) { finished = false; continue; }
      const tl = local - t.offset;
      if (tl < 0) {
        finished = false;
        // a hide that hasn't reached this text yet still shows it in full
        if (run.dir < 0 && t.text) t.render(t.dur, run.dir, paint);
        continue;
      }
      const lt = run.dir > 0 ? Math.min(tl, t.dur) : Math.max(0, t.dur - tl);
      t.render(lt, run.dir, paint);
      if (tl < t.dur) finished = false;
      else if (run.dir > 0 && t.text && L) {
        // landed: draw it once onto the bake layer and stop redrawing it every frame
        run.baked.add(t);
        (t as TextTarget).bakeInto(L.bctx);
      }
    }
    if (finished) finishRun(run);
  }
  for (const loop of Array.from(loops)) if (!loop.render(now)) loops.delete(loop);
}

function finishRun(run: Run) {
  runs.delete(run);
  for (const t of run.targets) {
    if (owner.get(t.key) !== run) continue;
    owner.delete(t.key);
    if (run.dir > 0) t.render(t.dur, 1, false);
    t.dispose();
    if (run.dir > 0) release(t.holdEl, t.holdName);
    else run.ctrl.markHidden(t.holdEl, t.holdName);
  }
  freeCanvas(run.layer?.live);
  freeCanvas(run.layer?.bake);
  run.ctrl.active(-1);
  run.resolve();
}

function claim(run: Run, t: Target) {
  const prev = owner.get(t.key);
  if (prev && prev !== run) {
    const pt = prev.byKey.get(t.key);
    if (pt) {
      pt.dispose();
      release(pt.holdEl, pt.holdName);
      prev.baked.delete(pt);
    }
  }
  owner.set(t.key, run);
  hold(t.holdEl, t.holdName);
}

/* ------------------------------------------------------------------ */
/* Environment: colors and fonts read once per run                     */
/* ------------------------------------------------------------------ */

interface Env {
  preset: Preset;
  timing: Timing;
  scale: number;
  ox: number; // overlay client left
  oy: number;
  ow: number;
  oh: number;
  /** region the text layer covers, in overlay coords (visible part of the element, plus a margin) */
  region: { x: number; y: number; w: number; h: number };
  overlay: HTMLElement;
  root: Element;
  font: string;
  accent: string | null;
  flash: [number, number, number, number] | null;
  glyph: string | null;
  dpr: number;
  origin: { x: number; y: number };
  styles: Map<Element, CSSStyleDeclaration>;
  clips: Map<Element, { x0: number; y0: number; x1: number; y1: number } | null>;
}

function cssVar(el: Element, name: string) {
  return getComputedStyle(el).getPropertyValue(name).trim();
}

function backdropColor(el: Element): [number, number, number] {
  const forced = parseColor(cssVar(el, '--ascii-surface'));
  if (forced) return [forced[0], forced[1], forced[2]];
  let n: Element | null = el.parentElement;
  while (n) {
    const c = parseColor(getComputedStyle(n).backgroundColor);
    if (c && c[3] >= 0.5) return [c[0], c[1], c[2]];
    n = n.parentElement;
  }
  return [255, 255, 255];
}

const styleOf = (env: Env, el: Element) => {
  let cs = env.styles.get(el);
  if (!cs) env.styles.set(el, (cs = getComputedStyle(el)));
  return cs;
};

/** Nearest clipping ancestor inside the root (scroll containers, overflow:hidden), in overlay coords. */
type Clip = { x0: number; y0: number; x1: number; y1: number } | null;
function clipOf(env: Env, el: Element): Clip {
  if (env.clips.has(el)) return env.clips.get(el)!;
  let res: { x0: number; y0: number; x1: number; y1: number } | null = null;
  const p = el.parentElement;
  if (el !== env.root && p) {
    const cs = styleOf(env, el);
    const clipping = /(hidden|auto|scroll|clip)/.test(cs.overflowX + cs.overflowY);
    const inherited: Clip = env.root.contains(p) ? clipOf(env, p) : null;
    if (clipping) {
      const r = el.getBoundingClientRect();
      res = { x0: r.left - env.ox, y0: r.top - env.oy, x1: r.right - env.ox, y1: r.bottom - env.oy };
      if (inherited) res = { x0: Math.max(res.x0, inherited.x0), y0: Math.max(res.y0, inherited.y0), x1: Math.min(res.x1, inherited.x1), y1: Math.min(res.y1, inherited.y1) };
    } else res = inherited;
  }
  env.clips.set(el, res);
  return res;
}

/* ------------------------------------------------------------------ */
/* Text: measured once, drawn on the run's canvas layer                */
/* ------------------------------------------------------------------ */

const SKIP_TAGS = new Set(['SCRIPT', 'STYLE', 'NOSCRIPT', 'TEXTAREA', 'INPUT', 'SELECT', 'OPTION', 'TEMPLATE']);

type Seg = { segment: string; index: number };
const segmenter: { segment(s: string): Iterable<Seg> } | null =
  typeof Intl !== 'undefined' && 'Segmenter' in Intl
    ? new (Intl as unknown as { Segmenter: new (l?: string, o?: object) => { segment(s: string): Iterable<Seg> } }).Segmenter(undefined, { granularity: 'grapheme' })
    : null;
function graphemes(s: string): Seg[] {
  if (segmenter) return Array.from(segmenter.segment(s));
  const out: Seg[] = [];
  let i = 0;
  for (const ch of s) { out.push({ segment: ch, index: i }); i += ch.length; }
  return out;
}

function transformText(s: string, tt: string, first: boolean) {
  if (tt === 'uppercase') return s.toUpperCase();
  if (tt === 'lowercase') return s.toLowerCase();
  if (tt === 'capitalize' && first) return s.toUpperCase();
  return s;
}

interface TextTarget extends Target {
  bakeInto(ctx: CanvasRenderingContext2D): void;
}

function textTarget(node: Text, env: Env, seed: number, layer: () => Layer): TextTarget | null {
  const parent = node.parentElement;
  if (!parent) return null;
  const cs = styleOf(env, parent);
  if (cs.visibility === 'hidden' || cs.display === 'none') return null;
  const str = node.nodeValue || '';
  const range = document.createRange();
  range.selectNodeContents(node);
  const br = range.getBoundingClientRect();
  const R = env.region;
  const bx = br.left - env.ox, by = br.top - env.oy;
  // skip text that isn't anywhere near the screen: it is left alone, not hidden
  if (bx > R.x + R.w || by > R.y + R.h || bx + br.width < R.x || by + br.height < R.y) return null;
  const clip = clipOf(env, parent);

  const xs: number[] = [], ys: number[] = [], ws: number[] = [], hs: number[] = [], chs: string[] = [];
  let prevSpace = true;
  for (const { segment, index } of graphemes(str)) {
    const space = /^\s+$/.test(segment);
    if (!space) {
      range.setStart(node, index);
      range.setEnd(node, index + segment.length);
      const r = range.getBoundingClientRect();
      if (r.width || r.height) {
        const x = r.left - env.ox, y = r.top - env.oy;
        const inRegion = x + r.width > R.x && y + r.height > R.y && x < R.x + R.w && y < R.y + R.h;
        const inClip = !clip || (x + r.width > clip.x0 && x < clip.x1 && y + r.height > clip.y0 && y < clip.y1);
        if (inRegion && inClip) {
          xs.push(x); ys.push(y); ws.push(r.width); hs.push(r.height);
          chs.push(transformText(segment, cs.textTransform, prevSpace));
        }
      }
    }
    prevSpace = space;
  }
  range.detach?.();
  const n = chs.length;
  if (!n) return null;

  const p = env.preset;
  const tm = env.timing;
  const sc = env.scale;
  const spread = tm.spread(n) * sc;
  const s0 = new Float32Array(n), s1 = new Float32Array(n), ph = new Float32Array(n);
  let dur = 1;
  for (let k = 0; k < n; k++) {
    s0[k] = (k / n) * spread * 0.72 + hash(k, seed, 1) * spread * 0.28;
    s1[k] = s0[k] + (tm.char[0] + hash(k, seed, 2) * tm.char[1]) * sc;
    ph[k] = hash(k, seed, 3) * 1000;
    if (s1[k] > dur) dur = s1[k];
  }
  const digits = !!parent.closest('[data-ascii-digits]') || /^[\d.,%:+\-\s€$£¥]+$/.test(str.trim());
  const set = digits ? '0123456789' : p.scramble;
  let color = cs.color;
  const pc = parseColor(color);
  if (!pc || pc[3] === 0) color = getComputedStyle(env.root).color; // gradient / transparent text
  const realFont = `${cs.fontStyle} ${cs.fontWeight} ${cs.fontSize} ${cs.fontFamily}`;
  const glyphFont = `400 ${cs.fontSize} ${env.font}`;
  const glyphColor = p.accentGlyphs && env.accent ? env.accent : env.glyph || color;
  const glyphAlpha = p.accentGlyphs && env.accent ? 0.9 : env.glyph ? 1 : 0.5;
  const rgb = parseColor(color) || [128, 128, 128, 1];
  const flash = env.flash;
  const flashMs = 420;
  let baseline = 0; // offset from a box's top to the alphabetic baseline, set on init

  const draw = (ctx: CanvasRenderingContext2D, lt: number, dir: 1 | -1, L: Layer) => {
    const d = L.dpr;
    ctx.save();
    ctx.setTransform(d, 0, 0, d, -L.x * d, -L.y * d);
    if (clip) {
      ctx.beginPath();
      ctx.rect(clip.x0, clip.y0, clip.x1 - clip.x0, clip.y1 - clip.y0);
      ctx.clip();
    }
    // pass 1: scrambling glyphs
    ctx.font = glyphFont;
    ctx.textAlign = 'center';
    ctx.textBaseline = 'middle';
    ctx.fillStyle = glyphColor;
    ctx.globalAlpha = glyphAlpha;
    let anyReal = false;
    for (let k = 0; k < n; k++) {
      if (lt < s0[k]) continue;
      if (lt >= s1[k]) { anyReal = true; continue; }
      const q = (lt - s0[k]) / (s1[k] - s0[k]);
      const t = Math.floor((lt + ph[k]) / p.tick);
      let g: string;
      if (p.cursor && q < 0.2) {
        ctx.fillStyle = env.accent || color;
        ctx.globalAlpha = 1;
        ctx.fillText(p.cursor, xs[k] + ws[k] / 2, ys[k] + hs[k] * 0.48);
        ctx.fillStyle = glyphColor;
        ctx.globalAlpha = glyphAlpha;
        continue;
      } else if (p.mode === 'ramp' && !digits && q < 0.7) {
        const Lr = p.ramp.length;
        let idx = 1 + Math.floor((q / 0.7) * (Lr - 1));
        if (hash(k, t, 5) < 0.35) idx += hash(k, t, 6) < 0.5 ? -1 : 1;
        g = p.ramp[clamp(idx, 1, Lr - 1)];
      } else g = set[Math.floor(hash(k, t, 7) * set.length)];
      ctx.fillText(g, xs[k] + ws[k] / 2, ys[k] + hs[k] * 0.48);
    }
    // pass 2: characters that have landed, in the element's own font
    if (anyReal) {
      ctx.font = realFont;
      ctx.textAlign = 'left';
      ctx.textBaseline = 'alphabetic';
      ctx.globalAlpha = 1;
      ctx.fillStyle = color;
      for (let k = 0; k < n; k++) {
        if (lt < s1[k]) continue;
        if (flash && dir > 0) {
          const f = 1 - clamp((lt - s1[k]) / flashMs);
          if (f > 0) {
            const m = (a: number, b: number) => Math.round(b + (a - b) * f);
            ctx.fillStyle = `rgb(${m(flash[0], rgb[0])},${m(flash[1], rgb[1])},${m(flash[2], rgb[2])})`;
            ctx.fillText(chs[k], xs[k], ys[k] + baseline);
            ctx.fillStyle = color;
            continue;
          }
        }
        ctx.fillText(chs[k], xs[k], ys[k] + baseline);
      }
    }
    ctx.restore();
  };

  let L: Layer | null = null;
  return {
    key: node,
    holdEl: parent,
    holdName: T,
    text: true,
    cx: br.left + br.width / 2,
    cy: br.top + br.height / 2,
    offset: 0,
    dur,
    rect: { x: bx, y: by, w: br.width, h: br.height },
    init() {
      L = layer();
      const ctx = L.lctx;
      ctx.save();
      ctx.font = realFont;
      const m = ctx.measureText('Hg');
      const asc = m.fontBoundingBoxAscent ?? m.actualBoundingBoxAscent;
      const desc = m.fontBoundingBoxDescent ?? m.actualBoundingBoxDescent;
      ctx.restore();
      const h = hs[0];
      baseline = (h - (asc + desc)) / 2 + asc;
    },
    render(lt, dir, paint) {
      if (L && paint) draw(L.lctx, lt, dir, L);
    },
    bakeInto(ctx) {
      if (L) draw(ctx, dur + 10_000, 1, L);
    },
    dispose() {
      L = null;
    },
  };
}

/* ------------------------------------------------------------------ */
/* Media: glyph grids precomputed per size step                        */
/* ------------------------------------------------------------------ */

let sInk = 0, sR = 0, sG = 0, sB = 0;

interface GridStep {
  ch: number;
  n: number;
  x: Float32Array;
  y: Float32Array;
  gate: Float32Array;
  ink: Float32Array;
  color: string[] | null;
}

function mediaTarget(el: HTMLElement, env: Env, layer: () => Layer): Target | null {
  const rc = el.getBoundingClientRect();
  if (rc.width < 2 || rc.height < 2) return null;
  const R = env.region;
  if (rc.left - env.ox > R.x + R.w || rc.top - env.oy > R.y + R.h || rc.right - env.ox < R.x || rc.bottom - env.oy < R.y) return null;
  const cs = styleOf(env, el);
  const dp = env.dpr;
  const W = Math.max(1, Math.round(rc.width * dp));
  const H = Math.max(1, Math.round(rc.height * dp));
  const x = rc.left - env.ox;
  const y = rc.top - env.oy;
  const p = env.preset;
  const dur = env.timing.media * env.scale;
  const cellScale = clamp(Math.min(rc.width, rc.height) / 230, 0.4, 1);
  const cells = p.cells.map((c) => Math.max(3, c * cellScale) * dp);
  const textColor = cs.color;
  const fixed = p.color === 'mono' ? textColor : p.color === 'accent' ? env.accent || textColor : null;
  const bg = backdropColor(el);
  const bgLum = (0.2126 * bg[0] + 0.7152 * bg[1] + 0.0722 * bg[2]) / 255;
  const ox = (env.origin.x - rc.left) * dp;
  const oy = (env.origin.y - rc.top) * dp;
  const maxD = Math.max(Math.hypot(ox, oy), Math.hypot(W - ox, oy), Math.hypot(ox, H - oy), Math.hypot(W - ox, H - oy)) || 1;
  let L: Layer | null = null;
  let data: Uint8ClampedArray | null = null;
  const steps: (GridStep | null)[] = cells.map(() => null);
  const rr = cs.borderTopLeftRadius;
  const radius = Math.min((rr.endsWith('%') ? (parseFloat(rr) / 100) * Math.min(W, H) : (parseFloat(rr) || 0) * dp), Math.min(W, H) / 2);

  const sample = (px: number, py: number) => {
    if (!data) {
      sInk = 0.22 + 0.5 * hash((px / (8 * dp)) | 0, (py / (8 * dp)) | 0, 41);
      sR = sG = sB = 128;
      return;
    }
    let ix = px | 0, iy = py | 0;
    if (ix < 0) ix = 0;
    if (iy < 0) iy = 0;
    if (ix >= W) ix = W - 1;
    if (iy >= H) iy = H - 1;
    const i = (iy * W + ix) * 4;
    const a = data[i + 3] / 255;
    const r = data[i] * a + bg[0] * (1 - a);
    const g = data[i + 1] * a + bg[1] * (1 - a);
    const b = data[i + 2] * a + bg[2] * (1 - a);
    const lum = (0.2126 * r + 0.7152 * g + 0.0722 * b) / 255;
    sInk = Math.abs(lum - bgLum) / Math.max(bgLum, 1 - bgLum);
    sR = r | 0;
    sG = g | 0;
    sB = b | 0;
  };

  /** Build a size step once: positions, reveal gate, ink and color for every cell with something to draw. */
  const buildStep = (si: number): GridStep => {
    const ch = cells[si], cw = ch * 0.62;
    const cols = Math.ceil(W / cw), rows = Math.ceil(H / ch);
    const dx = cw / 3, dy = ch / 3, qd = 6 * dp;
    const xs: number[] = [], ys: number[] = [], gs: number[] = [], ks: number[] = [];
    const colors: string[] | null = fixed ? null : [];
    for (let gy = 0; gy < rows; gy++) {
      const cy = (gy + 0.5) * ch;
      for (let gx = 0; gx < cols; gx++) {
        const cx = (gx + 0.5) * cw;
        let best = -1, br = 0, bgc = 0, bb = 0;
        sample(cx, cy); if (sInk > best) { best = sInk; br = sR; bgc = sG; bb = sB; }
        sample(cx - dx, cy - dy); if (sInk > best) { best = sInk; br = sR; bgc = sG; bb = sB; }
        sample(cx + dx, cy - dy); if (sInk > best) { best = sInk; br = sR; bgc = sG; bb = sB; }
        sample(cx - dx, cy + dy); if (sInk > best) { best = sInk; br = sR; bgc = sG; bb = sB; }
        sample(cx + dx, cy + dy); if (sInk > best) { best = sInk; br = sR; bgc = sG; bb = sB; }
        if (best < 0.04) continue; // nothing visible here at any flicker level
        xs.push(cx); ys.push(cy); ks.push(best);
        gs.push(0.5 * hash((cx / qd) | 0, (cy / qd) | 0, 11) + 0.5 * (Math.hypot(cx - ox, cy - oy) / maxD));
        colors?.push(`rgb(${br},${bgc},${bb})`);
      }
    }
    return { ch, n: xs.length, x: Float32Array.from(xs), y: Float32Array.from(ys), gate: Float32Array.from(gs), ink: Float32Array.from(ks), color: colors };
  };

  const setSrc = (o: number) => (el as HTMLElement).style.setProperty('--ar-o', o.toFixed(3));

  return {
    key: el,
    holdEl: el,
    holdName: M,
    cx: rc.left + rc.width / 2,
    cy: rc.top + rc.height / 2,
    offset: 0,
    dur,
    rect: { x, y, w: rc.width, h: rc.height },
    init(dir) {
      L = layer();
      setSrc(dir > 0 ? 0 : 1);
    },
    async ready() {
      if (el instanceof HTMLImageElement && !(el.complete && el.naturalWidth)) {
        await Promise.race([el.decode().catch(() => {}), new Promise((r) => setTimeout(r, 900))]);
      }
    },
    prepare() {
      let off: HTMLCanvasElement | null = null;
      try {
        off = document.createElement('canvas');
        off.width = W;
        off.height = H;
        const o = off.getContext('2d', { willReadFrequently: true })!;
        if (el instanceof HTMLImageElement) {
          if (!el.naturalWidth) throw 0;
          const iw = el.naturalWidth, ih = el.naturalHeight;
          if (cs.objectFit === 'cover' || cs.objectFit === 'contain') {
            const sc = cs.objectFit === 'cover' ? Math.max(W / iw, H / ih) : Math.min(W / iw, H / ih);
            // honour object-position, so the glyphs land exactly where the picture is drawn
            const [px, py] = objectPos(cs.objectPosition);
            o.drawImage(el, px(W - iw * sc, dp), py(H - ih * sc, dp), iw * sc, ih * sc);
          } else o.drawImage(el, 0, 0, W, H);
        } else if (el instanceof HTMLCanvasElement || el instanceof HTMLVideoElement) {
          o.drawImage(el, 0, 0, W, H);
        } else throw 0;
        data = o.getImageData(0, 0, W, H).data;
      } catch {
        data = null; // cross-origin or unknown element: use a neutral glyph texture
      } finally {
        freeCanvas(off);
      }
    },
    render(lt, _dir, paint) {
      const q = lt / dur;
      if (q >= 1) { setSrc(1); return; }
      setSrc(q < 0.68 ? 0 : easeIO((q - 0.68) / 0.32));
      if (q <= 0 || !paint || !L) return;
      const alpha = q < 0.74 ? 1 : 1 - ease((q - 0.74) / 0.26);
      const si = Math.min(cells.length - 1, Math.floor((q / 0.8) * cells.length));
      const st = steps[si] || (steps[si] = buildStep(si));
      const ctx = L.lctx;
      ctx.save();
      // cells are in device px relative to the element; place them on the shared layer and clip to its shape
      ctx.setTransform(1, 0, 0, 1, (x - L.x) * L.dpr, (y - L.y) * L.dpr);
      ctx.beginPath();
      if (radius > 0 && ctx.roundRect) ctx.roundRect(0, 0, W, H, radius);
      else ctx.rect(0, 0, W, H);
      ctx.clip();
      ctx.globalAlpha = alpha;
      ctx.font = `${Math.round(st.ch * 1.02)}px ${env.font}`;
      ctx.textAlign = 'center';
      ctx.textBaseline = 'middle';
      if (fixed) ctx.fillStyle = fixed;
      const t = Math.floor(lt / p.tick);
      const gate = q * 2.3, Lr = p.ramp.length;
      for (let i = 0; i < st.n; i++) {
        if (st.gate[i] > gate) continue;
        let lv = st.ink[i];
        if (hash(i, t, 3) < 0.16) lv += (hash(i, t, 77) - 0.5) * 0.35;
        const idx = Math.round(clamp(lv * 1.15) * (Lr - 1));
        if (idx <= 0) continue;
        if (st.color) ctx.fillStyle = st.color[i];
        ctx.fillText(p.ramp[idx], st.x[i], st.y[i]);
      }
      ctx.restore();
    },
    dispose() {
      L = null;
    },
  };
}

/* ------------------------------------------------------------------ */
/* Surfaces and fades                                                  */
/* ------------------------------------------------------------------ */

function surfaceTarget(el: HTMLElement, env: Env, layer: () => Layer): Target | null {
  const rc = el.getBoundingClientRect();
  if (rc.width < 2 || rc.height < 2) return null;
  const R = env.region;
  if (rc.left - env.ox > R.x + R.w || rc.top - env.oy > R.y + R.h || rc.right - env.ox < R.x || rc.bottom - env.oy < R.y) return null;
  const cs = styleOf(env, el);
  const dp = env.dpr;
  const W = Math.round(rc.width * dp), H = Math.round(rc.height * dp);
  const x = rc.left - env.ox, y = rc.top - env.oy;
  const dur = env.timing.surface * env.scale;
  const ox = (env.origin.x - rc.left) * dp, oy = (env.origin.y - rc.top) * dp;
  const maxD = Math.max(Math.hypot(ox, oy), Math.hypot(W - ox, oy), Math.hypot(ox, H - oy), Math.hypot(W - ox, H - oy)) || 1;
  const cell = 14 * dp, cols = Math.ceil(W / cell), rows = Math.ceil(H / cell);
  const rad = Math.min((parseFloat(cs.borderTopLeftRadius) || 0) * dp, Math.min(W, H) / 2);
  const color = env.preset.accentGlyphs && env.accent ? env.accent : env.glyph || cs.color;
  const stroke = env.accent && env.preset.accentGlyphs ? env.accent : cs.color;
  const dotA = env.preset.accentGlyphs ? 0.75 : 0.86;
  const dotAlpha = env.glyph ? 0.9 : 0.45;
  let L: Layer | null = null;
  return {
    key: el,
    holdEl: el,
    holdName: S,
    cx: rc.left + rc.width / 2,
    cy: rc.top + rc.height / 2,
    offset: 0,
    dur,
    rect: { x, y, w: rc.width, h: rc.height },
    init() {
      L = layer();
      el.style.setProperty('--ar-o', '0');
    },
    render(lt, _dir, paint) {
      const q = lt / dur;
      el.style.setProperty('--ar-o', easeIO((q - 0.15) / 0.6).toFixed(3));
      if (q >= 1 || q <= 0 || !paint || !L) return;
      const ctx = L.lctx;
      ctx.save();
      ctx.setTransform(1, 0, 0, 1, (x - L.x) * L.dpr, (y - L.y) * L.dpr);
      ctx.fillStyle = color;
      ctx.font = `${Math.round(11 * dp)}px ${env.font}`;
      ctx.textAlign = 'center';
      ctx.textBaseline = 'middle';
      for (let gy = 0; gy < rows; gy++)
        for (let gx = 0; gx < cols; gx++) {
          const cx = (gx + 0.5) * cell, cy = (gy + 0.5) * cell;
          const v = 0.45 * hash(gx, gy, 21) + 0.55 * (Math.hypot(cx - ox, cy - oy) / maxD);
          const l = (q - v * 0.55) / 0.32;
          if (l <= 0 || l >= 1) continue;
          ctx.globalAlpha = Math.sin(l * Math.PI) * dotAlpha;
          ctx.fillText(hash(gx, gy, 22) > dotA ? '+' : '·', cx, cy);
        }
      const a = q < 0.12 ? q / 0.12 : q > 0.55 ? 1 - (q - 0.55) / 0.3 : 1;
      if (a > 0) {
        ctx.globalAlpha = clamp(a) * 0.75;
        ctx.strokeStyle = stroke;
        ctx.lineWidth = 1.25 * dp;
        const o = 0.75 * dp, k = rad + 10 * dp;
        ctx.beginPath();
        ctx.moveTo(o, k); ctx.arcTo(o, o, k, o, rad); ctx.lineTo(k, o);
        ctx.moveTo(W - k, o); ctx.arcTo(W - o, o, W - o, k, rad); ctx.lineTo(W - o, k);
        ctx.moveTo(W - o, H - k); ctx.arcTo(W - o, H - o, W - k, H - o, rad); ctx.lineTo(W - k, H - o);
        ctx.moveTo(k, H - o); ctx.arcTo(o, H - o, o, H - k, rad); ctx.lineTo(o, H - k);
        ctx.stroke();
      }
      ctx.restore();
    },
    dispose() {
      L = null;
    },
  };
}

function fadeTarget(el: HTMLElement, env: Env): Target | null {
  const rc = el.getBoundingClientRect();
  if (!rc.width && !rc.height) return null;
  const dur = env.timing.fade * env.scale;
  return {
    key: el,
    holdEl: el,
    holdName: F,
    cx: rc.left + rc.width / 2,
    cy: rc.top + rc.height / 2,
    offset: 0,
    dur,
    init(dir) {
      el.style.setProperty('--ar-o', dir > 0 ? '0' : '1');
    },
    render(lt) {
      el.style.setProperty('--ar-o', ease(lt / dur).toFixed(3));
    },
    dispose() {},
  };
}

/* ------------------------------------------------------------------ */
/* Controller                                                          */
/* ------------------------------------------------------------------ */

class Controller implements AsciiRevealController {
  root: HTMLElement;
  opts: AsciiRevealOptions;
  overlay: HTMLElement;
  ownOverlay: boolean;
  hidden = new Map<Element, Set<string>>();
  lastRects: { x: number; y: number; w: number; h: number }[] = [];
  shimmerLoop: (Loop & { stopAt: number; canvas: HTMLCanvasElement }) | null = null;
  seed = (Math.random() * 1e6) | 0;
  n = 0;
  destroyed = false;
  private activeRuns = 0;
  private ro: ResizeObserver | null = null;
  private size = { w: 0, h: 0 };

  constructor(root: HTMLElement, opts: AsciiRevealOptions) {
    ensureStyles();
    trackPointer();
    this.root = root;
    this.opts = { ...opts };
    if (getComputedStyle(root).position === 'static') root.style.position = 'relative';
    this.ownOverlay = !opts.overlay;
    this.overlay = opts.overlay || document.createElement('div');
    this.overlay.classList.add('ar-overlay');
    this.overlay.setAttribute('aria-hidden', 'true');
    if (this.ownOverlay) root.appendChild(this.overlay);
    // if the layout changes size mid-run, positions are stale: land everything instead of misaligning
    if (typeof ResizeObserver !== 'undefined') {
      this.ro = new ResizeObserver(([e]) => {
        const { width: w, height: h } = e.contentRect;
        const changed = Math.abs(w - this.size.w) > 1 || Math.abs(h - this.size.h) > 1;
        this.size = { w, h };
        if (!changed || !this.activeRuns) return;
        for (const run of Array.from(runs)) if (run.ctrl === this && run.dir > 0) finishRun(run);
      });
      this.ro.observe(root);
    }
  }

  /** Content-box size of the root, in the same terms as ResizeObserver's contentRect. */
  private contentSize() {
    const r = this.root, cs = getComputedStyle(r);
    return {
      w: r.clientWidth - (parseFloat(cs.paddingLeft) || 0) - (parseFloat(cs.paddingRight) || 0),
      h: r.clientHeight - (parseFloat(cs.paddingTop) || 0) - (parseFloat(cs.paddingBottom) || 0),
    };
  }

  update(o: Partial<AsciiRevealOptions>) {
    Object.assign(this.opts, o);
  }

  /** Track running animations and tell the page (bubbling `asciireveal` event) when activity starts and stops. */
  active(d: 1 | -1) {
    const was = this.activeRuns > 0;
    this.activeRuns = Math.max(0, this.activeRuns + d);
    const is = this.activeRuns > 0;
    if (was !== is) this.root.dispatchEvent(new CustomEvent('asciireveal', { bubbles: true, detail: { active: is } }));
  }

  markHidden(el: Element, name: string) {
    let set = this.hidden.get(el);
    if (!set) this.hidden.set(el, (set = new Set()));
    if (set.has(name)) release(el, name); // already holding a hidden mark
    else set.add(name);
  }

  clearHidden(scope: Element) {
    for (const [el, names] of Array.from(this.hidden)) {
      if (!scope.contains(el) && el !== scope) continue;
      for (const n of names) release(el, n);
      this.hidden.delete(el);
    }
  }

  private origin(o: Origin | undefined, scope: Element) {
    const r = scope.getBoundingClientRect();
    const c = { x: r.left + r.width / 2, y: r.top + r.height / 2 };
    if (!o || o === 'center') return c;
    if (o === 'top-left') return { x: r.left, y: r.top };
    if (o === 'pointer') return pointer || c;
    return o;
  }

  private env(o: RunOptions, scope: Element): Env {
    const orc = this.overlay.getBoundingClientRect();
    const rs = getComputedStyle(this.root);
    const v = (n: string) => rs.getPropertyValue(n).trim();
    const flash = parseColor(v('--ascii-flash'));
    // the text layer covers the visible part of the element plus a margin, never the whole page
    const m = 120;
    const vx0 = Math.max(0, -orc.left - m), vy0 = Math.max(0, -orc.top - m);
    const vx1 = Math.min(orc.width, window.innerWidth - orc.left + m), vy1 = Math.min(orc.height, window.innerHeight - orc.top + m);
    return {
      preset: resolvePreset(o.preset ?? this.opts.preset),
      timing: TIMING[o.duration ?? this.opts.duration ?? 'fast'],
      scale: 1 / Math.max(0.1, o.speed ?? this.opts.speed ?? 1),
      ox: orc.left,
      oy: orc.top,
      ow: orc.width,
      oh: orc.height,
      region: { x: vx0, y: vy0, w: Math.max(0, vx1 - vx0), h: Math.max(0, vy1 - vy0) },
      overlay: this.overlay,
      root: this.root,
      font: v('--ascii-font') || DEFAULT_FONT,
      accent: v('--ascii-accent') || null,
      flash,
      glyph: v('--ascii-glyph') || null,
      dpr: getDpr(),
      origin: this.origin(o.origin ?? this.opts.origin, scope),
      styles: new Map(),
      clips: new Map(),
    };
  }

  private collect(scope: Element, env: Env, layer: () => Layer): Target[] {
    const sel = this.opts.selector;
    const inScope = (el: Element) => !el.closest('.ar-overlay') && (!sel || !!el.closest(sel));
    const all = (q: string) => {
      const list = Array.from(scope.querySelectorAll<HTMLElement>(q));
      if (scope.matches(q)) list.unshift(scope as HTMLElement);
      return list.filter((el) => inScope(el) && !el.closest('[data-ascii="skip"],[data-ascii-skeleton]'));
    };
    const surfaces = all(this.opts.surfaces || '[data-ascii="surface"]');
    const fades = all(this.opts.fades || '[data-ascii="fade"]');
    const media = all(this.opts.media || 'img, canvas, video, [data-ascii="media"]');
    const targets: Target[] = [];
    const seed = this.seed + ++this.n * 131;
    for (const el of surfaces) { const t = surfaceTarget(el, env, layer); if (t) targets.push(t); }
    for (const el of fades) { const t = fadeTarget(el, env); if (t) targets.push(t); }
    for (const el of media) { const t = mediaTarget(el, env, layer); if (t) targets.push(t); }
    const walker = document.createTreeWalker(scope, NodeFilter.SHOW_TEXT, {
      acceptNode: (node) => {
        const p = node.parentElement;
        if (!p || !node.nodeValue || !node.nodeValue.trim()) return NodeFilter.FILTER_REJECT;
        if (SKIP_TAGS.has(p.tagName) || p.closest('.ar-overlay')) return NodeFilter.FILTER_REJECT;
        if (p.closest('[data-ascii="skip"],[data-ascii-skeleton]')) return NodeFilter.FILTER_REJECT;
        if (sel && !p.closest(sel)) return NodeFilter.FILTER_REJECT;
        return NodeFilter.FILTER_ACCEPT;
      },
    });
    let k = 0;
    while (walker.nextNode()) {
      const t = textTarget(walker.currentNode as Text, env, seed + k++ * 977, layer);
      if (t) targets.push(t);
    }
    // content sitting on a surface starts a beat after the surface
    if (surfaces.length) {
      for (const t of targets) {
        if (t.holdName === S) continue;
        const el = t.holdEl;
        t.insideSurface = surfaces.some((s) => s !== el && s.contains(el));
      }
    }
    return targets;
  }

  /** Cheap pass for instant hides: which elements to hold, without measuring anything. */
  private collectLite(scope: Element): Pick<Target, 'key' | 'holdEl' | 'holdName'>[] {
    const sel = this.opts.selector;
    const ok = (el: Element) => !el.closest('.ar-overlay,[data-ascii="skip"],[data-ascii-skeleton]') && (!sel || !!el.closest(sel));
    const out: Pick<Target, 'key' | 'holdEl' | 'holdName'>[] = [];
    const add = (q: string, name: string) => {
      const list = Array.from(scope.querySelectorAll(q));
      if (scope.matches(q)) list.unshift(scope);
      for (const el of list) if (ok(el)) out.push({ key: el, holdEl: el, holdName: name });
    };
    add(this.opts.surfaces || '[data-ascii="surface"]', S);
    add(this.opts.fades || '[data-ascii="fade"]', F);
    add(this.opts.media || 'img, canvas, video, [data-ascii="media"]', M);
    const walker = document.createTreeWalker(scope, NodeFilter.SHOW_TEXT);
    while (walker.nextNode()) {
      const node = walker.currentNode as Text, p = node.parentElement;
      if (!p || !node.nodeValue?.trim() || SKIP_TAGS.has(p.tagName) || !ok(p)) continue;
      out.push({ key: node, holdEl: p, holdName: T });
    }
    return out;
  }

  private makeLayer(env: Env, r = env.region): Layer {
    const d = env.dpr;
    const mk = () => {
      const c = document.createElement('canvas');
      c.width = Math.max(1, Math.round(r.w * d));
      c.height = Math.max(1, Math.round(r.h * d));
      Object.assign(c.style, { left: r.x + 'px', top: r.y + 'px', width: r.w + 'px', height: r.h + 'px' });
      this.overlay.appendChild(c);
      return c;
    };
    const bake = mk(), live = mk();
    return { live, lctx: live.getContext('2d')!, bake, bctx: bake.getContext('2d')!, x: r.x, y: r.y, w: r.w, h: r.h, dpr: d };
  }

  private async run(dir: 1 | -1, o: RunOptions = {}): Promise<void> {
    if (!isBrowser()) return;
    const scope = o.within || this.root;
    if (dir < 0 && (o.instant || reducedMotion() || isDegraded())) {
      this.lastRects = [];
      for (const t of this.collectLite(scope)) {
        const prev = owner.get(t.key);
        if (prev) {
          // stop the running animation; its hold carries over as the hidden mark
          prev.byKey.get(t.key)?.dispose();
          prev.baked.delete(prev.byKey.get(t.key)!);
          owner.delete(t.key);
        } else hold(t.holdEl, t.holdName);
        if (t.holdName !== T) (t.holdEl as HTMLElement).style.setProperty('--ar-o', '0');
        this.markHidden(t.holdEl, t.holdName);
      }
      return;
    }
    const env = this.env(o, scope);
    // positions are measured against the layout as it is now; only a resize after this point makes them stale
    // (a loading swap usually resizes the wrapper right before the reveal starts, and that must not cancel it)
    this.size = this.contentSize();
    let layer: Layer | null = null;
    let area = env.region;
    const getLayer = () => (layer ||= this.makeLayer(env, area));
    const targets = this.collect(scope, env, getLayer);
    // the shared canvas only needs to cover what is animating (smaller canvas = cheaper frames)
    {
      let x0 = Infinity, y0 = Infinity, x1 = -Infinity, y1 = -Infinity;
      for (const t of targets) {
        const r = t.rect;
        if (!r || t.holdName === F) continue;
        x0 = Math.min(x0, r.x); y0 = Math.min(y0, r.y); x1 = Math.max(x1, r.x + r.w); y1 = Math.max(y1, r.y + r.h);
      }
      const R = env.region;
      if (isFinite(x0)) {
        x0 = Math.max(R.x, Math.floor(x0 - 4)); y0 = Math.max(R.y, Math.floor(y0 - 4));
        x1 = Math.min(R.x + R.w, Math.ceil(x1 + 4)); y1 = Math.min(R.y + R.h, Math.ceil(y1 + 4));
        area = { x: x0, y: y0, w: Math.max(1, x1 - x0), h: Math.max(1, y1 - y0) };
      }
    }
    if (dir < 0) this.lastRects = targets.filter((t) => t.rect).map((t) => t.rect!);

    if (dir > 0 && (reducedMotion() || isDegraded())) {
      // short fade instead of the decode: reduced motion, or a device that couldn't keep up
      for (const t of targets) {
        const prev = owner.get(t.key);
        if (prev) {
          prev.byKey.get(t.key)?.dispose();
          release(t.holdEl, t.holdName);
          owner.delete(t.key);
        }
      }
      this.clearHidden(scope);
      if ((scope as HTMLElement).animate) (scope as HTMLElement).animate([{ opacity: 0 }, { opacity: 1 }], { duration: 220, easing: 'ease-out' });
      return;
    }

    let resolve!: () => void;
    const run: Run = {
      ctrl: this,
      dir,
      start: Infinity,
      targets,
      byKey: new Map(targets.map((t) => [t.key, t])),
      baked: new Set(),
      layer: null,
      lastPaint: -Infinity,
      done: new Promise<void>((r) => (resolve = r)),
      resolve: () => resolve(),
      live: false,
    };
    for (const t of targets) {
      claim(run, t);
      t.init(dir);
    }
    run.layer = layer;
    if (dir > 0) this.clearHidden(scope);

    const tm = env.timing;
    const vel = tm.vel / env.scale;
    for (const t of targets) {
      const wave = Math.min(Math.hypot(t.cx - env.origin.x, t.cy - env.origin.y) / vel, tm.maxWave * env.scale);
      t.offset = wave + (t.insideSurface && dir > 0 ? tm.onSurface * env.scale : 0);
    }
    // text starts decoding right away; an image that is still loading joins the wave once it has decoded
    // (waiting for every image first left the whole block blank for as long as the slowest image took)
    for (const t of targets) {
      if (dir > 0 && t.ready) {
        t.pending = true;
        t.ready().then(() => {
          t.prepare?.();
          t.pending = false;
          if (run.live) t.offset = Math.max(t.offset, performance.now() - run.start);
        });
      } else t.prepare?.();
    }
    if (this.destroyed || !targets.some((t) => owner.get(t.key) === run)) {
      freeCanvas((layer as Layer | null)?.live);
      freeCanvas((layer as Layer | null)?.bake);
      run.resolve();
      return run.done;
    }
    run.start = performance.now() + 16;
    run.live = true;
    runs.add(run);
    this.active(1);
    kick();
    return run.done;
  }

  reveal(o: RunOptions = {}) {
    if (this.opts.lazy !== false && !o.within && isBrowser() && !reducedMotion() && 'IntersectionObserver' in window) return this.lazyReveal(o);
    return this.run(1, o);
  }

  private io: IntersectionObserver | null = null;
  /** Reveal what's on screen now; hide the rest and reveal each section when it scrolls into view. */
  private lazyReveal(o: RunOptions): Promise<void> {
    const kids = (el: Element) => Array.from(el.children).filter((c) => !c.classList.contains('ar-overlay'));
    let scope: Element = this.root;
    while (kids(scope).length === 1) scope = kids(scope)[0];
    const parts = kids(scope);
    const vh = window.innerHeight;
    const near: Element[] = [], far: Element[] = [];
    for (const el of parts) {
      const r = el.getBoundingClientRect();
      (r.top < vh * 1.02 && r.bottom > 0 ? near : far).push(el);
    }
    if (!far.length || !near.length) return this.run(1, o);
    this.io?.disconnect();
    const io = new IntersectionObserver((entries) => {
      for (const e of entries) {
        if (!e.isIntersecting) continue;
        io.unobserve(e.target);
        const r = e.target.getBoundingClientRect();
        // each section decodes from its own top edge as it arrives
        this.run(1, { ...o, within: e.target, origin: { x: r.left + r.width / 2, y: Math.max(r.top, 0) } });
      }
    }, { rootMargin: '0px 0px -8% 0px' });
    this.io = io;
    // off-screen sections are hidden (text stays in the DOM for readers and search engines) and decode on arrival
    for (const el of far) {
      this.run(-1, { ...o, within: el, instant: true });
      io.observe(el);
    }
    return Promise.all(near.map((el) => this.run(1, { ...o, within: el }))).then(() => {});
  }

  hide(o?: RunOptions) {
    return this.run(-1, o);
  }

  shimmer(on: boolean) {
    if (!isBrowser()) return;
    if (!on) {
      if (this.shimmerLoop && this.shimmerLoop.stopAt === Infinity) this.shimmerLoop.stopAt = performance.now();
      return;
    }
    if (this.shimmerLoop) {
      this.shimmerLoop.stopAt = Infinity;
      return;
    }
    if (reducedMotion()) return;
    const draw = this.mascotDraw();
    // a skeleton already shows the loading state; the plain shimmer would only add noise on top of it
    if (!draw && this.root.querySelector('[data-ascii-skeleton]')) return;
    const dp = getDpr();
    const orc = this.overlay.getBoundingClientRect();
    const canvas = document.createElement('canvas');
    canvas.width = Math.round(orc.width * dp);
    canvas.height = Math.round(orc.height * dp);
    Object.assign(canvas.style, { left: '0px', top: '0px', width: orc.width + 'px', height: orc.height + 'px' });
    this.overlay.appendChild(canvas);
    const ctx = canvas.getContext('2d')!;
    if (draw) {
      const loop = this.mascotLoop(canvas, ctx, draw, dp, orc);
      this.shimmerLoop = loop;
      loops.add(loop);
      kick();
      return;
    }
    const preset = resolvePreset(this.opts.preset);
    const ramp = preset.ramp.slice(1, Math.max(3, Math.ceil(preset.ramp.length * 0.6)));
    const font = cssVar(this.root, '--ascii-font') || DEFAULT_FONT;
    const accent = cssVar(this.root, '--ascii-accent');
    const color = (preset.accentGlyphs || preset.cursor) && accent ? accent : cssVar(this.root, '--ascii-glyph') || getComputedStyle(this.root).color;
    const rects = (this.lastRects.length ? this.lastRects : [{ x: 0, y: 0, w: orc.width, h: orc.height }]).map((r) => ({
      x: r.x * dp, y: r.y * dp, w: r.w * dp, h: r.h * dp,
    }));
    const ch = 11 * dp, cw = ch * 0.62;
    const t0 = performance.now();
    let lastDraw = -Infinity;
    const loop = {
      stopAt: Infinity,
      canvas,
      render: (now: number) => {
        const lt = now - t0;
        const fin = clamp(lt / 260);
        const fout = loop.stopAt === Infinity ? 1 : 1 - clamp((now - loop.stopAt) / 260);
        if (fout <= 0) {
          freeCanvas(canvas);
          if (this.shimmerLoop === loop) this.shimmerLoop = null;
          return false;
        }
        if (lt - lastDraw < 33 && fin >= 1 && fout >= 1) return true; // ~30 fps is plenty for a shimmer
        lastDraw = lt;
        ctx.clearRect(0, 0, canvas.width, canvas.height);
        ctx.globalAlpha = 0.6 * fin * fout;
        ctx.font = `${Math.round(ch)}px ${font}`;
        ctx.textAlign = 'center';
        ctx.textBaseline = 'middle';
        ctx.fillStyle = color;
        const tick = Math.floor(lt / 70);
        for (const R of rects) {
          const cols = Math.max(1, Math.floor(R.w / cw)), rows = Math.max(1, Math.floor(R.h / ch));
          const x0 = R.x + (R.w - cols * cw) / 2;
          for (let gy = 0; gy < rows; gy++)
            for (let gx = 0; gx < cols; gx++) {
              const w = 0.5 + 0.5 * Math.sin(gx * 0.5 - lt * 0.007 + gy * 1.7 + R.y * 0.01);
              let idx = Math.floor(w * ramp.length * 0.999);
              if (hash((gx + R.x) | 0, gy, tick) < 0.2) idx = Math.floor(hash(gx, (gy + R.y) | 0, tick) * ramp.length);
              ctx.fillText(ramp[idx], x0 + (gx + 0.5) * cw, R.y + (gy + 0.5) * ch + (R.h - rows * ch) / 2);
            }
        }
        ctx.globalAlpha = 1;
        return true;
      },
    };
    this.shimmerLoop = loop;
    loops.add(loop);
    kick();
  }

  private mascotCache: { key: unknown; draw: MascotDraw } | null = null;
  private mascotDraw(): MascotDraw | null {
    const m = this.opts.mascot;
    if (!m) return null;
    if (typeof m === 'function') return m;
    if (m in mascots) return mascots[m as MascotName];
    if (this.mascotCache?.key !== m) this.mascotCache = { key: m, draw: imageMascot(m) };
    return this.mascotCache.draw;
  }

  /** Animated mascot rendered through the glyph grid where the hidden content was. */
  private mascotLoop(canvas: HTMLCanvasElement, ctx: CanvasRenderingContext2D, draw: MascotDraw, dp: number, orc: DOMRect) {
    // area: the union of what was hidden, padded, at least 170px tall, kept inside the element
    let x0 = Infinity, y0 = Infinity, x1 = -Infinity, y1 = -Infinity;
    for (const r of this.lastRects) { x0 = Math.min(x0, r.x); y0 = Math.min(y0, r.y); x1 = Math.max(x1, r.x + r.w); y1 = Math.max(y1, r.y + r.h); }
    if (!isFinite(x0)) { x0 = 0; y0 = 0; x1 = orc.width; y1 = orc.height; }
    x0 -= 12; y0 -= 12; x1 += 12; y1 += 12;
    const minH = Math.min(170, orc.height);
    if (y1 - y0 < minH) { const c = (y0 + y1) / 2; y0 = c - minH / 2; y1 = c + minH / 2; }
    x0 = Math.max(0, x0); y0 = Math.max(0, y0); x1 = Math.min(orc.width, x1); y1 = Math.min(orc.height, y1);
    const preset = resolvePreset(this.opts.preset);
    const ramp = preset.ramp;
    const L = ramp.length;
    const font = cssVar(this.root, '--ascii-font') || DEFAULT_FONT;
    const color = cssVar(this.root, '--ascii-mascot') || getComputedStyle(this.root).color;
    const ch = 7 * dp, cw = ch * 0.62;
    const cols = Math.max(4, Math.floor(((x1 - x0) * dp) / cw)), rows = Math.max(4, Math.floor(((y1 - y0) * dp) / ch));
    const ox = x0 * dp + ((x1 - x0) * dp - cols * cw) / 2, oy = y0 * dp + ((y1 - y0) * dp - rows * ch) / 2;
    // the mascot is drawn at 2 samples per cell, then read back as glyph densities
    const off = document.createElement('canvas');
    const OW = cols * 2, OH = Math.round(rows * 2 * (ch / cw));
    off.width = OW; off.height = OH;
    const o = off.getContext('2d', { willReadFrequently: true })!;
    const sy = OH / rows;
    const t0 = performance.now();
    let lastDraw = -Infinity;
    const loop = {
      stopAt: Infinity,
      canvas,
      render: (now: number) => {
        const lt = now - t0;
        const fin = clamp(lt / 320);
        const gone = loop.stopAt === Infinity ? 0 : clamp((now - loop.stopAt) / 450);
        if (gone >= 1) {
          freeCanvas(canvas);
          freeCanvas(off);
          if (this.shimmerLoop === loop) this.shimmerLoop = null;
          return false;
        }
        if (lt - lastDraw < 33) return true; // mascots animate at ~30 fps
        lastDraw = lt;
        o.clearRect(0, 0, OW, OH);
        o.save();
        draw(o, OW, OH, lt / 1000, color);
        o.restore();
        const d = o.getImageData(0, 0, OW, OH).data;
        ctx.clearRect(0, 0, canvas.width, canvas.height);
        ctx.font = `${Math.round(ch)}px ${font}`;
        ctx.textAlign = 'center';
        ctx.textBaseline = 'middle';
        const tick = Math.floor(lt / 80);
        let lastFill = '';
        for (let gy = 0; gy < rows; gy++) {
          const py = Math.min(OH - 1, Math.floor((gy + 0.5) * sy));
          for (let gx = 0; gx < cols; gx++) {
            if (hash(gx, gy, 9) > fin || hash(gx, gy, 13) < gone) continue;
            const px = gx * 2 + 1;
            let a = 0, r = 0, g = 0, b = 0;
            for (let s = 0; s < 4; s++) {
              const i = (Math.max(0, py - (s >> 1)) * OW + Math.max(0, px - (s & 1))) * 4;
              const al = d[i + 3] / 255;
              a += al; r += d[i] * al; g += d[i + 1] * al; b += d[i + 2] * al;
            }
            if (a < 0.25) continue;
            r /= a; g /= a; b /= a; a /= 4;
            let lv = a;
            if (hash(gx, gy, tick) < 0.12) lv += (hash(gx, gy, tick + 5) - 0.5) * 0.4;
            const idx = Math.max(1, Math.min(L - 1, Math.round(clamp(lv) * (L - 1))));
            // quantize colors a little so most cells reuse the previous fillStyle
            const fill = `rgb(${r & 0xf8},${g & 0xf8},${b & 0xf8})`;
            if (fill !== lastFill) { ctx.fillStyle = fill; lastFill = fill; }
            ctx.fillText(ramp[idx], ox + (gx + 0.5) * cw, oy + (gy + 0.5) * ch);
          }
        }
        return true;
      },
    };
    return loop;
  }

  destroy() {
    this.destroyed = true;
    this.io?.disconnect();
    this.io = null;
    this.ro?.disconnect();
    this.ro = null;
    for (const run of Array.from(runs)) {
      if (run.ctrl !== this) continue;
      runs.delete(run);
      for (const t of run.targets) {
        if (owner.get(t.key) !== run) continue;
        owner.delete(t.key);
        t.dispose();
        release(t.holdEl, t.holdName);
      }
      freeCanvas(run.layer?.live);
      freeCanvas(run.layer?.bake);
      run.resolve();
    }
    this.activeRuns = 0;
    for (const [el, names] of this.hidden) for (const n of names) release(el, n);
    this.hidden.clear();
    if (this.shimmerLoop) {
      loops.delete(this.shimmerLoop);
      freeCanvas(this.shimmerLoop.canvas);
      this.shimmerLoop = null;
    }
    if (this.ownOverlay) this.overlay.remove();
    else this.overlay.replaceChildren();
  }
}

/** Attach the effect to any element. Works without React. */
export function createAsciiReveal(root: HTMLElement, options: AsciiRevealOptions = {}): AsciiRevealController {
  return new Controller(root, options);
}
