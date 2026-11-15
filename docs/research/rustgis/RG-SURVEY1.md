# RG-SURVEY1 — RustGIS v0.6.0 Engine Survey for Strata

- **Order**: `RG-SURVEY1`
- **Source studied**: `RustGIS` tag `v0.6.0` (`Apache-2.0`), workspace crates `rustgis-geom`, `rustgis-data`, `rustgis-map`, `rustgis-render`, `rustgis-tiles` (and `Cargo.toml`, `Cargo.lock`, `LICENSE`, `ATTRIBUTION.md`, `THIRD-PARTY-LICENSES.txt`, `deny.toml`, `about.toml`, `xtask/src/notices.rs`), plus locked dependency sources in the local Cargo registry cache (`crs-definitions-0.6.0`, `epaint_default_fonts-0.37.0`, `sysinfo-0.40.1`, etc.).
- **Line citation convention**: Relative paths below are relative to the RustGIS v0.6.0 repository root unless prefixed with `<cargo-registry>/`.

---

## 1. GeoPackage: Named Table Reading, `sqlite.rs`, and EPSG:5186 Coordinate Conversion

### Public functions and types to call

1. **Listing tables in a `.gpkg`**:
   - `rustgis_data::source::layers(path: &Path) -> Result<Vec<String>>` (`crates/data/src/source/mod.rs:533-539`), or directly `rustgis_data::source::gpkg::layers(path: &Path) -> Result<Vec<String>>` (`crates/data/src/source/gpkg.rs:54-57`), which queries `SELECT table_name FROM gpkg_geometry_columns ORDER BY table_name` (`crates/data/src/source/gpkg.rs:70-74`).
2. **Opening one named table and reading into a `Dataset`**:
   - **Do NOT use `rustgis_data::io::read_path(path)` for named tables**: `io::read_path(path: &Path) -> Result<(Dataset, ReadReport)>` (`crates/data/src/io.rs:269-321`) detects `.gpkg` via `source::is_source_name` (`crates/data/src/source/mod.rs:106, 121-125`) and calls `source::materialize(source::open_path(path, None)?.as_ref())` (`crates/data/src/io.rs:316`). Passing `None` always selects the alphabetically first table in `gpkg_geometry_columns` (`crates/data/src/source/gpkg.rs:133-138`). Moreover, `io::is_multi_layer_name` (`crates/data/src/io.rs:62-69`) and `io::read_layers` (`crates/data/src/io.rs:80-147`) only cover DXF, KML/KMZ, GPX, TopoJSON, and FileGDB—not `.gpkg`.
   - **Correct call for a named table (materialised into `Dataset`)**:
     - `let src = rustgis_data::source::open_path(path, Some(table_name))?;` (`crates/data/src/source/mod.rs:329-349`), or directly `rustgis_data::source::gpkg::GpkgSource::open(path, Some(table_name))` (`crates/data/src/source/gpkg.rs:77-82`); for in-memory bytes, `rustgis_data::source::open_bytes(name, bytes, Some(table_name))` / `open_shared` (`crates/data/src/source/mod.rs:352-386`).
     - `let (dataset, report) = rustgis_data::source::materialize(src.as_ref())?;` (`crates/data/src/source/mod.rs:544-569`, returning `(rustgis_data::Dataset, rustgis_data::io::ReadReport)`).
   - **Alternative streamed path (for 300 000 topographic lines)**:
     - `rustgis_map::source::SourceLayer::from_path(path, Some(table_name.into()), info)` -> `rustgis_map::Layer::new_streamed(id, source)` (`crates/map/src/lib.rs:475-493`) or `rustgis_map::Map::add_streamed(name, source)` (`crates/map/src/lib.rs:2398-2405`). `GpkgSource` uses the GeoPackage SQLite R-tree `rtree_<table>_<geom_col>` when present (`crates/data/src/source/gpkg.rs:186-190, 358-392`).

### Is the table chosen by name?

- **Yes, in `source::open_path` / `GpkgSource::open` / `open_bytes`**: `GpkgSource::open_conn` (`crates/data/src/source/gpkg.rs:126-138`) and `GpkgMemSource::open` (`crates/data/src/source/gpkg_mem.rs:94-103`) match `layer: Option<&str>` against `gpkg_geometry_columns.table_name` using `eq_ignore_ascii_case`, returning `DataError::Format("no layer '<name>' in GeoPackage")` if absent.

### Role of `crates/data/src/sqlite.rs`

- `crates/data/src/sqlite.rs` (`sqlite::Db::open`, `tables()`, `table(name)`, `rows()`, `crates/data/src/sqlite.rs:51-344`) is a pure-Rust read-only SQLite B-tree parser used **only on `wasm32`** via `gpkg_mem::GpkgMemSource` (`crates/data/src/source/gpkg_mem.rs:1-4, 88-193`; `crates/data/src/source/mod.rs:379-380`).
- On desktop (`not(target_arch = "wasm32")`), `.gpkg` files and byte buffers are read through `rusqlite` 0.41.1 (`bundled` SQLite, `Cargo.toml:83`, `crates/data/Cargo.toml:67`, `crates/data/src/source/gpkg.rs:1, 59-68`), including `open_shared` which writes a temp file and opens `GpkgSource` so SQLite R-tree indexes remain usable (`crates/data/src/source/mod.rs:367-377`).

### How coordinates are carried from `EPSG:5186` to internal longitude/latitude

- **Internal storage**: `Dataset` stores all vector geometries in **WGS 84 longitude/latitude degrees (`Coord { x: lon, y: lat }`, `f64`)** (`crates/data/src/lib.rs:3, 301-316`), while recording the source CRS in `Dataset::crs: Option<raster::Crs>` (`crates/data/src/lib.rs:315`, set in `materialize` at `crates/data/src/source/mod.rs:564`).
- **GeoPackage SRS lookup**:
  1. `GpkgSource::open_conn` reads `srs_id` from `gpkg_geometry_columns` and queries `SELECT organization, organization_coordsys_id, definition FROM gpkg_spatial_ref_sys WHERE srs_id = ?1` (`crates/data/src/source/gpkg.rs:139-161`).
  2. It passes `(srs_id, org, code, definition)` to `gpkg_reproject` (`crates/data/src/source/mod.rs:807-869`).
  3. When `organization` is `"EPSG"` (case-insensitive) and `organization_coordsys_id == 5186` (or `srs_id == 5186` with empty `organization`), `gpkg_reproject` calls `Crs::from_epsg(5186)` (`crates/data/src/source/mod.rs:824-836`; `crates/data/src/raster/crs.rs:256-265`). If the code lookup fails, it falls back to `Crs::from_wkt(def)` (`crates/data/src/source/mod.rs:838-843`).
- **Projection library and `towgs84` parameters**:
  1. `Crs::from_epsg(5186)` calls `epsg_proj4(5186)` (`crates/data/src/raster/crs.rs:16-22`), which looks up `crs_definitions::from_code(5186)` (`crs-definitions` v0.6.0; `crates/data/src/raster/epsg_gap.rs` has no override for 5186).
  2. In `<cargo-registry>/crs-definitions-0.6.0/src/defs.rs:2836`, `EPSG:5186` (`Korea 2000 / Central Belt 2010`) is defined as:
     `"+proj=tmerc +lat_0=38 +lon_0=127 +k=1 +x_0=200000 +y_0=600000 +ellps=GRS80 +towgs84=0,0,0,0,0,0,0 +units=m +no_defs"`
     (Likewise `EPSG:5187` at `defs.rs:2837` has `+lon_0=129 ... +ellps=GRS80 +towgs84=0,0,0,0,0,0,0`, and `EPSG:5179` at `defs.rs:2829` has `+proj=tmerc +lat_0=38 +lon_0=127.5 +k=0.9996 +x_0=1000000 +y_0=2000000 +ellps=GRS80 +towgs84=0,0,0,0,0,0,0`.)
  3. **Library**: Pure-Rust **`proj4rs` v0.3.0** (`Cargo.toml:78`, `crates/data/src/raster/crs.rs:395-428`), **not** C PROJ. `Crs::transformer()` -> `transformer_of(def)` parses `Proj::from_proj_string(def)` and `Proj::from_proj_string("+proj=longlat +datum=WGS84 +no_defs")` (`crates/data/src/raster/crs.rs:416-427`).
  4. Each GeoPackage StandardGeoPackageBinary WKB blob (`GP` header stripped at `crates/data/src/source/gpkg.rs:248-276` -> `wkb::decode_geom` at `crates/data/src/wkb.rs`) is transformed in place during reading by `Reproject::geometry` (`crates/data/src/source/mod.rs:753-759`) -> `Transformer::to_lonlat` (`crates/data/src/raster/crs.rs:574-587`), calling `proj4rs::transform::transform(&proj, &wgs84, &mut pt)` and converting radians to degrees. Because `+towgs84=0,0,0,0,0,0,0` is zero-shift on `GRS80`, GRS80 (`Korea 2000`) is treated as identical to `WGS84` (`a = 6378137`, sub-millimetre flattening difference only).

### Missing or limited

