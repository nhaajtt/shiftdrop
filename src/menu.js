// What the radial menu offers for the files being dragged.
// item: { label, glyph, title, action?, app?, sub?: [{ label, title, action?, app? }], def?: index into sub, editor? }
// `app` opens the files in an installed tool (see apps.rs) instead of running a job.

const EXT = {
  image: ['png', 'jpg', 'jpeg', 'webp', 'bmp', 'gif', 'tif', 'tiff', 'ico', 'avif', 'heic', 'heif', 'jxl', 'jxr', 'dng', 'cr2', 'cr3', 'nef', 'arw', 'orf', 'rw2', 'raf'],
  video: ['mp4', 'mov', 'mkv', 'webm', 'avi', 'm4v', 'wmv', 'flv', 'mts', 'm2ts', '3gp'],
  audio: ['mp3', 'wav', 'm4a', 'aac', 'flac', 'ogg', 'opus', 'wma', 'aiff', 'aif'],
  pdf: ['pdf'],
  text: ['txt', 'md', 'markdown', 'csv', 'tsv', 'json', 'log', 'xml', 'html', 'htm', 'yaml', 'yml', 'ini', 'cfg', 'conf', 'toml',
    'js', 'mjs', 'cjs', 'ts', 'tsx', 'jsx', 'py', 'rs', 'c', 'h', 'cpp', 'hpp', 'cc', 'cs', 'java', 'kt', 'go', 'rb', 'php', 'sh',
    'bash', 'ps1', 'bat', 'cmd', 'sql', 'css', 'scss', 'lua', 'swift', 'dart', 'vue', 'svelte'],
  archive: ['zip', 'tar', 'gz', 'tgz', '7z', 'rar', 'bz2', 'xz', 'zst'],
  exe: ['exe', 'dll', 'sys', 'ocx', 'scr', 'cpl', 'drv', 'efi', 'so', 'elf'],
  pcap: ['pcap', 'pcapng', 'cap'],
  office: ['doc', 'docx', 'docm', 'odt', 'rtf', 'xls', 'xlsx', 'xlsm', 'ods', 'ppt', 'pptx', 'odp'],
};

// What this computer offers besides the built-in actions; filled by loadContext().
export const ctx = { apps: [], sandbox: false, tools: new Set() };

export async function loadContext(invoke) {
  const [apps, sandbox, tools] = await Promise.all([
    invoke('get_apps').catch(() => []),
    invoke('sandbox_ok').catch(() => false),
    invoke('tools_status').catch(() => []),
  ]);
  Object.assign(ctx, { apps, sandbox, tools: new Set(tools.filter((t) => t.installed).map((t) => t.id)) });
  return ctx;
}

export const extOf = (p) => (p.match(/\.([^.\\/]+)$/)?.[1] || '').toLowerCase();

export function kindOf(path) {
  const e = extOf(path);
  return Object.keys(EXT).find((k) => EXT[k].includes(e)) || 'other';
}

const same = (e, to) => e === to || (e === 'jpeg' && to === 'jpg') || (e === 'tif' && to === 'tiff');

const convert = (to) => ({ label: to.toUpperCase(), title: `Converting to ${to.toUpperCase()}`, action: { op: 'convert', to } });

function convertItem(targets, e, label = 'Convert') {
  const sub = targets.filter((t) => !same(e, t)).map(convert);
  return { label, glyph: '⇄', sub, def: 0 };
}

const levels = (title) => ({
  glyph: '⇲',
  label: 'Compress',
  def: 1,
  sub: [
    { label: 'Light', title, action: { op: 'compress', level: 'low' } },
    { label: 'Med', title, action: { op: 'compress', level: 'medium' } },
    { label: 'Strong', title, action: { op: 'compress', level: 'high' } },
  ],
});

const clean = { label: 'Clean', glyph: '⌫', title: 'Removing metadata', action: { op: 'strip_metadata' } };
const merge = (title = 'Merging') => ({ label: 'Merge', glyph: '⊕', title, action: { op: 'merge' } });

