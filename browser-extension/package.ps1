param([string]$HostExecutable)
$ErrorActionPreference = 'Stop'
$repoRoot = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '..')).Path
if (-not $HostExecutable) { $HostExecutable = Join-Path $repoRoot 'src-tauri\target\debug\anya-browser-bridge.exe' }
$sourceHost = (Resolve-Path -LiteralPath $HostExecutable).Path
$bundle = Join-Path $repoRoot 'release\browser-selection-bridge'
New-Item -ItemType Directory -Path $bundle -Force | Out-Null
foreach ($name in @('manifest.json', 'content.js', 'background.js', 'install.ps1', 'README.md', 'README.zh-CN.md')) {
  Copy-Item -LiteralPath (Join-Path $PSScriptRoot $name) -Destination (Join-Path $bundle $name) -Force
}
Copy-Item -LiteralPath $sourceHost -Destination (Join-Path $bundle 'anya-browser-bridge.exe') -Force
$archive = Join-Path $repoRoot 'release\Anya-browser-selection-bridge.zip'
Compress-Archive -Path (Join-Path $bundle '*') -DestinationPath $archive -Force
Write-Output $archive
