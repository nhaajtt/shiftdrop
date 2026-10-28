import { createAsciiReveal } from '../vendor/ascii-reveal/core';
import { centerOf, speedFor } from './timing';

const reduced = () => matchMedia('(prefers-reduced-motion: reduce)').matches;

/**
 * Long ASCII reveal of one region, stretched to `targetMs` (4–6 s). Typing or clicking inside the
 * region lands it at once, so the UI never waits on the effect.
 */
export function timedReveal(el, preset, targetMs, origin) {
  if (!el) return;
  if (reduced()) {
    el.animate([{ opacity: 0 }, { opacity: 1 }], { duration: 220 });
    return;
  }
  const c = createAsciiReveal(el, { preset, duration: 'cinematic', lazy: false, fallback: true });
  const p = origin ?? centerOf(el);
  const finish = () => void c.reveal({ within: el, duration: 'fast', speed: 1000 });
  for (const ev of ['pointerdown', 'input', 'keydown']) el.addEventListener(ev, finish, { capture: true, once: true });
  void c.reveal({ origin: p, speed: speedFor(el, p, targetMs), duration: 'cinematic' });
}
