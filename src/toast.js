import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { startDrag } from '@crabnebula/tauri-plugin-drag';
import { relaunch } from '@tauri-apps/plugin-process';
import { check } from '@tauri-apps/plugin-updater';
import { fixFor, runFix } from './fixes.js';
import { t } from './i18n.js';
import { initTheme } from './theme.js';

initTheme();

const $ = (id) => document.getElementById(id);
const jobs = new Map(); // id → latest update
let shown = null; // id of the job on screen
let hideTimer = 0;
const seenFail = new Set(); // failures already shown, so each is shown once
const isFail = (j) => !!j && !!j.error && j.state !== 'running';

const TAGS = { running: 'Working', done: 'Done', error: 'Failed' };

function render(u) {
  shown = u.id;
  $('state').textContent = TAGS[u.state];
  $('state').className = `tag ${u.state}`;
  $('title').textContent = u.error && u.state !== 'running' ? u.title : u.state === 'done' ? doneTitle(u.title) : `${u.title}…`;
  $('fill').style.transform = `scaleX(${u.percent.toFixed(3)})`;
  $('count').textContent = `${u.done} of ${u.total} saved`;
  queueNote(u);
  $('show').hidden = !u.outputs.length;
  $('drag').hidden = !u.outputs.length || u.state !== 'done';
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
  showFailure(u);
}

// what waits behind the card on screen: a count while working, the next job's name under a failure
function queueNote(u) {
  const run = [...jobs.values()].filter((j) => j.state === 'running' && j.id !== u.id);
  if (!run.length) $('queue').textContent = '';
  else if (isFail(u)) $('queue').textContent = `${t('Next:')} ${t(`${run[0].title}…`)}${run.length > 1 ? ` +${run.length - 1}` : ''}`;
  else $('queue').textContent = `+${run.length} queued`;
}

// the window only talks to the backend when the measured height really changes
let sentH = 112;
function setHeight(px) {
  if (px === sentH) return;
  sentH = px;
  invoke('toast_height', { height: px }).catch(() => {});
}
// measure the content itself (the card is stretched to the window, so it could never shrink)
function fitHeight() {
  const c = $('card');
  c.style.height = 'auto';
  const px = Math.ceil(c.offsetHeight) + 18;
  c.style.height = '';
  setHeight(document.body.classList.contains('failed') ? px : 112);
}

// --- failures: stay on screen with what happened and how to fix it
let fixing = null; // the fix offered for the job on screen
function showFailure(u) {
  const failed = !!u.error && u.state !== 'running';
  document.body.classList.toggle('failed', failed);
  if (failed) {
    seenFail.add(u.id);
    const fix = fixFor(u.error);
    fixing = fix;
    $('why').textContent = fix.why;
    $('how').textContent = fix.how;
    $('fixbtn').hidden = !fix.action;
    $('fixbtn').disabled = false;
    $('fixbtn').textContent = fix.action?.label || '';
    $('retry').hidden = true;
    $('report').hidden = fix.action?.kind === 'report';
  }
  // grow the window to fit the explanation, shrink back for progress
  requestAnimationFrame(fitHeight);
}

// the user is done with the failure on screen: show whatever waited behind it
function dismissFailure() {
  document.body.classList.remove('failed');
  if (isFail(jobs.get(shown))) jobs.delete(shown);
  shown = null;
  const next = pickNext();
  if (next) render(next);
  else setHeight(112);
}

function pickNext() {
  const all = [...jobs.values()].filter((j) => j.id !== shown);
  return all.find((j) => isFail(j) && !seenFail.has(j.id)) || all.find((j) => j.state === 'running');
}

$('fixbtn').addEventListener('click', async () => {
  const u = jobs.get(shown);
  if (!fixing?.action) return;
  const btn = $('fixbtn');
  btn.disabled = true;
  const label = btn.textContent;
  if (fixing.action.kind === 'addon') btn.textContent = 'Downloading…';
  try {
    await runFix(fixing.action, invoke, u ? `${u.title}: ${u.error}` : null);
    if (fixing.action.kind === 'addon') {
      btn.textContent = 'Installed ✓';
      $('retry').hidden = false;
    } else btn.disabled = false;
  } catch (e) {
    btn.disabled = false;
    btn.textContent = label;
    $('how').textContent = String(e);
  }
});

// runs the same action again (read back from the Activity list)
$('retry').addEventListener('click', async () => {
  const u = jobs.get(shown);
  try {
    const rec = (await invoke('get_history').catch(() => [])).find((r) => r.id === u?.id);
    if (!rec?.action) return await invoke('open_hub', { paths: [], view: 'activity', item: null });
    await invoke('run_action', { paths: rec.files, action: rec.action, title: rec.title });
    dismissFailure();
  } catch (e) {
    $('how').textContent = String(e);
    fitHeight();
  }
});

$('details').addEventListener('click', () => invoke('open_hub', { paths: [], view: 'activity', item: null }));

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
// in Vietnamese "Đang nén" (working) becomes "Đã nén" (done)
function doneTitle(title) {
  const vi = t(title);
  if (vi !== title && vi.startsWith('Đang ')) return `Đã ${vi.slice(5)}`;
  return title.replace(/^(\w+)ing\b/, (_, w) => doneWord(w));
}

function scheduleHide(ms) {
  clearTimeout(hideTimer);
  hideTimer = setTimeout(() => {
    // :hover is read now, so a pointer that was inside when the window hid can't pin it open
    if (document.body.matches(':hover')) return scheduleHide(1500);
    if ([...jobs.values()].some((j) => j.state === 'running') || isFail(jobs.get(shown))) return;
    jobs.clear();
    invoke('toast_hide');
  }, ms);
}

listen('job', ({ payload: u }) => {
  document.body.classList.remove('update');
  jobs.set(u.id, u);
  const current = jobs.get(shown);
  // keep showing a running job or a failure until it is finished with; the rest queues behind it
  if (!current || (current.state !== 'running' && !isFail(current)) || u.id === shown) render(u);
  else queueNote(current);
  if (u.state !== 'running') {
    if (!isFail(jobs.get(shown))) {
      setTimeout(() => {
        const next = pickNext();
        const now = jobs.get(shown);
        if (next && (!now || now.state !== 'running') && !isFail(now)) render(next);
      }, 1200);
    }
    // failures wait for the user; successes slip away
    if (u.error) clearTimeout(hideTimer);
    else scheduleHide(3500);
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
  dismissFailure();
  invoke('toast_hide');
});

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
