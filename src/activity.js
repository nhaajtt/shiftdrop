import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { fixFor, runFix } from './fixes.js';
import { addView, h, name, shared } from './ui.js';

// Everything ShiftDrop did, newest first, kept across restarts. Failures show what went wrong,
// why, and a button for the fix; successes link to their results.

const SEEN = 'shiftdrop.seen'; // newest job id the user has looked at
let history = [];
let filter = 'all';

const failed = (r) => r.state === 'error' || r.state === 'partial';
const unseen = () => history.filter((r) => failed(r) && r.id > Number(localStorage.getItem(SEEN) || 0)).length;

function when(t) {
  const s = Math.floor(Date.now() / 1000) - t;
  if (s < 60) return 'just now';
  if (s < 3600) return `${Math.floor(s / 60)} min ago`;
  if (s < 86400) return `${Math.floor(s / 3600)} h ago`;
  return new Date(t * 1000).toLocaleString();
}

async function load() {
  history = await invoke('get_history').catch(() => []);
  shared.nav?.();
  if (shared.view?.() === 'activity') shared.show('activity');
}

function card(r) {
  const bad = failed(r);
  const el = h(
    'div',
    { class: `card glass act${bad ? ' error' : ''}` },
    h('header', {}, h('span', { class: `state ${r.state}` }, bad ? (r.state === 'partial' ? 'Partly failed' : 'Failed') : 'Done'), h('b', { title: r.title }, r.title), h('span', { class: 'when' }, when(r.time))),
    h('div', { class: 'files' }, r.files.slice(0, 6).map((p) => h('span', { class: 'chip', title: p }, name(p))), r.files.length > 6 && h('span', { class: 'chip' }, `+${r.files.length - 6}`)),
  );
  if (r.note) el.append(h('p', { class: 'note' }, r.note));
  if (bad && r.error) {
    const fix = fixFor(r.error);
    const btns = h('div', { class: 'row-btns' });
    if (fix.action) {
      const b = h('button', { class: 'btn primary' }, fix.action.label);
      b.onclick = async () => {
        b.disabled = true;
        if (fix.action.kind === 'addon') b.textContent = 'Downloading…';
        try {
          await runFix(fix.action, invoke, `${r.title}: ${r.error}`);
          if (fix.action.kind === 'addon') b.textContent = 'Installed ✓';
          else b.disabled = false;
        } catch (e) {
          b.disabled = false;
          b.textContent = fix.action.label;
          fixBox.querySelector('p').textContent = String(e);
        }
      };
      btns.append(b);
    }
    if (r.action) {
      btns.append(h('button', { class: 'btn', onclick: () => invoke('run_action', { paths: r.files, action: r.action, title: r.title }) }, 'Try again'));
    }
    if (fix.action?.kind !== 'report') btns.append(h('button', { class: 'btn', onclick: () => invoke('report_bug', { context: `${r.title}: ${r.error}` }) }, 'Report'));
    const fixBox = h('div', { class: 'fix' }, h('b', {}, fix.why), h('p', {}, fix.how), btns);
    el.append(h('pre', {}, r.error), fixBox);
  }
  if (r.outputs.length) {
    el.append(
      h('div', { class: 'row-btns' }, r.outputs.slice(0, 4).map((p) => h('button', { class: 'btn', title: p, onclick: () => invoke('reveal', { path: p }).catch(() => {}) }, `Show ${name(p)}`))),
    );
  }
  return el;
}

addView({
  id: 'activity',
  label: 'Activity',
  glyph: '◷',
  order: 5,
  badge: unseen,
  render(root) {
    // opening the list counts as having seen every failure in it
    if (history[0]) localStorage.setItem(SEEN, String(history[0].id));
    shared.nav?.();
    root.append(h('h1', {}, 'Activity'), h('p', { class: 'sub' }, 'Everything ShiftDrop did, newest first. Failures explain what went wrong and how to fix it.'));
    const tabs = [['all', 'All'], ['failed', 'Failed'], ['done', 'Done']];
    root.append(h('div', { class: 'tabs' }, tabs.map(([k, l]) => h('button', { class: k === filter ? 'on' : '', onclick: () => ((filter = k), shared.show('activity')) }, l))));
    const list = history.filter((r) => filter === 'all' || (filter === 'failed' ? failed(r) : !failed(r)));
    if (!list.length) root.append(h('div', { class: 'empty' }, filter === 'failed' ? 'No failures. Nice.' : 'Nothing yet. Drag a file and hold Shift.'));
    list.slice(0, 100).forEach((r) => root.append(card(r)));
    if (history.length) {
      root.append(h('div', { style: 'display:flex;justify-content:flex-end' }, h('button', { class: 'btn', onclick: () => invoke('clear_history').then(load) }, 'Clear list')));
    }
  },
});

listen('history', load);
load();
