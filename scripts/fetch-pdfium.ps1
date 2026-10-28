# Downloads pdfium.dll (Windows x64) from bblanchon/pdfium-binaries into src-tauri/resources/bin.
$ErrorActionPreference = 'Stop'
$dest = Join-Path $PSScriptRoot '..\src-tauri\resources\bin'
$tmp = Join-Path $env:TEMP 'pdfium-win-x64.tgz'
$url = 'https://github.com/bblanchon/pdfium-binaries/releases/latest/download/pdfium-win-x64.tgz'
Write-Host "Downloading $url"
Invoke-WebRequest $url -OutFile $tmp
$out = Join-Path $env:TEMP 'pdfium-win-x64'
New-Item -ItemType Directory -Force $out | Out-Null
tar -xzf $tmp -C $out
New-Item -ItemType Directory -Force $dest | Out-Null
Copy-Item (Join-Path $out 'bin\pdfium.dll') $dest -Force
Write-Host "pdfium.dll -> $dest"
