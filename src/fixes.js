// Turns an error message from a job into plain words and one thing the user can do about it.
// fix: { why, how, action? } where action is
//   { kind: 'addon', id, label } | { kind: 'settings', label } | { kind: 'url', url, label } | { kind: 'report', label }

const ADDONS = {
  qpdf: 'qpdf', ffmpeg: 'FFmpeg', whisper: 'Speech to text', ai: 'AI images', upscale: 'Upscaling',
  capa: 'capa', floss: 'FLOSS', upx: 'UPX', yara: 'YARA-X',
};

const addon = (id, why) => ({
  why,
  how: `Download the free ${ADDONS[id]} add-on once, then try again.`,
  action: { kind: 'addon', id, label: `Download ${ADDONS[id]}` },
});

const RULES = [
  [/qpdf add-on/i, () => addon('qpdf', 'Setting or removing a PDF password needs the qpdf add-on, which isn’t installed yet.')],
  [/FFmpeg add-on|ffmpeg not found|ffprobe/i, () => addon('ffmpeg', 'Video and audio actions need the FFmpeg add-on, which isn’t installed yet.')],
  [/Speech to text isn't installed|speech model/i, () => addon('whisper', 'Subtitles from speech need the Speech to text add-on.')],
  [/AI add-on/i, () => addon('ai', 'Background removal and face blurring need the AI images add-on.')],
  [/Upscaling isn't installed/i, () => addon('upscale', 'Upscaling needs the Upscaling add-on.')],
  [/^(capa|floss|upx|yara) isn't installed/i, (m) => addon(m[1].toLowerCase(), `This action needs the ${ADDONS[m[1].toLowerCase()]} add-on.`)],
  [/YARA rules folder/i, () => ({ why: 'A YARA scan needs a folder of .yar rules.', how: 'Pick your rules folder in Settings › Add-ons › YARA rules.', action: { kind: 'settings', label: 'Open Settings' } })],
  [/Office documents need/i, () => ({ why: 'Converting Word, Excel or PowerPoint needs Microsoft Office or the free LibreOffice, and neither is installed.', how: 'Install LibreOffice (free), then try again.', action: { kind: 'url', url: 'https://www.libreoffice.org/download/download-libreoffice/', label: 'Get LibreOffice' } })],
  [/Edge is needed/i, () => ({ why: 'Turning text into PDF uses Microsoft Edge, which wasn’t found.', how: 'Install or repair Microsoft Edge, then try again.', action: { kind: 'url', url: 'https://www.microsoft.com/edge', label: 'Get Edge' } })],
  [/wrong password/i, () => ({ why: 'The password doesn’t match this file.', how: 'Try again and check Caps Lock and the keyboard language.' })],
  [/password is too short/i, () => ({ why: 'The password is too short.', how: 'Use at least 4 characters; a longer phrase is safer.' })],
  [/Claude API key/i, () => ({ why: 'Translate and Summarize use your own Claude API key, which isn’t set.', how: 'Paste your key in Settings › Claude and Cloudflare R2.', action: { kind: 'settings', label: 'Open Settings' } })],
  [/R2/i, () => ({ why: 'Upload link needs your Cloudflare R2 bucket details.', how: 'Fill in account ID, bucket, key and secret in Settings.', action: { kind: 'settings', label: 'Open Settings' } })],
  [/Windows Sandbox needs/i, () => ({ why: 'Windows Sandbox is only in Windows Pro and Enterprise, with the feature turned on.', how: 'On Windows Home this action isn’t available. Use Signature or VirusTotal to check the file instead.' })],
  [/unsafe in a command line/i, () => ({ why: 'The file name contains & | < > ^ or %, which could turn into a second command.', how: 'Rename the file without those characters, then run your command again.' })],
  [/too small for a video this long/i, () => ({ why: 'That size limit is too small for a video this long.', how: 'Pick a bigger size, or trim the video first.' })],
  [/no text found|no QR code found|no faces found|no images in this PDF/i, (m) => ({ why: `Nothing to work with: ${m[0]}.`, how: 'Check that this is the right file; for scans, a sharper picture helps.' })],
  [/not a text file|not valid (JSON|YAML|TOML|XML)|not a Word document|can't read .* as data|not a Windows executable/i, () => ({ why: 'The file’s content isn’t what this action expects.', how: 'It may be damaged or only named like that type. Try “Real type” under More to see what it really is.' })],
  [/Vulkan/i, () => ({ why: 'Upscaling needs a graphics card with Vulkan support.', how: 'Update your graphics driver; on PCs without one, upscaling isn’t available.' })],
  [/clipboard is busy/i, () => ({ why: 'Another program was holding the clipboard.', how: 'Wait a second and try again.' })],
  [/os error 32|being used by another process/i, () => ({ why: 'Another program has the file open.', how: 'Close it (or use “In use by” under More to see which one), then try again.' })],
  [/os error 5\b|access is denied|permission denied/i, () => ({ why: 'Windows refused access to the file or its folder.', how: 'Move the file to a folder you own (like Documents) or close programs using it, then try again.' })],
  [/os error 112|not enough space|no space/i, () => ({ why: 'The disk is full.', how: 'Free some space, then try again.' })],
  [/file not found|os error 2\b|cannot find the file/i, () => ({ why: 'The file was moved or deleted before the action ran.', how: 'Drag it again from where it is now.' })],
  [/download failed|didn't answer|GitHub didn't answer/i, () => ({ why: 'The download didn’t complete.', how: 'Check the internet connection (and VPN), then try again.' })],
];

export function fixFor(error) {
  const first = (error || '').split('\n')[0].replace(/^[^:]+\.\w{2,5}: /, ''); // drop "file.ext: " in batches
  for (const [re, make] of RULES) {
    const m = first.match(re);
    if (m) return make(m);
  }
  return {
    why: 'Something went wrong that ShiftDrop doesn’t recognise.',
    how: 'Try once more. If it keeps failing, report it: the app and Windows versions are filled in for you.',
    action: { kind: 'report', label: 'Report problem' },
  };
}

/** Runs a fix's action. `invoke` is passed in so this file stays free of imports. */
export async function runFix(action, invoke, context) {
  if (!action) return;
  if (action.kind === 'addon') return invoke('tools_install', { id: action.id });
  if (action.kind === 'settings') return invoke('open_hub', { paths: [], view: 'settings', item: null });
  if (action.kind === 'url') return invoke('open_link', { url: action.url });
  if (action.kind === 'report') return invoke('report_bug', { context });
}
