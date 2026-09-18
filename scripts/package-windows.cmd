@echo off
setlocal
rem Build Volt.exe into dist\Volt\  (run from a Developer Command Prompt or any shell with cargo)
powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0package-windows.ps1" %*
if errorlevel 1 exit /b 1
