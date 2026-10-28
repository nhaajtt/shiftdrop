import { mascots, type MascotDraw, type Origin, type Preset } from '../vendor/ascii-reveal/core';

/**
 * Reveal-length math.
 *
 * The library's cinematic wave travels at VEL px/ms from the origin; every target starts when the wave reaches
 * its centre and then needs a tail (text spread + per-character scramble, or a surface fade) to land.
 * Natural length at speed 1 = max over targets of (distance / VEL + own duration). Every timing in the engine is divided by `speed`,
 * so `speed = natural / target` lands the whole reveal on the target length.
 */
export const VEL = 1.15; // px per ms (engine: cinematic.vel)
const CHAR_BASE = 200; // engine: cinematic.char = [200, 240] → 200 + hash·240
const CHAR_RAND = 240;
const SURFACE = 720; // engine: cinematic.surface
const ON_SURFACE = 140; // engine: cinematic.onSurface
const MEDIA = 1150; // engine: cinematic.media
const FADE = 420; // engine: cinematic.fade
const SKIP_TAGS = new Set(['SCRIPT', 'STYLE', 'NOSCRIPT', 'TEXTAREA', 'INPUT', 'SELECT', 'OPTION', 'TEMPLATE']);
const SKIPPED = '.ar-overlay,[data-ascii="skip"],[data-ascii-skeleton]';

/**
 * Text node tail. Engine: char k starts at (k/n·0.72 + h·0.28)·spread with spread = min(720, 90 + 13n) and
 * scrambles for 200 + h'·240 ms; q = n/(n+1) is the expected largest of n random draws.
 */
const textTail = (n: number) => {
  const spread = Math.min(720, 90 + n * 13);
  const q = n / (n + 1);
  return spread * ((0.72 * (n - 1)) / n + 0.28 * q) + CHAR_BASE + CHAR_RAND * q;
};

/** The engine only measures what is inside the viewport plus this margin. */
const MARGIN = 120;

export interface Point {
  x: number;
  y: number;
}

let lastPointer: Point | null = null;
if (typeof window !== 'undefined') {
  const track = (e: PointerEvent) => (lastPointer = { x: e.clientX, y: e.clientY });
  window.addEventListener('pointerdown', track, { passive: true, capture: true });
}

export function pointerOr(fallback: Point): Point {
  return lastPointer ?? fallback;
}

export function centerOf(el: Element): Point {
  const r = el.getBoundingClientRect();
  return { x: r.left + r.width / 2, y: r.top + r.height / 2 };
}

/** Turns any origin the library accepts into a concrete viewport point for `el`. */
export function resolveOrigin(origin: Origin | undefined, el: Element): Point {
  const r = el.getBoundingClientRect();
  if (!origin || origin === 'center') return { x: r.left + r.width / 2, y: r.top + r.height / 2 };
  if (origin === 'top-left') return { x: r.left, y: r.top };
  if (origin === 'pointer') return pointerOr({ x: r.left + r.width / 2, y: r.top + r.height / 2 });
  return origin;
}

/** Visible part of the element (what the engine will animate). */
function visibleRect(el: Element) {
  const r = el.getBoundingClientRect();
  const x0 = Math.max(r.left, -MARGIN);
  const y0 = Math.max(r.top, -MARGIN);
  const x1 = Math.min(r.right, window.innerWidth + MARGIN);
  const y1 = Math.min(r.bottom, window.innerHeight + MARGIN);
  return { x0, y0, x1: Math.max(x0, x1), y1: Math.max(y0, y1) };
}

/**
 * Natural cinematic length at speed 1 for a reveal of `el` from `p`: the engine's own schedule, replayed —
 * each target starts when the wave reaches its centre (distance / VEL) and then runs its own duration.
 */
export function naturalMs(el: Element, p: Point): number {
  const v = visibleRect(el);
  const inView = (r: DOMRect) => r.width + r.height > 0 && r.right > v.x0 && r.left < v.x1 && r.bottom > v.y0 && r.top < v.y1;
  const wave = (r: DOMRect) => Math.hypot(r.left + r.width / 2 - p.x, r.top + r.height / 2 - p.y) / VEL;
  const surfaces = Array.from(el.querySelectorAll('[data-ascii="surface"]')).filter((s) => !s.closest(SKIPPED));
  let end = 0;
  const take = (r: DOMRect, dur: number) => {
    if (inView(r)) end = Math.max(end, wave(r) + dur);
  };
  for (const s of surfaces) take(s.getBoundingClientRect(), SURFACE);
  el.querySelectorAll('[data-ascii="fade"]').forEach((f) => !f.closest(SKIPPED) && take(f.getBoundingClientRect(), FADE));
  el.querySelectorAll('img, canvas, video, [data-ascii="media"]').forEach((m) => !m.closest(SKIPPED) && take(m.getBoundingClientRect(), MEDIA));
  const range = document.createRange();
  const walker = document.createTreeWalker(el, NodeFilter.SHOW_TEXT);
  for (let i = 0; i < 1500 && walker.nextNode(); i++) {
    const node = walker.currentNode as Text;
    const parent = node.parentElement;
    const n = (node.nodeValue ?? '').trim().length;
    if (!parent || !n || SKIP_TAGS.has(parent.tagName) || parent.closest(SKIPPED)) continue;
    range.selectNodeContents(node);
    const onSurface = surfaces.some((s) => s.contains(parent)) ? ON_SURFACE : 0;
    take(range.getBoundingClientRect(), textTail(n) + onSurface);
  }
  return Math.max(end, 400);
}

/** Measured in headless Chrome at 1440/768/375 px: the replayed schedule lands ~5 % early on average. */
const CALIBRATION = 0.94;

export function speedFor(el: Element, p: Point, targetMs: number): number {
  return Math.max(0.1, (naturalMs(el, p) * CALIBRATION) / targetMs);
}

/** Context-panel preset: dots condense into place, brackets scramble, accent glyphs. */
export const signalPreset: Preset = {
  ramp: ' ·•●',
  scramble: '<>[]{}()',
  mode: 'ramp',
  color: 'accent',
  cells: [22, 15, 10, 7, 6],
  tick: 70,
  accentGlyphs: true,
};

export interface RevealRecord {
  name: string;
  target: number;
  speed: number;
  ms: number;
}

/** Measured reveal lengths, readable from devtools in development builds. */
export function record(r: RevealRecord) {
  if (!import.meta.env.DEV) return;
  const w = window as unknown as { __ocReveals?: RevealRecord[] };
  (w.__ocReveals ??= []).push(r);
}

/**
 * The library's pumpkin draws in fixed orange tones. This wrapper keeps its shape and animation but
 * repaints every pixel it drew in the mascot color (`--ascii-mascot`, our accent), so no orange appears.
 */
export const pumpkinMascot: MascotDraw = (ctx, w, h, t, color) => {
  mascots.pumpkin(ctx, w, h, t, color);
  ctx.save();
  ctx.globalCompositeOperation = 'source-in';
  ctx.fillStyle = color;
  ctx.fillRect(0, 0, w, h);
  ctx.restore();
};
