// Newest installer link, language and theme switches, and the small motion touches.

const REPO = 'nhaajtt/shiftdrop';
const reduce = matchMedia('(prefers-reduced-motion: reduce)').matches;

fetch(`https://api.github.com/repos/${REPO}/releases/latest`)
  .then((r) => (r.ok ? r.json() : null))
  .then((rel) => {
    const exe = rel?.assets?.find((a) => /-setup\.exe$/i.test(a.name));
    if (!exe) return;
    document.querySelectorAll('#download, .dl').forEach((a) => (a.href = exe.browser_download_url));
    document.getElementById('version').textContent = `v${rel.tag_name.replace(/^v/, '')} · ${(exe.size / 1048576).toFixed(0)} MB`;
  })
  .catch(() => {});

// --- English / Tiếng Việt: elements keep their Vietnamese text in data-vi
const nodes = [...document.querySelectorAll('[data-vi]')];
nodes.forEach((n) => (n.dataset.en = n.innerHTML));
function setLang(lang) {
  const vi = lang === 'vi';
  nodes.forEach((n) => (n.innerHTML = vi ? n.dataset.vi : n.dataset.en));
  document.documentElement.lang = lang;
  document.querySelectorAll('[data-lang]').forEach((b) => b.setAttribute('aria-pressed', String(b.dataset.lang === lang)));
  localStorage.setItem('lang', lang);
}
setLang(localStorage.getItem('lang') || ((navigator.language || '').toLowerCase().startsWith('vi') ? 'vi' : 'en'));
document.querySelectorAll('[data-lang]').forEach((b) => (b.onclick = () => setLang(b.dataset.lang)));

// --- light / dark
// a circle grows from the button into the new theme (View Transitions API where supported)
document.getElementById('theme').onclick = (e) => {
  const dark = document.documentElement.dataset.theme !== 'dark';
  const apply = () => {
    document.documentElement.dataset.theme = dark ? 'dark' : 'light';
    localStorage.setItem('theme', dark ? 'dark' : 'light');
  };
  if (!document.startViewTransition || reduce) return apply();
  const x = e.clientX || innerWidth - 60;
  const y = e.clientY || 30;
  const r = Math.hypot(Math.max(x, innerWidth - x), Math.max(y, innerHeight - y));
  document.startViewTransition(apply).ready.then(() =>
    document.documentElement.animate(
      { clipPath: [`circle(0 at ${x}px ${y}px)`, `circle(${r}px at ${x}px ${y}px)`] },
      { duration: 520, easing: 'cubic-bezier(0.2, 0.8, 0.2, 1)', pseudoElement: '::view-transition-new(root)' },
    ),
  );
};

// --- reveal on scroll (steps also draw their line)
const io = new IntersectionObserver(
  (entries) =>
    entries.forEach((e) => {
      if (!e.isIntersecting) return;
      e.target.classList.add('in');
      io.unobserve(e.target);
    }),
  { rootMargin: '0px 0px -12% 0px' },
);
document.querySelectorAll('.reveal, .track').forEach((el, i) => {
  if (el.classList.contains('reveal')) el.style.transitionDelay = `${(i % 3) * 70}ms`;
  io.observe(el);
});

// --- numbers count up once
const counter = new IntersectionObserver((entries) =>
  entries.forEach((e) => {
    if (!e.isIntersecting) return;
    counter.unobserve(e.target);
    const el = e.target;
    const to = Number(el.dataset.to);
    const suffix = el.dataset.suffix || '';
    if (reduce || to === 0) return (el.textContent = `${to}${suffix}`);
    const start = performance.now();
    const tick = (now) => {
      const p = Math.min(1, (now - start) / 1200);
      el.textContent = `${Math.round(to * (1 - Math.pow(1 - p, 3)))}${suffix}`;
      if (p < 1) requestAnimationFrame(tick);
    };
    requestAnimationFrame(tick);
  }),
);
document.querySelectorAll('.count').forEach((el) => counter.observe(el));

