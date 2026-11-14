//! Topographic sheets after they were received: unpacking, reading, placing and keeping them, as the C++
//! does in `TopographicArchive`, `TopographicCatalog`, `TopographicSourceCrs` and `TopographicShapefileCache`
//! (`target/cpp-ref/src/core/`). The rules, limits and words are the C++ ones; the code is this crate's own.

#![forbid(unsafe_code)]

pub mod archive;
