KA-HGIS submission package
created: 2026-11-13T10:42:15+09:00
timezone: Asia/Seoul
app: Strata 2.0.0 (b036c0905f6a)
crs: EPSG:5179
shp_encoding: UTF-8
survey_name: demo
survey_file: demo.gpkg
survey_sha256: 082674951f6233c5800dedcc4e954faca6fba6d5bece101a69b6b26178a0b74f
saved_state: saved
intranet: upload each domain SHP (feature_poly.shp = one file; merge polygons in-app first if required)

checklist:


shp_field_names:
survey_area survey_name=surv_name
artifact_point artifact_no=artif_no

shp_layers:
survey_area.shp features=1 crs=EPSG:5179 verified
feature_poly.shp features=2 crs=EPSG:5179 verified
feature_line.shp features=1 crs=EPSG:5179 verified
control_points.shp features=2 crs=EPSG:5179 verified
section_line.shp features=1 crs=EPSG:5179 verified
artifact_point.shp features=1 crs=EPSG:5179 verified
trial_trench.shp features=1 crs=EPSG:5179 verified

reference_data (도면에 그린 참조 지도):
- 없음

See MANIFEST.sha256 for file hashes.
도면 PDF는 도면만들기 용지(user_sheet)를 넣습니다.
조사도면.pdf 없음: 도면만들기에서 용지를 만든 뒤 다시 보내기 하세요.