function imageMenu(e, multi) {
  const items = [
    convertItem(['jpg', 'png', 'webp', 'avif', 'pdf', 'ico', 'bmp', 'tiff'], e),
    levels('Compressing'),
    {
      label: 'Resize',
      glyph: '⤡',
      def: 1,
      sub: [75, 50, 25].map((v) => ({ label: `${v}%`, title: `Resizing to ${v}%`, action: { op: 'resize', value: v } })),
    },
    toPhone,
    {
      label: 'Rotate',
      glyph: '↻',
      def: 0,
      sub: [90, 180, 270].map((d) => ({ label: `${d}°`, title: `Rotating ${d}°`, action: { op: 'rotate', deg: d } })),
    },
  ];
  if (multi) items.push(merge('Merging into PDF'));
  else
    items.push(
      { label: 'Crop', glyph: '⌗', editor: 'crop' },
      { label: 'Add BG', glyph: '▣', editor: 'addbg' },
      { label: 'Redact', glyph: '▬', editor: 'redact' },
    );
  items.push(clean);
  const center = convert(same(e, 'jpg') ? 'png' : 'jpg');
  const imageExtra = [
    { id: 'ocr', label: 'Copy text', glyph: 'T', def: 0, keywords: 'ocr recognise read text scan', sub: [
      { label: 'Copy', title: 'Reading text', action: { op: 'ocr', to: 'clipboard' } },
      { label: '.txt', title: 'Reading text', action: { op: 'ocr', to: 'txt' } },
    ] },
    { label: 'Read QR', glyph: '⌗', title: 'Reading QR code', action: { op: 'qr_read' }, keywords: 'qr code scan barcode' },
    { id: 'icon', label: 'Icon', glyph: '◰', def: 0, keywords: 'ico favicon app icon', sub: [
      { label: 'App .ico', title: 'Making icon', action: { op: 'icon_set', web: false } },
      { label: 'Favicons', title: 'Making favicons', action: { op: 'icon_set', web: true } },
    ] },
    { label: 'Colours', glyph: '◕', title: 'Finding colours', action: { op: 'palette' }, keywords: 'palette colors hex' },
    { label: 'Watermark', glyph: '©', title: 'Adding watermark', action: { op: 'watermark' }, ask: [{ key: 'text', label: 'Watermark text' }] },
    { label: 'To SVG', glyph: '✎', title: 'Tracing to SVG', action: { op: 'vectorize' }, keywords: 'vector trace svg' },
    { label: 'Remove BG', glyph: '◐', title: 'Removing background', action: { op: 'remove_bg' }, keywords: 'background cut out transparent ai' },
    { label: 'Blur faces', glyph: '☺', title: 'Blurring faces', action: { op: 'blur_faces' }, keywords: 'privacy anonymize pixelate' },
    { label: 'Upscale', glyph: '⤢', def: 0, keywords: 'enlarge super resolution esrgan sharpen', sub: [2, 4].map((n) => ({ label: `${n}×`, title: `Upscaling ${n}×`, action: { op: 'upscale', scale: n } })) },
  ];
  const extra = multi ? [{ label: 'Contact sheet', glyph: '▦', title: 'Making contact sheet', action: { op: 'contact_sheet' }, keywords: 'grid collage' }, ...imageExtra, { label: 'Slideshow', glyph: '▶', def: 1, keywords: 'video from images', sub: [1, 2, 3, 5].map((n) => ({ label: `${n}s each`, title: 'Making slideshow', action: { op: 'slideshow', seconds: n } })) }, ...common()] : [...imageExtra, ...common()];
  return { center, items, extra };
}

