import { invoke } from '@tauri-apps/api/core';

/** Runs what a menu entry stands for on the files: an editor, an installed app, or a job. */
export async function runTarget(t, paths) {
  if (t?.shelf) return invoke('shelf_add', { paths });
  if (t?.share) return invoke('open_hub', { paths, view: t.share, item: null });
  if (t?.editor) return invoke('open_editor', { path: paths[0], tool: t.editor });
  if (t?.app) return invoke('open_with', { app: t.app, paths });
  if (t?.action) return invoke('run_action', { paths, action: t.action, title: t.title || t.label });
}
