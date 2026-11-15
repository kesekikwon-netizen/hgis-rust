//! 「DEM 표현」: how the DEM of the open survey is shown (`target/cpp-ref/src/app/KaDemClassDialog.cpp`,
//! `src/core/DemPresentation.cpp`). The colour preset, whether the relief lies over it, how many times the
//! heights count for the relief's light, and how strongly the relief darkens.
//!
//! The C++ keeps these on the layer (`ka_hgis/dem_preset`, `dem_relief_enabled`, `dem_z_factor`,
//! `dem_relief_strength`); here the app keeps them for the survey that is open.

use super::{DEM_KEY, RELIEF_KEY};
use crate::app_state::{MainTab, StrataAppState};
use crate::dialogs_modal::ActiveModalDialog;
use crate::ref_vector::data::{HeightRamp, Look};

#[cfg(test)]
#[path = "dem_look_tests.rs"]
mod tests;

/// `KaDemClassDialog::applyStyle`, `:131`.
pub(crate) const APPLIED: &str = "적용했습니다. 색띠로 표고를 확인하세요. 음영은 지형 방향에 따라 밝기를 바꿉니다.";
/// `KaDemClassDialog::applyStyle`, `:120`.
pub(crate) const NO_HEIGHTS_HERE: &str = "이 화면에서 표고값을 읽지 못했습니다. DEM이 있는 곳으로 이동한 뒤 적용하세요.";
/// `MainWindow::editDemElevationClasses`, `MainWindowOverlay.cpp:288`.
pub(crate) const NEEDS_DEM: &str = "먼저 DEM을 켜 주세요. 표고 자료를 불러오면 색 표현과 음영을 조절할 수 있습니다.";
/// `DemPresentation::followCanvas`, `DemPresentation.cpp:134`.
pub(crate) const KEPT_RAMP: &str = "현재 화면에 표고 자료가 없어 이전 색띠를 유지합니다.";
/// How long the map must stand still before the colours follow it (`DemPresentation.cpp:122`), seconds.
const SETTLE_S: f64 = 0.35;

/// The colour presets, in the order of the dialog's list (`KaDemClassDialog.cpp:31-33`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum DemPreset {
    #[default]
    National,
    Lowland,
    Viewport,
}

impl DemPreset {
    pub(crate) const ALL: [(Self, &'static str); 3] = [(Self::National, "전국 표준 (0–2000m)"), (Self::Lowland, "저지대 강조"), (Self::Viewport, "현재 화면 맞춤")];
}

/// What 「DEM 표현」 sets. A new DEM has the first preset, its relief at 30 %, heights counted once
/// (`KaDemClassDialog.cpp:34, 47, 54, 60`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct DemLook {
    pub(crate) preset: DemPreset,
    pub(crate) relief: bool,
    /// 「수직과장」, 0.1 to 5 times.
    pub(crate) z_factor: f32,
    /// 「음영 세기」, 0 to 80 per cent.
    pub(crate) strength_pct: u32,
}

impl Default for DemLook {
    fn default() -> Self {
        Self { preset: DemPreset::National, relief: true, z_factor: 1.0, strength_pct: 30 }
    }
}

impl DemLook {
    /// The opacity of the relief's legend row.
    // A number between 0 and 80.
    #[allow(clippy::cast_precision_loss)]
    pub(crate) fn relief_opacity(self) -> f32 {
        self.strength_pct.min(80) as f32 / 100.0
    }
}

/// What following the view remembers: the map box seen last, since when, and whether it was acted on.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub(crate) struct DemFollow {
    seen: Option<[f64; 4]>,
    since: f64,
    settled: bool,
}

impl StrataAppState {
    /// The colours of a preset; for 「현재 화면 맞춤」 from the heights inside the map as it was painted
    /// last (`DemPresentation.cpp:53-64`). `None` when the map holds no height of the DEM.
    pub(crate) fn dem_ramp(&self, preset: DemPreset) -> Option<HeightRamp> {
        match preset {
            DemPreset::National => Some(HeightRamp::National),
            DemPreset::Lowland => Some(HeightRamp::Lowland),
            DemPreset::Viewport => {
                let (low, high) = self.ref_vectors.height_range(DEM_KEY, self.ref_vectors.map_box?)?;
                // Whole metres around what the map shows, and a metre of room for one height alone (`:60-63`).
                let (low, high) = (low.floor(), high.ceil());
                Some(HeightRamp::Range { low, high: if high <= low { low + 1.0 } else { high } })
            }
        }
    }