const fitSize = {
  id: 'fit-size',
  label: 'Fit size',
  glyph: '⊡',
  def: 1,
  keywords: 'target size limit discord zalo email mb',
  sub: [
    ...[8, 10, 25, 50].map((mb) => ({ label: `${mb} MB`, title: `Fitting into ${mb} MB`, action: { op: 'target_size', mb } })),
    { label: 'Custom', title: 'Fitting size', action: { op: 'target_size' }, ask: [{ key: 'mb', label: 'Size limit (MB)', type: 'number', number: true }] },
  ],
};
const speed = {
  label: 'Speed',
  glyph: '»',
  def: 2,
  sub: [0.5, 1.5, 2, 4].map((f) => ({ label: `${f}×`, title: `Changing speed to ${f}×`, action: { op: 'speed', factor: f } })),
};
const cleanAudio = {
  id: 'clean-audio',
  label: 'Fix sound',
  glyph: '∿',
  def: 0,
  keywords: 'denoise noise loudness normalize volume',
  sub: [
    { label: 'Denoise', title: 'Removing noise', action: { op: 'audio_fix', mode: 'denoise' } },
    { label: 'Loudness', title: 'Normalizing loudness', action: { op: 'audio_fix', mode: 'normalize' } },
  ],
};
const subtitles = (video) => ({
  id: 'subtitles',
  label: 'Subtitles',
  glyph: '▭',
  def: 0,
  keywords: 'transcribe speech to text whisper srt captions',
  sub: [
    { label: '.srt', title: 'Transcribing', action: { op: 'transcribe', burn: false } },
    ...(video ? [{ label: 'Burn in', title: 'Adding subtitles', action: { op: 'transcribe', burn: true } }] : []),
  ],
});

function videoMenu(e, multi) {
  const items = [
    convertItem(['mp4', 'mov', 'webm', 'mkv', 'gif', 'avi'], e),
    levels('Compressing video'),
    {
      label: 'Resize',
      glyph: '⤡',
      def: 1,
      sub: [1080, 720, 480].map((h) => ({ label: `${h}p`, title: `Resizing to ${h}p`, action: { op: 'resize', value: h } })),
    },
    {
      label: 'Audio',
      glyph: '♪',
      def: 0,
      sub: ['mp3', 'm4a', 'wav'].map((to) => ({ label: to.toUpperCase(), title: 'Extracting audio', action: { op: 'extract_audio', to } })),
    },
    { label: 'Mute', glyph: '∅', title: 'Removing sound', action: { op: 'mute' } },
  ];
  items.push(multi ? merge() : { label: 'Trim', glyph: '✂', editor: 'trim' });
  items.push(clean);
  items.splice(2, 0, fitSize);
  items.splice(4, 0, toPhone);
  const extra = [
    speed,
    { label: 'Flip', glyph: '⇋', def: 0, sub: [{ label: 'Mirror', title: 'Flipping', action: { op: 'flip', dir: 'h' } }, { label: 'Upside down', title: 'Flipping', action: { op: 'flip', dir: 'v' } }] },
    { label: 'Rotate', glyph: '↻', def: 0, sub: [90, 180, 270].map((d) => ({ label: `${d}°`, title: `Rotating ${d}°`, action: { op: 'rotate', deg: d } })) },
    { label: 'Frames', glyph: '▦', def: 0, keywords: 'screenshots extract images', sub: [1, 5, 10].map((n) => ({ label: `every ${n}s`, title: 'Extracting frames', action: { op: 'frames', every: n } })) },
    { label: 'Thumbnail', glyph: '▢', title: 'Making thumbnail', action: { op: 'thumbnail' }, keywords: 'cover poster' },
    cleanAudio,
    subtitles(true),
    ...common(),
  ];
  return { center: convert(same(e, 'mp4') ? 'gif' : 'mp4'), items, extra };
}

