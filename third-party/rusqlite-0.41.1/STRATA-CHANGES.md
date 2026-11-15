# rusqlite 0.41.1 as Strata builds it

This folder is rusqlite 0.41.1 from crates.io (MIT, see LICENSE) with four small changes, each marked
"Strata:" where it is made. They let it build on libsqlite3-sys 0.30, the SQLite binding the rest of the
program uses: GDAL's build crate (gdal-src 0.3.0) takes libsqlite3-sys below 0.36 only, the RustGIS engine
needs rusqlite 0.41, and one program can link one copy of SQLite.

1. Cargo.toml: the requirement on libsqlite3-sys is ">=0.30.1, <0.40" instead of "0.39.0" (both targets).
2. Cargo.toml: the features column_metadata and wasm32-wasi-vfs no longer name features of
   libsqlite3-sys that 0.30 does not have.
3. src/error.rs: set_errmsg does nothing (sqlite3_set_errmsg exists from SQLite 3.51).
4. src/lib.rs: FileControl::BlockOnConnect uses the number 44 instead of the missing constant.

Remove this folder and the [patch.crates-io] entry of the workspace Cargo.toml when gdal-src accepts
libsqlite3-sys 0.39 or newer.
