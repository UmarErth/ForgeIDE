$ErrorActionPreference = 'Stop'
Set-Location (Join-Path $PSScriptRoot '..')
npm ci
if ($LASTEXITCODE -ne 0) { throw 'Could not install frontend dependencies' }
cargo install tauri-cli --version '^2' --locked
if ($LASTEXITCODE -ne 0) { throw 'Could not install Tauri CLI' }
cargo tauri build --bundles nsis,msi
if ($LASTEXITCODE -ne 0) { throw 'Windows build failed' }
