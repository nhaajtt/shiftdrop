import { convertFileSrc, invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { extOf, kindOf } from './menu.js';
import { initTheme } from './theme.js';

initTheme();

const $ = (id) => document.getElementById(id);
const el = (tag, props = {}) => Object.assign(document.createElement(tag), props);
let path = new URLSearchParams(location.search).get('path') || '';

const size = (n) => (n < 1024 ? `${n} B` : n < 1048576 ? `${(n / 1024).toFixed(1)} KB` : n < 1073741824 ? `${(n / 1048576).toFixed(1)} MB` : `${(n / 1073741824).toFixed(2)} GB`);

async function show(p) {
  path = p;
  const view = $('view');
  view.replaceChildren();
  $('name').textContent = p.split(/[\\/]/).pop();
  const info = await invoke('file_info', { path: p }).catch(() => null);
  $('meta').textContent = info ? (info.is_dir ? `${info.items} items` : size(info.size)) : '';
  const src = convertFileSrc(p);
  const k = info?.is_dir ? 'folder' : kindOf(p);
  if (info?.warning) view.append(el('p', { className: 'warn', textContent: `⚠ ${info.warning}` }));
  if (k === 'image' && !['heic', 'heif', 'dng', 'cr2', 'cr3', 'nef', 'arw', 'orf', 'rw2', 'raf', 'jxr'].includes(extOf(p))) {
    view.append(el('img', { src, alt: '' }));
  } else if (k === 'video') {
    view.append(el('video', { src, controls: true, autoplay: true }));
  } else if (k === 'audio') {
    view.append(el('audio', { src, controls: true, autoplay: true }));
  } else if (k === 'pdf') {
    const box = el('div', { className: 'pages', textContent: 'Loading pages…' });
    view.append(box);
    invoke('pdf_thumbs', { path: p, width: 900 }).then((pages) => box.replaceChildren(...pages.map((s) => el('img', { src: s, alt: '' }))));
  } else if (k === 'text' || (info && info.size < 2_000_000 && !info.real_type && !info.is_dir)) {
    const text = await invoke('read_text', { path: p, max: 262144 }).catch(() => '');
    view.append(el('pre', { textContent: text + (info?.size > 262144 ? '\n…' : '') }));
  } else {
    const dl = el('dl', { className: 'info' });
    const row = (k, v) => v && dl.append(el('dt', { textContent: k }), el('dd', { textContent: v }));
    row('Kind', info?.is_dir ? 'Folder' : info?.real_type || extOf(p).toUpperCase() || 'File');
    row('Size', info && !info.is_dir ? size(info.size) : '');
    row('Items', info?.is_dir ? String(info.items) : '');
    row('Modified', info ? new Date(info.modified * 1000).toLocaleString() : '');
    row('Path', p);
    view.append(dl);
  }
}

$('actions').onclick = () => invoke('open_hub', { paths: [path], view: 'actions', item: null }).then(() => getCurrentWindow().hide());
$('open').onclick = () => invoke('reveal', { path });
addEventListener('keydown', (e) => {
  if (e.key === 'Escape' || (e.key === ' ' && !(e.target instanceof HTMLMediaElement))) {
    e.preventDefault();
    document.querySelectorAll('video,audio').forEach((m) => m.pause());
    getCurrentWindow().hide();
  }
});
listen('peek', ({ payload }) => show(payload));
show(path);
