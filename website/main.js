// Direct download link to the newest installer, the language switch, and reveal-on-scroll.

const REPO = 'nhaajtt/shiftdrop';

fetch(`https://api.github.com/repos/${REPO}/releases/latest`)
  .then((r) => (r.ok ? r.json() : null))
  .then((rel) => {
    const exe = rel?.assets?.find((a) => /-setup\.exe$/i.test(a.name));
    if (!exe) return;
    document.getElementById('download').href = exe.browser_download_url;
    document.getElementById('version').textContent = `v${rel.tag_name.replace(/^v/, '')} · ${(exe.size / 1048576).toFixed(0)} MB · x64`;
  })
  .catch(() => {});

// English / Tiếng Việt: elements carry their Vietnamese text in data-vi
const nodes = [...document.querySelectorAll('[data-vi]')];
nodes.forEach((n) => (n.dataset.en = n.innerHTML));
function setLang(vi) {
  nodes.forEach((n) => (n.innerHTML = vi ? n.dataset.vi : n.dataset.en));
  document.documentElement.lang = vi ? 'vi' : 'en';
  document.getElementById('lang').textContent = vi ? 'EN' : 'VI';
  localStorage.setItem('lang', vi ? 'vi' : 'en');
}
const saved = localStorage.getItem('lang');
let vi = saved ? saved === 'vi' : (navigator.language || '').toLowerCase().startsWith('vi');
setLang(vi);
document.getElementById('lang').onclick = () => setLang((vi = !vi));

// sections fade in once
const io = new IntersectionObserver(
  (entries) => entries.forEach((e) => e.isIntersecting && (e.target.classList.add('in'), io.unobserve(e.target))),
  { rootMargin: '0px 0px -10% 0px' },
);
document.querySelectorAll('.step, .card, .tile, .privacy, .install, section > h2').forEach((el) => {
  el.classList.add('reveal');
  io.observe(el);
});

// pause the film when it is off screen
const film = document.getElementById('film');
new IntersectionObserver(([e]) => (e.isIntersecting ? film.play().catch(() => {}) : film.pause())).observe(film);
