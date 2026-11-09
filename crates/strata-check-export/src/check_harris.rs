//! 해리스 행렬(층서 관계) 구성.
//!
//! `checklist_engine` 에서 분리했다.

/// 해리스 매트릭스 최대 분석 유구 수 (C++ `HGIS_MAX_HARRIS_NODES_LIMIT`)
pub const HGIS_MAX_HARRIS_NODES_LIMIT: usize = 50_000;

/// 유구 간 층위 선후관계 유형 (C++ `StratigraphicRelation`)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StratigraphicRelation {
    /// 파괴함 (신기 -> 고기)
    Cuts,
    /// 피파괴됨 (고기 <- 신기)
    IsCutBy,
    /// 상부 퇴적/덮음 (신기 -> 고기)
    Overlies,
    /// 하부 깔림 (고기 <- 신기)
    Underlies,
    /// 동시기/동일 층위
    Contemporary,
}

/// 해리스 매트릭스 방향 간선 (C++ `StratigraphicEdge`)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StratigraphicEdge {
    pub younger_feature_id: String,
    pub older_feature_id: String,
    pub relation: StratigraphicRelation,
}

/// 해리스 매트릭스 위상 정렬 및 순환 모순 탐지 결과 (C++ `HarrisMatrixResult`)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HarrisMatrixResult {
    pub is_acyclic: bool,
    pub topological_order: Vec<String>,
    pub stratigraphic_levels: Vec<Vec<String>>,
    pub detected_cycle_path: Vec<String>,
}

/// 유구 선후관계 DAG 구축, Kahn 위상 정렬 기반 상대 연대 순서 도출 및 순환 모순(Cycle) 경로 탐지
/// (C++ `KaHarrisMatrixBuilder::buildHarrisMatrix` & `KaStratigraphicCycleDetector` 1:1 대응)
#[must_use]
pub fn build_harris_matrix(
    feature_ids: &[&str],
    relations: &[StratigraphicEdge],
) -> HarrisMatrixResult {
    let mut nodes: Vec<String> = feature_ids
        .iter()
        .take(HGIS_MAX_HARRIS_NODES_LIMIT)
        .map(|&s| s.to_owned())
        .collect();
    for edge in relations {
        if nodes.len() < HGIS_MAX_HARRIS_NODES_LIMIT && !nodes.contains(&edge.younger_feature_id) {
            nodes.push(edge.younger_feature_id.clone());
        }
        if nodes.len() < HGIS_MAX_HARRIS_NODES_LIMIT && !nodes.contains(&edge.older_feature_id) {
            nodes.push(edge.older_feature_id.clone());
        }
    }
    let n = nodes.len();
    let idx_of = |name: &str| nodes.iter().position(|x| x == name);
    let mut adj = vec![Vec::<usize>::new(); n];
    let mut in_deg = vec![0_usize; n];

    for edge in relations {
        let (u_opt, v_opt) = match edge.relation {
            StratigraphicRelation::Cuts | StratigraphicRelation::Overlies => (
                idx_of(&edge.younger_feature_id),
                idx_of(&edge.older_feature_id),
            ),
            StratigraphicRelation::IsCutBy | StratigraphicRelation::Underlies => (
                idx_of(&edge.older_feature_id),
                idx_of(&edge.younger_feature_id),
            ),
            StratigraphicRelation::Contemporary => (None, None),
        };
        if let (Some(u), Some(v)) = (u_opt, v_opt) {
            if !adj[u].contains(&v) {
                adj[u].push(v);
                in_deg[v] += 1;
            }
        }
    }

    let mut queue: std::collections::VecDeque<usize> = (0..n).filter(|&i| in_deg[i] == 0).collect();
    let mut topological_order = Vec::with_capacity(n);
    let mut stratigraphic_levels = Vec::new();

    while !queue.is_empty() {
        let level_len = queue.len();
        let mut level_nodes = Vec::with_capacity(level_len);
        for _ in 0..level_len {
            if let Some(u) = queue.pop_front() {
                topological_order.push(nodes[u].clone());
                level_nodes.push(nodes[u].clone());
                for &v in &adj[u] {
                    in_deg[v] -= 1;
                    if in_deg[v] == 0 {
                        queue.push_back(v);
                    }
                }
            }
        }
        stratigraphic_levels.push(level_nodes);
    }

    if topological_order.len() == n {
        return HarrisMatrixResult {
            is_acyclic: true,
            topological_order,
            stratigraphic_levels,
            detected_cycle_path: Vec::new(),
        };
    }

    // Cycle detection via trace on remaining in_deg > 0 nodes
    let mut detected_cycle_path = Vec::new();
    if let Some(start) = (0..n).find(|&i| in_deg[i] > 0) {
        let mut visited: Vec<usize> = Vec::new();
        let mut curr = start;
        loop {
            if let Some(pos) = visited.iter().position(|&x| x == curr) {
                for &node_idx in &visited[pos..] {
                    detected_cycle_path.push(nodes[node_idx].clone());
                }
                detected_cycle_path.push(nodes[curr].clone());
                break;
            }
            visited.push(curr);
            if let Some(&next) = adj[curr].iter().find(|&&nb| in_deg[nb] > 0) {
                curr = next;
            } else {
                break;
            }
        }
    }

    HarrisMatrixResult {
        is_acyclic: false,
        topological_order,
        stratigraphic_levels,
        detected_cycle_path,
    }
}
