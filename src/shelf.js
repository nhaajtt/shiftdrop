import { startDrag } from '@crabnebula/tauri-plugin-drag';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { getCurrentWebview } from '@tauri-apps/api/webview';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { extOf } from './menu.js';
import { initTheme } from './theme.js';

initTheme();

// Files waiting on the shelf; kept across restarts.
const KEY = 'shiftdrop.shelf';
let files = [];
try {
  const saved = JSON.parse(localStorage.getItem(KEY) || '[]');
  if (Array.isArray(saved)) files = saved.filter((f) => typeof f === 'string');
} catch {
  // a damaged list starts empty instead of breaking the shelf
}
const sel = new Set();
const $ = (id) => document.getElementById(id);
const name = (p) => p.split(/[\\/]/).pop();
let icon = '';
invoke('drag_icon').then((p) => (icon = p));

function save() {
  localStorage.setItem(KEY, JSON.stringify(files));
}

function add(paths) {
  for (const p of paths) if (!files.includes(p)) files.push(p);
  save();
  render();
}

/** Starts a real file drag (Explorer, mail, chat apps accept it). */
function drag(paths) {
  if (!paths.length) return;
  startDrag({ item: paths, icon }).catch(() => {});
}

function render() {
  const list = $('list');
  list.replaceChildren(
    ...files.map((p) => {
      const li = document.createElement('li');
      li.title = p;
      li.classList.toggle('sel', sel.has(p));
      li.innerHTML = '<span class="k"></span><span class="n"></span><button class="icon x" title="Remove">✕</button>';
      li.querySelector('.k').textContent = extOf(p) || 'dir';
      li.querySelector('.n').textContent = name(p);
      li.querySelector('.x').onclick = (e) => {
        e.stopPropagation();
        files = files.filter((f) => f !== p);
        sel.delete(p);
        save();
        render();
      };
      li.onclick = (e) => {
        if (!e.ctrlKey) sel.clear();
        sel.has(p) ? sel.delete(p) : sel.add(p);
        render();
      };
      // drag the selection when this file is part of it, otherwise just this file
      li.onpointerdown = (e) => {
        if (e.button !== 0 || e.target.closest('.x')) return;
        const start = [e.clientX, e.clientY];
        const move = (ev) => {
          if (Math.hypot(ev.clientX - start[0], ev.clientY - start[1]) < 6) return;
          removeEventListener('pointermove', move);
          drag(sel.has(p) ? [...sel] : [p]);
        };
        addEventListener('pointermove', move);
        addEventListener('pointerup', () => removeEventListener('pointermove', move), { once: true });
      };
      return li;
    }),
  );
  $('empty').hidden = files.length > 0;
  $('count').textContent = files.length ? `${files.length}` : '';
}

$('all').onpointerdown = (e) => (e.preventDefault(), drag(files));
$('clear').onclick = () => {
  files = [];
  sel.clear();
  save();
  render();
};
$('act').onclick = () => files.length && invoke('open_hub', { paths: sel.size ? [...sel] : files, view: 'actions', item: null });
$('close').onclick = () => getCurrentWindow().hide();

getCurrentWebview().onDragDropEvent((e) => {
  document.body.classList.toggle('over', e.payload.type === 'over' || e.payload.type === 'enter');
  if (e.payload.type === 'drop') {
    document.body.classList.remove('over');
    add(e.payload.paths);
  }
});

// listen first, then take what is waiting, so nothing dropped in between is lost
listen('shelf:add', ({ payload }) => add(payload))
  .then(() => invoke('shelf_take'))
  .then(add);
render();
