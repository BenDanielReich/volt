@echo off
setlocal
rem Launch the Volt web IDE from a cmd.exe prompt.
rem Run this from anywhere; it locates voltc.exe relative to the repo.
set "ROOT=%~dp0..\.."
pushd "%ROOT%" >nul

if exist "%ROOT%\target\release\voltc.exe" (
  "%ROOT%\target\release\voltc.exe" ide %*
) else if exist "%ROOT%\target\debug\voltc.exe" (
  "%ROOT%\target\debug\voltc.exe" ide %*
) else (
  echo Building voltc.exe then starting the IDE...
  cargo run -- ide %*
)

popd >nul
