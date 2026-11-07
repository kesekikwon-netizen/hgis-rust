@echo off
setlocal EnableExtensions
set "HERE=%~dp0"

REM Next to ka-hgis.exe (build\Release): set OSGeo PATH then start.
REM Repo root: detached VBS so the console can close.
if exist "%HERE%ka-hgis.exe" goto :runexe
if exist "%HERE%scripts\start-ka-hgis.vbs" goto :runvbs
echo [유적 HGIS] ka-hgis.exe 를 찾지 못했습니다.
pause
exit /b 1

:runvbs
start "" wscript.exe //nologo "%HERE%scripts\start-ka-hgis.vbs"
exit /b 0

:runexe
set "OSGEO="
if defined OSGEO4W_ROOT if exist "%OSGEO4W_ROOT%\apps\qgis-dev\bin\qgis_core.dll" set "OSGEO=%OSGEO4W_ROOT%"
if not defined OSGEO if exist "C:\OSGeo4W\apps\qgis-dev\bin\qgis_core.dll" set "OSGEO=C:\OSGeo4W"
if not defined OSGEO if exist "D:\OSGeo4W\apps\qgis-dev\bin\qgis_core.dll" set "OSGEO=D:\OSGeo4W"
if not defined OSGEO if exist "A:\OSGeo4W\apps\qgis-dev\bin\qgis_core.dll" set "OSGEO=A:\OSGeo4W"
if not defined OSGEO (
  echo [유적 HGIS] OSGeo4W qgis-dev 가 없습니다. OSGEO4W_ROOT 또는 C:\OSGeo4W, D:\OSGeo4W, A:\OSGeo4W 를 확인하세요.
  pause
  exit /b 2
)
set "QGIS=%OSGEO%\apps\qgis-dev"
set "PATH=%QGIS%\bin;%OSGEO%\apps\Qt6\bin;%OSGEO%\apps\gdal-dev\bin;%OSGEO%\apps\pdal-dev\bin;%OSGEO%\bin;%PATH%"
set "OSGEO4W_ROOT=%OSGEO%"
set "QGIS_PREFIX_PATH=%QGIS%"
set "QT_PLUGIN_PATH=%OSGEO%\apps\Qt6\plugins;%QGIS%\qtplugins"
set "QGIS_PLUGIN_PATH=%QGIS%\plugins"
if exist "%OSGEO%\apps\gdal-dev\share\gdal" set "GDAL_DATA=%OSGEO%\apps\gdal-dev\share\gdal"
if exist "%OSGEO%\share\proj" set "PROJ_DATA=%OSGEO%\share\proj"
if exist "%OSGEO%\share\proj" set "PROJ_LIB=%OSGEO%\share\proj"
start "" /D "%HERE%" "%HERE%ka-hgis.exe"
exit /b 0
