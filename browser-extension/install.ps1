param([string]$HostExecutable, [switch]$Uninstall)
$ErrorActionPreference = 'Stop'
$bridgeDir = Join-Path $env:LOCALAPPDATA 'Anya'
$registryKeys = @('HKCU:\Software\Google\Chrome\NativeMessagingHosts\ai.anya.selection', 'HKCU:\Software\Microsoft\Edge\NativeMessagingHosts\ai.anya.selection')
if ($Uninstall) {
  foreach ($key in $registryKeys) { if (Test-Path -LiteralPath $key) { Remove-Item -LiteralPath $key } }
  Write-Host 'Anya selection host unregistered. Remove the extension in your browser.'
  exit
}
if (-not $HostExecutable) {
  $bundledHost = Join-Path $PSScriptRoot 'anya-browser-bridge.exe'
  $HostExecutable = if (Test-Path -LiteralPath $bundledHost) { $bundledHost } else { Join-Path $PSScriptRoot '..\src-tauri\target\debug\anya-browser-bridge.exe' }
}
$executable = (Resolve-Path -LiteralPath $HostExecutable).Path
$manifest = Get-Content -LiteralPath (Join-Path $PSScriptRoot 'manifest.json') -Raw | ConvertFrom-Json
$keyBytes = [Convert]::FromBase64String($manifest.key)
$sha = [Security.Cryptography.SHA256]::Create()
$digest = $sha.ComputeHash($keyBytes)
$sha.Dispose()
$extensionId = -join ($digest[0..15] | ForEach-Object { [char](97 + ($_ -shr 4)); [char](97 + ($_ -band 15)) })
New-Item -ItemType Directory -Path $bridgeDir -Force | Out-Null
$origin = "chrome-extension://$extensionId/"
[IO.File]::WriteAllText((Join-Path $bridgeDir 'browser-extension-origin.txt'), $origin)
$nativeManifest = @{ name = 'ai.anya.selection'; description = 'Anya local selection bridge'; path = $executable; type = 'stdio'; allowed_origins = @($origin) }
$nativeManifestPath = Join-Path $bridgeDir 'native-messaging.json'
[IO.File]::WriteAllText($nativeManifestPath, ($nativeManifest | ConvertTo-Json -Depth 5))
foreach ($registryKey in $registryKeys) {
  New-Item -Path $registryKey -Force | Out-Null
  Set-Item -LiteralPath $registryKey -Value $nativeManifestPath
}
Write-Host "Registered for Chrome and Edge. Extension ID: $extensionId"
Write-Host "Load unpacked extension directory: $PSScriptRoot"
