//! `--snap` ?ㅽ뻾湲???egui ?꾨젅???덉뿉???≪뀡?믪벙泥섎? ?쒖꽌?濡?泥섎━?쒕떎.

use std::io::Write as _;

use eframe::egui;

use crate::app_state::StrataAppState;

impl StrataAppState {
    /// ?ㅻ깄???④퀎瑜????꾨젅??吏꾪뻾?쒕떎. 紐⑤뱺 ?④퀎媛 ?앸굹硫?酉고룷?몃? ?ル뒗??
    pub(crate) fn snap_tick(&mut self, ctx: &egui::Context) {
        let Some(snap) = self.snap.as_ref() else {
            return;
        };
        if !snap.has_work() {
            return;
        }

        // 媛믪쓣 誘몃━ 爰쇰궡 ?먭퀬(鍮뚮┝ 異⑸룎 ?뚰뵾) ?꾨옒?먯꽌 媛깆떊?쒕떎.
        let waiting = snap.waiting;
        let settle = snap.settle;
        let next = snap.steps.first().cloned();

        // 1) 罹≪쿂瑜?湲곕떎由щ뒗 以???Screenshot ?대깽?멸? ?붿쑝硫???ν븳??
        if waiting {
            let taken = ctx.input(|i| {
                i.events.iter().find_map(|e| match e {
                    egui::Event::Screenshot { image, .. } => Some(image.clone()),
                    _ => None,
                })
            });
            if let Some(img) = taken {
                if let Some(snap) = self.snap.as_mut() {
                    let name = snap
                        .steps
                        .first()
                        .map_or_else(|| "snap".to_owned(), |s| s.name.clone());
                    let mut path = snap.prefix.clone();
                    path.push(format!("{name}.png"));
                    match write_png(&path, &img) {
                        Ok(bytes) => {
                            eprintln!("[SNAP] {name} -> {} ({bytes} bytes)", path.display());
                        }
                        Err(e) => eprintln!("[SNAP] {name} ????ㅽ뙣: {e}"),
                    }
                    snap.steps.remove(0);
                    snap.waiting = false;
                    snap.settle = 8;
                }
            }
            ctx.request_repaint();
            return;
        }

        // 2) ?먮━ ?≪쓣 ?쒓컙??以???고듃쨌?덉씠?꾩썐 ?뺤젙).
        if settle > 0 {
            if let Some(snap) = self.snap.as_mut() {
                snap.settle -= 1;
            }
            ctx.request_repaint();
            return;
        }

        // 3) ?ㅼ쓬 ?④퀎 ?ㅽ뻾.
        let Some(step) = next else {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            return;
        };
        if let Some(action) = step.action.clone() {
            self.snap_apply_action(&action);
            if let Some(snap) = self.snap.as_mut() {
                // ?≪뀡? ??踰덈쭔 ?ㅽ뻾?쒕떎. ?④퀎???④꺼 ?먭퀬 action 留?吏?뚯꽌
                // ?ㅼ쓬 諛뷀댁뿉 ???④퀎媛 "罹≪쿂" 濡?泥섎━?섍쾶 ?쒕떎.
                // (吏?곗? ?딆쑝硫?媛숈? ?≪뀡??臾댄븳??諛섎났?쒕떎 ???ㅼ젣濡?洹몃옱??)
                if let Some(first) = snap.steps.first_mut() {
                    first.action = None;
                }
                snap.settle = 16;
            }
            ctx.request_repaint();
            return;
        }
        if let Some(snap) = self.snap.as_mut() {
            snap.waiting = true;
        }
        ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(Default::default()));
        ctx.request_repaint();
    }
    /// 리본 라벨 하나를 누르는 것과 같은 효과를 낸다.
    ///
    /// ## 검증용 제어 통로
    ///
    /// `@` 로 시작하는 이름은 앱을 스크립트로 조종하는 명령이다.
    /// 화면을 눈으로만 보지 않고 "무엇을 눌렀을 때 무엇이 되는가"를
    /// 기계적으로 확인하기 위해 만들었다(사용자 지시 2026-11-10).
    ///
    /// | 명령 | 하는 일 |
    /// |---|---|
    /// | `@tab_map` `@tab_home` `@tab_drawing` `@tab_paleo` `@tab_terrain3d` | 탭 전환 |
    /// | `@close_modal` | 열린 대화상자 닫기 |
    /// | `@new_survey_go` | 새 조사 확정 |
    /// | `@draw:<화면x>,<화면y>` | 지도에 점을 찍는다(조사구역 그리기) |
    /// | `@finish_draw` | 그리기 마침(우클릭과 같은 효과) |
    /// | `@clear_draw` | 그리는 중인 점 지우기 |
    /// | `@select_tool` `@draw_polygon_tool` `@pan_tool` | 도구 전환 |
    /// | `@zoom:<배율>` | 화면 중심 기준 확대·축소 |
    /// | `@snap_on` `@snap_off` | 스냅 켜고 끄기 |
    /// | `@dump_ring` | 지금 그리는 링을 로그로 남긴다 |
    fn snap_apply_action(&mut self, action: &str) {
        let (name, arg) = match action.split_once(':') {
            Some((n, a)) => (n, Some(a)),
            None => (action, None),
        };
        match name {
            "@new_survey_go" => {
                // 새 조사 모달에 채워진 값으로 실제 조사 화면으로 간다.
                let name = self.modals.new_survey_name.trim().to_owned();
                let crs = self.modals.new_survey_crs;
                let dir = if self.modals.new_survey_dir.trim().is_empty() {
                    std::env::temp_dir()
                } else {
                    std::path::PathBuf::from(self.modals.new_survey_dir.trim())
                };
                let path = dir.join(format!("{name}.gpkg"));
                let _ = strata_storage::gpkg::GpkgRepository::create_new_survey(&path, crs);
                self.start_new_survey_project_with_path(&name, &path, crs);
                self.modals.active = None;
            }
            "@close_modal" => self.modals.active = None,
            "@tab_home" => self.active_tab = crate::app_state::MainTab::Home,
            "@tab_map" => self.active_tab = crate::app_state::MainTab::Map,
            "@tab_drawing" => self.open_studio_tab(crate::app_state::MainTab::Drawing),
            "@tab_paleo" => self.open_studio_tab(crate::app_state::MainTab::Paleo),
            "@tab_terrain3d" => self.open_studio_tab(crate::app_state::MainTab::Terrain3d),

            // ── 그리기 제어 ─────────────────────────────────────────
            "@draw" => {
                if let Some(world) = self.snap_arg_to_world(arg) {
                    self.home_map.scene.active_rubberband.push(world);
                    self.home_map.cursor_world_xy = world;
                }
            }
            "@finish_draw" => {
                // 실제 우클릭 마침(`cad_session::on_right_click`)과 같은 순서로 한다.
                // **3점 이상일 때만** 링을 꺼내 저장한다. 2점 이하면 링을 지우지 않고
                // 그대로 둬야 사용자가 계속 그릴 수 있다.
                if self.home_map.scene.active_rubberband.len() >= 3 {
                    let ring = std::mem::take(&mut self.home_map.scene.active_rubberband);
                    let area = crate::real_survey::ring_area_m2(&ring);
                    self.home_map.real_survey.save_drawn_polygon(ring);
                    self.is_dirty = true;
                    self.home_map.draw_guide_message =
                        Some(format!("조사구역을 저장했습니다 ({area:.2} m²)."));
                }
            }
            "@clear_draw" => self.home_map.scene.active_rubberband.clear(),

            // ── 도구·화면 제어 ──────────────────────────────────────
            "@select_tool" => self.active_tool = strata_canvas2d::map_tools::MapToolKind::Select,
            "@draw_polygon_tool" => {
                self.active_tool = strata_canvas2d::map_tools::MapToolKind::DrawPolygon;
            }
            "@pan_tool" => self.active_tool = strata_canvas2d::map_tools::MapToolKind::Pan,
            "@zoom" => {
                if let Some(f) = arg.and_then(|a| a.trim().parse::<f64>().ok()) {
                    let rect = self.last_canvas_rect.unwrap_or_else(|| {
                        egui::Rect::from_min_size(egui::Pos2::ZERO, egui::Vec2::new(800.0, 600.0))
                    });
                    let center = [
                        f64::from(rect.width()) * 0.5,
                        f64::from(rect.height()) * 0.5,
                    ];
                    self.home_map.viewport.zoom_at(f, center);
                }
            }
            // 레이어 토글 — 리본 라벨이 한글이라 셸에서 인자를 넘기기 어렵다.
            "@layer" => {
                if let Some(name) = arg {
                    let on = !self.home_map.real_survey.is_layer_visible(name);
                    self.home_map
                        .real_survey
                        .layer_visible
                        .insert(name.to_owned(), on);
                    eprintln!("[SNAP] layer {name} -> {on}");
                }
            }
            "@soil" => {
                self.handle_ribbon_click("토양");
                eprintln!("[SNAP] soil hint: {}", self.status_bar_hint);
            }
            "@snap_on" => self.home_map.cad.snap_enabled = true,
            "@snap_off" => self.home_map.cad.snap_enabled = false,

            // ── 꼭짓점 편집 ────────────────────────────────────────
            // 「꼭짓점 수정」 메뉴는 레이어 패널 안에 있어 리본 라벨로는 못 누른다.
            // 검증에서 그 상태를 직접 만들 수 있게 명령으로 노출한다.
            "@vertex_edit" => {
                let idx = arg
                    .and_then(|a| a.trim().parse::<usize>().ok())
                    .or(self.home_map.selected_feature_idx)
                    .unwrap_or(0);
                self.active_tab = crate::app_state::MainTab::Map;
                self.active_tool = strata_canvas2d::map_tools::MapToolKind::Select;
                self.home_map.vertex_edit =
                    Some(crate::vertex_edit_state::VertexEditState::new(idx));
                self.home_map.selected_feature_idx = Some(idx);
            }
            "@vertex_edit_off" => self.home_map.vertex_edit = None,
            "@move_vertex" => {
                // `@move_vertex:<점>;<x>;<y>` — 지도 좌표로 옮긴다.
                if let Some(spec) = arg {
                    let parts: Vec<&str> = spec.split(';').collect();
                    if parts.len() == 3 {
                        if let (Ok(i), Ok(x), Ok(y)) = (
                            parts[0].trim().parse::<usize>(),
                            parts[1].trim().parse::<f64>(),
                            parts[2].trim().parse::<f64>(),
                        ) {
                            self.snap_apply_vertex_edit(i, [x, y], false);
                        }
                    }
                }
            }
            "@insert_vertex" => {
                // `@insert_vertex:<선분>;<x>;<y>`
                if let Some(spec) = arg {
                    let parts: Vec<&str> = spec.split(';').collect();
                    if parts.len() == 3 {
                        if let (Ok(s), Ok(x), Ok(y)) = (
                            parts[0].trim().parse::<usize>(),
                            parts[1].trim().parse::<f64>(),
                            parts[2].trim().parse::<f64>(),
                        ) {
                            self.snap_apply_vertex_edit(s, [x, y], true);
                        }
                    }
                }
            }
            "@delete_vertex" => {
                if let Some(i) = arg.and_then(|a| a.trim().parse::<usize>().ok()) {
                    self.snap_delete_vertex(i);
                }
            }
            "@dump_vertices" => {
                let idx = self
                    .home_map
                    .vertex_edit
                    .as_ref()
                    .map(|v| v.feature_index)
                    .unwrap_or(0);
                if let Some(ring) = self.home_map.real_survey.survey_rings.get(idx) {
                    let pts: Vec<String> = ring
                        .iter()
                        .map(|p| format!("({:.2},{:.2})", p[0], p[1]))
                        .collect();
                    eprintln!("[SNAP] vertices idx={idx} n={} {}", ring.len(), pts.join(" "));
                }
            }
            "@undo" => {
                let ok = self.snap_undo_ring();
                eprintln!("[SNAP] undo -> {ok}");
            }
            "@redo" => {
                let ok = self.snap_redo_ring();
                eprintln!("[SNAP] redo -> {ok}");
            }

            // ── 진단 ────────────────────────────────────────────────
            "@dump_ring" => {
                let ring = &self.home_map.scene.active_rubberband;
                let pts: Vec<String> = ring
                    .iter()
                    .map(|p| format!("({:.2},{:.2})", p[0], p[1]))
                    .collect();
                eprintln!(
                    "[SNAP] ring points={} mpp={:.4} {}",
                    ring.len(),
                    self.home_map.viewport.meters_per_pixel,
                    pts.join(" ")
                );
            }

            _ => self.handle_ribbon_click(action),
        }
    }

    /// `@draw:120,340` 의 인자를 화면 픽셀로 보고 지도 좌표로 바꾼다.
    ///
    /// 인자는 지도 탭 캔버스 안의 화면 좌표(좌상단 기준)다.
    /// `@draw:120,340` 의 인자를 화면 픽셀로 보고 지도 좌표로 바꾼다.
    ///
    /// 인자는 지도 탭 캔버스 안의 화면 좌표(좌상단 기준)다.
    /// 단계 구분자가 쉼표라서 인자 안에서는 `x` 또는 `;` 를 쓴다.
    #[cfg(test)]
    pub(crate) fn snap_apply_action_for_test(&mut self, action: &str) {
        self.snap_apply_action(action);
    }

    /// 편집 중인 링에 정점 이동·삽입을 적용한다.
    fn snap_apply_vertex_edit(&mut self, index: usize, at: [f64; 2], insert: bool) {
        let Some(ve) = self.home_map.vertex_edit.as_ref().cloned() else {
            return;
        };
        let edit = if insert {
            strata_canvas2d::vertex_handle::VertexEdit::Insert { segment: index, at }
        } else {
            strata_canvas2d::vertex_handle::VertexEdit::Move { index, to: at }
        };
        // 실행취소를 위해 편집 전 링을 남긴다.
        self.snap_record_ring(ve.feature_index);
        let Some(ring) = self
            .home_map
            .real_survey
            .survey_rings
            .get_mut(ve.feature_index)
        else {
            return;
        };
        self.is_dirty = ve.apply_to(ring, &edit).is_ok();
    }

    /// 편집 중인 링에서 정점을 지운다(최소 점 수 가드 포함).
    fn snap_delete_vertex(&mut self, index: usize) {
        let Some(ve) = self.home_map.vertex_edit.as_ref().cloned() else {
            return;
        };
        self.snap_record_ring(ve.feature_index);
        let Some(ring) = self
            .home_map
            .real_survey
            .survey_rings
            .get_mut(ve.feature_index)
        else {
            return;
        };
        let edit = strata_canvas2d::vertex_handle::VertexEdit::Delete { index };
        if ve.apply_to(ring, &edit).is_ok() {
            self.is_dirty = true;
        } else {
            self.home_map.draw_guide_message =
                Some("점을 더 지울 수 없습니다 — 면은 최소 3점이 필요합니다.".to_owned());
        }
    }

    fn snap_arg_to_world(&self, arg: Option<&str>) -> Option<[f64; 2]> {
        let a = arg?;
        // 단계 구분자가 쉼표라서 인자 안의 쉼표는 쓸 수 없다. `x` 또는 `x;y` 도 받는다.
        let mut it = a.split([',', 'x', ';']);
        let sx: f64 = it.next()?.trim().parse().ok()?;
        let sy: f64 = it.next()?.trim().parse().ok()?;
        let rect = self.last_canvas_rect?;
        Some(self.home_map.viewport.screen_to_world([
            sx - f64::from(rect.left()),
            sy - f64::from(rect.top()),
        ]))
    }
}
/// `egui::ColorImage` 瑜?PNG 濡???ν븳???몃? ?щ젅?댄듃 ?놁씠 zlib ?섎룞 湲곕줉).
fn write_png(path: &std::path::Path, img: &egui::ColorImage) -> std::io::Result<usize> {
    let w = img.width();
    let h = img.height();
    let mut raw: Vec<u8> = Vec::with_capacity(h * (1 + w * 4));
    for y in 0..h {
        raw.push(0);
        for x in 0..w {
            let c = img.pixels[y * w + x];
            raw.extend_from_slice(&[c.r(), c.g(), c.b(), c.a()]);
        }
    }
    let idat = zlib_stored(&raw);
    let mut out: Vec<u8> = Vec::new();
    out.extend_from_slice(&[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]);
    let mut ihdr = Vec::new();
    ihdr.extend_from_slice(&(w as u32).to_be_bytes());
    ihdr.extend_from_slice(&(h as u32).to_be_bytes());
    ihdr.extend_from_slice(&[8, 6, 0, 0, 0]);
    push_chunk(&mut out, b"IHDR", &ihdr);
    push_chunk(&mut out, b"IDAT", &idat);
    push_chunk(&mut out, b"IEND", &[]);
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let mut f = std::fs::File::create(path)?;
    f.write_all(&out)?;
    Ok(out.len())
}