function audioMenu(e, multi) {
  const items = [
    convertItem(['mp3', 'm4a', 'wav', 'flac', 'ogg', 'opus'], e),
    {
      label: 'Compress',
      glyph: '⇲',
      def: 1,
      sub: [
        { label: '192K', title: 'Compressing', action: { op: 'compress', level: 'low' } },
        { label: '128K', title: 'Compressing', action: { op: 'compress', level: 'medium' } },
        { label: '96K', title: 'Compressing', action: { op: 'compress', level: 'high' } },
      ],
    },
    multi ? merge() : { label: 'Trim', glyph: '✂', editor: 'trim' },
    toPhone,
    clean,
  ];
  return { center: convert(same(e, 'mp3') ? 'm4a' : 'mp3'), items, extra: [fitSize, speed, cleanAudio, subtitles(false), ...common()] };
}

function pdfMenu(multi) {
  const items = [
    convertItem(['jpg', 'png', 'webp'], 'pdf', 'Image'),
    {
      ...levels('Compressing PDF'),
      sub: [
        { label: 'Light', title: 'Compressing PDF', action: { op: 'compress', level: 'low' } },
        { label: 'Med', title: 'Compressing PDF', action: { op: 'compress', level: 'medium' } },
        { label: 'Strong', title: 'Compressing PDF', action: { op: 'compress', level: 'high' } },
      ],
    },
    { label: 'Split', glyph: '⊟', title: 'Splitting pages', action: { op: 'split' } },
  ];
  items.push(toPhone);
  if (multi) items.push(merge('Merging PDFs'));
  else items.push({ label: 'Pages', glyph: '▤', editor: 'pages', keywords: 'reorder delete remove rotate extract' }, { label: 'Sign', glyph: '✍', editor: 'sign', keywords: 'signature stamp' });
  items.push({ label: 'Text', glyph: 'T', title: 'Getting text', action: { op: 'pdf_text' }, keywords: 'ocr extract txt scanned' });
  const extra = [
    { label: 'Rotate', glyph: '↻', def: 0, sub: [90, 180, 270].map((d) => ({ label: `${d}°`, title: `Rotating ${d}°`, action: { op: 'rotate', deg: d } })) },
    { label: 'Page numbers', glyph: '№', title: 'Numbering pages', action: { op: 'page_numbers' } },
    { label: 'Watermark', glyph: '©', title: 'Adding watermark', action: { op: 'watermark' }, ask: [{ key: 'text', label: 'Watermark text' }], keywords: 'draft confidential' },
    { label: 'Images', glyph: '▦', title: 'Extracting images', action: { op: 'extract_images' }, keywords: 'extract pictures' },
    { label: 'Password', glyph: '⚿', title: 'Protecting', action: { op: 'protect' }, ask: [{ key: 'password', label: 'Password', type: 'password', confirm: true }], keywords: 'encrypt lock protect' },
    { label: 'Unlock', glyph: '⚿', title: 'Unlocking', action: { op: 'unprotect' }, ask: [{ key: 'password', label: 'Password', type: 'password' }], keywords: 'remove password decrypt' },
    ...common(),
  ];
  return { center: convert('jpg'), items, extra };
}

function officeMenu(e) {
  const to = (f, label = f.toUpperCase()) => ({ label, title: `Converting to ${f.toUpperCase()}`, action: { op: 'convert', to: f } });
  const words = ['doc', 'docx', 'docm', 'odt', 'rtf'].includes(e);
  const items = [{ ...to('pdf', 'PDF'), glyph: '⇄' }, toPhone];
  if (words) items.push({ ...to('txt', 'Text'), glyph: 'T' });
  if (e !== 'docx' && words) items.push({ ...to('docx', 'DOCX'), glyph: 'W' });
  if (e === 'docx') items.push({ ...to('md', 'Markdown'), glyph: 'M' });
  return { center: to('pdf'), items, extra: common() };
}

// Actions every kind of file can use.
const zip = { label: 'Zip', glyph: '▤', title: 'Zipping', action: { op: 'zip' } };
const hash = {
  id: 'hashes',
  label: 'Hash',
  glyph: '#',
  def: 2,
  keywords: 'checksum md5 sha',
  sub: [
    ...['md5', 'sha1', 'sha256', 'sha512'].map((a) => ({ label: a.toUpperCase().replace('SHA', 'SHA-'), title: `Copying ${a.toUpperCase()}`, action: { op: 'hashes', algo: a } })),
    { label: 'All', title: 'Writing hash report', action: { op: 'hashes', algo: 'all' } },
  ],
};

