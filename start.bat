@echo off
REM Double-click QoL wrapper for start.ps1
powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0start.ps1" %*
