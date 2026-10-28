// Vietnamese interface. Text is written in English in the code; every window swaps known
// strings as they appear (wheel, hub, toast, editors). File names and paths are marked
// translate="no" and left alone.

const VI = {
  // wheel and actions
  Convert: 'Chuyển', Compress: 'Nén', Resize: 'Đổi cỡ', Rotate: 'Xoay', Crop: 'Cắt', 'Add BG': 'Thêm nền', Redact: 'Che',
  Clean: 'Xoá metadata', Merge: 'Gộp', 'Fit size': 'Vừa dung lượng', Audio: 'Tách tiếng', Mute: 'Tắt tiếng', Trim: 'Cắt đoạn',
  Split: 'Tách trang', Pages: 'Trang', Sign: 'Ký tên', Text: 'Lấy chữ', Image: 'Ảnh', 'Copy text': 'Chép chữ', 'Read QR': 'Đọc QR',
  Icon: 'Icon', Colours: 'Bảng màu', Watermark: 'Watermark', 'To SVG': 'Sang SVG', 'Remove BG': 'Xoá nền', 'Blur faces': 'Làm mờ mặt',
  Upscale: 'Phóng to', Shelf: 'Kệ tạm', 'To phone': 'Sang điện thoại', 'Upload link': 'Tải lên lấy link', 'Real type': 'Loại thật',
  Encrypt: 'Mã hoá', Decrypt: 'Giải mã', Shred: 'Xoá vĩnh viễn', 'In use by': 'Ai đang dùng', 'Copy path as': 'Chép đường dẫn',
  Hash: 'Mã hash', Zip: 'Nén ZIP', Copy: 'Chép', Path: 'Đường dẫn', Extract: 'Giải nén', Format: 'Định dạng', 'Open in': 'Mở bằng',
  Debug: 'Debug', Reverse: 'Dịch ngược', Inspect: 'Soi', Info: 'Thông tin', Strings: 'Chuỗi', Signature: 'Chữ ký số',
  Sandbox: 'Sandbox', Capabilities: 'Khả năng', 'Hidden strings': 'Chuỗi ẩn', 'Unpack UPX': 'Giải nén UPX', More: 'Thêm',
  Speed: 'Tốc độ', Flip: 'Lật', Frames: 'Khung hình', Thumbnail: 'Ảnh bìa', 'Fix sound': 'Sửa âm', Subtitles: 'Phụ đề',
  Slideshow: 'Trình chiếu', 'Contact sheet': 'Ghép lưới', Compare: 'So sánh', Translate: 'Dịch', Summarize: 'Tóm tắt',
  'QR code': 'Mã QR', 'Decode Base64': 'Giải Base64', 'Page numbers': 'Đánh số trang', Images: 'Lấy ảnh', Password: 'Mật khẩu',
  Unlock: 'Mở khoá', Light: 'Nhẹ', Med: 'Vừa', Strong: 'Mạnh', Mirror: 'Gương', 'Upside down': 'Lộn ngược', Denoise: 'Lọc ồn',
  Loudness: 'Âm lượng', 'Burn in': 'In vào video', Custom: 'Tuỳ chọn', All: 'Tất cả', Quoted: 'Có ngoặc', Pretty: 'Đẹp',
  Minify: 'Thu gọn', 'App .ico': 'Icon app', Favicons: 'Favicon web', 'Other…': 'Khác…', Word: 'Word',
  'drop here': 'thả vào đây', 'not': 'không', 'supported': 'hỗ trợ', '⚠ fake type': '⚠ giả đuôi',
  // hub
  Actions: 'Thao tác', Wheel: 'Vòng tròn', Settings: 'Cài đặt', Rename: 'Đổi tên', Recipes: 'Công thức', Confirm: 'Xác nhận',
  'Drop files here': 'Thả file vào đây', 'Paste': 'Dán', 'Nothing matches.': 'Không có kết quả.', 'opens app': 'mở app', editor: 'trình sửa',
  'Drop files here, or choose "More" on the wheel to see everything you can do with them.':
    'Thả file vào đây, hoặc chọn "Thêm" trên vòng tròn để xem mọi thao tác.',
  ' files or an image (Ctrl+V)': ' file hoặc ảnh (Ctrl+V)', 'or ': 'hoặc ',
  'Pick what shows on the wheel for each kind of file. The rest stays one search away under More. Items you use often move onto the wheel by themselves.':
    'Chọn mục nào hiện trên vòng tròn cho từng loại file. Phần còn lại nằm trong "Thêm", tìm là thấy. Mục dùng nhiều sẽ tự lên vòng tròn.',
  'on the wheel': 'trên vòng tròn', 'under More': 'trong "Thêm"', 'set centre': 'đặt ở giữa', centre: 'ở giữa',
  'Everything runs on this computer. Features that need the internet stay off until you turn them on.':
    'Mọi thứ chạy trên máy này. Tính năng cần Internet chỉ bật khi bạn bật.',
  'Shift + drag menu': 'Menu Shift + kéo', 'Start dragging a file, then hold Shift.': 'Bắt đầu kéo file rồi giữ Shift.',
  'Only from File Explorer and desktop': 'Chỉ khi kéo từ File Explorer và desktop', 'Start with Windows': 'Chạy cùng Windows',
  'Quick look': 'Xem nhanh', 'Ctrl+Space in File Explorer previews the selected file.': 'Ctrl+Space trong File Explorer để xem trước file đang chọn.',
  Appearance: 'Giao diện', Language: 'Ngôn ngữ', system: 'theo hệ thống', light: 'sáng', dark: 'tối',
  'Add-ons': 'Tiện ích thêm', Download: 'Tải về', Installed: 'Đã cài', 'Downloading…': 'Đang tải…', Retry: 'Thử lại',
  'Online services': 'Dịch vụ trực tuyến', Explorer: 'Explorer', 'Right-click menu': 'Menu chuột phải',
  'Cancel': 'Huỷ', 'Run': 'Chạy', 'Yes, do it': 'Đồng ý', 'The two entries are different.': 'Hai lần nhập không khớp.',
  'Overwrite and permanently delete. This cannot be undone.': 'Ghi đè rồi xoá vĩnh viễn. Không thể khôi phục.',
  'Watermark text': 'Nội dung watermark', 'Size limit (MB)': 'Giới hạn dung lượng (MB)', 'Into which language?': 'Dịch sang ngôn ngữ nào?',
  'New name': 'Tên mới', Find: 'Tìm', 'Replace with': 'Thay bằng', 'Counter from': 'Đếm từ', Case: 'Chữ hoa/thường', regex: 'regex',
  'Drop files on the Actions page first.': 'Hãy thả file ở trang Thao tác trước.',
  'Rename many files at once. {name} is the old name, {n} a counter, {date} today.': 'Đổi tên nhiều file cùng lúc. {name} là tên cũ, {n} là số đếm, {date} là hôm nay.',
  'Your recipes': 'Công thức của bạn', 'New recipe': 'Công thức mới', 'Add step': 'Thêm bước', 'Save recipe': 'Lưu công thức',
  'Your commands': 'Lệnh của bạn', Add: 'Thêm', 'Watched folders': 'Thư mục tự động', Watch: 'Theo dõi', Stop: 'Dừng', Delete: 'Xoá',
  'Make a recipe first.': 'Hãy tạo một công thức trước.',
  'Chain actions into one drop, add your own commands, and let folders process new files by themselves.':
    'Nối nhiều thao tác thành một lần thả, thêm lệnh riêng, và để thư mục tự xử lý file mới.',
  'Send to phone': 'Gửi sang điện thoại', 'Preparing…': 'Đang chuẩn bị…',
  'Scan with the phone camera. The phone must be on the same Wi-Fi.': 'Quét bằng camera điện thoại. Điện thoại phải cùng mạng Wi-Fi.',
  'Links copied to the clipboard:': 'Đã chép link vào clipboard:', 'Uploading to your Cloudflare R2 bucket…': 'Đang tải lên bucket Cloudflare R2…',
  // toast, shelf, peek, editors
  'Check for updates': 'Kiểm tra cập nhật', Updates: 'Cập nhật', 'Checking…': 'Đang kiểm tra…',
  'You have the latest version.': 'Bạn đang dùng bản mới nhất.', Working: 'Đang làm', Done: 'Xong', Failed: 'Lỗi', Undo: 'Hoàn tác', Undone: 'Đã hoàn tác', 'Show ›': 'Mở ›', '⠿ Drag': '⠿ Kéo', done: 'xong', Close: 'Đóng',
  '⠿ Drag all': '⠿ Kéo tất cả', Clear: 'Xoá hết', Open: 'Mở', 'Actions ⌘': 'Thao tác ⌘',
  'Drop files here to keep them for later, from any folder. Drag them out when you need them.':
    'Thả file vào đây để giữ tạm, từ bất kỳ thư mục nào. Cần thì kéo ra.',
  'Save ↵': 'Lưu ↵', 'Loading pages…': 'Đang tải trang…', 'Draw or type a signature first.': 'Hãy vẽ hoặc gõ chữ ký trước.',
};

