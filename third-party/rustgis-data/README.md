# Sample data the RustGIS drawing crate embeds when it is built

Five Natural Earth 1:110m files (public domain, https://www.naturalearthdata.com/about/terms-of-use/), as
published by the RustGIS project at https://assets.rustgis.app/data (copied 2026-11-15, RustGIS v0.6.0).
`rustgis-render`'s build script would download them at every clean build; `.cargo/config.toml` points
`RUSTGIS_DATA_DIR` here instead, so that a build contacts no site. Strata does not show this data.
