import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { getCurrentWebview } from '@tauri-apps/api/webview';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { SAMPLES, buildMenu, ctx, flatActions, kindOf, loadContext, MORE, menuFor, noteUse, prefs, savePrefs } from './menu.js';
import { runTarget } from './run.js';
import { initTheme } from './theme.js';
import { addView, h, name, settings, settingsSections, shared, toggle, views } from './ui.js';
import './features.js';
import './recipes.js';
import './share-ui.js';
import './welcome.js';
import './activity.js';

initTheme();

const $main = document.getElementById('main');
const $nav = document.getElementById('nav');

let apps = [];
let files = []; // files the actions view works on
Object.defineProperty(shared, 'files', { get: () => files, set: (v) => (files = v) });
const state = { view: 'actions' };

function nav() {
  $nav.querySelectorAll('button, .spacer').forEach((b) => b.remove());
  for (const v of views) {
    if (v.hidden) continue;
    if (v.bottom && !$nav.querySelector('.spacer')) $nav.append(h('div', { class: 'spacer' }));
    const badge = v.badge?.();
    $nav.append(
      h('button', { class: v.id === state.view ? 'on' : '', onclick: () => show(v.id) }, h('span', { class: 'g' }, v.glyph), v.label, badge ? h('span', { class: 'badge' }, String(badge)) : null),
    );
  }
}

shared.show = (id) => show(id);
shared.nav = () => nav();
shared.view = () => state.view;

export function show(id) {
  state.view = id;
  nav();
  $main.replaceChildren();
  views.find((v) => v.id === id)?.render($main);
}

// --- Actions: every action for the files, searchable ("More…" on the wheel) ---------------

function actionsFor(paths) {
  const m = menuFor(paths, apps);
  return { kind: m.kind, list: [{ ...m.center, name: `${m.center.label} (centre)`, glyph: '●', parent: 'center' }, ...flatActions(m.all)] };
}

function match(item, q) {
  const hay = `${item.name} ${item.title || ''} ${item.keywords || ''}`.toLowerCase();
  return q
    .toLowerCase()
    .split(/\s+/)
    .every((w) => hay.includes(w));
}

addView({
  id: 'actions',
  label: 'Actions',
  glyph: '⌘',
  order: 0,
  render(root) {
    root.append(h('h1', {}, 'Actions'));
    if (!files.length) {
      root.append(h('p', { class: 'sub' }, 'Drop files here, or choose "More" on the wheel to see everything you can do with them.'));
      root.append(h('div', { class: 'drop', id: 'dropzone' }, 'Drop files here', h('p', { class: 'muted' }, 'or ', h('button', { class: 'btn', onclick: paste }, 'Paste'), ' files or an image (Ctrl+V)')));
      return;
    }
    const { kind, list } = actionsFor(files);
    root.append(h('div', { class: 'files' }, files.slice(0, 12).map((p) => h('span', { class: 'chip', title: p }, name(p))), files.length > 12 && h('span', { class: 'chip' }, `+${files.length - 12}`)));
    const q = h('input', { type: 'search', class: 'search', placeholder: `Search ${list.length} actions…`, autofocus: true });
    const ul = h('div', { class: 'list' });
    let sel = 0;
    let shown = list;
    const go = async (it) => {
      if (!it) return;
      if (it.ask) {
        asking = it;
        return show('ask');
      }
      if (it.parent && it.parent !== 'center') noteUse(kind, it.parent);
      try {
        await runTarget(it, files);
      } catch (e) {
        alertLine(String(e));
        return;
      }
      if (!it.editor) getCurrentWindow().hide();
    };
    const paint = () => {
      shown = list.filter((it) => match(it, q.value.trim()));
      sel = Math.min(sel, Math.max(0, shown.length - 1));
      ul.replaceChildren(
        ...shown.map((it, i) =>
          h(
            'div',
            { class: `row${i === sel ? ' sel' : ''}`, onclick: () => go(it), onmousemove: () => i !== sel && ((sel = i), paint()) },
            h('span', { class: 'g' }, it.glyph || '·'),
            it.name,
            h('span', { class: 'hint' }, it.app ? 'opens app' : it.editor ? 'editor' : ''),
          ),
        ),
      );
      if (!shown.length) ul.append(h('div', { class: 'empty' }, 'Nothing matches.'));
      ul.children[sel]?.scrollIntoView({ block: 'nearest' });
    };
    q.addEventListener('input', () => ((sel = 0), paint()));
    q.addEventListener('keydown', (e) => {
      if (e.key === 'ArrowDown') sel = Math.min(sel + 1, shown.length - 1);
      else if (e.key === 'ArrowUp') sel = Math.max(sel - 1, 0);
      else if (e.key === 'Enter') return go(shown[sel]);
      else return;
      e.preventDefault();
      paint();
    });
    root.append(q, ul);
    paint();
    setTimeout(() => q.focus(), 30);
  },
});