// Titles with numbers or names in them.
const RULES = [
  [/^(\d+) of (\d+) saved$/, '$1/$2 đã lưu'],
  [/^\+(\d+) queued$/, '+$1 đang chờ'],
  [/^Converting to (.+)$/, 'Đang chuyển sang $1'], [/^Converted to (.+)$/, 'Đã chuyển sang $1'],
  [/^Resizing to (.+)$/, 'Đang đổi cỡ $1'], [/^Rotating (.+)$/, 'Đang xoay $1'], [/^Fitting into (.+)$/, 'Đang nén vừa $1'],
  [/^Changing speed to (.+)$/, 'Đang đổi tốc độ $1'], [/^Copying (.+)$/, 'Đang chép $1'], [/^Upscaling (.+)$/, 'Đang phóng to $1'],
  [/^Search (\d+) actions…$/, 'Tìm trong $1 thao tác…'], [/^Reset (.+)$/, 'Đặt lại $1'], [/^Renamed (\d+)$/, 'Đã đổi tên $1'],
  [/^used (\d+)×$/, 'đã dùng $1 lần'], [/^(\d+) files$/, '$1 file'], [/^every (\d+)s$/, 'mỗi $1 giây'], [/^(\d+)s each$/, '$1 giây/ảnh'],
  [/^(\d+) pages?$/, '$1 trang'], [/^(\d+) items$/, '$1 mục'], [/^Page (\d+)$/, 'Trang $1'],
];