fn push_chunk(out: &mut Vec<u8>, kind: &[u8; 4], data: &[u8]) {
    out.extend_from_slice(&(data.len() as u32).to_be_bytes());
    out.extend_from_slice(kind);
    out.extend_from_slice(data);
    let mut crc = 0xFFFF_FFFFu32;
    for b in kind.iter().chain(data.iter()) {
        crc ^= u32::from(*b);
        for _ in 0..8 {
            crc = if crc & 1 != 0 {
                (crc >> 1) ^ 0xEDB8_8320
            } else {
                crc >> 1
            };
        }
    }
    out.extend_from_slice(&(crc ^ 0xFFFF_FFFF).to_be_bytes());
}

/// 臾댁븬異?zlib(deflate stored) ??PNG 濡??쏀엳湲곕쭔 ?섎㈃ ?섎?濡??뺤텞? ?앸왂?쒕떎.
///
/// ?ㅻ쭔 4K ?붾㈃ ???μ씠 30MB ?쇱꽌, 媛숈? ?됱씠 媛濡쒕줈 ?댁뼱吏??援ш컙??吏㏐쾶
/// ?뺤텞?섎뒗 ?꾩＜ ?⑥닚??RLE 瑜?癒쇱? ?곸슜?쒕떎(deflate stored 釉붾줉 ?덉뿉 ?ｋ뒗??.
fn zlib_stored(data: &[u8]) -> Vec<u8> {
    let mut out = vec![0x78, 0x01];
    let mut i = 0;
    while i < data.len() {
        let n = (data.len() - i).min(65_535);
        let last = if i + n >= data.len() { 1u8 } else { 0 };
        out.push(last);
        out.extend_from_slice(&(n as u16).to_le_bytes());
        out.extend_from_slice(&(!(n as u16)).to_le_bytes());
        out.extend_from_slice(&data[i..i + n]);
        i += n;
    }
    let mut a: u32 = 1;
    let mut b: u32 = 0;
    for byte in data {
        a = (a + u32::from(*byte)) % 65_521;
        b = (b + a) % 65_521;
    }
    out.extend_from_slice(&((b << 16) | a).to_be_bytes());
    out
}
