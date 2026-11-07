# QGIS 3.44 manuals (local) — design reference only

Not product docs. Use for project / layer / edit wiring.

| File | Use for |
| --- | --- |
| `QGIS-3.44-DesktopUserGuide-en.pdf` / `-ko.pdf` | UX lifecycle |
| `QGIS-3.44-PyQGISDeveloperCookbook-en.pdf` / `-ko.pdf` | API: vector layer, edit buffer, addMapLayer |

- User: https://docs.qgis.org/3.44/en/docs/user_manual/
- Cookbook: https://docs.qgis.org/3.44/en/docs/pyqgis_developer_cookbook/vector.html

## Layer rule (developer cookbook)

1. GPKG table on disk ≠ map layer in project.
2. Legend only after `QgsProject::addMapLayer` (ka-hgis: user draw/import → `ensureDomainLayer`).
3. Digitize: `startEditing` → `addFeature` → `commitChanges`.

See repo root `AGENTS.md`.

## Download large Desktop User Guide PDFs

```powershell
# from repo root
.\scripts\download-qgis-manuals.ps1
```

Cookbooks are already in this folder (git). User Guide EN/KO are large and gitignored.
