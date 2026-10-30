import { invoke } from '@tauri-apps/api/core';

// Shared building blocks for the hub window and the feature panels that plug into it.

export const h = (tag, props = {}, ...kids) => {
  const e = document.createElement(tag);
  for (const [k, v] of Object.entries(props)) {
    if (k.startsWith('on')) e.addEventListener(k.slice(2), v);
    else if (k === 'class') e.className = v;
    else if (v !== false && v != null) e.setAttribute(k, v === true ? '' : v);
  }
  for (const k of kids.flat()) if (k != null && k !== false) e.append(k);
  return e;
};
export const name = (p) => p.split(/[\\/]/).pop();

/** State the hub shares with feature panels (files is wired up by hub.js). */
export const shared = { files: [], show: (_id) => {} };

// Views register here; later features add theirs to the same list.
export const views = [];
export function addView(v) {
  views.push(v);
  views.sort((a, b) => (a.order ?? 50) - (b.order ?? 50));
}

export function toggle(label, hint, checked, onchange) {
  const sw = h('input', { type: 'checkbox', class: 'switch' });
  sw.checked = checked;
  sw.onchange = async () => {
    const v = sw.checked;
    try {
      await onchange(v);
    } catch (e) {
      // the change didn't stick: show the switch as it really is
      sw.checked = !v;
      invoke('ui_log', { msg: `setting failed: ${e}` });
    }
  };
  return h('label', { class: 'field' }, h('div', { class: 'what' }, label, hint && h('small', {}, hint)), sw);
}

export const settings = {
  value: {},
  async load() {
    this.value = await invoke('get_settings');
    return this.value;
  },
  async set(patch) {
    this.value = await invoke('update_settings', { patch });
  },
};

/** Other modules add cards to the settings view (features, keys, …). */
export const settingsSections = [];

