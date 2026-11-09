//! 유물 DBSCAN 군집·재질 동반출토, 폐합 트래버스 조정.
//!
//! C++ 대응은 각 항목 주석의 원본 함수명을 따른다.

use geo::{Area, Coord, LineString, Polygon};

/// 유물 클러스터링 기본 반경 (m, C++ `HGIS_DBSCAN_DEFAULT_EPSILON_M`)
pub const HGIS_DBSCAN_DEFAULT_EPSILON_M: f64 = 1.5;
/// DBSCAN 코어 점 최소 이웃 수 (C++ `HGIS_DBSCAN_DEFAULT_MIN_PTS`)
pub const HGIS_DBSCAN_DEFAULT_MIN_PTS: usize = 4;
/// 3급 도근점 최소 요구 폐합비 분모 (1/3000, C++ `HGIS_TRAVERSE_3RD_ORDER_RATIO`)
pub const HGIS_TRAVERSE_3RD_ORDER_RATIO: i32 = 3000;
/// 3급 도근점 각오차 허용 한계 계수 (초, C++ `HGIS_ANGULAR_SEC_FACTOR_3RD`)
pub const HGIS_ANGULAR_SEC_FACTOR_3RD: f64 = 30.0;

/// DBSCAN 유물 밀집 군집 결과 (C++ `ArtifactClusterResult`)
#[derive(Debug, Clone, PartialEq)]
pub struct ArtifactClusterResult {
    pub cluster_id: i32,
    pub member_fids: Vec<i64>,
    pub centroid: [f64; 2],
    pub convex_hull_area_m2: f64,
    pub material_counts: std::collections::BTreeMap<String, usize>,
}

/// 유구별 공반 유물 재질 대칭 빈도 행렬 (C++ `CoOccurrenceMatrixResult`)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoOccurrenceMatrixResult {
    pub material_labels: Vec<String>,
    pub matrix: Vec<Vec<usize>>,
    pub total_per_feature: Vec<usize>,
}

/// 출토 유물 점군에 대한 DBSCAN 공간 군집화 수행 (유효 클러스터 목록과 노이즈 점 개수 반환)
/// (C++ `KaArtifactClusterAnalyzer::computeDbscanClusters` 1:1 대응)
#[must_use]
pub fn compute_dbscan_artifact_clusters(
    artifacts: &[(i64, [f64; 2], &str)],
    epsilon_m: f64,
    min_pts: usize,
) -> (Vec<ArtifactClusterResult>, usize) {
    use geo::ConvexHull;

    let n = artifacts.len();
    if n == 0 || min_pts == 0 || n < min_pts {
        return (Vec::new(), n);
    }
    let eps = epsilon_m.max(0.0);
    // -2: unvisited, -1: noise, >= 0: cluster_id
    let mut labels = vec![-2_i32; n];
    let mut cluster_id = 0_i32;

    let region_query = |idx: usize| -> Vec<usize> {
        let p = artifacts[idx].1;
        (0..n)
            .filter(|&j| {
                let q = artifacts[j].1;
                (p[0] - q[0]).hypot(p[1] - q[1]) <= eps
            })
            .collect()
    };

    for i in 0..n {
        if labels[i] != -2 {
            continue;
        }
        let neighbors = region_query(i);
        if neighbors.len() < min_pts {
            labels[i] = -1;
            continue;
        }
        labels[i] = cluster_id;
        let mut queue: std::collections::VecDeque<usize> =
            neighbors.into_iter().filter(|&j| j != i).collect();
        while let Some(curr) = queue.pop_front() {
            if labels[curr] == -1 {
                labels[curr] = cluster_id;
            }
            if labels[curr] != -2 {
                continue;
            }
            labels[curr] = cluster_id;
            let curr_neighbors = region_query(curr);
            if curr_neighbors.len() >= min_pts {
                for nb in curr_neighbors {
                    if labels[nb] == -2 || labels[nb] == -1 {
                        queue.push_back(nb);
                    }
                }
            }
        }
        cluster_id += 1;
    }

    let noise_count = labels.iter().filter(|&&l| l == -1).count();
    let mut clusters = Vec::with_capacity(cluster_id as usize);
    for cid in 0..cluster_id {
        let mut member_fids = Vec::new();
        let mut coords = Vec::new();
        let mut sx = 0.0_f64;
        let mut sy = 0.0_f64;
        let mut material_counts = std::collections::BTreeMap::new();
        for (idx, &(fid, xy, mat)) in artifacts.iter().enumerate() {
            if labels[idx] == cid {
                member_fids.push(fid);
                coords.push(Coord { x: xy[0], y: xy[1] });
                sx += xy[0];
                sy += xy[1];
                *material_counts.entry(mat.to_owned()).or_insert(0) += 1;
            }
        }
        let cnt = member_fids.len() as f64;
        let centroid = if cnt > 0.0 {
            [sx / cnt, sy / cnt]
        } else {
            [0.0, 0.0]
        };
        let poly = Polygon::new(LineString::new(coords), vec![]);
        let hull_area = poly.convex_hull().unsigned_area();
        clusters.push(ArtifactClusterResult {
            cluster_id: cid,
            member_fids,
            centroid,
            convex_hull_area_m2: hull_area,
            material_counts,
        });
    }
    (clusters, noise_count)
}

