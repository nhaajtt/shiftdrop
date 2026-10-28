// Renders film.html frame by frame in headless Edge (Chrome DevTools Protocol), then
// encodes the frames with ffmpeg. Every frame is drawn from its timestamp, so the video
// is perfectly smooth regardless of machine speed.
//
//   node film/record.mjs [seconds=40] [fps=30] [page=film]

import { spawn, spawnSync } from 'node:child_process';
import { copyFileSync, existsSync, mkdirSync, readdirSync, rmSync, writeFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

const here = dirname(fileURLToPath(import.meta.url));
const SECONDS = Number(process.argv[2] || 40);
const FPS = Number(process.argv[3] || 30);
const PAGE = process.argv[4] || 'film';
const BASE = PAGE === 'film' ? 'demo' : PAGE;
const [WIDTH, HEIGHT] = PAGE === 'vertical' ? [1080, 1920] : [1920, 1080];
const PORT = 9333;
const frames = join(here, 'frames', PAGE);
const media = resolve(here, '..', 'docs', 'media');

const edge = ['ProgramFiles(x86)', 'ProgramFiles']
  .map((v) => process.env[v] && join(process.env[v], 'Microsoft', 'Edge', 'Application', 'msedge.exe'))
  .find((p) => p && existsSync(p));
if (!edge) throw new Error('Microsoft Edge not found');

// empty the folder rather than removing it (Windows keeps folders busy while a shell sits in them)
mkdirSync(frames, { recursive: true });
for (const f of readdirSync(frames)) rmSync(join(frames, f), { force: true });
mkdirSync(frames, { recursive: true });
mkdirSync(media, { recursive: true });

const profile = join(process.env.TEMP || '/tmp', 'shiftdrop-film-edge');
const browser = spawn(edge, [
  '--headless=new', `--remote-debugging-port=${PORT}`, `--user-data-dir=${profile}`,
  `--window-size=${WIDTH},${HEIGHT}`, '--hide-scrollbars', '--force-device-scale-factor=1', '--allow-file-access-from-files', 'about:blank',
]);

const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
let target;
for (let i = 0; i < 50 && !target; i++) {
  await sleep(200);
  target = await fetch(`http://127.0.0.1:${PORT}/json/list`).then((r) => r.json()).then((l) => l.find((t) => t.type === 'page')).catch(() => null);
}
if (!target) throw new Error('Edge did not start');

const ws = new WebSocket(target.webSocketDebuggerUrl);
await new Promise((r) => ws.addEventListener('open', r));
let id = 0;
const waiting = new Map();
ws.addEventListener('message', (e) => {
  const m = JSON.parse(e.data);
  if (m.id && waiting.has(m.id)) {
    waiting.get(m.id)(m);
    waiting.delete(m.id);
  }
});
const send = (method, params = {}) =>
  new Promise((res, rej) => {
    const i = ++id;
    waiting.set(i, (m) => (m.error ? rej(new Error(`${method}: ${m.error.message}`)) : res(m.result)));
    ws.send(JSON.stringify({ id: i, method, params }));
  });

await send('Emulation.setDeviceMetricsOverride', { width: WIDTH, height: HEIGHT, deviceScaleFactor: 1, mobile: false });
await send('Page.enable');
await send('Page.navigate', { url: pathToFileURL(join(here, `${PAGE}.html`)).href });
for (let i = 0; i < 100; i++) {
  const r = await send('Runtime.evaluate', { expression: 'window.ready === true', returnByValue: true });
  if (r.result.value) break;
  await sleep(100);
}

// FILM_AT=1,8.5,… renders only those moments (for checking the look)
if (process.env.FILM_AT) {
  for (const t of process.env.FILM_AT.split(',')) {
    await send('Runtime.evaluate', { expression: `render(${t})` });
    const shot = await send('Page.captureScreenshot', { format: 'jpeg', quality: 80 });
    writeFileSync(join(frames, `at-${t}.jpg`), Buffer.from(shot.data, 'base64'));
  }
  ws.close();
  browser.kill();
  process.exit(0);
}

const total = Math.round(SECONDS * FPS);
for (let f = 0; f < total; f++) {
  await send('Runtime.evaluate', { expression: `render(${(f / FPS).toFixed(4)})` });
  const shot = await send('Page.captureScreenshot', { format: 'jpeg', quality: 93 });
  writeFileSync(join(frames, `${String(f).padStart(5, '0')}.jpg`), Buffer.from(shot.data, 'base64'));
  if (f % FPS === 0) process.stdout.write(`\r${(f / FPS).toFixed(0)}s / ${SECONDS}s`);
}
console.log('\nframes done');
ws.close();
browser.kill();

const ff = (args) => {
  const r = spawnSync('ffmpeg', ['-v', 'error', '-y', ...args], { stdio: 'inherit' });
  if (r.status !== 0) throw new Error(`ffmpeg failed: ${args.join(' ')}`);
};
const input = ['-framerate', String(FPS), '-i', join(frames, '%05d.jpg')];
ff([...input, '-c:v', 'libx264', '-preset', 'slow', '-crf', '18', '-pix_fmt', 'yuv420p', '-movflags', '+faststart', join(media, `${BASE}.mp4`)]);
ff([...input, '-c:v', 'libvpx-vp9', '-crf', '32', '-b:v', '0', '-row-mt', '1', join(media, `${BASE}.webm`)]);
// poster: the wheel open with the PNG option highlighted
copyFileSync(join(frames, `${String(Math.round(8.5 * FPS)).padStart(5, '0')}.jpg`), join(media, BASE === 'demo' ? 'poster.jpg' : `${BASE}-poster.jpg`));
// a light GIF of the first scene for the README
if (BASE === 'demo') ff(['-ss', '3.6', '-t', '9.6', ...input, '-vf', 'fps=15,scale=760:-1:flags=lanczos,split[a][b];[a]palettegen=stats_mode=diff[p];[b][p]paletteuse=dither=bayer:bayer_scale=4', join(media, 'demo.gif')]);
console.log('written to', media);