// --- Ask: actions that need a password or a confirmation first ------------------------------

let asking = null;

addView({
  id: 'ask',
  label: 'Confirm',
  glyph: '…',
  order: 1,
  hidden: true,
  render(root) {
    const it = asking;
    if (!it) return show('actions');
    root.append(h('h1', {}, it.name || it.label), h('div', { class: 'files' }, files.slice(0, 12).map((p) => h('span', { class: 'chip', title: p }, name(p)))));
    if (it.danger) root.append(h('p', { class: 'tag err', style: 'display:inline-block;padding:6px 10px' }, it.danger));
    const form = h('form', { class: 'card glass' });
    const inputs = [];
    for (const f of it.ask) {
      const inp = h('input', { type: f.type || 'text', required: true, autocomplete: 'off', style: 'flex:1', step: f.number ? 'any' : null, min: f.number ? '0' : null });
      inputs.push([f, inp]);
      form.append(h('label', { class: 'field' }, h('div', { class: 'what' }, f.label), inp));
      if (f.confirm) {
        const again = h('input', { type: f.type || 'text', required: true, autocomplete: 'off', style: 'flex:1' });
        inputs.push([{ ...f, again: f.key }, again]);
        form.append(h('label', { class: 'field' }, h('div', { class: 'what' }, `${f.label} again`), again));
      }
    }
    const err = h('p', { class: 'tag err', hidden: true });
    form.append(
      err,
      h('div', { style: 'display:flex;gap:8px;justify-content:flex-end;padding-top:12px' },
        h('button', { type: 'button', class: 'btn', onclick: () => show('actions') }, 'Cancel'),
        h('button', { type: 'submit', class: `btn primary` }, it.danger ? 'Yes, do it' : 'Run'),
      ),
    );
    form.onsubmit = async (e) => {
      e.preventDefault();
      const values = {};
      for (const [f, inp] of inputs) {
        if (f.again) {
          if (inp.value !== values[f.again]) {
            err.hidden = false;
            err.textContent = 'The two entries are different.';
            return;
          }
        } else values[f.key] = f.number ? Number(inp.value) : inp.value;
      }
      try {
        await runTarget({ ...it, ask: undefined, action: { ...it.action, ...values } }, files);
        asking = null;
        getCurrentWindow().hide();
      } catch (x) {
        err.hidden = false;
        err.textContent = String(x);
      }
    };
    root.append(form);
    setTimeout(() => inputs[0]?.[1].focus(), 30);
  },
});

function alertLine(msg) {
  const p = h('p', { class: 'tag err' }, msg);
  $main.prepend(p);
  setTimeout(() => p.remove(), 6000);
}

// --- Menu: choose what each kind of file shows on the wheel ----------------------------------

let menuTab = 'image';

