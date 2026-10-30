import { invoke } from '@tauri-apps/api/core';
import { addView, h, name, settings, settingsSections, shared, toggle } from './ui.js';

// Settings cards for optional features. Anything that talks to the internet is off or empty
// until the user fills it in here.

settingsSections.push(async (s) => {
  const card = h('div', { class: 'card glass' }, h('div', { class: 'label', style: 'padding-bottom:6px' }, 'Add-ons'));
  const tools = await invoke('tools_status');
  const about = {
    capa: ['capa', 'What a program can do (Mandiant)'],
    floss: ['FLOSS', 'Strings hidden or built at run time (Mandiant)'],
    upx: ['UPX', 'Unpacks programs compressed with UPX'],
    ai: ['AI images', 'Background removal and face blurring, on this PC (≈180 MB)'],
    upscale: ['Upscaling', 'Real-ESRGAN, sharp 2× and 4× enlargements; needs a Vulkan graphics card (≈45 MB)'],
    ffmpeg: ['FFmpeg', 'Needed for every video and audio action (official Windows build, ≈90 MB)'],
    yara: ['YARA-X', 'Scan programs with your own YARA rules (VirusTotal’s YARA-X)'],
    qpdf: ['qpdf', 'PDF passwords: protect and unlock'],
    whisper: ['Speech to text', 'Subtitles from speech, offline (whisper.cpp + 150 MB model)'],
  };
  for (const t of tools) {
    const [label, hint] = about[t.id] || [t.id, ''];
    const btn = h('button', { class: 'btn' }, t.installed ? 'Installed' : 'Download');
    btn.disabled = t.installed;
    btn.onclick = async () => {
      btn.disabled = true;
      btn.textContent = 'Downloading…';
      try {
        await invoke('tools_install', { id: t.id });
        btn.textContent = 'Installed';
      } catch (e) {
        btn.disabled = false;
        btn.textContent = 'Retry';
        btn.title = String(e);
      }
    };
    card.append(h('div', { class: 'field' }, h('div', { class: 'what' }, label, h('small', {}, `${hint}. Downloaded from its official GitHub release.`)), btn));
  }
  const rules = h('input', { type: 'text', value: s.yara_rules || '', placeholder: 'Folder with .yar rules, e.g. C:\\Tools\\rules', style: 'flex:1' });
  rules.onchange = () => settings.set({ yara_rules: rules.value.trim().replace(/^"|"$/g, '') });
  card.append(h('label', { class: 'field' }, h('div', { class: 'what' }, 'YARA rules', h('small', {}, 'Used by YARA scan on programs.')), rules));
  const sandbox = await invoke('sandbox_ok');
  card.append(
    h('div', { class: 'field' }, h('div', { class: 'what' }, 'Windows Sandbox', h('small', {}, sandbox ? 'Available: programs can be opened in a throwaway sandbox.' : 'Not available on this PC (needs Windows Pro or Enterprise with the feature turned on).')), h('span', { class: `tag ${sandbox ? 'ok' : ''}` }, sandbox ? 'ready' : 'off')),
  );
  return card;
});

settingsSections.push(async (s) => {
  const card = h('div', { class: 'card glass' }, h('div', { class: 'label', style: 'padding-bottom:6px' }, 'Online services'));
  const key = h('input', { type: 'password', placeholder: 'VirusTotal API key (optional)', value: s.vt_key || '', style: 'flex:1;min-width:260px' });
  key.onchange = () => settings.set({ vt_key: key.value.trim() });
  card.append(
    h('label', { class: 'field' }, h('div', { class: 'what' }, 'VirusTotal', h('small', {}, 'Only the file’s SHA-256 is sent, never the file. Without a key the public report page opens instead.')), key),
  );
  return card;
});

settingsSections.push(async () => {
  const card = h('div', { class: 'card glass' }, h('div', { class: 'label', style: 'padding-bottom:6px' }, 'Explorer'));
  card.append(
    toggle('Right-click menu', 'Adds "ShiftDrop…" to Explorer’s menu (under "Show more options" on Windows 11).', await invoke('context_menu_on'), (on) => invoke('context_menu', { enable: on })),
  );
  return card;
});

// --- Batch rename --------------------------------------------------------------------------

const split = (p) => {
  const i = Math.max(p.lastIndexOf('\\'), p.lastIndexOf('/'));
  const dir = p.slice(0, i + 1);
  const file = p.slice(i + 1);
  const dot = file.lastIndexOf('.');
  return dot > 0 ? { dir, stem: file.slice(0, dot), ext: file.slice(dot) } : { dir, stem: file, ext: '' };
};

const cases = {
  keep: (s) => s,
  lower: (s) => s.toLowerCase(),
  upper: (s) => s.toUpperCase(),
  title: (s) => s.toLowerCase().replace(/(^|[\s_-])(\p{L})/gu, (_, a, b) => a + b.toUpperCase()),
  slug: (s) => s.normalize('NFD').replace(/[̀-ͯ]/g, '').replace(/đ/g, 'd').replace(/Đ/g, 'D').toLowerCase().replace(/[^a-z0-9]+/g, '-').replace(/^-|-$/g, ''),
};

