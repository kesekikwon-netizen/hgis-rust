@echo off
set PATH=C:\OSGeo4W\apps\Qt6\bin;C:\OSGeo4W\apps\qgis-dev\bin;C:\OSGeo4W\apps\gdal-dev\bin;C:\OSGeo4W\bin;%PATH%
set QT_PLUGIN_PATH=C:\OSGeo4W\apps\Qt6\plugins;C:\OSGeo4W\apps\qgis-dev\qtplugins
cd /d "
C:\Users\??\Documents\hgis
\build\Release"
start "" "
C:\Users\??\Documents\hgis
\build\Release\ka-hgis.exe"
