# Launch the Volt web IDE from PowerShell.
$ErrorActionPreference = "Stop"
$Root = Resolve-Path (Join-Path $PSScriptRoot "..\..")
Set-Location $Root

$release = Join-Path $Root "target\release\voltc.exe"
$debug = Join-Path $Root "target\debug\voltc.exe"

if (Test-Path $release) {
    & $release ide @args
} elseif (Test-Path $debug) {
    & $debug ide @args
} else {
    Write-Host "Building voltc.exe then starting the IDE..."
    cargo run -- ide @args
}
