# Build Volt.exe (native WebView2 IDE) into dist\Volt\
$ErrorActionPreference = "Stop"
$Root = Resolve-Path (Join-Path $PSScriptRoot "..")
Set-Location $Root

Write-Host "Building Volt (Windows app)…"
cargo build --release --features app --bin volt --bin voltc
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

$Out = Join-Path $Root "dist\Volt"
if (Test-Path $Out) { Remove-Item -Recurse -Force $Out }
New-Item -ItemType Directory -Force -Path $Out | Out-Null

Copy-Item (Join-Path $Root "target\release\volt.exe") (Join-Path $Out "Volt.exe")
Copy-Item (Join-Path $Root "target\release\voltc.exe") (Join-Path $Out "voltc.exe")
Copy-Item -Recurse (Join-Path $Root "std") (Join-Path $Out "std")
Copy-Item -Recurse (Join-Path $Root "examples") (Join-Path $Out "examples")
if (Test-Path (Join-Path $Root "runtime")) {
    Copy-Item -Recurse (Join-Path $Root "runtime") (Join-Path $Out "runtime")
}
if (Test-Path (Join-Path $Root "ide\windows\Volt.ico")) {
    Copy-Item (Join-Path $Root "ide\windows\Volt.ico") (Join-Path $Out "Volt.ico")
}

$Shortcut = Join-Path $Out "Volt IDE.lnk"
$Wsh = New-Object -ComObject WScript.Shell
$Sc = $Wsh.CreateShortcut($Shortcut)
$Sc.TargetPath = Join-Path $Out "Volt.exe"
$Sc.WorkingDirectory = $Out
$Sc.Description = "Volt IDE"
if (Test-Path (Join-Path $Out "Volt.ico")) {
    $Sc.IconLocation = Join-Path $Out "Volt.ico"
}
$Sc.Save()

Write-Host "Built $Out"
Write-Host "Run:  $Out\Volt.exe"
