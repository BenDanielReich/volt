# Build dist\Volt with optional AVR tools, plus Install Volt.cmd
$ErrorActionPreference = "Stop"
$Root = Resolve-Path (Join-Path $PSScriptRoot "..")
Set-Location $Root

& (Join-Path $PSScriptRoot "package-windows.ps1")
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

$Out = Join-Path $Root "dist\Volt"
$ToolsAvr = Join-Path $Root "tools\avr"
if (Test-Path $ToolsAvr) {
    Write-Host "Bundling AVR tools…"
    $Dest = Join-Path $Out "tools\avr"
    New-Item -ItemType Directory -Force -Path (Join-Path $Out "tools") | Out-Null
    Copy-Item -Recurse -Force $ToolsAvr $Dest
}

$Setup = Join-Path $Root "dist\Install-Volt.cmd"
@"
@echo off
setlocal
set SRC=%~dp0Volt
set DEST=%LOCALAPPDATA%\Volt
echo Installing Volt to %DEST% ...
if not exist "%SRC%\Volt.exe" (
  echo Run this from the folder that contains the Volt directory.
  pause
  exit /b 1
)
xcopy /E /I /Y "%SRC%" "%DEST%" >nul
powershell -NoProfile -Command ^
  "$s=(New-Object -ComObject WScript.Shell).CreateShortcut([Environment]::GetFolderPath('StartMenu') + '\Volt.lnk'); $s.TargetPath='%DEST%\Volt.exe'; $s.WorkingDirectory='%DEST%'; $s.Save()"
echo.
echo Installed. AVR compiler is inside the app if this folder has tools\avr.
echo Pico / ESP: open Volt and click Get compiler.
echo.
start "" "%DEST%\Volt.exe"
"@ | Set-Content -Path $Setup -Encoding ASCII

Write-Host "Windows payload: $Out"
Write-Host "Installer:       $Setup"
Write-Host "Zip those two together, or run Install-Volt.cmd after copying dist\Volt."