// Long-tail actions for any file: reachable under More, promoted to the wheel once used.
const identify = { label: 'Real type', glyph: '?', title: 'Checking real type', action: { op: 'identify' }, keywords: 'identify magic fake extension disguised' };
const virusTotal = { label: 'VirusTotal', glyph: '⛨', title: 'Looking up on VirusTotal', action: { op: 'virus_total' }, keywords: 'malware virus scan reputation' };
const encrypt = {
  label: 'Encrypt',
  glyph: '⚿',
  title: 'Encrypting',
  action: { op: 'encrypt' },
  ask: [{ key: 'password', label: 'Password', type: 'password', confirm: true }],
  keywords: 'password protect lock age',
};
const decrypt = { label: 'Decrypt', glyph: '⚿', title: 'Decrypting', action: { op: 'decrypt' }, ask: [{ key: 'password', label: 'Password', type: 'password' }] };
const shred = {
  label: 'Shred',
  glyph: '⌧',
  title: 'Shredding',
  action: { op: 'shred' },
  ask: [],
  danger: 'Overwrite and permanently delete. This cannot be undone.',
  keywords: 'secure delete wipe destroy',
};
const locks = { label: 'In use by', glyph: '⊘', title: 'Finding programs using it', action: { op: 'locks' }, keywords: 'locked who using process' };
const pathAs = {
  id: 'copy-path-as',
  label: 'Copy path as',
  glyph: '⧉',
  def: 0,
  sub: [
    ['Quoted', 'quoted'],
    ['WSL', 'wsl'],
    ['file://', 'url'],
    ['Markdown', 'markdown'],
  ].map(([label, style]) => ({ label, title: 'Copying path', action: { op: 'copy_path_as', style } })),
};
const base64 = { label: 'Base64', glyph: '⌁', title: 'Encoding Base64', action: { op: 'base64', decode: false }, keywords: 'encode data url' };
const shelf = { label: 'Shelf', glyph: '⊞', shelf: true, keywords: 'keep later stash basket collect' };
const toPhone = { label: 'To phone', glyph: '▯', share: 'phone', keywords: 'send share wifi lan qr mobile transfer' };
const upload = { label: 'Upload link', glyph: '☁', share: 'upload', keywords: 'share r2 cloudflare link cloud' };
const common = () => [shelf, upload, identify, virusTotal, encrypt, shred, locks, pathAs, base64];
const copyPath = { label: 'Path', title: 'Copying path', action: { op: 'copy_path' } };

/** "Open in" with the installed tools for these kinds, or null when none are installed. */
function openIn(apps, kinds, label = 'Open in', glyph = '↗') {
  const sub = apps.filter((a) => a.kinds.some((k) => kinds.includes(k))).map((a) => ({ label: a.label, app: a.id }));
  return sub.length ? { label, glyph, sub, def: 0 } : null;
}

const app = (apps, id) => apps.find((a) => a.id === id);

