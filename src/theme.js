import '@fontsource-variable/jetbrains-mono';
import '@fontsource-variable/inter';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { setLanguage } from './i18n.js';

const dark = matchMedia('(prefers-color-scheme: dark)');
let current = { accent: 'gold', theme: 'system' };

function apply() {
  const root = document.documentElement;
  root.dataset.accent = current.accent || 'gold';
  const isDark = current.theme === 'dark' || (current.theme === 'system' && dark.matches);
  root.classList.toggle('dark', isDark);
  setLanguage(current.lang || 'system');
}

/** Applies accent + light/dark from the tray settings and keeps them in sync. */
export async function initTheme() {
  apply();
  dark.addEventListener('change', apply);
  listen('settings', (e) => {
    current = e.payload;
    apply();
  });
  try {
    current = await invoke('get_settings');
  } catch {}
  apply();
}
