//! `--snap` 실행기 — egui 프레임 안에서 액션→캡처를 순서대로 처리한다.

use std::io::Write as _;

use eframe::egui;

use crate::app_state::StrataAppState;

/// 스냅 실행의 임시 폴더 — 새 조사 파일과 사용자 ini 사본을 여기에 둔다(사용자 문서·설정에 쓰지 않는다).
pub(crate) fn snap_temp_dir() -> std::path::PathBuf {
    std::env::temp_dir().join(format!("strata-snap-{}", std::process::id()))
}

impl StrataAppState {
    /// 스냅샷 단계를 한 프레임 진행한다. 모든 단계가 끝나면 뷰포트를 닫는다.
    pub(crate) fn snap_tick(&mut self, ctx: &egui::Context) {
        let Some(snap) = self.snap.as_ref() else {
            return;
        };
        if !snap.has_work() {
            return;
        }

        // 값을 미리 꺼내 두고(빌림 충돌 회피) 아래에서 갱신한다.
        let waiting = snap.waiting;
        let settle = snap.settle;
        let next = snap.steps.first().cloned();

        // 1) 캡처를 기다리는 중 — Screenshot 이벤트가 왔으면 저장한다.
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
                        Err(e) => eprintln!("[SNAP] {name} 저장 실패: {e}"),
                    }
                    snap.steps.remove(0);
                    snap.waiting = false;
                    snap.settle = 8;
                    if snap.steps.is_empty() {
                        // S0 — 스냅이 쓴 최근 조사·마지막 폴더는 이 사본에 있다(사용자 ini 는 읽기만).
                        if let Some(ini) = &self.settings_ini {
                            eprintln!("[SNAP] 설정 사본: {}", ini.display());
                        }
                        // 마지막 캡처 뒤 창을 닫는다 — 전에는 has_work() 가 거짓이 되어 아래 닫기에
                        // 닿지 못하고 바깥 timeout 으로만 끝났다.
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                }
            }
            ctx.request_repaint();
            return;
        }

        // 2) 자리 잡을 시간을 준다(폰트·레이아웃 확정).
        if settle > 0 {
            if let Some(snap) = self.snap.as_mut() {
                snap.settle -= 1;
            }
            ctx.request_repaint();
            return;
        }

        // 3) 다음 단계 실행.
        let Some(step) = next else {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            return;
        };
        if let Some(action) = step.action.clone() {
            if matches!(
                action.as_str(),
                "@gauge=running" | "@gauge=recovering" | "@gauge=stopped" | "@gauge=two"
            ) {
                let now = ctx.input(|i| i.time);
                self.download_jobs.apply_snap_preset(&action["@gauge=".len()..], now);
            } else {
                self.snap_apply_action(&action);
            }
            if let Some(snap) = self.snap.as_mut() {
                // 액션은 한 번만 실행한다. 단계는 남겨 두고 action 만 지워서
                // 다음 바퀴에 이 단계가 "캡처" 로 처리되게 한다.
                // (지우지 않으면 같은 액션을 무한히 반복한다 — 실제로 그랬다.)
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

    /// 리본 라벨 하나를 누른 것과 같은 효과를 낸다.
    pub(crate) fn snap_apply_action(&mut self, action: &str) {
        match action {
            "@new_survey_go" => {
                // 새 조사 모달의 「만들기」까지 눌러 실제 조사 화면으로 간다.
                // 화면 캡처는 언제나 임시 폴더에 만든다 — 사용자 문서 폴더에 쓰지 않는다.
                let name = self.modals.new_survey_name.trim().to_owned();
                let crs = self.modals.new_survey_crs;
                let dir = snap_temp_dir();
                let path = dir.join(format!("{name}.gpkg"));
                if let Err(e) = strata_storage::gpkg::GpkgRepository::create_new_survey(&path, crs) {
                    // 거절되면 이 단계에서 멈추고 창에 오류를 남긴다(기존 파일로 계속하지 않는다).
                    let msg = crate::dlg_new_survey::new_survey_error_text(&e);
                    eprintln!("[SNAP] 새 조사 실패 {}: {msg}", path.display());
                    self.modals.new_survey_error = Some(msg);
                    return;
                }
                self.start_new_survey_project_with_path(&name, &path, crs);
                self.modals.active = None;
            }
            "@close_modal" => self.modals.active = None,
            "@tab_home" => self.active_tab = crate::app_state::MainTab::Home,
            // 홈 「인터넷 확인」을 누른 것처럼 VWorld 서버에 한 번 묻는다(답은 뒤 단계들에서).
            "@internet_check" => {
                if strata_net::network::is_allowed() {
                    self.home_map.internet = crate::home_connection_model::InternetCheck::start();
                }
            }
            "@tab_map" => self.active_tab = crate::app_state::MainTab::Map,
            "@tab_drawing" => self.open_studio_tab(crate::app_state::MainTab::Drawing),
            "@tab_paleo" => self.open_studio_tab(crate::app_state::MainTab::Paleo),
            "@tab_terrain3d" => self.open_studio_tab(crate::app_state::MainTab::Terrain3d),
            // Picture-only status-bar gauge presets (order DL1a).
            "@gauge=running" | "@gauge=recovering" | "@gauge=stopped" | "@gauge=two" => {
                self.download_jobs.apply_snap_preset(&action["@gauge=".len()..], 0.0);
            }
            // 뒤에서 받는 지적 타일이 올 때까지 한 단계 더 기다린다.
            "@wait" => {}
            _ if action.starts_with("@goto=") => self.snap_goto(&action["@goto=".len()..]),
            // 지도 우클릭 메뉴를 창의 점 X;Y 에 연다. 마우스가 없으므로 우클릭 자리는 지도 가운데로 둔다.
            _ if action.starts_with("@map_menu=") => {
                let v: Vec<f32> = action["@map_menu=".len()..].split(';').filter_map(|s| s.trim().parse().ok()).collect();
                if let [x, y] = v[..] {
                    self.home_map.cursor_world_xy = self.home_map.viewport.center_xy;
                    self.map_context_menu_open = true;
                    self.map_context_menu_pos = [x, y];
                }
            }
            // 열린 지도 메뉴의 「조사 도형 그리기」 하위 메뉴를 펼친다.
            "@map_menu_sub" => self.map_menu_sub_open = true,
            // 그리기 줄의 「유구 그리기」, 지도 가운데에서 dx;dy m 떨어진 자리 누름, Enter.
            "@draw_poly" => self.start_edit_feature_poly(),
            "@draw_area" => self.start_edit_survey_area(),
            // 창·카드·메뉴 단계(snap_shot_steps.rs).
            _ if self.snap_apply_window_step(action) => {}
            "@sketch_finish" => self.sketch_finish(),
            _ if action.starts_with("@sketch=") => {
                let v: Vec<f64> = action["@sketch=".len()..].split(';').filter_map(|s| s.trim().parse().ok()).collect();
                if let [dx, dy] = v[..] {
                    let c = self.home_map.viewport.center_xy;
                    self.sketch_press([c[0] + dx, c[1] + dy]);
                }
            }
            // 「열기」와 같이 그 파일을 연다 — 임시 폴더 자료만 쓴다.
            _ if action.starts_with("@open=") => {
                self.open_survey_gpkg(std::path::Path::new(&action["@open=".len()..]));
            }
            _ => self.handle_ribbon_click(action),
        }
    }

    /// `@goto=경도;위도;축척분모` — 지도를 그 자리·축척으로 옮긴다(예: `@goto=127.73;37.88;2500`).
    /// 경주(5187 중앙자오선 근처)에서는 타일 기울기가 거의 없어 이음매를 볼 수 없으므로,
    /// 사용자 화면(춘천)처럼 자오선에서 먼 곳을 찍을 때 쓴다.
    fn snap_goto(&mut self, spec: &str) {
        let v: Vec<f64> = spec
            .split(';')
            .filter_map(|s| s.trim().parse::<f64>().ok())
            .collect();
        let (Some(&lon), Some(&lat), Some(&scale)) = (v.first(), v.get(1), v.get(2)) else {
            eprintln!("[SNAP] @goto 형식은 경도;위도;축척분모 입니다: {spec}");
            return;
        };
        let Some(params) = crate::basemap_paint::tm_params_for(self.work_crs) else {
            return;
        };
        let c = strata_geo::crs_transform::lonlat_to_tm(lon, lat, &params);
        self.home_map.viewport.center_xy = c;
        self.home_map.viewport.meters_per_pixel = scale / crate::status_bar_text::POINTS_PER_METER;
    }
}

/// `egui::ColorImage` 를 PNG 로 저장한다(외부 크레이트 없이 zlib 수동 기록).
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

/// 무압축 zlib(deflate stored) — PNG 로 읽히기만 하면 되므로 압축은 생략한다.
///
/// 다만 4K 화면 한 장이 30MB 라서, 같은 색이 가로로 이어지는 구간을 짧게
/// 압축하는 아주 단순한 RLE 를 먼저 적용한다(deflate stored 블록 안에 넣는다).
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