function textMenu(e, multi, apps, paths) {
  const to = ['pdf'];
  if (!['html', 'htm'].includes(e)) to.push('html');
  if (['csv', 'tsv'].includes(e)) to.push('json');
  if (e === 'json') to.push('csv');
  for (const f of ['json', 'yaml', 'toml']) if (['json', 'yaml', 'yml', 'toml', 'xml'].includes(e) && f !== e && !(f === 'yaml' && e === 'yml') && !to.includes(f)) to.push(f);
  const items = [convertItem(to, e), toPhone];
  if (multi && paths.length === 2) items.unshift({ label: 'Compare', glyph: '⇆', title: 'Comparing', action: { op: 'diff' }, keywords: 'diff' });
  if (e === 'json')
    items.push({
      label: 'Format',
      glyph: '{}',
      def: 0,
      sub: [
        { label: 'Pretty', title: 'Formatting JSON', action: { op: 'format_json', pretty: true } },
        { label: 'Minify', title: 'Minifying JSON', action: { op: 'format_json', pretty: false } },
      ],
    });
  items.push(openIn(apps, ['text']));
  items.push({
    label: 'Copy',
    glyph: '⧉',
    def: 0,
    sub: [{ label: 'Text', title: 'Copying text', action: { op: 'copy_text' } }, copyPath],
  });
  items.push(zip, hash);
  const claude = [
    { id: 'translate', label: 'Translate', glyph: '文', def: 0, keywords: 'claude ai language vietnamese english', sub: [
      { label: 'Tiếng Việt', title: 'Translating', action: { op: 'ask_claude', task: 'translate', lang: 'Vietnamese' } },
      { label: 'English', title: 'Translating', action: { op: 'ask_claude', task: 'translate', lang: 'English' } },
      { label: 'Other…', title: 'Translating', action: { op: 'ask_claude', task: 'translate' }, ask: [{ key: 'lang', label: 'Into which language?' }] },
    ] },
    { label: 'Summarize', glyph: '≣', title: 'Summarizing', action: { op: 'ask_claude', task: 'summarize', lang: 'Vietnamese' }, keywords: 'claude ai summary tl;dr' },
  ];
  const extra = [...claude, ...(['md', 'markdown', 'html', 'htm'].includes(e) ? [{ label: 'Word', glyph: 'W', title: 'Converting to DOCX', action: { op: 'convert', to: 'docx' }, keywords: 'docx word document' }] : []), { label: 'QR code', glyph: '⌗', title: 'Making QR code', action: { op: 'qr_make' }, keywords: 'qr' }, { label: 'Decode Base64', glyph: '⌁', title: 'Decoding Base64', action: { op: 'base64', decode: true } }, ...common()];
  return { center: convert('pdf'), items: items.filter(Boolean), extra };
}

function exeMenu(multi, apps) {
  const debug = app(apps, 'x64dbg');
  const items = [
    debug && { label: 'Debug', glyph: '◉', app: 'x64dbg' },
    openIn(apps.filter((a) => ['ida', 'ghidra', 'binja'].includes(a.id)), ['exe'], 'Reverse', '⌬'),
    openIn(apps.filter((a) => ['die', 'pebear', 'dnspy', 'ilspy', 'hxd'].includes(a.id)), ['exe'], 'Inspect', '⌕'),
    !multi && { label: 'Info', glyph: 'ⓘ', title: 'Reading PE header', action: { op: 'pe_info' } },
    !multi && { label: 'Strings', glyph: '≡', title: 'Extracting strings', action: { op: 'strings' } },
    openIn(apps, ['text'], 'IDE', '⌨'),
    hash,
    zip,
  ].filter(Boolean);
  const extra = [
    { label: 'Signature', glyph: '✓', title: 'Verifying signature', action: { op: 'verify_signature' }, keywords: 'authenticode signed certificate' },
    ctx.sandbox && { label: 'Sandbox', glyph: '◫', title: 'Opening in Windows Sandbox', action: { op: 'sandbox' }, keywords: 'run safely isolated' },
    !multi && { label: 'Capabilities', glyph: '⌬', title: 'Running capa', action: { op: 'tool', tool: 'capa' }, keywords: 'capa mandiant behaviour' },
    !multi && { label: 'Hidden strings', glyph: '≡', title: 'Running FLOSS', action: { op: 'tool', tool: 'floss' }, keywords: 'floss obfuscated strings' },
    !multi && { label: 'YARA scan', glyph: '⌖', title: 'Scanning with YARA', action: { op: 'tool', tool: 'yara' }, keywords: 'yara rules malware detect' },
    !multi && { label: 'Unpack UPX', glyph: '⇪', title: 'Unpacking', action: { op: 'tool', tool: 'upx' }, keywords: 'upx packer' },
    ...common(),
  ].filter(Boolean);
  const center = debug
    ? { label: 'x64dbg', app: 'x64dbg' }
    : { label: 'Info', title: 'Reading PE header', action: { op: 'pe_info' } };
  return { center, items, extra };
}