    /// 「적용」 (`KaDemClassDialog::applyStyle`): the DEM takes the preset, the relief its exaggeration,
    /// strength and whether it is shown. Returns what the dialog then says.
    pub(crate) fn apply_dem_look(&mut self, look: DemLook) -> &'static str {
        let Some(dem_name) = self.dem_row_name() else {
            return NEEDS_DEM;
        };
        let Some(ramp) = self.dem_ramp(look.preset) else {
            return NO_HEIGHTS_HERE;
        };
        // What the dialog offers: 0.1 to 5 times, 0 to 80 per cent (`KaDemClassDialog.cpp:51, 58`).
        let look = DemLook { z_factor: look.z_factor.clamp(0.1, 5.0), strength_pct: look.strength_pct.min(80), ..look };
        self.ref_vectors.set_look(DEM_KEY, Look::Heights { ramp });
        self.ref_vectors.set_look(RELIEF_KEY, Look::Relief { z_factor: look.z_factor });
        // `ensureDemRelief`: the relief is shown while it is wanted and the DEM is shown (BasemapOps.cpp:2106).
        let shown = look.relief && self.home_map.real_survey.is_layer_visible(&dem_name);
        if let Some(row) = self.left_layers.entries.iter_mut().find(|e| e.layer_key == RELIEF_KEY) {
            row.opacity = look.relief_opacity();
            self.home_map.real_survey.layer_visible.insert(row.label_ko.clone(), shown);
        }
        (self.dem_look, self.dem_follow) = (look, DemFollow::default());
        APPLIED
    }

    /// 「DEM 표현…」 (`MainWindow::editDemElevationClasses`, MainWindowOverlay.cpp:269-296): the dialog opens
    /// over a DEM only, and shows what the DEM has now.
    pub(crate) fn open_dem_look_dialog(&mut self) {
        if self.dem_row_name().is_none() {
            self.show_status_message(NEEDS_DEM);
            return;
        }
        self.active_tab = MainTab::Map;
        let (look, hint) = (self.dem_look, self.dem_view_hint().unwrap_or_default());
        let modals = &mut self.modals;
        modals.dem_preset_idx = DemPreset::ALL.iter().position(|(preset, _)| *preset == look.preset).unwrap_or(0);
        (modals.dem_relief_enabled, modals.dem_exaggeration, modals.dem_relief_strength_pct) = (look.relief, look.z_factor, look.strength_pct);
        modals.dem_status_text = hint;
        modals.active = Some(ActiveModalDialog::DemClass);
    }

    /// What the dialog says when it opens over level ground (`KaDemClassDialog::suggestViewport`): the
    /// heights of the map differ by less than 100 m, so the national colours show nearly one colour.
    pub(crate) fn dem_view_hint(&self) -> Option<String> {
        if self.dem_look.preset == DemPreset::Viewport {
            return None;
        }
        let (low, high) = self.ref_vectors.height_range(DEM_KEY, self.ref_vectors.map_box?)?;
        let apart = f64::from(high) - f64::from(low);
        (apart < 100.0).then(|| format!("지금 화면의 높이차는 약 {} m뿐이라 이 색 표현으로는 거의 한 색으로 보입니다. 「현재 화면에 색 맞추기」를 누르면 화면 안의 높이로 색을 나눕니다.", apart.round().max(1.0)))
    }

    /// Whether the colours still wait for the map to stand still (a scripted screenshot waits for them).
    pub(crate) fn dem_follow_waits(&self) -> bool {
        self.dem_look.preset == DemPreset::Viewport && self.ref_vectors.map_box.is_some() && self.dem_row_name().is_some() && !(self.dem_follow.settled && self.dem_follow.seen == self.ref_vectors.map_box)
    }

    /// Every frame: with 「현재 화면 맞춤」 the colours follow the map once it has stood still for 350 ms
    /// (`DemPresentation::followCanvas`). Returns whether it still waits for the map to stand still.
    pub(crate) fn follow_dem_view(&mut self, now: f64) -> bool {
        let (true, Some(map)) = (self.dem_look.preset == DemPreset::Viewport, self.ref_vectors.map_box) else {
            return false;
        };
        if self.dem_follow.seen != Some(map) {
            self.dem_follow = DemFollow { seen: Some(map), since: now, settled: false };
            return true;
        }
        if self.dem_follow.settled {
            return false;
        }
        if now - self.dem_follow.since < SETTLE_S {
            return true;
        }
        self.dem_follow.settled = true;
        match self.dem_ramp(DemPreset::Viewport) {
            Some(ramp) => self.ref_vectors.set_look(DEM_KEY, Look::Heights { ramp }),
            None if self.dem_row_name().is_some() => self.show_status_message(KEPT_RAMP),
            None => {}
        }
        false
    }
}