- **`io::read_path` ignores table names**: Always picks the alphabetically first geometry table (`crates/data/src/io.rs:316`, `crates/data/src/source/gpkg.rs:70-74`).
- **Non-spatial attribute-only GeoPackage tables**: `"not found"` in `GpkgSource` — `GpkgSource::open_conn` only queries `gpkg_geometry_columns` (`crates/data/src/source/gpkg.rs:70-74, 126-138`), so tables without a geometry column in `gpkg_geometry_columns` cannot be opened through `GpkgSource`.
- **Silent pass-through on unknown CRS (violates Strata's strict-CRS rule unless checked by caller)**: If `gpkg_reproject` cannot parse `srs_id` or `definition`, it does **not** return `Err`; it returns `(Reproject::None, Some(note), None)` (`crates/data/src/source/mod.rs:859-868`), leaving raw projected metres in `Dataset` as if they were degrees! Callers must inspect `src.info().crs` / `ReadReport.notes` (`crates/data/src/source/mod.rs:244-256, 560-566`) and reject `crs.is_none()` when the table is not genuinely WGS 84.
- **Legacy Korean Bessel 1985 CRSs (`EPSG:5173`–`5178`)**: In `<cargo-registry>/crs-definitions-0.6.0/src/defs.rs:2823-2828`, `EPSG:5173`–`5178` carry `+ellps=bessel` **without `+towgs84`**, and `national_datum()` in `crates/data/src/raster/wkt.rs:964-988` has no entry for `Korean_Datum_1985` (looked in `crates/data/src/raster/wkt.rs:940-995` and `crates/data/src/raster/epsg_gap.rs`). Only `EPSG:5185`–`5188` and `5179` (Korea 2000 / GRS80) are accurate out of the box.
- **Mandatory WGS 84 round-trip**: Even when the source `.gpkg` is `EPSG:5186` and the map is drawn in `EPSG:5186` or `EPSG:5187`, `GpkgSource` converts every vertex `EPSG:5186 -> WGS84 lon/lat` on read, and `Map` converts `WGS84 lon/lat -> map CRS` on draw.
- **2D only**: Z and M coordinates in GeoPackage WKB are dropped (`crates/data/src/source/gpkg.rs:250-259`, `crates/data/src/lib.rs:321-324`), setting `zm_dropped = true`.
- **Materialise memory budget**: `source::materialize` stops with an error if estimated in-memory size exceeds `MAX_READ_BYTES` (`4 GiB` on 64-bit desktop, `crates/data/src/source/mod.rs:115, 601-637`).

---

## 2. Projected Map: Drawing in `EPSG:5187`, Camera `center` / `res`, and Limits

### Public functions and types to call

- **Setting the map projection**:
  - `let proj = rustgis_map::Projection::from_code("EPSG:5187")?;` (`crates/map/src/globe.rs:58-85`), which constructs `Projection::Custom("EPSG:5187".to_string())` (`crates/map/src/globe.rs:23-32`).
  - `map.set_projection(proj)` (`crates/map/src/lib.rs:2589-2606`), which updates `map.projection` while preserving the camera's geographic centre (`map.center_lonlat()`) and ground resolution (`map.ground_res()`).
- **How `Projection::Custom("EPSG:5187")` projects coordinates**:
  - `Map::flat(&self) -> projection::Flat` (`crates/map/src/lib.rs:2494-2496`) calls `Flat::of(&self.projection)` (`crates/map/src/projection.rs:916-928`), which looks up `"EPSG:5187"` in a 32-entry thread-local LRU cache (`CACHE`, `crates/map/src/projection.rs:895-913`) of `Arc<Projector>`.
  - `Projector::new("EPSG:5187")` (`crates/map/src/projection.rs:596-642`) calls `lookup("EPSG:5187")` (`crates/map/src/projection.rs:340-406`) -> `rustgis_data::raster::epsg_proj4(5187)` (`"+proj=tmerc +lat_0=38 +lon_0=129 +k=1 +x_0=200000 +y_0=600000 +ellps=GRS80 +towgs84=0,0,0,0,0,0,0 +units=m +no_defs"`) and builds `Engine::Proj { proj: proj4rs::Proj, wgs84: proj4rs::Proj }` (`crates/map/src/projection.rs:514-525`).
  - Vector layers cache their projected geometries on `Layer::projected` (`Arc<Vec<Option<Geometry>>>`, keyed by `(geometry_rev, "epsg:5187")`) in `Layer::projected_in(&mut self, flat: &Flat, key: &str)` (`crates/map/src/lib.rs:769-802`).

### What `Camera.center` and `Camera.res` mean in `EPSG:5187`

- Defined in `crates/map/src/camera.rs:14-35` and `crates/map/src/projection.rs:4-6, 635`:
  - **`camera.center: Coord`**: Planar **EPSG:5187 projected metres** (`x` = Easting around `200_000.0` m, `y` = Northing around `600_000.0` m; +x east, +y north). `Projector::to_meter` is `1.0` for `EPSG:5187` (`crates/map/src/projection.rs:635`, since `+units=m`).
  - **`camera.res: f64`**: **EPSG:5187 projected units (metres) per logical UI point (`pt`)** (`crates/map/src/camera.rs:21-22`; `to_screen` / `to_map` at `crates/map/src/camera.rs:91-125` divide/multiply planar coordinate offsets by `self.res`).
  - **Ground resolution and scale denominator**:
    - `map.ground_res()` (`crates/map/src/lib.rs:2515-2522, 2550-2557`) computes `camera.res * map.ground_per_unit(camera.center)`, where `Projector::ground_per_unit` (`crates/map/src/projection.rs:768-779`) measures a finite-difference WGS 84 geodesic step (`rustgis_geom::geodesic_inverse`) across 100 planar units at `p` (nearly `1.0` across Korea's Eastern Belt).
    - `map.scale()` (`crates/map/src/lib.rs:2559-2561`) is `map.ground_res() / PIXEL_M`, with `PIXEL_M = 0.00028` m (OGC 0.28 mm standard pixel, ~90.714 DPI, `crates/geom/src/lib.rs:44`).
  - **Camera API footgun**: Methods directly on `Camera`—`Camera::center_lonlat`, `Camera::ground_res`, `Camera::scale`, `Camera::set_lonlat`, `Camera::fit_lonlat` (`crates/map/src/camera.rs:54-79, 241-257`)—explicitly assume **Web Mercator** (`merc_to_lonlat` / `lonlat_to_merc`). In `EPSG:5187`, callers **must** use the projection-aware `Map` methods instead: `map.center_lonlat()`, `map.set_lonlat()`, `map.ground_res()`, `map.scale()`, `map.set_scale()`, `map.fit_extent()`, `map.fit_lonlat()` (`crates/map/src/lib.rs:2500-2585`). (Note: an internal bug exists in `crates/render/src/lib.rs:2854, 3402`, where tilted-view `clip_distance` calls `view.camera.center_lonlat()` instead of `map.center_lonlat()`; harmless when `pitch == 0.0`.)

### Missing or limited

1. **Vector double-conversion via WGS 84**: Vectors are read into `Dataset` as WGS 84 lon/lat (`proj4rs` inverse) and projected into `EPSG:5187` in `Layer::projected_in` (`proj4rs` forward, `crates/map/src/lib.rs:769-802`). By contrast, rasters already tagged `EPSG:5187` skip reprojection completely (`crates/render/src/raster.rs:38-40`).
2. **Only named/EPSG/ESRI codes accepted**: `projection::lookup` (`crates/map/src/projection.rs:340-406`) accepts built-in names, `AUTO2:*`, `EPSG:<code>`, and `ESRI:<code>`; raw `+proj=...` strings and custom WKT are rejected (`392-406`).
3. **Transverse Mercator domain clipping**: `Domain::of` (`crates/map/src/projection.rs:426-445`) restricts `+proj=tmerc` to `|lon - lon_0| <= 30°` (`99°E..=159°E` for `EPSG:5187`) and `|lat| <= 84°`, and `Projector::forward_checked` (`crates/map/src/projection.rs:726-741`) drops points whose inverse round-trip drifts by `>= 1e-3°`. (Fine for all of Korea, clips global backdrops.)
4. **Zoom clamp**: `Camera::clamp` (`crates/map/src/camera.rs:9-11, 297-303`) clamps `camera.res` to `MIN_RES = 1e-4` m/pt (~1:0.36 scale) .. `MAX_RES = 160_000.0` m/pt.
5. **No lazy Mercator display cache for custom CRS**: `Layer::prepare_display` (`crates/map/src/lib.rs:814-816`) immediately returns on `Flat::Custom`, so the first render of a large materialised layer projects all features upfront (`Layer::projected_in`) before building thinned LOD levels (`Layer::thinned_in`, `crates/map/src/lib.rs:881-922`), and streamed chunk LOD (`crates/render/src/lib.rs:3511-3517`) projects each chunk before simplifying.
6. **Web tiles in `EPSG:5187` warp on the CPU**: Standard `EPSG:3857` XYZ and vector tile layers cannot use GPU tile quads (`LiveTiles`, `TileMeshes`, `Tiles`) in `Flat::Custom` (`crates/render/src/tiles.rs:331, 420-464`); instead, `draw_warped` (`crates/render/src/tiles.rs:486-580`) renders a Web Mercator mosaic up to `MOSAIC = 2048.0` px (with MVT tile labels suppressed: `labels = false` at `576`) and warps it pixel-by-pixel to `EPSG:5187` on the CPU. Only WMS/WMTS layers with `#rustgis-warp=` that advertise `EPSG:5187` natively are fetched directly in `EPSG:5187` without warping (`crates/render/src/native_tiles.rs:43-82`).
7. **No 3D terrain mesh in `EPSG:5187`**: `terrain::Ground::of` (`crates/render/src/terrain.rs:57-61`) returns `None` whenever `map.projection != Projection::WebMercator`.

---

## 3. Fill by Attribute: `Renderer`, `#RRGGBB` Column, Opacity, Outline Colour, and Width Units

### Public functions and types to call

- **`rustgis_map::Renderer`** (`crates/map/src/symbology.rs:723-792`, re-exported at `crates/map/src/lib.rs:59`):
  - `Renderer::Single(Symbol)` (`725`): one `Symbol` for all features.
  - `Renderer::Unique { field, classes: Vec<(String, Symbol)>, other: Symbol, ramp: Ramp, class_labels: Vec<String>, other_label: String, class_strokes: Vec<Vec<LineStroke>>, other_strokes: Vec<LineStroke>, class_fills: Vec<Vec<PolygonFill>>, other_fills: Vec<PolygonFill>, expression: Option<String> }` (`727-755`): categorical styling by exact `field` string value (`Feature.attributes[idx].display()`) or an `expression`. Helpers: `Renderer::unique(ds, field, geom_type, ramp)` (`crates/map/src/symbology.rs:942-969`, auto-collects up to `MAX_UNIQUE = 64` classes at `821`), `Renderer::unique_by_up_to(..., max)` (up to `MAX_EXPRESSION_CLASSES = 200` at `824, 1003`), or construct `Renderer::Unique { classes, .. }` directly with any number of `(String, Symbol)` pairs (`symbol_for` scans `classes` linearly at `crates/map/src/symbology.rs:845-858`).
  - `Renderer::Graduated { field, breaks: Vec<f64>, symbols: Vec<Symbol>, ramp: Ramp, method: ClassMethod, min: f64, classes: usize, class_labels: Vec<String>, class_strokes: Vec<Vec<LineStroke>>, class_fills: Vec<Vec<PolygonFill>> }` (`757-779`).
  - `Renderer::Proportional { field, base: Symbol, min_size: f32, max_size: f32, min: f64, max: f64 }` (`781-791`).
- **`rustgis_map::Symbol`** (`crates/map/src/symbology.rs:105-124`):
  - `pub fill: Rgba`, `pub stroke: Rgba`, `pub stroke_width: f32`, `pub size: f32`, `pub shape: MarkerShape`, `pub dash: DashPattern`, `pub character: Option<char>`, `pub fill_style: FillStyle`, `pub hatch_angle: Option<f32>`, `pub hatch_spacing: Option<f32>`.

### Can a colour come straight from a column holding `'#RRGGBB'`?

- **Yes**, via data-driven **`Layer::symbol_by`** (`crates/map/src/lib.rs:431`, type `rustgis_map::symbol_by::SymbolBy` in `crates/map/src/symbol_by.rs:21-39`):
  - `SymbolBy` has five `Option<String>` expression fields: `size` (`24`), `fill` (`28`), `stroke` (`31`), `stroke_width` (`34`), and `angle` (`38`).
  - Set `layer.symbol_by.fill = Some("\"color_col\"".to_string());` (quoting the column identifier in `rustgis_data::query` syntax).
  - Once per draw, `Layer::styler` -> `Styler::new` (`crates/map/src/symbol_by.rs:156-165`) compiles expressions via `Compiled::new` (`81-95`, calling `rustgis_data::query::parse`), and per feature `Compiled::apply` (`117-136`) calls `Compiled::color` (`101-113`) -> `rustgis_data::color::parse_value` (`crates/data/src/color.rs:9-24`) -> `color::parse` / `parse_hex` (`crates/data/src/color.rs:28-85`).
  - `color::parse_value` accepts `#RGB`, `#RGBA`, `#RRGGBB`, `#RRGGBBAA` (`crates/data/src/color.rs:65-85`), `r,g,b[,a]` (`11-21`), `rgb(...)`, `rgba(...)`, `hsl(...)`, `hsla(...)`, and 148 CSS named colours (`60-62`), caching the most recent string per slot (`seen`, `crates/map/src/symbol_by.rs:75, 103-112`).
  - **Alpha detail with `'#RRGGBB'`**: `parse_hex` initialises `c = [255u8; 4]` (`crates/data/src/color.rs:70`) and `Compiled::apply` (`crates/map/src/symbol_by.rs:129-131`) overwrites `sym.fill = c`, so a 6-digit `'#RRGGBB'` column produces **opaque alpha `255`** (overriding `Symbol.fill`'s alpha). To get translucent polygon fills from a `'#RRGGBB'` column while keeping opaque outlines, either append a hex alpha byte in the expression (e.g. `layer.symbol_by.fill = Some("\"color_col\" || 'AA'".into())` -> `#RRGGBBAA`), or build a `Renderer::Unique` table with `Rgba::new(r, g, b, alpha)` per distinct colour.

### Fill opacity, outline colour, and outline width unit (`mm` vs `px`/`pt`)

- **Fill opacity**:
  1. Per-symbol / per-fill alpha: `Symbol.fill = Rgba::new(r, g, b, a)` (`a: u8` in `0..=255`, `crates/map/src/color.rs:12-26`; `Symbol::default_polygon` uses `a = 170` at `crates/map/src/symbology.rs:147`), or `layer.no_fill = Some(outline_rgba)` (`crates/map/src/lib.rs:355-360`) for hollow polygons.
  2. Per-feature alpha in `symbol_by.fill`: via `#RRGGBBAA` / `rgba(...)` / `r,g,b,a` string expressions (`crates/data/src/color.rs:11-21, 77-81`; dedicated `SymbolBy.opacity` field is `"not found"` in `crates/map/src/symbol_by.rs:21-39`).
  3. Whole-layer opacity: `layer.transparency: f32` (`0.0..=100.0` %, `Layer::opacity()` = `1.0 - transparency / 100.0` at `crates/map/src/lib.rs:334, 1109-1111`, affecting both fill and stroke).
- **Outline colour**: `Symbol.stroke: Rgba` (`crates/map/src/symbology.rs:108`), or stacked `LineStroke.color: Rgba` (`crates/map/src/symbology.rs:335`) in `layer.line_strokes` (`crates/map/src/lib.rs:351`). Setting `Symbol.stroke = Rgba::TRANSPARENT` or `Symbol.stroke_width = 0.0` omits the outline (`crates/render/src/lib.rs:699-702, 756`).
- **Outline width unit (`mm` or `px`)**:
  - **Logical points / UI logical pixels (`pt`)**, **NOT millimetres (`mm`)**: `Symbol.stroke_width: f32` (`crates/map/src/symbology.rs:109-110`, `"Stroke width in logical pixels"`, clamped to `0.0..=20.0` at `2932`) and `LineStroke.width: f32` (`crates/map/src/symbology.rs:336`) are multiplied by `ppp` (`pixels_per_point`) in `stroke_paint` (`crates/render/src/lib.rs:671-697, 706, 775`: `width: (sym.stroke_width * ppp).max(0.5)`). In print/PDF layouts (`crates/render/src/layout.rs:182`), `pixels_per_point = dpi / 96.0`, so `1.0` logical px = `1/96` inch = `0.264583 mm` (`1 mm = 96.0 / 25.4 ≈ 3.7795` logical px).
  - Optional map-scale locking: `layer.reference_scale: Option<f64>` (`crates/map/src/lib.rs:373`, `crates/render/src/lib.rs:3580-3588`) scales widths by `(reference_scale / current_scale).clamp(0.05, 20.0)`.
  - Built-in millimetre (`mm`) stroke unit enum on `Symbol` or `LineStroke`: **`"not found"`** (looked in `crates/map/src/symbology.rs:105-420`, `crates/map/src/symbol_by.rs:1-140`, and `crates/render/src/lib.rs:670-790`). Callers wanting `w_mm` millimetres convert via `w_mm * (96.0 / 25.4)`.

### Missing or limited

- **`symbol_by` forces CPU vector rendering**: `gpu_may_turn` (`crates/render/src/lib.rs:2484`) and `gpu_layer` (`crates/render/src/lib.rs:3696`) require `layer.symbol_by.is_empty()` and `layer.renderer.expression().is_none()`. Layers using `symbol_by.fill` for `#RRGGBB` are always rasterised on the CPU with `tiny_skia` (`DrawCmd::Image` in `GpuScene`), never tessellated into `DrawCmd::Mesh`. (If a finite set of colours exists, building `Renderer::Unique` keeps GPU mesh eligibility.)
- **`symbol_by` generates no automatic legend swatches**: `Legend` reads `Renderer` classes (`crates/render/src/layout.rs:1022-1120`), not `symbol_by` row colours.
- **`Renderer::Unique` linear lookup**: `symbol_for` (`crates/map/src/symbology.rs:845-858`) does a linear `.iter().find(...)` over `classes` per feature.

---

## 4. Labels: `Labeling` / `LabelClass`, Collision Engine, Polygon Placement, Styling, Korean Fonts, and Unrasterised Label Output

### `Labeling` and `LabelClass` types and fields

- **`rustgis_map::Labeling`** (`crates/map/src/symbology.rs:2195-2210`, stored on `Layer.labels` at `crates/map/src/lib.rs:335`):
  - `pub enabled: bool` (`2196`)
  - `pub default: LabelClass` (`2198`, used when `classes.is_empty()`)
  - `pub classes: Vec<LabelClass>` (`2205`, up to `MAX_LABEL_CLASSES = 64` at `2172`)
- **`rustgis_map::LabelClass`** (`crates/map/src/symbology.rs:2242-2337`):
  - Identity & filter: `name: String` (`2245`), `visible: bool` (`2248`), `where_: Option<String>` (`2251`, SQL-like filter expression).
  - Text & typography: `field: Option<String>` (`2253`), `size: f32` (`2254`, default `11.0`), `color: Rgba` (`2255`), `bold: bool` (`2257`), `italic: bool` (`2275`), `font: Option<String>` (`2281`), `spacing: f32` (`2290`, letter spacing `-2.0..=20.0`), `wrap_char: Option<String>` (`2303`), `wrap_length: Option<u32>` (`2307`), `line_align: LabelAlign` (`2310`: `Left`, `Center`, `Right`).
  - Halo, background & callout: `halo: bool` (`2256`), `halo_color: Option<Rgba>` (`2284`, default white), `halo_width: Option<f32>` (`2287`, default `1.5`), `background: Option<LabelBackground>` (`2299`: `Shape { kind, fill, stroke, stroke_width, padding }` with `Rectangle`, `Rounded`, `Pill`, `Circle`), `callout: Option<LabelCallout>` (`2321`: `Simple` or `Balloon`).
  - Scale & 3D distance limits: `min_scale: Option<f64>` (`2260`), `max_scale: Option<f64>` (`2262`), `closest: Option<f64>` (`2266`), `farthest: Option<f64>` (`2269`).
  - Placement & overrides: `placement: LabelPlacement` (`2271`: `Auto`, `Point(PointPos)` [8 compass positions + `Center`], `CurvedLine`, `HorizontalLine`), `other_positions: bool` (`2273`), `offset: [f32; 2]` (`2295`), `repeat: Option<f32>` (`2314`, min distance in pt between repeated labels of same text), `show_all: bool` (`2318`, bypass collision hiding), `moved: BTreeMap<u64, LabelMove>` (`2325`, manual per-feature pin/hide/angle/leader), `by: LabelBy` (`2336`, per-feature expressions).

### Collision-free label placement (`label_places.rs` and callers)

- **Candidate collection**: `render_inner` (`crates/render/src/lib.rs:4017-4495`) iterates layers top-to-bottom (`order.iter().rev()`), evaluates `LabelClass` + `LabelBy` + `moved`, measures glyph boxes via `text::measure_Font` (`crates/render/src/text.rs`), and pushes `Candidate` structs.
- **Line placement helper (`crates/render/src/label_places.rs:1-131`)**: `LineLabel` (`label_places.rs:13-22`), `LineTask::Curved` / `LineTask::Straight` (`label_places.rs:25-39`), and `LineLabel::place(&self, accept)` (`label_places.rs:58-92`) defer expensive curved-polyline glyph layout (`curved_glyphs_from` in `crates/render/src/lib.rs:5028-5135`) until a candidate's turn in the collision pass, trying anchor positions (`[0.5, 0.35, 0.65, 0.2, 0.8]` along the visible clipped polyline, `crates/render/src/lib.rs:4385-4435`) and multi-band tilt fallbacks (`fallbacks`).
- **Collision solver**: `place_labels_with_previews` (`crates/render/src/lib.rs:5372-5553`, called at `2450` and `4507`) inserts accepted labels into a spatial hash **`LabelGrid`** (`CELL = 64.0` pt, `crates/render/src/lib.rs:5140-5350`) using Separating Axis Theorem (SAT) oriented-rectangle / glyph-box intersection (`LabelBox`, `PAD = 2.0` pt padding), point marker obstacles (`point_obstacle`), `other_positions` 8-direction fallback (`point_tries` at `5006-5025`), and `repeat` same-text spacing (`too_close`). Labels with `show_all: true` or manual `LabelMove` bypass collision rejection (`5421-5438`).

### Polygon placement (centroid, inside, or visible part?)

- **Largest-part centroid, falling back to widest horizontal interior span — NOT clipped to the visible viewport**:
  - In `crates/render/src/lib.rs:4054-4076`, polygon label anchors come from `g.label_point()` on the **full projected `Geometry::Polygon`**, after a viewport bounding-box check (`visible(&e)`) and a minimum screen extent filter (`ew.max(eh) < 18.0` pt unless `show_all` is true at `4066`).
  - `Geometry::label_point(&self)` (`crates/data/src/geometry.rs:214-275`) picks the polygon part with the largest `ring_signed_area`, computes its outer-ring area centroid (`ring_centroid`), and if `polygon_contains(rings, centroid)` is true (including hole exclusion!), returns that centroid (`241-244`).
  - If the centroid falls outside the polygon or inside a hole (e.g. C-shaped or donut polygons), `interior_point(rings)` (`crates/data/src/geometry.rs:246-274`) scans `LABEL_GRID_STEPS = 64` horizontal scanlines across the bounding box, intersects all outer + hole edges, and returns the midpoint of the widest interior horizontal span.
  - **Visible-part polygon re-centring**: **`"not found"`** (looked in `crates/render/src/lib.rs:4054-4150` and `crates/data/src/geometry.rs:214-275`; unlike lines, which clip to the view box via `clip_polyline` at `crates/render/src/lib.rs:4377`, partially off-screen polygons keep their whole-polygon `label_point()`, so if that point lies outside `[0..width, 0..height]`, `place_labels_with_previews` drops the label unless `other_positions` / `moved` keeps it inside).

### Halo, bold, size unit, scale limits, and expressions

- **Halo**: `LabelClass.halo: bool`, `halo_color: Option<Rgba>` (`LabelClass::halo_rgba()` defaults to opaque white `Rgba::WHITE` at `crates/map/src/symbology.rs:2362-2364`), `halo_width: Option<f32>` (`LabelClass::halo_px()` defaults to `1.5` pt, clamped to `0.0..=10.0` at `2367-2370, 2948`).
- **Bold (and italic)**: `LabelClass.bold: bool` (`2257`) and `LabelClass.italic: bool` (`2275`). Selects bold/italic font faces when available (`crates/render/src/fonts.rs:146-184`), or synthesises faux bold/oblique on rasterisation (`crates/render/src/text.rs:297-315`).
- **Size unit**: **Points (`pt`)**, clamped to `4.0..=72.0` (`crates/map/src/symbology.rs:2946`), converted to `ab_glyph::PxScale` via `size * 1.33 * ppp` (`crates/render/src/text.rs:153-157, 271, 703`, converting 72 dpi typographic points to 96 dpi logical pixels).
- **Scale limits**: `LabelClass.min_scale: Option<f64>` (most zoomed-out denominator, e.g. `Some(50_000.0)`) and `max_scale: Option<f64>` (most zoomed-in denominator, e.g. `Some(1_000.0)`), checked by `LabelClass::in_scale_range(scale)` (`crates/map/src/symbology.rs:2905-2910`).
- **Expression vs field only**: **Both are supported**. `LabelClass.field: Option<String>` names a single column, while `LabelClass.by: rustgis_map::label_by::LabelBy` (`crates/map/src/label_by.rs:19-34`, compiled via `LabelExprs::new` at `70-83` using `rustgis_data::query::parse`) provides per-feature expressions (`"name" || ' (' || ...`) for **`text`** (`22`), **`size`** (`25`), **`color`** (`28`), and **`angle`** (`33`), and `LabelClass.where_: Option<String>` filters which rows get labelled (`crates/render/src/lib.rs:4160-4215`).

### Fonts used and Korean font (`Malgun Gothic`) support

- **Built-in & system default fonts** (`crates/render/src/fonts.rs:9-66`):
  - Primary UI/map sans on Windows: `C:\Windows\Fonts\segoeui.ttf` (regular) and `C:\Windows\Fonts\seguisb.ttf` (bold) (`crates/render/src/fonts.rs:12, 20`); falls back to embedded `epaint_default_fonts::UBUNTU_LIGHT` (`crates/render/src/fonts.rs:52, 63`) if absent.
  - **Automatic Korean fallback**: When shaping/measuring or drawing a character missing from the primary font, `fonts::fallback(ch)` (`crates/render/src/fonts.rs:222-282`, called from `crates/render/src/text.rs:168-207`) inspects the Unicode block of `ch` (`script_candidates`) and, for Hangul (`\u{AC00}..=\u{D7AF}`, `\u{1100}..=\u{11FF}`, `\u{3130}..=\u{318F}`), loads **`C:\Windows\Fonts\malgun.ttf`** (`Malgun Gothic`) automatically (`crates/render/src/fonts.rs:241-244`)!
- **Explicitly setting `Malgun Gothic`**:
  - **Yes**, via `label_class.font = Some("Malgun Gothic".to_string())` (and on print layouts via `ElementKind::Text { font_name: Some("Malgun Gothic".into()), .. }`, `crates/map/src/layout.rs:978`).
  - `crates/render/src/families.rs:99-147, 185-242` scans `C:\Windows\Fonts` + `%LOCALAPPDATA%\Microsoft\Windows\Fonts` (`272-284`) and resolves `"Malgun Gothic"` (case-insensitive) to `malgun.ttf` / `malgunbd.ttf`.
  - **Two font caveats from `families.rs`**:
    1. **Use English `"Malgun Gothic"`, NOT Korean `"맑은 고딕"`**: `families::name()` (`crates/render/src/families.rs:342-372`) parses the OpenType `name` table (`name_id` 16 or 1) and keeps **only one** string per `name_id`, ranking US English (`lang_id == 0x0409`, rank 3) ahead of Korean (`rank 1`). Thus `malgun.ttf` is indexed only as `"Malgun Gothic"`, not `"맑은 고딕"`.
    2. **Warm up the font index once at startup**: During rendering, `fonts::named_face` calls non-blocking `families::find_ready(family)` (`crates/render/src/families.rs:107-109`), which kicks off a background scan thread on first call and returns `None` until that scan completes. Calling `rustgis_render::families::installed()` (`crates/render/src/families.rs:99-101`) once at startup blocks until the system font table is ready so the very first frame uses `"Malgun Gothic"` + `malgunbd.ttf`.

### Public call returning placed labels WITHOUT rasterising them

- **Yes — that is the default behaviour of `rustgis_render::render`!**
  - `pub fn render(map: &mut Map, view: &View) -> Option<Rendered>` (`crates/render/src/lib.rs:3-6, 267-289, 1620-1622`) rasterises map features into `Rendered.pixmap` **without** burning label text into the pixmap (`crates/render/src/lib.rs:3-6`: *"Labels are laid out and collision-checked here, then returned as `PlacedLabel` values so the caller can paint them crisply through egui's font atlas (or rasterise them into a PNG via [`render_png`])."*).
  - Similarly, `rustgis_render::render_backend(map, view, Backend::Gpu | Backend::GpuLive)` (`crates/render/src/lib.rs:295-323`) returns `RenderOutput::Gpu(GpuScene)` with `scene.labels: Vec<PlacedLabel>` (`crates/render/src/gpu_scene.rs:47`) unrasterised.
  - Each **`rustgis_render::PlacedLabel`** (`crates/render/src/lib.rs:191-233`) gives: `x: f32`, `y: f32` (screen logical pt), `text: String`, `size: f32`, `color: Rgba`, `halo: bool`, `halo_color: Rgba`, `halo_width: f32`, `bold: bool`, `italic: bool`, `font: Option<String>`, `spacing: f32`, `align: LabelAlign`, `anchor: TextAnchor`, `angle: f32` (radians), `glyphs: Vec<PlacedGlyph>` (`ch`, `x`, `y`, `angle` for curved line labels, `crates/render/src/lib.rs:176-184`), `background: Option<PlacedBackground>`, `callout: Option<PlacedCallout>`, `icon: Option<PlacedIcon>`, `layer_id: u32`, `feature_id: u64`.
  - (Only `render_with_labels`, `render_png`, and `layout::render` burn labels into the `Pixmap` via `draw_labels`, `crates/render/src/lib.rs:1632-1645, 5584`.)
  - Note: A standalone public function that computes *only* `Vec<PlacedLabel>` without also running `render` / `render_backend` is **`"not found"`** (`place_labels_with_previews` at `crates/render/src/lib.rs:5372` is private `fn`).

---

## 5. Tiles: Supported Sources, Custom Headers, Caching, Concurrency, and Hex `L%02d/R%08x/C%08x`

### Supported tile sources

- **`rustgis_tiles::TileSource`** (`crates/tiles/src/source.rs:20-53`) & **`rustgis_map::TileLayer`** / **`TileKind`** (`crates/map/src/tiles.rs:17-37, 95-201`):
  1. **XYZ / TMS URL templates (`TileSource::Raster` / `TileKind::Raster`)**: Expanded in `crates/tiles/src/template.rs:144-199` with placeholders `{z}` / `{zoom}`, `{x}`, `{y}`, `{-y}` (or `tms: true` OSGeo TMS flip), `{s}` (a/b/c subdomain), `{switch:a,b,c}`, `{q}` / `{quadkey}` (Bing quadkey), `{width}`, `{height}`, `{r}` (`""` or `"@2x"`), `{ratio}` (`"1"` or `"2"`).
  2. **WMS**:
     - Tiled EPSG:3857 WMS via `{bbox-epsg-3857}` (`crates/tiles/src/template.rs:181-184`), built from `GetCapabilities` by `rustgis_data::ogc::parse` (`crates/data/src/ogc.rs:945-1330`).
     - Non-Mercator WMS via `#rustgis-warp=` fragment (`crates/data/src/ogc.rs:1053-1070`, `crates/data/src/ogc_warp.rs:21-22`), which requests tiles directly in the map's CRS without warping when the WMS advertises that CRS (e.g. `EPSG:5187` / `EPSG:5186`, `crates/render/src/native_tiles.rs:1-66, 226-250`) or warps on the CPU otherwise.
     - Single-image untiled WMS per view via `TileLayer { single_image: true, .. }` (`crates/map/src/tiles.rs:195-200`, `crates/render/src/single_wms.rs`).
  3. **WMTS**:
     - Both RESTful resource URLs (`{TileMatrix}`, `{TileRow}`, `{TileCol}` + dimension placeholders) and KVP `GetTile` URLs parsed from WMTS `GetCapabilities` by `rustgis_data::ogc::parse` (`crates/data/src/ogc.rs:1334-1653`).
     - Supports both GoogleMapsCompatible (`EPSG:3857`) matrix sets and custom/national CRS `TileMatrixSet`s (e.g. Korean `EPSG:5179` / `EPSG:5186` / `EPSG:5187` grids) via `#rustgis-warp=` (`{nm}`, `{nrow}`, `{ncol}`), drawn unwarped when `map.projection` matches the matrix set's EPSG code (`crates/render/src/native_tiles.rs:68-71, 252-283`).
  4. **Other tile sources**: OGC API – Tiles (`crates/data/src/ogc.rs:2774-2924`), ArcGIS MapServer `/tile/{z}/{y}/{x}` & `/export` and ImageServer `/exportImage` (`crates/data/src/ogc.rs:2349-2392`), Mapbox/MapLibre Vector Tiles (`TileSource::Vector`, `.pbf` / TileJSON) & MapLibre styles (`TileSource::Style`, `crates/tiles/src/style.rs`), `PMTiles` (`TileSource::PmTiles` local file or HTTP Range), `MBTiles` (`TileSource::MbTiles` local SQLite), and Cloud-Optimised GeoTIFF (`TileSource::Cog`).

### Custom request headers (`User-Agent`, `Referer`)

- **Built-in HTTP client (`ureq` 3.5.1)**:
  - Hardcodes `const USER_AGENT: &str = "RustGIS/0.6.0 (+https://rustgis.app)";` (`crates/tiles/src/cache.rs:1601, 1886`).
  - Sends **no `Referer` header** (`"not found"` in `crates/tiles/src/cache.rs:1875-1930`).
  - `rustgis_tiles::logins::insert(origin, header, value)` (`crates/tiles/src/logins.rs:26-42, 74-77`) attaches one credential header per origin over HTTPS/loopback, and **`valid_header` (`26-42`) explicitly rejects `"user-agent"` and `"referer"`**!
- **Public hook to supply `User-Agent`, `Referer`, rate-limiting, or custom transport**:
  - **`rustgis_tiles::cache::set_fetcher(f: impl Fn(&str) -> Result<Vec<u8>, String> + Send + Sync + 'static)`** (`crates/tiles/src/cache.rs:152-154`) and **`set_fetcher_omitting`** (`crates/tiles/src/cache.rs:158-160`) are **public**!
  - Installing a custom fetcher replaces `ureq` inside `fetch_bytes` (`crates/tiles/src/cache.rs:1833-1837`) for all tile requests (while still keeping RustGIS's worker pool, deduplication, decoding, and memory cache), letting Strata attach arbitrary `User-Agent` and `Referer` headers (e.g. for Korean VWorld / NGII WMTS) and enforce polite per-host pacing.

### Disk cache or memory only?

- **Memory only**: `crates/tiles/src/cache.rs:88-90, 114-126, 1370-1404` stores decoded RGBA tiles, raw vector `.pbf` bytes, and sprites in a process-wide `Mutex<Cache>` LRU bounded by `BUDGET = 384 * 1024 * 1024` (`384 MiB` on desktop; `160 MiB` on `wasm32`), evicting oldest entries down to 75% (`288 MiB`) when full (`1389-1399`).
- Persistent HTTP tile disk cache in `rustgis-tiles`: **`"not found"`** (looked across `crates/tiles/src/cache.rs`, `crates/tiles/src/lib.rs`, `crates/tiles/src/pmtiles.rs`, `crates/tiles/src/cog.rs`; only `mbtiles.rs` and local `pmtiles.rs` read user-supplied archive files). A disk cache can be implemented transparently inside the `cache::set_fetcher` callback.

### Max parallel requests

- In `crates/tiles/src/cache.rs:1500-1509, 1551-1590` (`mod native`, desktop thread pool):
  - **Global worker threads**: **`WORKERS = 32`** (`2 * PER_HOST`, spawned lazily as `rustgis-tiles-0` .. `rustgis-tiles-31` at `1506, 1612-1620`).
  - **Per-host limit**: **`PER_HOST = 16`** concurrent requests per `host[:port]` (`1503, 1551-1561`).
  - **OpenStreetMap limit**: **`PER_HOST_OSM = 8`** concurrent requests when host is `tile.openstreetmap.org` or `*.tile.openstreetmap.org` (`1505, 1524-1527`).
  - **Pending queue cap**: **`QUEUE = 512`** waiting URLs (`1509`); per-host queues pop **LIFO (newest pan/zoom request first)** (`1564-1566`) and drop the oldest queued tile when total queued jobs reach 512 (`1582-1590`), plus in-flight URL deduplication (`inflight` set at `1576`).
  - **HEAD probes**: Up to **`MAX_HEADS = 8`** concurrent `HEAD` threads (`1667`).
  - (Note: `crates/tiles/src/schedule.rs:21-26` defines matching `PER_HOST = 16`, `PER_HOST_OSM = 8`, `LOCAL = 32` for non-native scheduling.)

### Zero-padded hex row/column (`L%02d/R%08x/C%08x` ArcGIS compact/exploded cache)

- **`"not found"`**:
  - Looked in `crates/tiles/src/template.rs:144-199`, `crates/render/src/native_tiles.rs:121-124`, `crates/data/src/ogc_warp.rs:485`, and `crates/data/src/ogc.rs:2349-2392`.
  - All tile coordinate substitutions format `z`, `x`, `y` / `nrow`, `ncol` strictly with decimal `.to_string()` (plus quadkey base-4 digits in `quadkey(z, x, y)` at `crates/tiles/src/template.rs:202-212`). ArcGIS support in `ogc.rs:2349-2353` only emits REST `/tile/{z}/{y}/{x}`.
  - **Workaround if needed**: Pass a template URL like `https://.../tile/{z}/{y}/{x}.png` and rewrite `{z}/{y}/{x}` into `format!("L{z:02}/R{y:08x}/C{x:08x}.png")` inside `rustgis_tiles::cache::set_fetcher` (or from local disk inside `set_fetcher`).

---

## 6. DXF (`crates/data/src/dxf.rs`): Entities, Korean Text Encoding, and Layer Attribute

### Public functions and types to call

- `rustgis_data::dxf::read(file_name: &str, bytes: &[u8], options: &dxf::Options) -> Result<(Vec<Dataset>, ReadReport)>` (`crates/data/src/dxf.rs:61-65, 1297-1410`), where `dxf::Options { layer: Option<&str> }` (`61-65`) optionally filters by geometry-type bucket (`"points"`, `"lines"`, `"polygons"`, or `"<stem> (lines)"`), returning up to three `Dataset`s (Points, Lines, Polygons).
- `rustgis_data::dxf::layers(file_name: &str, bytes: &[u8]) -> Result<Vec<String>>` (`crates/data/src/dxf.rs:1413-1445`).
- Also dispatched automatically by `rustgis_data::io::read_path`, `io::read_layers` (`crates/data/src/io.rs:64, 94`), and `rustgis_data::source::open_path` (`crates/data/src/source/mod.rs:106`).

### Which entities are read

- Parsed in `parse_entity` (`crates/data/src/dxf.rs:281-306`), `convert` (`481-705`), and `emit` (`1117-1285`), skipping paper-space entities (`group 67 == 1`, `1124-1127`):
  - **Point `Dataset`** (`Geometry::Point`): `POINT`, `TEXT`, `MTEXT`, standalone `ATTRIB`, non-constant `ATTDEF` skipped / constant `ATTDEF` kept (`502-505`), and `DIMENSION` fallback definition point when its anonymous `*D...` block is missing (`1257-1269`).
  - **Line `Dataset`** (`Geometry::Line` / `MultiLine`): `LINE`, `LWPOLYLINE` (including bulge-interpolated arcs; **even closed `LWPOLYLINE`s are emitted as closed `Shape::Line`**, `527-552`), 2D/3D `POLYLINE` + `VERTEX` (`SEQEND`; cubic/quadratic spline-fit vertices preferred, bulge arcs interpolated; polygon meshes `flags & 16 != 0` skipped at `762`), `CIRCLE` (64 segments), `ARC`, `ELLIPSE` (parametric, rotated), `SPLINE` (clamped/unclamped rational B-spline via Cox–de Boor or fit points, `610-632, 858-969`), `HELIX`, `LEADER`, and `MLINE` (`640-655`).
  - **Polygon `Dataset`** (`Geometry::Polygon` / `MultiPolygon`): Polyface mesh `POLYLINE` (`flags & 64 != 0`, faces emitted as polygon rings, `764-815`), `SOLID`, `TRACE`, `3DFACE` (bowtie vertex order swapped for `SOLID`/`TRACE`, `656-677`), and `HATCH` (polyline loops and line/arc/ellipse/spline edge loops, even-odd containment hierarchy for islands/holes, `679-702, 1448-1725`).
  - **Expanded container entities**: `INSERT` (expands `BLOCKS` table definitions recursively up to `MAX_DEPTH = 8` with row/column `MINSERT` arrays, scale, rotation, OCS extrusion normal, `BYBLOCK`/`BYLAYER` colour inheritance, and attached `ATTRIB`s, `1118, 1134-1228`) and `DIMENSION` (expands its `*D...` block if present, `1231-1270`).
  - **Unsupported / skipped**: Binary DXF (`AutoCAD Binary DXF` sentinel rejected with clear error at `1298-1303`), `3DSOLID` / `BODY` / `REGION` (ACIS SAT), `MESH`, `WIPEOUT`, `OLE2FRAME`, `VIEWPORT`, paper-space entities (`code 67 == 1`).

### How text encoding is chosen (CP949 / EUC-KR Korean?)

- **UTF-8 with ISO-8859-1 (Latin-1) byte fallback ONLY — CP949 / EUC-KR is NOT decoded**:
  - `decode(bytes: &[u8]) -> Cow<'_, str>` (`crates/data/src/dxf.rs:78-85`):
    ```rust
    fn decode(bytes: &[u8]) -> Cow<'_, str> {
        match std::str::from_utf8(bytes) {
            Ok(s) => Cow::Borrowed(s),
            Err(_) => Cow::Owned(bytes.iter().map(|&b| char::from(b)).collect()),
        }
    }
    ```
  - AutoCAD `\U+XXXX` hex Unicode escapes inside `TEXT`/`MTEXT` are decoded in `plain_text` (`crates/data/src/dxf.rs:431-443`), while `\M+nXXXX` CIF multibyte escapes are **not** decoded (`"not found"` in `dxf.rs:365-475`).
  - The DXF `HEADER` section (including `$ACADVER` and `$DWGCODEPAGE` such as `ANSI_949` / `dos949`) is completely skipped (`parse_sections` in `crates/data/src/dxf.rs:971-1047` only reads `TABLES`, `BLOCKS`, and `ENTITIES`).
  - **CP949 / EUC-KR Korean DXF support**: **`"not found"`** in `crates/data/src/dxf.rs` (looked throughout `crates/data/src/dxf.rs:1-310, 365-480, 970-1050`). Pre-R2007 Korean AutoCAD DXF files encoded in CP949/EUC-KR will turn into Latin-1 mojibake unless Strata transcodes non-UTF-8 DXF bytes (e.g. via `encoding_rs::EUC_KR`, which `rustgis-data` already depends on at `crates/data/Cargo.toml:18`!) to UTF-8 before passing `bytes` to `rustgis_data::dxf::read`.
  - Also note: DXF files carry no CRS metadata (`Dataset.crs` is `None`), so if a Korean DXF is in `EPSG:5186` or `EPSG:5187` metres, `dxf::read` leaves raw planar coordinates in `Dataset`; Strata must reproject them to WGS 84 lon/lat (or wrap via `Crs::from_epsg(5187)`) before adding to `Map`.

### Are CAD layers kept as an attribute?

- **Yes**: Every emitted DXF `Dataset` (Points, Lines, and Polygons) defines **`layer` (`FieldType::Text`) as field `0`**, alongside `entity` (`Text`), `handle` (`Text`), `color` (`Int`, resolved ACI index), `color_rgb` (`Text`, `#RRGGBB` from ACI or group 420 truecolor!), `linetype` (`Text`), `elevation` (`Float`), `text` (`Text`), and `block` (`Text`) (`crates/data/src/dxf.rs:1085-1105, 1274-1284`). Note that `dxf::layers` (`1413-1445`) lists the 1–3 geometry buckets (`points`/`lines`/`polygons`), not individual CAD layer names—CAD layer names live in the `layer` attribute column (and `color_rgb` works directly with Q3's `layer.symbol_by`!).

---

## 7. Raster: Reading GeoTIFF / DEM, Hillshade Parameters, and Multiply Blending

### Reading GeoTIFF / DEM (`crates/data/src/raster/`)

- **Public functions and types to call**:
  - `rustgis_data::raster::read_path(path: &Path) -> Result<Raster>` (`crates/data/src/raster/mod.rs:1029-1100`) or `read_path_with(path, subdataset, crs_override)` (`1034-1100`); from memory: `rustgis_data::raster::read_bytes(file_name, bytes)` (`1370-1391`) or `rustgis_data::raster::geotiff::read_file(path)` / `geotiff::read(name, bytes)` (`crates/data/src/raster/geotiff.rs:103-178`).
  - Built-in GeoTIFF reader uses pure-Rust `tiff` v0.11.3 (+ `zstd` and `weezl` LZW, `crates/data/Cargo.toml:35-36, 61`), parsing GeoKeys (`ModelTiepointTag`, `ModelPixelScaleTag`, `ModelTransformationTag`, `ProjectedCSTypeGeoKey` e.g. `5186`/`5187`, `GDAL_NODATA`), `Int8`/`UInt8`/`Int16`/`UInt16`/`Int32`/`UInt32`/`Float32`/`Float64` bands, and overviews. (Non-GeoTIFF formats like `.img`/`.hgt`/`.dem`/`.jp2` optionally shell out to `gdal_translate` CLI if installed on PATH—nothing links to `libgdal`, `crates/data/src/raster/gdal.rs:1-8, 67-74`.)
  - Wrap into a map layer:
    - `let mut rl = rustgis_map::raster::RasterLayer::from_path(raster, path_str);` (`crates/map/src/raster.rs:237-264`; or `from_bytes` at `267-294`, or streamed COG `RasterLayer::in_place(overview, address)` at `297-299`).
    - `let layer_id = map.add_raster("DEM Hillshade", rl);` (`crates/map/src/lib.rs:2453-2457`, calling `Layer::new_raster` at `457-464`).

### Drawing a hillshade and its parameters (`azimuth`, `altitude`, `z_factor`)

RustGIS has **two** separate hillshade paths; for Strata's 2D `EPSG:5187` / `EPSG:5186` map, **#1 (`RasterRenderer::ShadedRelief`)** is the one to use:

1. **2D Raster Layer Hillshade (`rustgis_map::raster::RasterRenderer::ShadedRelief`)**:
   - Configure on `RasterLayer.style.renderer` (`crates/map/src/raster.rs:922, 1014-1038`):
     ```rust
     rl.style.renderer = rustgis_map::raster::RasterRenderer::ShadedRelief {
         band: 0,
         ramp: None,      // None = greyscale relief; Some(ramp) = elevation colour-tinted relief
         azimuth: 315.0,  // Sun azimuth in degrees clockwise from north (0=N, 90=E, 315=NW)
         altitude: 45.0,  // Sun elevation angle above horizon in degrees (0..=90)
         z_factor: 1.0,   // Vertical exaggeration multiplier on dz/dx and dz/dy
     };
     ```
   - **Exact Horn 3×3 algorithm**: `pub fn hillshade(z: &[f64; 9], cell_x: f64, cell_y: f64, azimuth: f64, altitude: f64, z_factor: f64) -> u8` (`crates/map/src/raster.rs:1517-1527`), called per pixel during raster rendering in `crates/render/src/raster.rs:680-691`:
     - Computes `dzdx` and `dzdy` with Horn's 3×3 kernel (`((z[2] + 2*z[5] + z[8]) - (z[0] + 2*z[3] + z[6])) / (8 * cell_x)`, etc.), `slope = (z_factor * dzdx.hypot(dzdy)).atan()`, `az = (360.0 - azimuth + 90.0).rem_euclid(360.0).to_radians()`, and `zenith = (90.0 - altitude).to_radians()`, returning `0..=255`.
     - **Ambient floor detail**: In `Colorizer::paint` (`crates/map/src/raster.rs:1504-1510`), `shade` (`0..=255`) is mapped through **`k = 0.25 + 0.75 * (shade as f64 / 255.0)`**, so full shadow renders as `64` (`25%` grey) rather than pure black `0`, and full illumination renders as `255` (`[255, 255, 255, 255]` when `ramp: None`, ideal for `Multiply` blending!).
     - **Fast path in `EPSG:5187`**: In `crates/render/src/raster.rs:38-40`, when `map.projection` is `EPSG:5187` and `raster.crs.epsg() == 5187`, `source()` maps screen coordinates directly to DEM pixels via `r.transform.to_pixel(m)` with **zero CRS reprojection** and parallelises across up to 8 threads (`threads_for` at `crates/render/src/raster.rs:708-713`).
2. **3D Terrain Hillshade (`crates/render/src/terrain.rs`) — NOT usable in `EPSG:5187`**:
   - `terrain::Ground::of` (`crates/render/src/terrain.rs:57-61`) immediately returns `None` when `map.projection != Projection::WebMercator` (or when `pitch == 0.0`), and hardcodes `SUN: [f64; 3] = [-0.62, 0.62, 0.48]` (`crates/render/src/terrain.rs:169-214`, only `map.terrain.exaggeration` is configurable).

### Multiply blending over another layer

- **Yes, supported on both CPU map renders and vector/raster PDF exports**:
  - Set on the `rustgis_map::Layer` wrapping the hillshade `RasterLayer`:
    ```rust
    map.layer_mut(layer_id).unwrap().blend = rustgis_map::effects::Blend::Multiply;
    ```
    (`Layer.blend` at `crates/map/src/lib.rs:342`, `rustgis_map::effects::Blend::Multiply` at `crates/map/src/effects.rs:9-15`, along with `Normal`, `Screen`, `Overlay`, `Darken`, `Lighten`, `ColorBurn`, `ColorDodge`, `HardLight`, `SoftLight`, `Difference`, `Exclusion`).
  - In `render_inner` (`crates/render/src/lib.rs:3428-3491, 4605-4616`), `layer.blend != Blend::Normal` sets `offscreen = true` (`3430`), draws the hillshade into an offscreen `Pixmap::new(w, h)` (`3485-3486`), and composites it over the underlying layers (tiles, coloured polygons, etc.) via `tiny_skia::PixmapPaint { opacity, blend_mode: tiny_skia::BlendMode::Multiply, .. }` (`3447, 4609`).
  - In vector PDF export (`crates/render/src/pdf/content.rs:331-354`), `Blend::Multiply` emits a PDF ExtGState with `/BM /Multiply` (`blend_name` at `crates/render/src/pdf/content.rs:384-399`).
  - **GPU-mode caveat**: In `Backend::Gpu` (`GpuScene`), a raster or blended layer flushes into the CPU `DrawCmd::Image` accumulator (`crates/render/src/lib.rs:3487-3491`). If a vector layer *beneath* the `Blend::Multiply` hillshade was emitted as a separate `DrawCmd::Mesh` command (`3707-3729`), the CPU `Pixmap` beneath the hillshade is transparent at those pixels so `tiny_skia` `Multiply` only multiplies against earlier CPU-drawn layers (e.g. tiles/`symbol_by` layers), not earlier `DrawCmd::Mesh` commands (`gpu_may_turn` at `crates/render/src/lib.rs:2473-2477` already rejects rasters/blends for turned views). Using `rustgis_render::render` (`Backend::Cpu`, default) composites every layer beneath `Blend::Multiply` accurately.

---

## 8. Layout and PDF (`crates/map/src/layout.rs`, `crates/render/src/layout.rs`, `crates/render/src/pdf.rs`)

### Page elements (`rustgis_map::layout::Element` & `ElementKind`)

- Positioned in millimetres (`x`, `y`, `w`, `h` from page top-left, `crates/map/src/layout.rs:1109-1123`). All `ElementKind` variants (`crates/map/src/layout.rs:857-1022`, rendered in `crates/render/src/layout.rs:150-1560`):
  1. **`MapFrame`** (`858-882`): `{ map: usize, center: Option<Coord>, scale: Option<f64>, shape: FrameShape (Rectangle | Circle | Ellipse | Polygon(Vec<[f64; 2]>)), grid: Option<Grid>, extent_of: Option<u32>, extent_color: Option<Rgba>, extent_width: Option<f32>, bearing: f64, pitch: f64 }`.
     - Includes **`Grid`** (`crates/map/src/layout.rs:584-669`), supporting both `GridKind::Graticule` (lon/lat DMS/decimal) and **`GridKind::Measured`** (planar Easting/Northing grid in map CRS such as `EPSG:5187`/`5186` metres, including `MeasuredFormat::Full` and topographic `MeasuredFormat::Short` two-digit km grid numbers, ticks/lines/crosses, Zebra/ticks borders, and rotated edge labels, `crates/render/src/layout.rs:540-960`)!
  2. **`NorthArrow`** (`883-893`): `{ frame: Option<u32>, style: ArrowStyle (Classic | Compass | Triangle | Star), fill: Rgba, outline: Rgba }` (rotates automatically with linked `MapFrame.bearing`).
  3. **`ScaleBar`** (`894-909`): `{ frame: Option<u32>, units: ScaleUnits (Auto | Kilometers | Meters | Miles | Feet), divisions: usize (1..=10), style: ScaleBarStyle (Alternating | Stepped | Line), color: Rgba, fill: Rgba }`.
  4. **`Legend`** (`910-930`): `{ frame: Option<u32>, title: String, all_classes: bool, hide_other: bool, exclude: Vec<u32>, columns: usize (1..=3) }` (renders vector symbols, line strokes, polygon hatches, diagrams, and raster ramps/classes).
  5. **`Colorbar`** (`931-953`): `{ frame, layer, title, orientation (Vertical | Horizontal), ticks (2..=11), tick_side, reverse }` (continuous gradient bar for raster/graduated layers).
  6. **`Text`** (`955-979`): `{ text: String, size: f32 (pt), color: Rgba, bold: bool, italic: bool, align: TextAlign, letter_spacing: f32, font: FontFamily (Sans | Serif), font_name: Option<String> }` — supports dynamic tokens `{page}`, `{pages}`, `{pageName}`, `{layout}`, `{scale}`, `{crs}`, `{date}`, `{field:Col}` (`crates/render/src/layout.rs:474-508`) and custom fonts like `"Malgun Gothic"`.
  7. **`Picture`** (`980-983`): `{ image: String }` (base64-encoded PNG or JPEG, decoded in `crates/render/src/layout.rs:1242-1275`).
  8. **`Table`** (`985-990`): `{ frame: Option<u32>, layer: Option<u32>, rows: usize (1..=500) }` (draws an attribute table header + rows from a map vector layer, `crates/render/src/layout.rs:1278-1370`).
  9. **`Chart`** (`992-999`): `{ frame, layer, field, label }` (horizontal category/value bar chart).
  10. **Graphic primitives**: **`Rectangle`** (`1002-1007`: `fill`, `outline`, `width`, `radius`), **`Ellipse`** (`1009-1014`), **`Line`** (`1016-1021`: `color`, `width`, `dir: LineDir`, `head: ArrowHead`).
  - Also supports **`MapSeries`** (`crates/map/src/layout.rs:1133-1147`, multi-page atlas iterating features of an index layer).

### Page sizes

- Presets in `rustgis_map::layout::PAGES` (`crates/map/src/layout.rs:37-47`):
  - **`A5`** (`148 × 210` mm), **`A4`** (`210 × 297` mm), **`A3`** (`297 × 420` mm), **`A2`** (`420 × 594` mm), **`A1`** (`594 × 841` mm), **`A0`** (`841 × 1189` mm), **`Letter`** (`215.9 × 279.4` mm), **`Legal`** (`215.9 × 355.6` mm), **`Tabloid`** (`279.4 × 431.8` mm).
- Custom sizes & orientation: `Layout.width: f64` and `Layout.height: f64` are arbitrary millimetres clamped to **`20.0..=5000.0` mm** (`Layout::new` at `crates/map/src/layout.rs:1193-1203`, `Layout::clamp` at `1284-1285`; swap `width`/`height` for landscape).

### Vector or raster PDF?

- **Both are implemented** (`%PDF-1.4` writer in `crates/render/src/pdf.rs:1-288`, `crates/render/src/pdf/content.rs:1-400`, `crates/render/src/pdf/fonts.rs:1-276`):
  1. **Pure Raster PDF**:
     - Call `let pm = rustgis_render::layout::render(&layout, &mut maps, dpi, info)?;` (`crates/render/src/layout.rs:55-68`), then `let page = rustgis_render::pdf::Page::new(&pm, layout.width, layout.height);` (`crates/render/src/pdf.rs:60-76`), then `let bytes = rustgis_render::pdf::write(&[page], Some(title));` (`crates/render/src/pdf.rs:163-200`).
     - Embeds one zlib-compressed `/DeviceRGB` `/Image` XObject per page; capped on desktop at `MAX_SIDE = 12_000` px and `MAX_PIXELS = 60_000_000` px (~310 DPI on A3, ~440 DPI on A4; `crates/render/src/layout.rs:29-30`).
  2. **Hybrid Vector PDF (recommended for sharp survey lines, grids, and Korean labels)**:
     - Call `let (drawing, warnings) = rustgis_render::layout::render_drawing(&layout, &mut maps, dpi, info)?;` (`crates/render/src/layout.rs:76-87`), then `let page = rustgis_render::pdf::Page::drawing(drawing, layout.width, layout.height);` (`crates/render/src/pdf.rs:80-88`), then `rustgis_render::pdf::write(&[page], Some(title))` (`crates/render/src/pdf.rs:163-200`).
     - **Kept as true PDF vectors**: Flat north-up (`bearing == 0 && pitch == 0`) `MapFrame` vector layers (polygons, 300k topo lines, hatches, markers), clip paths (including circle/ellipse/polygon map frames), measured/graticule grids, north arrows, scale bars, legends, colorbars, tables, charts, shapes, and PDF blend modes (`/BM /Multiply` etc., `crates/render/src/layout.rs:70-75, 201-253`; `crates/render/src/pdf/content.rs:21-360`).
     - **Text in Vector PDF**: Emitted as **embedded PDF Type 3 outline fonts** built from `ab_glyph` glyph outlines (up to 256 glyphs per font chunk) with `/ToUnicode` UTF-16BE CMaps (`crates/render/src/pdf/fonts.rs:1-3, 88-226`), so Korean Hangul and ASCII labels stay infinitely sharp at any zoom and remain text-selectable/searchable without embedding multi-megabyte `.ttf` files.
     - **Embedded as raster XObjects inside the vector PDF**: Rasters/DEM hillshade, web tiles, `Picture` elements, masked vector layers, tilted/rotated/globe map frames, or map frames exceeding the recording cap (`vector::MAX_COST = 256 MiB` or `vector::MAX_ITEMS = 20_000_000` at `crates/render/src/vector.rs:13-18`; falls back to a `dpi` bitmap for that frame and records a `(frame_id, reason)` warning in `warnings`, `crates/render/src/layout.rs:210-220`).

---

## 9. Embedding: Standalone Engine Crates, `Rendered` & `gpu_scene`, `egui`/`wgpu` Versions, and MSRV

### Can `data + map + render + tiles` (with `geom`) be used WITHOUT `ui-egui` and `engine`?

- **Yes**: In `Cargo.lock` and the crate `Cargo.toml` manifests, the dependency DAG is strictly one-way and pulls in neither `rustgis-ui-egui` nor `rustgis-engine`:
  - `rustgis-geom` (`crates/geom/Cargo.toml:11-13`) -> `geographiclib-rs`, `serde`.
  - `rustgis-data` (`crates/data/Cargo.toml:11-73`) -> `rustgis-geom` + file/DB/CRS crates.
  - `rustgis-map` (`crates/map/Cargo.toml:11-25`) -> `rustgis-geom`, `rustgis-data`, `proj4rs`, `serde`, `roxmltree`, `web-time`, `base64`.
  - `rustgis-tiles` (`crates/tiles/Cargo.toml:11-39`) -> `rustgis-geom`, `rustgis-data`, `tiny-skia`, `image`, `ureq`, etc.
  - `rustgis-render` (`crates/render/Cargo.toml:11-28`) -> `rustgis-geom`, `rustgis-data`, `rustgis-map`, `rustgis-tiles`, `tiny-skia`, `lyon`, `ab_glyph`, `epaint_default_fonts`, etc.
- **Critical offline-build requirement (`crates/render/build.rs:9-56`)**:
  - `rustgis-render`'s `build.rs` checks `env::var_os("RUSTGIS_DATA_DIR")` (`crates/render/build.rs:26`). If unset, **`build.rs` invokes `curl -fsSL https://assets.rustgis.app/data/<name>` at compile time** (`34-54`) to download 5 Natural Earth GeoJSON files (`countries.geojson`, `countries_chn.geojson`, `cities.geojson`, `rivers.geojson`, `lakes.geojson`, embedded via `include_bytes!` in `crates/render/src/sample_data.rs`).
  - For Strata's offline/hermetic builds, **`RUSTGIS_DATA_DIR` MUST be set** to a tracked local directory containing those 5 `.geojson` files (or empty valid GeoJSON stubs if `Map.basemap` is set to `Basemap::None`).
- **Heavy un-gated transitive dependencies in `rustgis-data`**: `crates/data/Cargo.toml:1-73` defines **no `[features]`** to slim down optional formats, so linking `rustgis-data` unconditionally compiles `tokio`, `postgres`/`tokio-postgres`, `mysql`, `tiberius` (SQL Server), `parquet`/`arrow-*`, `calamine`, `laz`, `flatgeobuf`, `ureq`, `rustls`/`aws-lc-sys`/`ring`, `rusqlite` (`bundled` C SQLite), `zstd-sys` (C zstd), and `sysinfo` (435 total packages in the 5-crate closure in `Cargo.lock`).

### What `rustgis_render::render` returns (`Rendered`)

- `rustgis_render::render(map: &mut Map, view: &View) -> Option<Rendered>` (`crates/render/src/lib.rs:267-289, 1620-1622`) returns **`rustgis_render::Rendered`**:
  - `pub pixmap: tiny_skia::Pixmap` (`268`, premultiplied RGBA8 pixel buffer, `view.width * view.pixels_per_point` × `view.height * view.pixels_per_point`)
  - `pub labels: Vec<PlacedLabel>` (`269`, collision-checked placed labels, NOT drawn into `pixmap`)
  - `pub unplaced: Vec<PlacedLabel>` (`272`, rejected candidates when `map.label_display.show_unplaced` is true)
  - `pub attribution: Vec<String>` (`274`, deduplicated active layer/tile attributions)
  - `pub tiles_pending: bool` (`276`, true while async web tiles or in-place COG windows are still fetching -> schedule repaint)
  - `pub data_pending: bool` (`279`, true while streamed `SourceLayer`s or async rasters are still reading -> schedule repaint)
  - `pub tile_errors: Vec<String>` (`281`)
  - `pub partial: Vec<u32>` (`284`, raster layer IDs drawn from a coarser overview than needed)
  - `pub pending_in_place: Vec<u32>` (`287`, in-place raster layer IDs still awaiting tiles)
  - `pub(crate) recorded: Option<Canvas>` (`288`, private vector recording used by `layout::render_drawing`)

### What `gpu_scene` offers (`crates/render/src/gpu_scene.rs`)

- Calling `rustgis_render::render_backend(map, view, Backend::Gpu | Backend::GpuLive) -> Option<RenderOutput>` (`crates/render/src/lib.rs:295-323, 1656-1671`) returns `RenderOutput::Gpu(GpuScene)` (`crates/render/src/gpu_scene.rs:43-58`):
  - `GpuScene` is a **wgpu/egui-version-independent scene graph** containing `clear: [f32; 4]`, `cmds: Vec<DrawCmd>`, `clips: Vec<CmdClip>`, `tilt: Option<SceneTilt>`, plus the exact same `labels`, `unplaced`, `attribution`, `tiles_pending`, `data_pending`, `tile_errors`, `partial`, and `pending_in_place` fields as `Rendered`.
  - **`DrawCmd` variants** (`crates/render/src/gpu_scene.rs:103-127`):
    - `Image { pixmap: Pixmap }`: CPU-rendered `tiny_skia` layer group (basemap, DEM/hillshade rasters, warped custom-CRS tiles, `Blend != Normal` layers, transparent/masked layers, `symbol_by` layers).
    - `Mesh { origin: [f64; 2], vertices: Vec<MeshVertex>, indices: Vec<u32> }`: `lyon`-tessellated polygon fills and screen-width miter/round/dashed line strokes (including 300k topographic lines with `Single`/`Unique`/`Graduated` renderers, up to `MAX_SCENE_VERTICES = 8_000_000` at `crates/render/src/mesh.rs`), using RTE-style `f64` `origin` + `f32` `pos` (`[f32; 2]`) + `normal` (`[f32; 2]`, multiplied by `1/zoom` in vertex shader) + `color` (`[u8; 4]`).
    - `Points { origin: [f64; 2], instances: Vec<PointInstance> }`: instanced SDF circle/square/triangle/diamond/cross/star point markers.
    - `Extrusion { origin, vertices, indices, shadows }`: 3D extruded polygons.
    - `Tiles`, `LiveTiles`, `TileMeshes`: GPU Mercator tile paths (unused when `map.projection` is `EPSG:5187`, `crates/render/src/tiles.rs:331, 420-464`).
  - **What is NOT in `gpu_scene`**: `rustgis-render` contains **no WGSL shaders and no `wgpu` pipelines** (those live inside `crates/ui-egui/src/mapgpu.rs` targeting `wgpu 30` + `egui-wgpu 0.37`). In `egui 0.31`, Strata can either:
    1. Call `rustgis_render::render` (`Backend::Cpu`), upload `Rendered.pixmap` to an `egui::TextureHandle`, and paint `Rendered.labels` via `egui::Painter` (simplest, supports `symbol_by` + `Blend::Multiply` hillshade + `EPSG:5187` tiles with zero shader code); or
    2. Write small custom `egui_wgpu` 0.31 pipelines for `DrawCmd::Image` + `DrawCmd::Mesh` + `DrawCmd::Points` if 300k-line interactive panning needs GPU tessellation.

### Which `egui` / `epaint` / `wgpu` versions the 5 engine crates pull in

- Verified from `Cargo.toml:66-112`, `crates/*/Cargo.toml`, and the 435-crate reachability closure in `Cargo.lock`:
  - **`egui`**: **None** (`egui = "0.37.0"` is only in `rustgis-ui-egui` and `apps/rustgis`, `Cargo.toml:106`, `crates/ui-egui/Cargo.toml:16`).
  - **`epaint`**: **None** (`epaint = "0.37.0"` is only transitive under `rustgis-ui-egui`).
  - **`egui-wgpu` / `wgpu`**: **None** (`egui-wgpu = "0.37.0"` and `wgpu = "30.0.1"` are only under `rustgis-ui-egui` / `apps/rustgis`, `Cargo.toml:108-110`).
  - **`epaint_default_fonts`**: **`0.37.0`** (`Cargo.toml:72`, `crates/render/Cargo.toml:20`, `Cargo.lock`). `<cargo-registry>/epaint_default_fonts-0.37.0/Cargo.toml` has **zero dependencies** (`[dependencies]` is empty; it only exposes `include_bytes!` byte slices like `UBUNTU_LIGHT`), so Cargo links `epaint_default_fonts 0.37.0` alongside Strata's `egui 0.31` / `epaint 0.31` with **zero type or crate conflicts**.

### Minimum `rustc` version (MSRV)

- **Workspace manifest**: `Cargo.toml:8, 10` sets **`edition = "2024"`** (requires `rustc >= 1.85.0`) and **`rust-version = "1.90"`**.
- **Actual transitive MSRV in `Cargo.lock` is `rustc 1.95`** (exceeding Strata's `Rust 1.91.1`):
  - Across all crates in the `geom + data + map + render + tiles` closure of `Cargo.lock` present in `<cargo-registry>` (399 crates inspected; 36 platform/arrow/wasm crates not unpacked in the local cache), **only two crates declare `rust-version > 1.90`**:
    1. **`epaint_default_fonts 0.37.0`** (`<cargo-registry>/epaint_default_fonts-0.37.0/Cargo.toml:4`): **`rust-version = "1.95"`** (pulled in directly by `crates/render/Cargo.toml:20`, used at `crates/render/src/fonts.rs:52, 63` solely for `UBUNTU_LIGHT`).
    2. **`sysinfo 0.40.1`** (`<cargo-registry>/sysinfo-0.40.1/Cargo.toml:5`): **`rust-version = "1.95"`** (pulled in directly by `crates/data/Cargo.toml:69` on desktop, used to size available RAM budgets).
  - Consequently, linking unmodified RustGIS v0.6.0 with its locked crate versions requires either upgrading Strata's toolchain from `1.91.1` to **`rustc >= 1.95.0`**, or pinning/patching `epaint_default_fonts` (e.g. `0.31`, already used by Strata!) and `sysinfo` (e.g. `<= 0.37`) in a vendor/fork manifest.

---

## 10. Notices and Licences (`LICENSE`, `ATTRIBUTION.md`, `THIRD-PARTY-LICENSES.txt`, and Permissiveness Check)

### Licence and attribution files that must travel with a program linking `geom + data + map + render + tiles`

1. **RustGIS's own licence (`LICENSE:1-191`)**:
   - **`Apache-2.0`** (`Copyright 2026 the RustGIS contributors`, `Cargo.toml:11`, `LICENSE:189`). Under Apache-2.0 §4(a), a binary distribution must include a copy of `LICENSE` (`NOTICE` file in RustGIS root: `"not found"`).
2. **Third-party Cargo crate notices (`THIRD-PARTY-LICENSES.txt:1-11875`, generated by `xtask/src/notices.rs:28-54` + `about.toml:1-69`)**:
   - Carries the licence texts and copyright notices of all linked Cargo crates (`MIT`, `Apache-2.0`, `BSD-2-Clause`, `BSD-3-Clause`, `ISC`, `Zlib`, `0BSD`, `BSL-1.0`, `CC0-1.0`, `CDLA-Permissive-2.0`, `Unicode-3.0`, `OFL-1.1`, `Ubuntu-font-1.0`), plus upstream `NOTICE` files (`THIRD-PARTY-LICENSES.txt:11450-11550`). (Note: the pre-generated `THIRD-PARTY-LICENSES.txt` in RustGIS also lists UI-only crates like `ash`/`wayland-*`; Strata's `THIRD_PARTY_NOTICES.md` should include all crates actually linked by `geom + data + map + render + tiles`.)
   - **Native C / assembly libraries compiled into the binary via `-sys` crates** (per Strata's `AGENTS.md` §Copyright rule to inspect compiled native code):
     - `libsqlite3-sys 0.39.1` (`rusqlite 0.41.1` `bundled`): crate wrapper is `MIT` (`THIRD-PARTY-LICENSES.txt:9797-9820`); bundled C SQLite3 amalgamation is **Public Domain**.
     - `zstd-sys 2.1.2+zstd.1.5.7` (`tiff` / `parquet`): C `libzstd` is upstream dual `BSD-3-Clause OR GPL-2.0`, elected and packaged under **`BSD-3-Clause`** (`THIRD-PARTY-LICENSES.txt:8178-8284`).
     - `aws-lc-sys 0.46.0` & `ring 0.17.14` (`rustls` / `ureq`): C/asm BoringSSL/AWS-LC crypto under **`ISC AND Apache-2.0 AND MIT AND BSD-3-Clause`** (includes OpenSSL/SSLeay-derived files re-licensed by BoringSSL/AWS-LC; full notices in `THIRD-PARTY-LICENSES.txt:5355-5630, 7965-7980, 8527-9068, 9168-9194`).
     - `proj4rs 0.3.0`: pure Rust (`MIT` / `Apache-2.0`; optional C geodesic code is behind disabled feature).
3. **Adapted code, colour tables, style specs, and geodetic parameter notices (`ATTRIBUTION.md:1-91`, `third-party/*`, appended at `THIRD-PARTY-LICENSES.txt:11550-11875` by `xtask/src/notices.rs:56-130`)**:
   - **`rustgis-geom`**: `crates/geom/src/sinusoidal.rs` (from OSGeo PROJ) -> `third-party/PROJ-COPYING.txt` (`MIT`).
   - **`rustgis-data`**:
     - `crates/data/src/raster/epsg_gap.rs`, `epsg_axes.rs`, and `crs-definitions`: **IOGP EPSG Geodetic Parameter Dataset ownership notice** (`THIRD-PARTY-LICENSES.txt:11855-11875`, `xtask/src/notices.rs:119-128`, linking `https://epsg.org/terms-of-use.html`).
     - `crates/data/src/raster/esri.rs` (Esri Projection Engine tables) -> `third-party/projection-engine-db-doc-README-licensing.txt` (`Apache-2.0` + `PROJ MIT`).
     - `crates/data/src/esripbf.rs` -> `third-party/terrio-LICENSE.txt` (`MIT` + Esri `arcgis-pbf` `Apache-2.0`).
   - **`rustgis-map`**: `crates/map/src/color/scientific.rs` & `matplotlib.rs` -> `third-party/matplotlib-LICENSE.txt` (`PSF/BSD-style`), `matplotlib-LICENSE_COLORBREWER.txt` (`Apache-2.0`, Cynthia Brewer), `matplotlib-LICENSE_YORICK.txt`, `rio-tiler-LICENSE.txt` (`BSD-3-Clause`), `cmcrameri-LICENSE.txt` (`MIT`, Fabio Crameri), `cmocean-LICENSE.txt` (`MIT`, Kristen Thyng).
   - **`rustgis-tiles`**: `crates/tiles/src/symbols.rs` -> `third-party/polylabel-LICENSE.txt` (`ISC`, Mapbox); `crates/tiles/src/style.rs` -> `third-party/maplibre-style-spec-LICENSE.txt` (`BSD-3-Clause`, MapLibre/Mapbox + `glfx.js` `MIT` + `d3-color` `BSD-3-Clause`).
   - **`rustgis-render`**: Natural Earth GeoJSON embedded by `crates/render/build.rs` (`ATTRIBUTION.md:19`, **Public Domain**).

### Any dependency that is NOT permissive (`GPL`, `LGPL`, `MPL`, `CC-BY-NC`)?

- **Checked `deny.toml:10-27`, `about.toml:2-19`, `THIRD-PARTY-LICENSES.txt`, and all `Cargo.toml` licence expressions in the 435-crate `geom + data + map + render + tiles` closure**:
  - **`GPL` / `AGPL`**: **None** (`zstd-sys` is dual BSD-3-Clause/GPL-2.0 and used under `BSD-3-Clause`).
  - **`LGPL`**: **None required** (`r-efi 5.3.0 / 6.0.0`, UEFI target-only under `getrandom`, is triple-licensed `MIT OR Apache-2.0 OR LGPL-2.1-or-later`, so elected permissively under `MIT` or `Apache-2.0`).
  - **`MPL`**: **None** (`webpki-roots 1.0.11` uses **`CDLA-Permissive-2.0`**, `THIRD-PARTY-LICENSES.txt:8333-8399`, not MPL-2.0; `option-ext` is not in the dependency tree).
  - **`CC-BY-NC` / non-commercial**: **None** — **with one data-terms caveat to verify**:
    1. **IOGP EPSG Geodetic Parameter Dataset Terms of Use** (`ATTRIBUTION.md:20, 74-76`, `xtask/src/notices.rs:119-128`, affecting `crs-definitions` and `crates/data/src/raster/epsg_gap.rs` / `epsg_axes.rs`): EPSG data may be distributed at no charge in commercial/non-commercial software provided ownership of IOGP is acknowledged and **data are not distributed for profit as a standalone dataset** (`https://epsg.org/terms-of-use.html`). (Strata already uses PROJ's EPSG database under the same IOGP terms.)
    2. **Font licences in `epaint_default_fonts 0.37.0`** (`crates/render/Cargo.toml:20`, `(MIT OR Apache-2.0) AND OFL-1.1 AND Ubuntu-font-1.0`, `THIRD-PARTY-LICENSES.txt:10943-11141`): **`SIL Open Font License 1.1`** and **`Ubuntu Font Licence 1.0`** are free/open font licences (allowed by RustGIS `deny.toml:22-24` and already present in Strata via `egui 0.31`'s own `epaint_default_fonts`), carrying font-renaming conditions if the font files themselves are modified rather than embedded unmodified.