function archiveMenu(apps) {
  const items = [
    { label: 'Extract', glyph: '⇪', title: 'Extracting', action: { op: 'extract' } },
    toPhone,
    hash,
    { label: 'Copy', glyph: '⧉', title: copyPath.title, action: copyPath.action },
    openIn(apps, ['any']),
  ].filter(Boolean);
  return { center: { label: 'Extract', title: 'Extracting', action: { op: 'extract' } }, items, extra: common() };
}

/** Anything else: folders, unknown types, mixed selections. */
function genericMenu(paths, apps, lead = []) {
  const files = paths.every((p) => extOf(p));
  const items = [
    ...lead,
    zip,
    toPhone,
    files && hash,
    { label: 'Copy', glyph: '⧉', title: copyPath.title, action: copyPath.action },
    openIn(apps, ['any']),
  ].filter(Boolean);
  const age = paths.length === 1 && extOf(paths[0]) === 'age';
  const extra = [...(files ? common() : [pathAs]), ...(files && paths.length === 2 ? [{ label: 'Compare', glyph: '⇆', title: 'Comparing', action: { op: 'diff' } }] : [])];
  if (age) return { center: decrypt, items: [decrypt, ...items], extra };
  return { center: { label: 'Zip', title: zip.title, action: zip.action }, items, extra };
}

/** Every action for a set of dragged paths, before trimming to what fits on the wheel. */
export function buildMenu(paths, apps = []) {
  const kinds = [...new Set(paths.map(kindOf))];
  const multi = paths.length > 1;
  if (kinds.length === 1) {
    const e = extOf(paths[0]);
    const k = kinds[0];
    if (k === 'image') return imageMenu(e, multi);
    if (k === 'video') return videoMenu(e, multi);
    if (k === 'audio') return audioMenu(e, multi);
    if (k === 'pdf') return pdfMenu(multi);
    if (k === 'text') return textMenu(e, multi, apps, paths);
    if (k === 'exe') return exeMenu(multi, apps);
    if (k === 'archive' && !multi) return archiveMenu(apps);
    if (k === 'office') return officeMenu(e);
    if (k === 'pcap' && app(apps, 'wireshark')) {
      return { center: { label: 'Wireshark', app: 'wireshark' }, items: genericMenu(paths, apps).items };
    }
  }
  if (kinds.every((k) => k === 'image' || k === 'pdf')) {
    return genericMenu(paths, apps, [merge('Merging into PDF')]);
  }
  return genericMenu(paths, apps);
}

// --- Keeping the wheel small -------------------------------------------------------------
// At most MAX wedges. The ones used least (and anything only reachable through `extra`) move
// to "More…", a searchable list with every action. Usage and the user's choices live in
// localStorage, shared by all windows.

const MAX = 8;
const KEY = 'shiftdrop.menu';
const idOf = (it) => it.id || it.label.toLowerCase();

export const menuKind = (paths) => {
  const kinds = [...new Set(paths.map(kindOf))];
  return kinds.length === 1 ? kinds[0] : 'mixed';
};

export function prefs() {
  try {
    return JSON.parse(localStorage.getItem(KEY)) || {};
  } catch {
    return {};
  }
}

export function savePrefs(p) {
  localStorage.setItem(KEY, JSON.stringify(p));
}

/** Counts a use of a wheel item so frequently used ones stay on the wheel. */
export function noteUse(kind, id) {
  const p = prefs();
  p.usage ??= {};
  const k = `${kind}:${id}`;
  p.usage[k] = (p.usage[k] || 0) + 1;
  savePrefs(p);
}

/** What dropping on an item does without picking a sub-option. */
export const defaultOf = (it) => (it.editor || it.action || it.app || it.shelf || it.share ? it : it.sub?.[it.def ?? 0] ?? null);

