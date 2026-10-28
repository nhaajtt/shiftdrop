# Third-party software

ShiftDrop ships with:

| Component | Use | License |
|---|---|---|
| [PDFium](https://pdfium.googlesource.com/pdfium/) (`pdfium.dll`, build from [bblanchon/pdfium-binaries](https://github.com/bblanchon/pdfium-binaries)) | Reading and rendering PDFs | BSD-3-Clause / Apache-2.0 |
| [Tauri](https://tauri.app) and its plugins | App shell | MIT / Apache-2.0 |
| Rust crates listed in `src-tauri/Cargo.lock` | Image, PDF, archive, crypto and data handling | MIT / Apache-2.0 / BSD and similar |
| [Inter](https://rsms.me/inter/) and [JetBrains Mono](https://www.jetbrains.com/lp/mono/) | Fonts | SIL Open Font License 1.1 |

Add-ons are **not** included in the installer. They are downloaded from their official
releases only when you press *Download* in Settings › Add-ons, and keep their own licenses:

| Add-on | Source | License |
|---|---|---|
| FFmpeg | [BtbN/FFmpeg-Builds](https://github.com/BtbN/FFmpeg-Builds) | GPL-3.0 (this build) |
| whisper.cpp + `ggml-base` model | [ggml-org/whisper.cpp](https://github.com/ggml-org/whisper.cpp) | MIT |
| Real-ESRGAN ncnn Vulkan | [xinntao/Real-ESRGAN](https://github.com/xinntao/Real-ESRGAN) | BSD-3-Clause |
| ONNX Runtime | [microsoft/onnxruntime](https://github.com/microsoft/onnxruntime) | MIT |
| IS-Net (background removal) | [danielgatis/rembg](https://github.com/danielgatis/rembg) | Apache-2.0 / model terms |
| UltraFace RFB-320 | [onnx/models](https://github.com/onnx/models) | MIT |
| capa, FLOSS | [mandiant](https://github.com/mandiant) | Apache-2.0 |
| UPX | [upx/upx](https://github.com/upx/upx) | GPL-2.0+ with exception |
| qpdf | [qpdf/qpdf](https://github.com/qpdf/qpdf) | Apache-2.0 |