addView({
  id: 'rename',
  label: 'Rename',
  glyph: '✎',
  order: 10,
  render(root) {
    root.append(h('h1', {}, 'Rename'), h('p', { class: 'sub' }, 'Rename many files at once. {name} is the old name, {n} a counter, {date} today.'));
    if (!shared.files.length) {
      root.append(h('div', { class: 'empty' }, 'Drop files on the Actions page first.'));
      return;
    }
    const o = { pattern: '{name}', find: '', replace: '', regex: false, start: 1, pad: 2, kase: 'keep' };
    const input = (key, attrs = {}) => {
      const e = h('input', { type: 'text', value: o[key], ...attrs, style: 'flex:1' });
      e.oninput = () => ((o[key] = attrs.type === 'number' ? +e.value : e.value), paint());
      return e;
    };
    const kase = h('select', { onchange: (e) => ((o.kase = e.target.value), paint()) }, Object.keys(cases).map((k) => h('option', { value: k }, k)));
    const re = h('input', { type: 'checkbox', class: 'switch' });
    re.onchange = () => ((o.regex = re.checked), paint());
    const form = h(
      'div',
      { class: 'card glass' },
      h('label', { class: 'field' }, h('div', { class: 'what' }, 'New name'), input('pattern')),
      h('label', { class: 'field' }, h('div', { class: 'what' }, 'Find'), input('find'), h('span', { class: 'muted' }, 'regex'), re),
      h('label', { class: 'field' }, h('div', { class: 'what' }, 'Replace with'), input('replace')),
      h('div', { class: 'field' }, h('div', { class: 'what' }, 'Counter from'), input('start', { type: 'number', min: 0 }), h('div', { class: 'what', style: 'flex:0' }, 'Case'), kase),
    );
    const list = h('div', { class: 'list' });
    const err = h('p', { class: 'tag err', hidden: true });
    const today = new Date().toISOString().slice(0, 10);
    let plan = [];
    const paint = () => {
      err.hidden = true;
      plan = shared.files.map((p, i) => {
        const { dir, stem, ext } = split(p);
        let name = o.pattern.replaceAll('{name}', stem).replaceAll('{n}', String(o.start + i).padStart(o.pad, '0')).replaceAll('{date}', today);
        if (o.find) {
          try {
            name = o.regex ? name.replace(new RegExp(o.find, 'g'), o.replace) : name.replaceAll(o.find, o.replace);
          } catch (e) {
            err.hidden = false;
            err.textContent = String(e.message);
          }
        }
        name = cases[o.kase](name).replace(/[<>:"/\\|?*]/g, '_').trim() || stem;
        return [p, dir + name + ext];
      });
      list.replaceChildren(...plan.map(([a, b]) => h('div', { class: 'row' }, h('span', { class: 'muted', style: 'flex:1;overflow:hidden;text-overflow:ellipsis' }, name(a)), '→', h('span', { style: 'flex:1;overflow:hidden;text-overflow:ellipsis' }, name(b)))));
    };
    const apply = h('button', {
      class: 'btn primary',
      onclick: async () => {
        try {
          const n = await invoke('rename_files', { pairs: plan });
          shared.files = plan.map(([, b]) => b);
          apply.textContent = `Renamed ${n}`;
          // refresh the list, unless the user has moved on to another view meanwhile
          setTimeout(() => shared.view?.() === 'rename' && shared.show('rename'), 1200);
        } catch (e) {
          err.hidden = false;
          err.textContent = String(e);
        }
      },
    }, 'Rename');
    root.append(form, err, list, h('div', { style: 'display:flex;justify-content:flex-end;padding-top:12px' }, apply));
    paint();
  },
});

settingsSections.push(async () => {
  const { getVersion } = await import('@tauri-apps/api/app');
  const status = h('span', { class: 'muted' }, `Version ${await getVersion()}`);
  const btn = h('button', { class: 'btn' }, 'Check for updates');
  btn.onclick = async () => {
    btn.disabled = true;
    status.textContent = 'Checking…';
    try {
      const { check } = await import('@tauri-apps/plugin-updater');
      const update = await check();
      if (!update) {
        status.textContent = 'You have the latest version.';
        return;
      }
      status.textContent = `Installing ${update.version}…`;
      await update.downloadAndInstall();
      const { relaunch } = await import('@tauri-apps/plugin-process');
      await relaunch();
    } catch (e) {
      status.textContent = String(e);
    } finally {
      btn.disabled = false;
    }
  };
  return h('div', { class: 'card glass' }, h('div', { class: 'field' }, h('div', { class: 'what' }, 'Updates', h('small', {}, 'Updates are signed; only releases from the official repository are accepted.')), status, btn));
});

settingsSections.push(async () =>
  h(
    'div',
    { class: 'card glass' },
    h(
      'div',
      { class: 'field' },
      h('div', { class: 'what' }, 'Report a problem', h('small', {}, 'Opens a GitHub issue with your app and Windows version filled in. You see everything before it is sent.')),
      h('button', { class: 'btn', onclick: () => invoke('open_logs') }, 'Open log folder'),
      h('button', { class: 'btn', onclick: () => invoke('report_bug', { context: null }) }, 'Report'),
    ),
  ),
);