export const MORE = { id: 'more', label: 'More', glyph: '⋯', more: true };
/** How many extra actions fit around the More wedge before "All…". */
const MORE_RING = 10;

/** The menu as shown: { center, items (≤ MAX, maybe ending in More), all, kind }. */
export { extList };

export function menuFor(paths, apps = []) {
  const kind = menuKind(paths);
  const raw = buildMenu(paths, apps);
  const p = prefs();
  raw.extra = [...(raw.extra || []), ...ownActions(p, kind, paths)];
  const all = [...raw.items, ...(raw.extra || [])].map((it) => ({ ...it, id: idOf(it) }));
  const hidden = new Set(p.hidden?.[kind] || []);
  const score = (it) => p.usage?.[`${kind}:${it.id}`] || 0;
  const base = raw.items.length;
  // wheel items by default, plus extra ones the user has actually used
  const cand = all.filter((it, i) => !hidden.has(it.id) && (i < base || score(it) > 0));
  const overflow = cand.length > MAX || all.length > cand.length;
  let items = cand;
  if (overflow) {
    const keep = new Set(
      cand
        .map((it, i) => [it, i])
        .sort((a, b) => score(b[0]) - score(a[0]) || a[1] - b[1])
        .slice(0, MAX - 1)
        .map(([it]) => it),
    );
    // hovering More shows the next actions in the outer ring, like any other wedge,
    // with "All…" at the end for the searchable list
    const rest = all
      .map((it, i) => [it, i])
      .filter(([it]) => !keep.has(it) && defaultOf(it))
      .sort((a, b) => score(b[0]) - score(a[0]) || a[1] - b[1])
      .slice(0, MORE_RING)
      .map(([it]) => ({ ...defaultOf(it), label: it.label, title: defaultOf(it).title || it.label, from: it.id }));
    const allItem = { label: 'All…', more: true };
    items = [...cand.filter((it) => keep.has(it)), { ...MORE, sub: [...rest, allItem], def: rest.length }];
  }
  const pinned = all.find((it) => it.id === p.center?.[kind]);
  const center = (pinned && defaultOf(pinned)) || raw.center;
  return { center, items, all, kind };
}

/** Sample file names, one per kind, to build a kind's menu without real files. */
export const SAMPLES = {
  image: ['a.png'], video: ['a.mp4'], audio: ['a.mp3'], pdf: ['a.pdf'], text: ['a.txt'], office: ['a.docx'],
  exe: ['a.exe'], archive: ['a.zip'], other: ['a.bin'], mixed: ['a.png', 'b.mp3'],
};

const extList = (s) => (s || '').split(/[\s,;]+/).map((e) => e.trim().replace(/^\./, '').toLowerCase()).filter(Boolean);

/** The user's recipes and commands that apply to these files. */
function ownActions(p, kind, paths) {
  const recipes = (p.recipes || [])
    .filter((r) => r.kind === kind || r.kind === 'any')
    .map((r) => ({ id: `recipe-${r.id}`, label: r.name, glyph: '⚡', title: r.name, keywords: 'recipe', action: { op: 'chain', steps: r.steps.map((s) => s.action) } }));
  const commands = (p.commands || [])
    .filter((c) => !extList(c.exts).length || paths.every((x) => extList(c.exts).includes(extOf(x))))
    .map((c) => ({ id: `cmd-${c.id}`, label: c.name, glyph: '›', title: c.name, keywords: 'command custom', action: { op: 'custom', command: c.command } }));
  return [...recipes, ...commands];
}

/** Every action as one flat list for the searchable "More…" view. */
export function flatActions(all) {
  const out = [];
  for (const it of all) {
    if (it.sub) for (const s of it.sub) out.push({ ...s, parent: it.id, name: `${it.label} › ${s.label}`, glyph: it.glyph });
    else out.push({ ...it, parent: it.id, name: it.label });
  }
  return out;
}
