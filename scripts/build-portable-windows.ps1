$ErrorActionPreference = 'Stop'
Set-Location (Join-Path $PSScriptRoot '..')
npm ci
if ($LASTEXITCODE -ne 0) { throw 'Could not install frontend dependencies' }
cargo install tauri-cli --version '^2' --locked
if ($LASTEXITCODE -ne 0) { throw 'Could not install Tauri CLI' }
cargo tauri build --no-bundle
if ($LASTEXITCODE -ne 0) { throw 'Portable build failed' }
$portable = Join-Path (Get-Location) 'target\portable'
New-Item -ItemType Directory -Force -Path $portable | Out-Null
Copy-Item -LiteralPath (Join-Path (Get-Location) 'target\release\forge-ide.exe') -Destination (Join-Path $portable 'Forge-IDE.exe') -Force
@'
Forge IDE Portable
Run Forge-IDE.exe directly. The app does not require an installer and writes no application files next to the executable. Your Gemini API key is stored in Windows Credential Manager; user-specific configuration is stored in the Windows profile.
'@ | Set-Content -LiteralPath (Join-Path $portable 'README.txt')
