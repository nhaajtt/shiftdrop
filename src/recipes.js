import { invoke } from '@tauri-apps/api/core';
import { SAMPLES, buildMenu, flatActions, prefs, savePrefs } from './menu.js';
import { addView, h, settings, shared } from './ui.js';

// Recipes (several actions in a row), the user's own commands, and folders that run a recipe
// on every new file. Recipes and commands appear under More and climb onto the wheel when used.

const KINDS = ['any', ...Object.keys(SAMPLES).filter((k) => k !== 'mixed')];
const uid = () => Math.random().toString(36).slice(2, 9);

/** Actions a recipe step can use: plain jobs only (no editors, apps or questions). */
function stepChoices(kind) {
  const raw = buildMenu(SAMPLES[kind === 'any' ? 'other' : kind]);
  return flatActions([...raw.items, ...(raw.extra || [])]).filter((a) => a.action && !a.ask && !a.editor && !a.app && a.action.op !== 'chain');
}

const card = (title, ...kids) => h('div', { class: 'card glass' }, h('div', { class: 'label', style: 'padding-bottom:6px' }, title), ...kids);

addView({
  id: 'recipes',
  label: 'Recipes',
  glyph: '⚡',
  order: 20,
  async render(root) {
    root.append(h('h1', {}, 'Recipes'), h('p', { class: 'sub' }, 'Chain actions into one drop, add your own commands, and let folders process new files by themselves.'));
    const p = prefs();
    const recipes = p.recipes || [];

    // --- existing recipes
    const list = card('Your recipes');
    if (!recipes.length) list.append(h('p', { class: 'muted' }, 'None yet. Make one below, e.g. Resize 50% → JPG → Remove metadata.'));
    for (const r of recipes) {
      list.append(
        h(
          'div',
          { class: 'field' },
          h('div', { class: 'what' }, `⚡ ${r.name}`, h('small', {}, `${r.kind} · ${r.steps.map((s) => s.name).join(' → ')}`)),
          h('button', { class: 'btn', onclick: () => (savePrefs({ ...prefs(), recipes: prefs().recipes.filter((x) => x.id !== r.id) }), shared.show('recipes')) }, 'Delete'),
        ),
      );
    }
    root.append(list);

    // --- new recipe
    let kind = 'image';
    const steps = [];
    const nameIn = h('input', { type: 'text', placeholder: 'Recipe name, e.g. "Web photo"', style: 'flex:1' });
    const kindSel = h('select', {}, KINDS.map((k) => h('option', { value: k, selected: k === kind }, k)));
    const stepSel = h('select', { style: 'flex:1' });
    const stepList = h('div', { class: 'files' });
    const fillSteps = () => stepSel.replaceChildren(...stepChoices(kind).map((a, i) => h('option', { value: i }, a.name)));
    const paintSteps = () =>
      stepList.replaceChildren(
        ...steps.map((s, i) => h('span', { class: 'chip', title: 'Click to remove', style: 'cursor:pointer', onclick: () => (steps.splice(i, 1), paintSteps()) }, `${i + 1}. ${s.name} ✕`)),
      );
    kindSel.onchange = () => ((kind = kindSel.value), (steps.length = 0), fillSteps(), paintSteps());
    fillSteps();
    const err = h('p', { class: 'tag err', hidden: true });
    root.append(
      card(
        'New recipe',
        h('div', { class: 'field' }, nameIn, kindSel),
        h('div', { class: 'field' }, stepSel, h('button', { class: 'btn', onclick: () => {
          const a = stepChoices(kind)[+stepSel.value];
          if (a) steps.push({ name: a.name, action: a.action });
          paintSteps();
        } }, 'Add step')),
        stepList,
        err,
        h('div', { style: 'display:flex;justify-content:flex-end' }, h('button', { class: 'btn primary', onclick: () => {
          if (!nameIn.value.trim() || !steps.length) {
            err.hidden = false;
            err.textContent = 'Give it a name and at least one step.';
            return;
          }
          const q = prefs();
          q.recipes = [...(q.recipes || []), { id: uid(), name: nameIn.value.trim(), kind, steps }];
          savePrefs(q);
          shared.show('recipes');
        } }, 'Save recipe')),
      ),
    );

    // --- own commands
    const cmds = p.commands || [];
    const cName = h('input', { type: 'text', placeholder: 'Name', style: 'width:140px' });
    const cExt = h('input', { type: 'text', placeholder: 'Extensions (empty = all)', style: 'width:180px' });
    const cCmd = h('input', { type: 'text', placeholder: 'magick {file} -resize 50% "{stem} small.{ext}"', style: 'flex:1' });
    root.append(
      card(
        'Your commands',
        h('p', { class: 'muted', style: 'margin-top:0' }, 'Runs in the file’s folder. {file} {dir} {name} {stem} {ext} are filled in. New files in the folder count as the result.'),
        ...cmds.map((c) =>
          h('div', { class: 'field' }, h('div', { class: 'what' }, `› ${c.name}`, h('small', { class: 'mono' }, c.command)), h('button', { class: 'btn', onclick: () => (savePrefs({ ...prefs(), commands: prefs().commands.filter((x) => x.id !== c.id) }), shared.show('recipes')) }, 'Delete')),
        ),
        h('div', { class: 'field' }, cName, cExt),
        h('div', { class: 'field' }, cCmd, h('button', { class: 'btn', onclick: () => {
          if (!cName.value.trim() || !cCmd.value.trim()) return;
          const q = prefs();
          q.commands = [...(q.commands || []), { id: uid(), name: cName.value.trim(), exts: cExt.value, command: cCmd.value.trim() }];
          savePrefs(q);
          shared.show('recipes');
        } }, 'Add')),
      ),
    );

    // --- watched folders (stored by the app itself, they run without any window open)
    const s = await settings.load();
    const fIn = h('input', { type: 'text', placeholder: 'C:\\Users\\you\\Downloads', style: 'flex:1' });
    const rSel = h('select', {}, recipes.map((r) => h('option', { value: r.id }, r.name)));
    const fExt = h('input', { type: 'text', placeholder: 'only these extensions (optional)', style: 'width:220px' });
    const ferr = h('p', { class: 'tag err', hidden: true });
    root.append(
      card(
        'Watched folders',
        h('p', { class: 'muted', style: 'margin-top:0' }, 'New files that land in the folder get the recipe. Files already there are left alone.'),
        ...s.watches.map((w, i) =>
          h('div', { class: 'field' }, h('div', { class: 'what' }, w.folder, h('small', {}, `${w.title}${w.exts.length ? ` · ${w.exts.join(', ')}` : ''}`)), h('button', { class: 'btn', onclick: async () => {
            await settings.set({ watches: s.watches.filter((_, j) => j !== i) });
            shared.show('recipes');
          } }, 'Stop')),
        ),
        recipes.length
          ? h('div', {}, h('div', { class: 'field' }, fIn), h('div', { class: 'field' }, rSel, fExt, h('button', { class: 'btn', onclick: async () => {
              const folder = fIn.value.trim().replace(/^"|"$/g, '');
              if (!(await invoke('folder_exists', { path: folder }))) {
                ferr.hidden = false;
                ferr.textContent = 'That folder does not exist.';
                return;
              }
              const r = recipes.find((x) => x.id === rSel.value);
              const exts = fExt.value.split(/[\s,;]+/).map((e) => e.replace(/^\./, '').toLowerCase()).filter(Boolean);
              await settings.set({ watches: [...s.watches, { folder, exts, title: r.name, action: { op: 'chain', steps: r.steps.map((x) => x.action) } }] });
              shared.show('recipes');
            } }, 'Watch')), ferr)
          : h('p', { class: 'muted' }, 'Make a recipe first.'),
      ),
    );
  },
});