/// 동일 유구 내 출토 유물 재질 간 공반(Co-occurrence) 대칭 빈도 행렬 산출
/// (C++ `KaCoOccurrenceMatrix::computeMaterialCoOccurrence` 1:1 대응)
#[must_use]
pub fn compute_material_co_occurrence(
    feature_artifacts: &[Vec<&str>],
    material_labels: &[&str],
) -> CoOccurrenceMatrixResult {
    let m = material_labels.len();
    let mut matrix = vec![vec![0_usize; m]; m];
    let mut total_per_feature = Vec::with_capacity(feature_artifacts.len());
    for mats in feature_artifacts {
        total_per_feature.push(mats.len());
        for (i, &label_a) in material_labels.iter().enumerate() {
            let has_a = mats.contains(&label_a);
            if !has_a {
                continue;
            }
            for (j, &label_b) in material_labels.iter().enumerate() {
                if mats.contains(&label_b) {
                    matrix[i][j] += 1;
                }
            }
        }
    }
    CoOccurrenceMatrixResult {
        material_labels: material_labels.iter().map(|&s| s.to_owned()).collect(),
        matrix,
        total_per_feature,
    }
}

/// 다각망 측선 관측 레코드 (C++ `TraverseTraverseLeg`)
#[derive(Debug, Clone, PartialEq)]
pub struct TraverseLeg {
    pub from_station: String,
    pub to_station: String,
    pub delta_x_m: f64,
    pub delta_y_m: f64,
    pub measured_distance_m: f64,
}

/// 폐합 다각망(Traverse) 각오차·선형 폐합비 검증 및 Bowditch 좌표 보정 결과 (C++ `NetworkAdjustmentResult`)
#[derive(Debug, Clone, PartialEq)]
pub struct NetworkAdjustmentResult {
    pub is_acceptable: bool,
    pub angular_error_sec: f64,
    pub max_allowable_angular_sec: f64,
    pub linear_closing_error_m: f64,
    pub total_traverse_length_m: f64,
    pub closing_ratio_denominator: i32,
    pub adjusted_coords: Vec<[f64; 2]>,
}

/// 3급 도근점 기준 폐합 다각망 선형 폐합비(`1/N >= 1/3000`)·각오차 검증 및 콤파스 법칙(Bowditch) 좌표 조정
/// (C++ `KaSurveyNetworkAdjuster::adjustClosedTraverse` 1:1 대응)
#[must_use]
pub fn adjust_closed_traverse(
    legs: &[TraverseLeg],
    fixed_known_coord: [f64; 2],
    angular_error_sec: f64,
) -> NetworkAdjustmentResult {
    let n = legs.len();
    let total_len: f64 = legs.iter().map(|l| l.measured_distance_m.max(0.0)).sum();
    if n < 2 || total_len <= 1e-6 || !total_len.is_finite() {
        return NetworkAdjustmentResult {
            is_acceptable: false,
            angular_error_sec,
            max_allowable_angular_sec: 0.0,
            linear_closing_error_m: 0.0,
            total_traverse_length_m: 0.0,
            closing_ratio_denominator: 0,
            adjusted_coords: Vec::new(),
        };
    }
    let fx: f64 = legs.iter().map(|l| l.delta_x_m).sum();
    let fy: f64 = legs.iter().map(|l| l.delta_y_m).sum();
    let fs = fx.hypot(fy);
    let ratio_denom = if fs <= 1e-9 {
        999_999
    } else {
        (total_len / fs).round() as i32
    };
    let max_allowable_angular_sec = HGIS_ANGULAR_SEC_FACTOR_3RD * (n as f64).sqrt();
    let is_acceptable = ratio_denom >= HGIS_TRAVERSE_3RD_ORDER_RATIO
        && angular_error_sec.abs() <= max_allowable_angular_sec;

    let mut adjusted_coords = Vec::with_capacity(n + 1);
    let mut curr = fixed_known_coord;
    adjusted_coords.push(curr);
    for leg in legs {
        let weight = leg.measured_distance_m.max(0.0) / total_len;
        curr[0] += leg.delta_x_m - fx * weight;
        curr[1] += leg.delta_y_m - fy * weight;
        adjusted_coords.push(curr);
    }

    NetworkAdjustmentResult {
        is_acceptable,
        angular_error_sec,
        max_allowable_angular_sec,
        linear_closing_error_m: fs,
        total_traverse_length_m: total_len,
        closing_ratio_denominator: ratio_denom,
        adjusted_coords,
    }
}
