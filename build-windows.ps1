# Noah Windows Release Build Script
# Usage: powershell -File C:\Users\xulea\src\itman\build-windows.ps1

$env:PATH = "C:\Users\xulea\AppData\Roaming\nvm\v22.13.1;C:\Users\xulea\AppData\Local\pnpm;C:\Users\xulea\.cargo\bin;$env:PATH"

Set-Location C:\Users\xulea\src\itman

Write-Host "==> Pulling latest..."
git pull

Write-Host "==> Building..."
node scripts/release.mjs --build

# Read version from tauri.conf.json
$conf = Get-Content apps\desktop\src-tauri\tauri.conf.json | ConvertFrom-Json
$v = $conf.version

Write-Host ""
Write-Host "==> Done! From Mac, run:"
Write-Host "  scp xulea@100.87.199.115:C:/Users/xulea/src/itman/target/release/bundle/nsis/Noah_${v}_x64-setup.exe /tmp/"
Write-Host "  scp xulea@100.87.199.115:C:/Users/xulea/src/itman/target/release/bundle/msi/Noah_${v}_x64_en-US.msi /tmp/"
Write-Host "  gh release upload v${v} /tmp/Noah_${v}_x64-setup.exe /tmp/Noah_${v}_x64_en-US.msi --clobber"
