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
  "Use at least 8 characters; a longer phrase is safer.": "Dùng ít nhất 8 ký tự; cụm từ dài hơn sẽ an toàn hơn.",
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

  // job titles (the toast shows them while working)
  Cropping: 'Đang cắt', 'Adding background': 'Đang thêm nền', Redacting: 'Đang che', Trimming: 'Đang cắt đoạn', 'Saving pages': 'Đang lưu trang',
  Signing: 'Đang ký', 'Removing metadata': 'Đang xoá metadata', 'Reading text': 'Đang đọc chữ', 'Reading QR code': 'Đang đọc mã QR',
  'Making icon': 'Đang tạo icon', 'Making favicons': 'Đang tạo favicon', 'Finding colours': 'Đang tìm màu', 'Adding watermark': 'Đang thêm watermark',
  'Tracing to SVG': 'Đang chuyển sang SVG', 'Removing background': 'Đang xoá nền', 'Blurring faces': 'Đang làm mờ mặt',
  'Making contact sheet': 'Đang ghép lưới', 'Making slideshow': 'Đang tạo trình chiếu', 'Fitting size': 'Đang nén vừa dung lượng',
  'Removing noise': 'Đang lọc ồn', 'Normalizing loudness': 'Đang cân âm lượng', Transcribing: 'Đang nhận dạng giọng nói',
  'Adding subtitles': 'Đang thêm phụ đề', 'Extracting audio': 'Đang tách tiếng', 'Removing sound': 'Đang tắt tiếng', Flipping: 'Đang lật',
  'Extracting frames': 'Đang tách khung hình', 'Making thumbnail': 'Đang tạo ảnh bìa', Compressing: 'Đang nén', 'Compressing PDF': 'Đang nén PDF',
  'Splitting pages': 'Đang tách trang', 'Getting text': 'Đang lấy chữ', 'Numbering pages': 'Đang đánh số trang', 'Extracting images': 'Đang lấy ảnh',
  Protecting: 'Đang đặt mật khẩu', Unlocking: 'Đang mở khoá', Zipping: 'Đang nén ZIP', 'Writing hash report': 'Đang ghi báo cáo hash',
  'Checking real type': 'Đang kiểm tra loại thật', 'Looking up on VirusTotal': 'Đang tra trên VirusTotal', Encrypting: 'Đang mã hoá',
  Decrypting: 'Đang giải mã', Shredding: 'Đang xoá vĩnh viễn', 'Finding programs using it': 'Đang tìm chương trình đang dùng file',
  'Encoding Base64': 'Đang mã hoá Base64', Comparing: 'Đang so sánh', 'Formatting JSON': 'Đang định dạng JSON', 'Minifying JSON': 'Đang thu gọn JSON',
  Translating: 'Đang dịch', Summarizing: 'Đang tóm tắt', 'Making QR code': 'Đang tạo mã QR', 'Decoding Base64': 'Đang giải Base64',
  'Reading PE header': 'Đang đọc PE header', 'Extracting strings': 'Đang tách chuỗi', 'Verifying signature': 'Đang kiểm tra chữ ký số',
  'Opening in Windows Sandbox': 'Đang mở trong Windows Sandbox', 'Running capa': 'Đang chạy capa', 'Running FLOSS': 'Đang chạy FLOSS',
  'YARA scan': 'Quét YARA', 'Scanning with YARA': 'Đang quét bằng YARA', Unpacking: 'Đang giải nén', Extracting: 'Đang giải nén', 'All…': 'Tất cả…',
  Installing: 'Đang cài', 'Installing…': 'Đang cài…', Install: 'Cài', Later: 'Để sau', Update: 'Cập nhật', Report: 'Báo lỗi',
  'Updates are signed and installed in a few seconds.': 'Bản cập nhật có chữ ký và cài trong vài giây.',

  // editors
  'Custom colour': 'Màu tuỳ chọn', '0 areas': '0 vùng', 'Drag over what to black out.': 'Kéo chuột qua phần cần che.',
  'Set to playhead': 'Lấy vị trí đang phát', 'Preview ▶': 'Xem thử ▶', 'or type your name': 'hoặc gõ tên của bạn',
  'Move, turn or remove pages. A new PDF is saved; the original stays as it is.': 'Di chuyển, xoay hoặc bỏ trang. ShiftDrop lưu một PDF mới; bản gốc giữ nguyên.',
  'Drag a box on the page where the signature goes.': 'Kéo một khung trên trang ở chỗ cần đặt chữ ký.', save: 'lưu', close: 'đóng',

  // settings
  'YARA rules': 'Luật YARA', 'Folder with .yar rules, e.g. C:\\Tools\\rules': 'Thư mục chứa luật .yar, ví dụ C:\\Tools\\rules', 'Used by YARA scan on programs.': 'Dùng khi quét YARA chương trình.', 'VirusTotal API key (optional)': 'Khoá API VirusTotal (không bắt buộc)',
  'Only the file’s SHA-256 is sent, never the file. Without a key the public report page opens instead.': 'Chỉ gửi mã SHA-256 của file, không bao giờ gửi file. Không có khoá thì mở trang báo cáo công khai.',
  'Adds "ShiftDrop…" to Explorer’s menu (under "Show more options" on Windows 11).': 'Thêm "ShiftDrop…" vào menu chuột phải của Explorer (trong "Show more options" trên Windows 11).',
  'Updates are signed; only releases from the official repository are accepted.': 'Bản cập nhật có chữ ký; chỉ nhận bản phát hành từ kho chính thức.',
  'Report a problem': 'Báo lỗi', 'Opens a GitHub issue with your app and Windows version filled in. You see everything before it is sent.': 'Mở một issue trên GitHub, điền sẵn phiên bản app và Windows. Bạn xem hết trước khi gửi.',
  'Open log folder': 'Mở thư mục nhật ký', 'Allowed on the wheel': 'Cho hiện trên vòng', 'Make this the centre action': 'Đặt làm thao tác ở giữa', or: 'hoặc',
  'None yet. Make one below, e.g. Resize 50% → JPG → Remove metadata.': 'Chưa có. Tạo ở dưới, ví dụ Đổi cỡ 50% → JPG → Xoá metadata.',
  'Recipe name, e.g. "Web photo"': 'Tên công thức, ví dụ "Ảnh web"', 'Click to remove': 'Bấm để bỏ', 'Give it a name and at least one step.': 'Đặt tên và thêm ít nhất một bước.',
  Name: 'Tên', 'Extensions (empty = all)': 'Đuôi file (trống = tất cả)', 'only these extensions (optional)': 'chỉ các đuôi này (không bắt buộc)',
  'Runs in the file’s folder. {file} {dir} {name} {stem} {ext} are filled in. New files in the folder count as the result.': 'Chạy trong thư mục của file. {file} {dir} {name} {stem} {ext} được điền sẵn. File mới xuất hiện trong thư mục được tính là kết quả.',
  'New files that land in the folder get the recipe. Files already there are left alone.': 'File mới vào thư mục sẽ được áp công thức. File có sẵn được giữ nguyên.',
  'That folder does not exist.': 'Thư mục đó không tồn tại.', expired: 'đã hết hạn', 'Link works for': 'Link dùng được trong', Upload: 'Tải lên',
  'Claude and Cloudflare R2': 'Claude và Cloudflare R2',
  'Keys are stored encrypted for your Windows account. Files are sent only when you pick Translate, Summarize or Upload link.': 'Khoá được lưu mã hoá theo tài khoản Windows của bạn. File chỉ được gửi đi khi bạn chọn Dịch, Tóm tắt hoặc Tải lên lấy link.',

  // welcome
  Welcome: 'Chào mừng', 'ShiftDrop is ready': 'ShiftDrop đã sẵn sàng',
  'It runs quietly in the system tray (the ⇄ icon by the clock). There is no window to keep open.': 'App chạy ngầm ở khay hệ thống (biểu tượng ⇄ cạnh đồng hồ). Không cần giữ cửa sổ nào mở.',
  'So the wheel is always there when you drag.': 'Để vòng thao tác luôn sẵn sàng khi bạn kéo file.', 'Video and audio': 'Video và âm thanh',
  'Needs the free FFmpeg add-on (≈90 MB, official Windows build). You can also do this later in Settings.': 'Cần tiện ích FFmpeg miễn phí (≈90 MB, bản Windows chính thức). Bạn cũng có thể làm sau trong Cài đặt.',
  'Got it, let me try': 'Đã hiểu, thử ngay',

  // failures added in 1.0.9
  'The helper program got stuck on this file, so ShiftDrop stopped it. Nothing was changed.': 'Chương trình hỗ trợ bị treo với file này nên ShiftDrop đã dừng nó. Không có gì bị thay đổi.',
  'The file may be damaged or very large. Open it in its usual app to check, then try again.': 'File có thể bị hỏng hoặc quá lớn. Mở bằng app quen dùng để kiểm tra rồi thử lại.',
  'The downloaded add-on didn’t match its known fingerprint, so it was thrown away instead of installed.': 'Tiện ích tải về không khớp mã kiểm tra đã biết nên đã bị bỏ, không được cài.',
  'Usually a broken download. Try again later; if it keeps happening, report it.': 'Thường do tải bị lỗi. Thử lại sau; nếu vẫn bị, hãy báo lỗi.',
  'That add-on is already downloading.': 'Tiện ích này đang được tải.', 'Wait for it to finish, then try again.': 'Đợi tải xong rồi thử lại.',
  'The old add-on is still being used by a running action.': 'Bản tiện ích cũ vẫn đang được một thao tác sử dụng.', 'Wait for running actions to finish, then try again.': 'Đợi các thao tác đang chạy xong rồi thử lại.',
  'This picture is animated; the action would keep only its first frame.': 'Ảnh này là ảnh động; thao tác này sẽ chỉ giữ khung hình đầu.',
  'Convert it to MP4 or WebM video first if you want to keep the animation.': 'Hãy chuyển sang video MP4 hoặc WebM trước nếu muốn giữ chuyển động.',
  'Compressing would not make this file smaller.': 'Nén không làm file nhỏ hơn được.', 'Nothing to do: the file is already compact.': 'Không cần làm gì: file đã gọn sẵn.',
  'This PDF has no pages.': 'PDF này không có trang nào.', 'It may be damaged. Open it in a PDF reader to check.': 'File có thể bị hỏng. Mở bằng trình đọc PDF để kiểm tra.',
  'PowerPoint is open, and ShiftDrop won’t take over your open presentations.': 'PowerPoint đang mở, và ShiftDrop không can thiệp vào các bài trình chiếu bạn đang mở.',
  'Save and close PowerPoint, then try again.': 'Lưu và đóng PowerPoint rồi thử lại.', 'The document has a password.': 'Tài liệu có mật khẩu.',
  'Open it in its app, remove the password (or save an unprotected copy), then try again.': 'Mở bằng app của nó, gỡ mật khẩu (hoặc lưu một bản không khoá) rồi thử lại.',
  'The picture is already so large that enlarging it would make a file too big to open.': 'Ảnh đã lớn đến mức phóng to sẽ tạo ra file quá lớn để mở.',
  'Pick 2× instead, or crop the picture first.': 'Chọn 2× thay vào đó, hoặc cắt ảnh trước.',
  'The saved key looks damaged (it contains quotes or line breaks).': 'Khoá đã lưu có vẻ bị hỏng (chứa dấu ngoặc kép hoặc xuống dòng).',
  'Copy the key again from its website and paste it in Settings.': 'Chép lại khoá từ trang web của dịch vụ rồi dán vào Cài đặt.',
  'None of the pictures could be read.': 'Không đọc được ảnh nào.', 'Check that they are real image files.': 'Kiểm tra xem chúng có đúng là file ảnh không.',
  'ShiftDrop hit an internal error on this file. Nothing was changed.': 'ShiftDrop gặp lỗi nội bộ với file này. Không có gì bị thay đổi.',
  'Please report it so it can be fixed: the details are filled in for you.': 'Hãy báo lỗi để được sửa: thông tin đã được điền sẵn.',
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
  // function replacements: a "$&" in a file name must stay as it is
  if (VI[k]) return s.replace(k, () => VI[k]);
  for (const [re, to] of RULES) if (re.test(k)) return s.replace(k, () => k.replace(re, to));
  // "Compressing…" while a job runs
  if (k.endsWith('…') && k.length > 1) {
    const base = t(k.slice(0, -1));
    if (base !== k.slice(0, -1)) return s.replace(k, () => `${base}…`);
  }
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
