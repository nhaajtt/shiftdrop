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
  "Activity": "Hoạt động",
  "Everything ShiftDrop did, newest first. Failures explain what went wrong and how to fix it.": "Mọi việc ShiftDrop đã làm, mới nhất ở trên. Việc lỗi có giải thích nguyên nhân và cách khắc phục.",
  "Failed": "Lỗi",
  "Partly failed": "Lỗi một phần",
  "Try again": "Thử lại",
  "Details": "Chi tiết",
  "Clear list": "Xoá danh sách",
  "just now": "vừa xong",
  "No failures. Nice.": "Không có lỗi nào. Tuyệt.",
  "Nothing yet. Drag a file and hold Shift.": "Chưa có gì. Hãy kéo một file và giữ Shift.",
  "Installed ✓": "Đã cài ✓",
  "Open Settings": "Mở Cài đặt",
  "Get LibreOffice": "Tải LibreOffice",
  "Get Edge": "Tải Edge",
  "Report problem": "Báo lỗi",
  "Setting or removing a PDF password needs the qpdf add-on, which isn’t installed yet.": "Đặt hoặc gỡ mật khẩu PDF cần tiện ích qpdf, mà máy bạn chưa tải.",
  "Video and audio actions need the FFmpeg add-on, which isn’t installed yet.": "Các thao tác video và âm thanh cần tiện ích FFmpeg, mà máy bạn chưa tải.",
  "Subtitles from speech need the Speech to text add-on.": "Tạo phụ đề từ giọng nói cần tiện ích Speech to text.",
  "Background removal and face blurring need the AI images add-on.": "Xoá nền và làm mờ mặt cần tiện ích AI images.",
  "Upscaling needs the Upscaling add-on.": "Phóng to ảnh cần tiện ích Upscaling.",
  "A YARA scan needs a folder of .yar rules.": "Quét YARA cần một thư mục chứa luật .yar.",
  "Pick your rules folder in Settings › Add-ons › YARA rules.": "Chọn thư mục luật trong Cài đặt › Tiện ích thêm › YARA rules.",
  "Converting Word, Excel or PowerPoint needs Microsoft Office or the free LibreOffice, and neither is installed.": "Chuyển Word, Excel, PowerPoint cần Microsoft Office hoặc LibreOffice (miễn phí), mà máy chưa có cái nào.",
  "Install LibreOffice (free), then try again.": "Cài LibreOffice (miễn phí) rồi thử lại.",
  "Turning text into PDF uses Microsoft Edge, which wasn’t found.": "Chuyển văn bản sang PDF cần Microsoft Edge, mà không tìm thấy.",
  "Install or repair Microsoft Edge, then try again.": "Cài hoặc sửa Microsoft Edge rồi thử lại.",
  "The password doesn’t match this file.": "Mật khẩu không đúng với file này.",
  "Try again and check Caps Lock and the keyboard language.": "Thử lại và kiểm tra Caps Lock cùng bộ gõ bàn phím.",
  "The password is too short.": "Mật khẩu quá ngắn.",
  "Use at least 4 characters; a longer phrase is safer.": "Dùng ít nhất 4 ký tự; cụm từ dài hơn sẽ an toàn hơn.",
  "Translate and Summarize use your own Claude API key, which isn’t set.": "Dịch và Tóm tắt dùng API key Claude của bạn, mà chưa được nhập.",
  "Paste your key in Settings › Claude and Cloudflare R2.": "Dán key vào Cài đặt › Claude and Cloudflare R2.",
  "Upload link needs your Cloudflare R2 bucket details.": "Tải lên lấy link cần thông tin bucket Cloudflare R2 của bạn.",
  "Fill in account ID, bucket, key and secret in Settings.": "Điền account ID, bucket, key và secret trong Cài đặt.",
  "Windows Sandbox is only in Windows Pro and Enterprise, with the feature turned on.": "Windows Sandbox chỉ có trên Windows Pro và Enterprise, và phải bật tính năng này.",
  "On Windows Home this action isn’t available. Use Signature or VirusTotal to check the file instead.": "Trên Windows Home không dùng được. Hãy dùng Chữ ký số hoặc VirusTotal để kiểm tra file.",
  "The file name contains & | < > ^ or %, which could turn into a second command.": "Tên file chứa & | < > ^ hoặc %, có thể bị biến thành một lệnh khác.",
  "Rename the file without those characters, then run your command again.": "Đổi tên file bỏ các ký tự đó rồi chạy lại lệnh.",
  "That size limit is too small for a video this long.": "Giới hạn dung lượng đó quá nhỏ cho video dài như vậy.",
  "Pick a bigger size, or trim the video first.": "Chọn dung lượng lớn hơn, hoặc cắt bớt video trước.",
  "Check that this is the right file; for scans, a sharper picture helps.": "Kiểm tra lại đúng file chưa; với bản scan, ảnh nét hơn sẽ dễ đọc hơn.",
  "The file’s content isn’t what this action expects.": "Nội dung file không đúng loại mà thao tác này cần.",
  "It may be damaged or only named like that type. Try “Real type” under More to see what it really is.": "File có thể bị hỏng hoặc chỉ đặt tên giống loại đó. Thử “Loại thật” trong Thêm để xem nó thực sự là gì.",
  "Upscaling needs a graphics card with Vulkan support.": "Phóng to ảnh cần card đồ hoạ hỗ trợ Vulkan.",
  "Update your graphics driver; on PCs without one, upscaling isn’t available.": "Cập nhật driver card đồ hoạ; máy không có card phù hợp thì không dùng được.",
  "Another program was holding the clipboard.": "Một chương trình khác đang giữ clipboard.",
  "Wait a second and try again.": "Chờ một giây rồi thử lại.",
  "Another program has the file open.": "Một chương trình khác đang mở file này.",
  "Close it (or use “In use by” under More to see which one), then try again.": "Đóng chương trình đó (dùng “Ai đang dùng” trong Thêm để xem là chương trình nào) rồi thử lại.",
  "Windows refused access to the file or its folder.": "Windows từ chối quyền truy cập file hoặc thư mục.",
  "Move the file to a folder you own (like Documents) or close programs using it, then try again.": "Chuyển file vào thư mục của bạn (như Documents) hoặc đóng chương trình đang dùng nó rồi thử lại.",
  "The disk is full.": "Ổ đĩa đã đầy.",
  "Free some space, then try again.": "Giải phóng bớt dung lượng rồi thử lại.",
  "The file was moved or deleted before the action ran.": "File đã bị chuyển đi hoặc xoá trước khi thao tác chạy.",
  "Drag it again from where it is now.": "Kéo lại file từ vị trí hiện tại.",
  "The download didn’t complete.": "Tải về không thành công.",
  "Check the internet connection (and VPN), then try again.": "Kiểm tra kết nối mạng (và VPN) rồi thử lại.",
  "Something went wrong that ShiftDrop doesn’t recognise.": "Có lỗi mà ShiftDrop chưa nhận ra.",
  "Try once more. If it keeps failing, report it: the app and Windows versions are filled in for you.": "Thử lại một lần nữa. Nếu vẫn lỗi, hãy báo lỗi: phiên bản app và Windows được điền sẵn.",
  'You have the latest version.': 'Bạn đang dùng bản mới nhất.', Working: 'Đang làm', Done: 'Xong', Failed: 'Lỗi', Undo: 'Hoàn tác', Undone: 'Đã hoàn tác', 'Show ›': 'Mở ›', '⠿ Drag': '⠿ Kéo', done: 'xong', Close: 'Đóng',
  '⠿ Drag all': '⠿ Kéo tất cả', Clear: 'Xoá hết', Open: 'Mở', 'Actions ⌘': 'Thao tác ⌘',
  'Drop files here to keep them for later, from any folder. Drag them out when you need them.':
    'Thả file vào đây để giữ tạm, từ bất kỳ thư mục nào. Cần thì kéo ra.',
  'Save ↵': 'Lưu ↵', 'Loading pages…': 'Đang tải trang…', 'Draw or type a signature first.': 'Hãy vẽ hoặc gõ chữ ký trước.',
};

// Titles with numbers or names in them.
const RULES = [
  [/^(\d+) of (\d+) saved$/, '$1/$2 đã lưu'],
  [/^(\d+) min ago$/, '$1 phút trước'],
  [/^(\d+) h ago$/, '$1 giờ trước'],
  [/^Download (.+)$/, 'Tải $1'],
  [/^Show (.+)$/, 'Mở $1'],
  [/^Nothing to work with: (.+)\.$/, 'Không tìm thấy gì để xử lý: $1.'],
  [/^Download the free (.+) add-on once, then try again\.$/, 'Tải tiện ích $1 miễn phí một lần rồi thử lại.'],
  [/^This action needs the (.+) add-on\.$/, 'Thao tác này cần tiện ích $1.'],
  [/^ShiftDrop (.+) is ready to install$/, 'ShiftDrop $1 đã sẵn sàng để cài'],
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
