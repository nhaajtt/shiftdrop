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
document.getElementById('theme').onclick = () => {
  const dark = document.documentElement.dataset.theme !== 'dark';
  document.documentElement.dataset.theme = dark ? 'dark' : 'light';
  localStorage.setItem('theme', dark ? 'dark' : 'light');
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
  // spotlight follows the pointer on cards
  document.querySelectorAll('.card').forEach((c) =>
    c.addEventListener('pointermove', (e) => {
      const r = c.getBoundingClientRect();
      c.style.setProperty('--x', `${e.clientX - r.left}px`);
      c.style.setProperty('--y', `${e.clientY - r.top}px`);
    }),
  );

  // hero video tilts toward the pointer
  const tilt = document.querySelector('.tilt');
  const hero = document.querySelector('.hero');
  hero.addEventListener('pointermove', (e) => {
    const r = tilt.getBoundingClientRect();
    const x = (e.clientX - (r.left + r.width / 2)) / r.width;
    const y = (e.clientY - (r.top + r.height / 2)) / r.height;
    tilt.style.transform = `rotateY(${x * 8}deg) rotateX(${-y * 8}deg)`;
  });
  hero.addEventListener('pointerleave', () => (tilt.style.transform = ''));

  // download buttons lean toward the pointer
  document.querySelectorAll('.magnetic').forEach((b) => {
    b.addEventListener('pointermove', (e) => {
      const r = b.getBoundingClientRect();
      b.style.transform = `translate(${(e.clientX - r.left - r.width / 2) * 0.15}px, ${(e.clientY - r.top - r.height / 2) * 0.25}px)`;
    });
    b.addEventListener('pointerleave', () => (b.style.transform = ''));
  });
}

// --- videos pause when off screen
document.querySelectorAll('video').forEach((v) =>
  new IntersectionObserver(([e]) => {
    if (!e.isIntersecting) v.pause();
    else if (v.autoplay) v.play().catch(() => {});
  }).observe(v),
);
