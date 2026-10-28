import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { startDrag } from '@crabnebula/tauri-plugin-drag';
import { relaunch } from '@tauri-apps/plugin-process';
import { check } from '@tauri-apps/plugin-updater';
import { initTheme } from './theme.js';

initTheme();

const $ = (id) => document.getElementById(id);
const jobs = new Map(); // id → latest update
let shown = null; // id of the job on screen
let hideTimer = 0;
let hovering = false;

const TAGS = { running: 'Working', done: 'Done', error: 'Failed' };

function render(u) {
  shown = u.id;
  $('state').textContent = TAGS[u.state];
  $('state').className = `tag ${u.state}`;
  $('title').textContent = u.state === 'done' ? u.title.replace(/^(\w+)ing\b/, (_, w) => doneWord(w)) : `${u.title}…`;
  $('fill').style.transform = `scaleX(${u.percent.toFixed(3)})`;
  $('count').textContent = `${u.done} of ${u.total} saved`;
  const waiting = [...jobs.values()].filter((j) => j.state === 'running' && j.id !== u.id).length;
  $('queue').textContent = waiting ? `+${waiting} queued` : '';
  $('show').hidden = !u.outputs.length;
  $('drag').hidden = !u.outputs.length || u.state !== 'done';
  $('report').hidden = u.state !== 'error';
  $('undo').hidden = !u.outputs.length || u.state !== 'done';
  $('undo').disabled = false;
  $('undo').textContent = 'Undo';
  // results without files (hash copied, signature status…) are shown as a note
  const msg = u.error || u.note || '';
  $('error').hidden = !msg;
  $('error').classList.toggle('note', !u.error);
  $('error').textContent = msg;
  $('error').title = msg;
  if (u.note && !u.outputs.length && u.state === 'done') $('count').textContent = 'done';
}

// "Converting to JPG" → "Converted to JPG", "Merging" → "Merged"
const DONE = {
  Convert: 'Converted', Compress: 'Compressed', Resiz: 'Resized', Rotat: 'Rotated', Remov: 'Removed',
  Merg: 'Merged', Splitt: 'Split', Extract: 'Extracted', Cropp: 'Cropped', Add: 'Added', Redact: 'Redacted',
  Trimm: 'Trimmed', Check: 'Checked', Verify: 'Verified', Encrypt: 'Encrypted', Decrypt: 'Decrypted',
  Look: 'Looked', Find: 'Found', Shredd: 'Shredded', Encod: 'Encoded', Decod: 'Decoded', Compar: 'Compared', Read: 'Read', Mak: 'Made', Trac: 'Traced', Fitt: 'Fitted', Chang: 'Changed',
  Flipp: 'Flipped', Transcrib: 'Transcribed', Normaliz: 'Normalized', Copy: 'Copied', Writ: 'Written', Unpack: 'Unpacked',
  Runn: 'Ran', Open: 'Opened', Blurr: 'Blurred', Upscal: 'Upscaled', Translat: 'Translated', Summariz: 'Summarized', Sign: 'Signed', Sav: 'Saved', Protect: 'Protected', Unlock: 'Unlocked', Number: 'Numbered', Gett: 'Got',
};
const doneWord = (w) => DONE[w] || `${w}ed`;

function scheduleHide(ms) {
  clearTimeout(hideTimer);
  hideTimer = setTimeout(() => {
    if (hovering) return scheduleHide(1500);
    if ([...jobs.values()].some((j) => j.state === 'running')) return;
    jobs.clear();
    invoke('toast_hide');
  }, ms);
}

listen('job', ({ payload: u }) => {
  document.body.classList.remove('update');
  jobs.set(u.id, u);
  const current = jobs.get(shown);
  // keep showing a running job until it finishes; then move on to the newest
  if (!current || current.state !== 'running' || u.id === shown) render(u);
  if (u.state !== 'running') {
    const next = [...jobs.values()].find((j) => j.state === 'running');
    if (next && next.id !== u.id) setTimeout(() => render(jobs.get(next.id)), 1200);
    scheduleHide(u.state === 'error' ? 9000 : 3500);
  } else {
    clearTimeout(hideTimer);
  }
});

$('show').addEventListener('click', () => {
  const u = jobs.get(shown);
  if (u?.outputs[0]) invoke('reveal', { path: u.outputs[0] });
});
let dragIcon = '';
invoke('drag_icon').then((p) => (dragIcon = p));
$('drag').addEventListener('pointerdown', (e) => {
  e.preventDefault();
  const u = jobs.get(shown);
  if (u?.outputs.length) startDrag({ item: u.outputs, icon: dragIcon }).catch(() => {});
});
$('report').addEventListener('click', () => {
  const u = jobs.get(shown);
  invoke('report_bug', { context: u ? `${u.title}: ${u.error || ''}` : null });
});
$('undo').addEventListener('click', async () => {
  const u = jobs.get(shown);
  if (!u?.outputs.length) return;
  try {
    await invoke('trash', { paths: u.outputs });
    $('undo').textContent = 'Undone';
    $('undo').disabled = true;
    $('show').hidden = true;
    $('drag').hidden = true;
    u.outputs = [];
  } catch (e) {
    $('error').hidden = false;
    $('error').textContent = String(e);
  }
});
$('close').addEventListener('click', () => {
  clearTimeout(hideTimer);
  invoke('toast_hide');
});
document.body.addEventListener('pointerenter', () => (hovering = true));
document.body.addEventListener('pointerleave', () => (hovering = false));

// --- updates: checked shortly after start and then once a day; never installed without a click

const DAY = 24 * 60 * 60 * 1000;
let pending = null;

async function checkForUpdate() {
  if (import.meta.env.DEV) return;
  try {
    const update = await check();
    if (!update) return;
    // "Later" quiets this version for three days
    const later = JSON.parse(localStorage.getItem('shiftdrop.later') || '{}');
    if (later.version === update.version && Date.now() - later.at < 3 * DAY) return;
    if ([...jobs.values()].some((j) => j.state === 'running')) return setTimeout(checkForUpdate, 60000);
    pending = update;
    document.body.classList.add('update');
    $('state').textContent = 'Update';
    $('state').className = 'tag';
    $('title').textContent = `ShiftDrop ${update.version} is ready to install`;
    clearTimeout(hideTimer);
    invoke('toast_show');
  } catch {
    // offline or GitHub unreachable: try again tomorrow
  }
}

$('later').addEventListener('click', () => {
  if (pending) localStorage.setItem('shiftdrop.later', JSON.stringify({ version: pending.version, at: Date.now() }));
  document.body.classList.remove('update');
  invoke('toast_hide');
});

$('install').addEventListener('click', async () => {
  if (!pending) return;
  $('install').disabled = $('later').disabled = true;
  $('install').textContent = 'Installing…';
  try {
    await pending.downloadAndInstall();
    await relaunch();
  } catch (e) {
    $('updnote').textContent = String(e);
    $('install').disabled = $('later').disabled = false;
    $('install').textContent = 'Install';
  }
});

setTimeout(checkForUpdate, 20000);
setInterval(checkForUpdate, DAY);