addView({
  id: 'menu',
  label: 'Wheel',
  glyph: '◔',
  order: 80,
  bottom: true,
  render(root) {
    root.append(h('h1', {}, 'Wheel'), h('p', { class: 'sub' }, 'Pick what shows on the wheel for each kind of file. The rest stays one search away under More. Items you use often move onto the wheel by themselves.'));
    root.append(h('div', { class: 'tabs' }, Object.keys(SAMPLES).map((k) => h('button', { class: k === menuTab ? 'on' : '', onclick: () => ((menuTab = k), show('menu')) }, k))));
    const kind = menuTab;
    const raw = buildMenu(SAMPLES[kind], apps);
    const all = [...raw.items, ...(raw.extra || [])].map((it) => ({ ...it, id: it.id || it.label.toLowerCase() }));
    const p = prefs();
    const hidden = new Set(p.hidden?.[kind] || []);
    const wheel = menuFor(SAMPLES[kind], apps).items.filter((i) => i !== MORE && i.id !== 'more').map((i) => i.id);
    const card = h('div', { class: 'card glass' });
    for (const it of all) {
      const uses = p.usage?.[`${kind}:${it.id}`] || 0;
      const sw = h('input', { type: 'checkbox', class: 'switch', title: 'Allowed on the wheel' });
      sw.checked = !hidden.has(it.id);
      sw.onchange = () => {
        const q = prefs();
        q.hidden ??= {};
        const set = new Set(q.hidden[kind] || []);
        sw.checked ? set.delete(it.id) : set.add(it.id);
        q.hidden[kind] = [...set];
        savePrefs(q);
        show('menu');
      };
      const centre = p.center?.[kind] === it.id;
      card.append(
        h(
          'div',
          { class: 'field' },
          h('span', { class: 'g', style: 'width:22px;text-align:center' }, it.glyph || '·'),
          h('div', { class: 'what' }, it.label, h('small', {}, [wheel.includes(it.id) ? 'on the wheel' : 'under More', uses ? ` · used ${uses}×` : ''].join(''))),
          h('button', {
            class: `pin${centre ? ' on' : ''}`,
            title: 'Make this the centre action',
            onclick: () => {
              const q = prefs();
              q.center ??= {};
              q.center[kind] = centre ? undefined : it.id;
              savePrefs(q);
              show('menu');
            },
          }, centre ? 'centre' : 'set centre'),
          sw,
        ),
      );
    }
    root.append(card);
    root.append(
      h('button', {
        class: 'btn',
        onclick: () => {
          const q = prefs();
          for (const part of ['hidden', 'center']) if (q[part]) delete q[part][kind];
          if (q.usage) for (const k of Object.keys(q.usage)) if (k.startsWith(`${kind}:`)) delete q.usage[k];
          savePrefs(q);
          show('menu');
        },
      }, `Reset ${kind}`),
    );
  },
});

// --- Settings ------------------------------------------------------------------------------

addView({
  id: 'settings',
  label: 'Settings',
  glyph: '⚙',
  order: 90,
  bottom: true,
  async render(root) {
    const s = await settings.load();
    const auto = await invoke('get_autostart');
    root.append(h('h1', {}, 'Settings'), h('p', { class: 'sub' }, 'Everything runs on this computer. Features that need the internet stay off until you turn them on.'));
    const general = h('div', { class: 'card glass' });
    general.append(
      toggle('Shift + drag menu', 'Start dragging a file, then hold Shift.', s.enabled, (v) => settings.set({ enabled: v })),
      toggle('Only from File Explorer and desktop', null, s.explorer_only, (v) => settings.set({ explorer_only: v })),
      toggle('Quick look', 'Ctrl+Space in File Explorer previews the selected file.', s.peek, (v) => settings.set({ peek: v })),
      toggle('Start with Windows', null, auto, (v) => invoke('set_autostart', { on: v })),
    );
    const theme = h('select', { onchange: (e) => settings.set({ theme: e.target.value }) }, ['system', 'light', 'dark'].map((t) => h('option', { value: t, selected: s.theme === t }, t)));
    general.append(h('div', { class: 'field' }, h('div', { class: 'what' }, 'Appearance'), theme));
    const lang = h('select', { onchange: (e) => settings.set({ lang: e.target.value }).then(() => location.reload()) }, [['system', 'system'], ['en', 'English'], ['vi', 'Tiếng Việt']].map(([v, l]) => h('option', { value: v, selected: (s.lang || 'system') === v, translate: 'no' }, l)));
    general.append(h('div', { class: 'field' }, h('div', { class: 'what' }, 'Language'), lang));
    root.append(general);
    for (const extra of settingsSections) root.append(await extra(s));
  },
});

// --- Opening -------------------------------------------------------------------------------

function open(req) {
  if (!req) return;
  if (req.paths?.length) files = req.paths;
  if (req.item) asking = req.item;
  show(req.view === 'more' ? 'actions' : req.view || state.view);
}

listen('hub:open', ({ payload }) => open(payload));

// files dropped onto the window
getCurrentWebview().onDragDropEvent((e) => {
  const z = document.getElementById('dropzone');
  if (e.payload.type === 'over') z?.classList.add('over');
  else if (e.payload.type === 'leave') z?.classList.remove('over');
  else if (e.payload.type === 'drop' && e.payload.paths.length) {
    files = e.payload.paths;
    show('actions');
  }
});

async function paste() {
  try {
    files = await invoke('clipboard_files');
    show('actions');
  } catch (e) {
    alertLine(String(e));
  }
}

addEventListener('keydown', (e) => {
  if (e.ctrlKey && e.key.toLowerCase() === 'v' && !(e.target instanceof HTMLInputElement)) paste();
  if (e.key === 'Escape') getCurrentWindow().hide();
});

(async () => {
  apps = (await loadContext(invoke)).apps;
  const req = await invoke('hub_take');
  open(req || { view: 'actions' });
})();