if (!reduce) {
  // pointer effects write styles at most once per frame, whatever the mouse rate
  const frame = (fn) => {
    let last = null;
    return (e) => {
      if (!last) requestAnimationFrame(() => (fn(last), (last = null)));
      last = e;
    };
  };

  // positions are measured once when the pointer enters, never inside the move handler,
  // so moving the mouse causes no layout at all
  const tracked = (el, onMove, onLeave) => {
    let rect = null;
    el.addEventListener('pointerenter', () => (rect = el.getBoundingClientRect()));
    el.addEventListener('pointermove', frame((e) => rect && onMove(e, rect)));
    el.addEventListener('pointerleave', () => ((rect = null), onLeave?.()));
  };

  // spotlight follows the pointer on cards
  document.querySelectorAll('.card').forEach((c) => {
    const spot = document.createElement('i');
    spot.className = 'spot';
    spot.setAttribute('aria-hidden', 'true');
    c.prepend(spot);
    tracked(c, (e, r) => (spot.style.transform = `translate(${(e.clientX - r.left).toFixed(0)}px, ${(e.clientY - r.top).toFixed(0)}px)`));
  });

  // hero video tilts toward the pointer
  const tilt = document.querySelector('.tilt');
  tracked(
    tilt,
    (e, r) => {
      const x = (e.clientX - (r.left + r.width / 2)) / r.width;
      const y = (e.clientY - (r.top + r.height / 2)) / r.height;
      tilt.style.transform = `rotateY(${(x * 8).toFixed(2)}deg) rotateX(${(-y * 8).toFixed(2)}deg)`;
    },
    () => (tilt.style.transform = ''),
  );

  // download buttons lean toward the pointer
  document.querySelectorAll('.magnetic').forEach((b) =>
    tracked(
      b,
      (e, r) => (b.style.transform = `translate(${((e.clientX - r.left - r.width / 2) * 0.15).toFixed(1)}px, ${((e.clientY - r.top - r.height / 2) * 0.25).toFixed(1)}px)`),
      () => (b.style.transform = ''),
    ),
  );
}

// --- phones: send the download page to a PC; no autoplay on mobile data
const phone = matchMedia('(max-width: 880px)').matches;
const page = 'https://shiftdrop.nhaajt.com';
async function shareLink(btn) {
  const vi = document.documentElement.lang === 'vi';
  const text = vi ? 'ShiftDrop: kéo file, giữ Shift. Cài trên máy tính Windows:' : 'ShiftDrop: drag a file, hold Shift. Install it on a Windows PC:';
  try {
    if (navigator.share) return await navigator.share({ title: 'ShiftDrop', text, url: page });
  } catch {
    return;
  }
  copyLink(btn);
}
async function copyLink(btn) {
  const vi = document.documentElement.lang === 'vi';
  try {
    await navigator.clipboard.writeText(page);
    const old = btn.textContent;
    btn.textContent = vi ? 'Đã chép ✓' : 'Copied ✓';
    setTimeout(() => (btn.textContent = old), 1600);
  } catch {}
}
document.getElementById('shareLink')?.addEventListener('click', (e) => shareLink(e.currentTarget));
document.getElementById('copyLink')?.addEventListener('click', (e) => copyLink(e.currentTarget));
document.querySelectorAll('.share-again').forEach((b) => b.addEventListener('click', () => shareLink(b)));
const heroVideo = document.getElementById('heroVideo');
if (phone && heroVideo) {
  heroVideo.autoplay = false;
  heroVideo.preload = 'none';
  heroVideo.pause();
  heroVideo.controls = true;
}

// --- videos pause when off screen
document.querySelectorAll('video').forEach((v) =>
  new IntersectionObserver(([e]) => {
    if (!e.isIntersecting) v.pause();
    else if (v.autoplay) v.play().catch(() => {});
  }).observe(v),
);
