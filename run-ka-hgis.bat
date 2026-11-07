@echo off
title ka-hgis
cd /d "%~dp0"
powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0scripts\run-ka-hgis.ps1" %*
if errorlevel 1 (
  echo FAILED exit %ERRORLEVEL%
  pause
)
