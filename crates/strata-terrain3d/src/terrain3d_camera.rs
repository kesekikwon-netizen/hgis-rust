//! 궤도 카메라와 Z 과장·피치 한계 상수.
//!
//! `viewport3d_renderer` 에서 분리했다.

pub const HGIS_Z_EXAG_MIN: f32 = 1.0;

pub const HGIS_Z_EXAG_MAX: f32 = 5.0;

pub const HGIS_Z_EXAG_DEFAULT: f32 = 1.5;

pub const HGIS_PALEO3D_Z_EXAG_MIN: f32 = 0.5;

pub const HGIS_PALEO3D_Z_EXAG_MAX: f32 = 10.0;

pub const HGIS_CAMERA_PITCH_MIN_DEG: f32 = 0.0;

pub const HGIS_CAMERA_PITCH_MAX_DEG: f32 = 85.0;

pub const HGIS_CAMERA_DEFAULT_DIST: f32 = 1000.0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OrbitCamera3D {
    pub yaw_deg: f32,
    pub pitch_deg: f32,
    pub zoom: f32,
    pub z_exaggeration: f32,
}

impl Default for OrbitCamera3D {
    fn default() -> Self {
        Self {
            yaw_deg: 35.0,
            pitch_deg: 42.0,
            zoom: 1.0,
            z_exaggeration: 2.0,
        }
    }
}

impl OrbitCamera3D {
    /// C++ `TerrainOrbitController::updateMouseDrag` — 방위각(`yaw_deg`) 회전, 짐벌락 방어 앙각(`[-89.0°, +89.0°]`) 및 수직 과장비(`[1.0, 5.0]`) 클램핑 1:1
    pub fn apply_orbit_drag(&mut self, delta_yaw_deg: f32, delta_pitch_deg: f32) {
        self.yaw_deg = (self.yaw_deg + delta_yaw_deg).rem_euclid(360.0);
        self.pitch_deg = (self.pitch_deg + delta_pitch_deg).clamp(-89.0, 89.0);
        self.z_exaggeration = self.z_exaggeration.clamp(HGIS_Z_EXAG_MIN, HGIS_Z_EXAG_MAX);
    }

    /// C++ `Terrain3dRenderer::setVerticalExaggeration` — 수직 과장 배율 `[1.0, 5.0]` 클램핑
    pub fn set_vertical_exaggeration(&mut self, scale: f32) {
        self.z_exaggeration = scale.clamp(HGIS_Z_EXAG_MIN, HGIS_Z_EXAG_MAX);
    }

    /// C++ `KaPaleoViewer3D::setVerticalExaggeration` — 고지형 3D 뷰어 수직 과장 배율 `[0.5, 10.0]` 클램핑 및 NaN 방어
    pub fn set_paleo_vertical_exaggeration(&mut self, factor: f32) {
        if !factor.is_finite() {
            self.z_exaggeration = HGIS_Z_EXAG_DEFAULT;
        } else {
            self.z_exaggeration = factor.clamp(HGIS_PALEO3D_Z_EXAG_MIN, HGIS_PALEO3D_Z_EXAG_MAX);
        }
    }

    /// C++ `KaTerrainCamera3D::resetToNorth` — 방위각(`yaw_deg`)을 정북(`0.0°`)으로 복원하고 비정상 거리를 기본값으로 복구
    pub fn reset_to_north(&mut self, distance: f32) -> f32 {
        self.yaw_deg = 0.0;
        if !distance.is_finite() || distance <= 1e-4 {
            HGIS_CAMERA_DEFAULT_DIST
        } else {
            distance
        }
    }
}
