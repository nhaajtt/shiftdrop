import { invoke } from '@tauri-apps/api/core';
import { addView, h, name, settings, settingsSections, shared } from './ui.js';

// "To phone" and "Upload link": opened from the wheel or the Actions list with the files.

const chips = () => h('div', { class: 'files' }, shared.files.slice(0, 12).map((p) => h('span', { class: 'chip', title: p }, name(p))));

addView({
  id: 'phone',
  label: 'To phone',
  glyph: '▯',
  order: 2,
  hidden: true,
  async render(root) {
    root.append(h('h1', {}, 'Send to phone'), chips());
    const box = h('div', { class: 'card glass', style: 'display:flex;gap:20px;align-items:center' }, h('p', { class: 'muted' }, 'Preparing…'));
    root.append(box);
    try {
      const s = await invoke('share_lan', { paths: shared.files });
      const qr = h('div', { style: 'background:#fff;padding:10px;border-radius:12px;line-height:0' });
      qr.innerHTML = s.qr;
      const left = h('span', { class: 'tag' }, `${s.minutes}:00`);
      const end = Date.now() + s.minutes * 60000;
      const tick = setInterval(() => {
        const ms = end - Date.now();
        if (ms <= 0 || !left.isConnected) {
          clearInterval(tick);
          left.textContent = 'expired';
          return;
        }
        left.textContent = `${Math.floor(ms / 60000)}:${String(Math.floor((ms % 60000) / 1000)).padStart(2, '0')}`;
      }, 1000);
      box.replaceChildren(
        qr,
        h('div', {},
          h('p', { style: 'margin-top:0' }, 'Scan with the phone camera. The phone must be on the same Wi-Fi.'),
          h('p', { class: 'mono', style: 'font-size:12px;word-break:break-all' }, s.url),
          h('p', { class: 'muted' }, 'Link works for ', left, '. Only this file is reachable, only from your network, and nothing can be sent back. Windows may ask once to allow ShiftDrop on private networks.'),
        ),
      );
    } catch (e) {
      box.replaceChildren(h('p', { class: 'tag err' }, String(e)));
    }
  },
});

addView({
  id: 'upload',
  label: 'Upload',
  glyph: '☁',
  order: 3,
  hidden: true,
  async render(root) {
    root.append(h('h1', {}, 'Upload link'), chips());
    const box = h('div', { class: 'card glass' }, h('p', { class: 'muted' }, 'Uploading to your Cloudflare R2 bucket…'));
    root.append(box);
    try {
      const links = await invoke('share_r2', { paths: shared.files });
      box.replaceChildren(h('p', { style: 'margin-top:0' }, 'Links copied to the clipboard:'), ...links.map((l) => h('p', { class: 'mono', style: 'font-size:12px;word-break:break-all' }, l)));
    } catch (e) {
      box.replaceChildren(h('p', { class: 'tag err' }, String(e)));
    }
  },
});

settingsSections.push(async (s) => {
  const field = (key, label, hint, type = 'text', ph = '') => {
    const i = h('input', { type, value: s[key] || '', placeholder: ph, style: 'flex:1;min-width:240px', autocomplete: 'off' });
    i.onchange = () => settings.set({ [key]: i.value.trim() });
    return h('label', { class: 'field' }, h('div', { class: 'what' }, label, hint && h('small', {}, hint)), i);
  };
  return h(
    'div',
    { class: 'card glass' },
    h('div', { class: 'label', style: 'padding-bottom:6px' }, 'Claude and Cloudflare R2'),
    h('p', { class: 'muted', style: 'margin-top:0' }, 'Keys are stored encrypted for your Windows account. Files are sent only when you pick Translate, Summarize or Upload link.'),
    field('anthropic_key', 'Claude API key', 'For Translate and Summarize on text files.', 'password', 'sk-ant-…'),
    field('r2_account', 'R2 account ID', null),
    field('r2_bucket', 'R2 bucket', null),
    field('r2_key', 'R2 access key ID', 'A token limited to this one bucket is enough.', 'password'),
    field('r2_secret', 'R2 secret', null, 'password'),
    field('r2_public', 'Public URL', 'The bucket’s r2.dev address or your own domain.', 'text', 'https://files.example.com'),
  );
});
