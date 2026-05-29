@echo off
REM Run from anywhere: opens Windows firewall rules for PC Host (TCP 8787, mDNS).
REM Requires Administrator (right-click -> Run as administrator).
set "SCRIPT=%~dp0scripts\enable-pc-host-mdns-windows.ps1"
if not exist "%SCRIPT%" (
  echo Not found: %SCRIPT%
  echo Clone DeepSeek-Mobile and run this file from the repo root.
  exit /b 1
)
powershell -NoProfile -ExecutionPolicy Bypass -File "%SCRIPT%"
pause
