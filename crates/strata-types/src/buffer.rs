//! Saved distance geometry, target/cpp-ref/src/core/BufferAnalysis.cpp:233-291.
use crate::crs::WorkCrs;

/// Coordinates carry their own CRS, independently of the current view (target/cpp-ref/src/core/BufferAnalysis.cpp:233-234).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct DistanceRing {
    pub crs: WorkCrs,
    pub lines: Vec<Vec<[f64; 2]>>,
    pub labels: Vec<[f64; 2]>,
}