let on = false;

export function t(s) {
  if (!on || !s) return s;
  const k = s.trim();
  if (VI[k]) return s.replace(k, VI[k]);
  for (const [re, to] of RULES) if (re.test(k)) return s.replace(k, k.replace(re, to));
  // "Convert › PNG", "Zip (centre)"
  if (k.includes(' › ')) return s.replace(k, k.split(' › ').map((p) => t(p)).join(' › '));
  const c = k.match(/^(.+) \(centre\)$/);
  if (c) return s.replace(k, `${t(c[1])} (ở giữa)`);
  return s;
}

const skip = (n) => n.parentElement?.closest('[translate="no"], .chip, .mono, pre, input, textarea, .n, #name, #file');

function walk(root) {
  const it = document.createTreeWalker(root, NodeFilter.SHOW_TEXT);
  for (let n = it.nextNode(); n; n = it.nextNode()) {
    if (!skip(n)) {
      const v = t(n.nodeValue);
      if (v !== n.nodeValue) n.nodeValue = v;
    }
  }
  if (root.querySelectorAll) {
    for (const e of root.querySelectorAll('[placeholder],[title]')) {
      if (e.placeholder) e.placeholder = t(e.placeholder);
      if (e.title && !skip(e.firstChild || e)) e.title = t(e.title);
    }
  }
}

let observer = null;

/** Turns Vietnamese on or off for this window ('system' follows Windows' language). */
export function setLanguage(lang) {
  on = lang === 'vi' || (lang !== 'en' && (navigator.language || '').toLowerCase().startsWith('vi'));
  document.documentElement.lang = on ? 'vi' : 'en';
  if (!on) return;
  walk(document.body);
  if (observer) return;
  observer = new MutationObserver((list) => {
    for (const m of list) {
      if (m.type === 'characterData') {
        const v = skip(m.target) ? m.target.nodeValue : t(m.target.nodeValue);
        if (v !== m.target.nodeValue) m.target.nodeValue = v;
      } else for (const n of m.addedNodes) n.nodeType === 3 ? !skip(n) && (n.nodeValue = t(n.nodeValue)) : n.nodeType === 1 && walk(n);
    }
  });
  observer.observe(document.body, { childList: true, subtree: true, characterData: true });
}
