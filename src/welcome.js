import { invoke } from '@tauri-apps/api/core';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { addView, h, settings, toggle } from './ui.js';

// First launch (and tray › How to use): ShiftDrop has no window of its own, so this shows
// where it lives and how to use it, with the few choices worth making up front.

const STEPS = [
  ['1', 'Start dragging a file', 'In File Explorer or on the desktop.'],
  ['2', 'Then hold Shift', 'The wheel opens around the file. Keys 1–8 pick a wedge, Esc cancels.'],
  ['3', 'Drop it on an action', 'The result is saved next to the original, which is never changed.'],
];

addView({
  id: 'welcome',
  label: 'Welcome',
  glyph: '✦',
  order: 0,
  hidden: true,
  async render(root) {
    const s = await settings.load();
    const auto = await invoke('get_autostart');
    root.append(
      h('div', { class: 'welcome' },
        h('div', { class: 'demo', 'aria-hidden': 'true' },
          h('div', { class: 'demo-file' }, h('i'), h('span', {}, 'Photo.heic')),
          h('div', { class: 'demo-wheel' }, ...Array.from({ length: 8 }, (_, i) => h('b', { style: `--i:${i}` })), h('em', {}, 'PNG')),
          h('div', { class: 'demo-key' }, 'Shift'),
          h('div', { class: 'demo-cursor' }),
        ),
        h('h1', {}, 'ShiftDrop is ready'),
        h('p', { class: 'sub' }, 'It runs quietly in the system tray (the ⇄ icon by the clock). There is no window to keep open.'),
      ),
    );
    root.append(h('div', { class: 'steps3' }, STEPS.map(([n, t, d]) => h('div', { class: 'card glass' }, h('span', { class: 'n' }, n), h('b', {}, t), h('p', { class: 'muted' }, d)))));

    const setup = h('div', { class: 'card glass' });
    const lang = h(
      'select',
      { onchange: (e) => settings.set({ lang: e.target.value }).then(() => location.reload()) },
      [['system', 'system'], ['en', 'English'], ['vi', 'Tiếng Việt']].map(([v, l]) => h('option', { value: v, selected: (s.lang || 'system') === v, translate: 'no' }, l)),
    );
    setup.append(
      h('div', { class: 'field' }, h('div', { class: 'what' }, 'Language'), lang),
      toggle('Start with Windows', 'So the wheel is always there when you drag.', auto, (v) => invoke('set_autostart', { on: v })),
    );
    // video and audio need FFmpeg; offer it here so the first video drop just works
    const tools = await invoke('tools_status').catch(() => []);
    const ff = tools.find((t) => t.id === 'ffmpeg');
    if (ff && !ff.installed) {
      const btn = h('button', { class: 'btn' }, 'Download');
      btn.onclick = async () => {
        btn.disabled = true;
        btn.textContent = 'Downloading…';
        try {
          await invoke('tools_install', { id: 'ffmpeg' });
          btn.textContent = 'Installed';
        } catch (e) {
          btn.disabled = false;
          btn.textContent = 'Retry';
          btn.title = String(e);
        }
      };
      setup.append(h('div', { class: 'field' }, h('div', { class: 'what' }, 'Video and audio', h('small', {}, 'Needs the free FFmpeg add-on (≈90 MB, official Windows build). You can also do this later in Settings.')), btn));
    }
    root.append(setup);
    root.append(
      h('div', { style: 'display:flex;gap:10px;justify-content:flex-end;padding-top:6px' },
        h('button', { class: 'btn', onclick: () => invoke('open_hub', { paths: [], view: 'actions', item: null }) }, 'Open ShiftDrop'),
        h('button', { class: 'btn primary', onclick: () => getCurrentWindow().hide() }, 'Got it, let me try'),
      ),
    );
  },
});
