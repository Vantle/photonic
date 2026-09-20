@echo off
powershell.exe -NoLogo -NoProfile -ExecutionPolicy Bypass -File .buildkite/bootstrap.ps1
if errorlevel 1 exit /b 1
set "PATH=%BUILDKITE_BUILD_CHECKOUT_PATH%\.cache\bootstrap\1.28.1;%PATH%"
set "BAZELISK_HOME=%BUILDKITE_BUILD_CHECKOUT_PATH%\.cache\bazelisk"
