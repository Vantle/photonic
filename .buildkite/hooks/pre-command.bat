@echo off
powershell.exe -NoLogo -NoProfile -ExecutionPolicy Bypass -File .buildkite/bootstrap.ps1
if errorlevel 1 exit /b 1
set "PATH=%LOCALAPPDATA%\photonic\bootstrap\1.28.1;%PATH%"
set "BAZELISK_HOME=%LOCALAPPDATA%\photonic\bazelisk"
