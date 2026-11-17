//! `--snap` 의 창·카드·메뉴 단계 — snap_shot_run.rs 가 300줄을 넘어 옮겼다. 처리했으면 true.

use crate::app_state::StrataAppState;

impl StrataAppState {
    pub(crate) fn snap_apply_window_step(&mut self, action: &str) -> bool {
        match action {
            "@new_survey_fixture" => {
                self.modals.new_survey_name = "fixture".into();
                self.modals.new_survey_crs = strata_types::crs::WorkCrs::Epsg5187;
                self.snap_apply_action("@new_survey_go");
            }
            _ if action.starts_with("@heritage_fixture=") => {
                self.snap_heritage_fixture(&action["@heritage_fixture=".len()..]);
            }
            "@heritage_cancel" => {
                self.drop_heritage_run();
                self.heritage.snap_capture_pending = false;
            }
            // 열린 「이름·번호」 창에 종류(이름);시대[;번호]를 적고(번호를 안 주면 다음 번호), 「저장」·「건너뛰기」.
            _ if action.starts_with("@form=") => {
                let mut parts = action["@form=".len()..].splitn(3, ';');
                if let Some(form) = self.home_map.feature_form.as_mut() {
                    form.name = parts.next().unwrap_or_default().to_owned();
                    form.period = parts.next().unwrap_or_default().to_owned();
                    if let Some(number) = parts.next() {
                        form.number = number.to_owned();
                        form.number_typed = true;
                    }
                    form.update_number_hint();
                }
            }
            "@form_save" => self.save_feature_form(),
            "@form_skip" => self.skip_feature_form(),
            // 「조사구역 그리기 설정」 창의 「그리기 시작」.
            "@area_setup_ok" => self.accept_survey_area_setup(),
            // 「도형 속성」 창: 표;fid 로 열기, 칸=글 적기, 「저장」.
            _ if action.starts_with("@record=") => {
                if let Some((table, fid)) = action["@record=".len()..].split_once(';') {
                    self.open_feature_record(table, fid.parse().unwrap_or(0));
                }
            }
            _ if action.starts_with("@record_set=") => {
                if let (Some((field, text)), Some(form)) = (action["@record_set=".len()..].split_once('='), self.home_map.feature_record.as_mut()) {
                    if let Some(row) = form.rows.iter_mut().find(|r| r.field == field) {
                        row.text = text.to_owned();
                    }
                }
            }
            "@record_save" => self.save_feature_record(),
            // 도형선택으로 표;fid 를 고른 것처럼(카드가 따라간다), 카드 칸=글 고치기.
            _ if action.starts_with("@select=") => {
                if let Some((table, fid)) = action["@select=".len()..].split_once(';') {
                    self.home_map.selected_shape = Some((table.to_owned(), fid.parse().unwrap_or(0)));
                    self.sync_card_with_selection();
                }
            }
            _ if action.starts_with("@card_set=") => {
                if let Some((field, text)) = action["@card_set=".len()..].split_once('=') {
                    self.card_set_value(field, text);
                }
            }
            // 레이어 목록 우클릭 메뉴: layer_key 로 열기(목록 옆 자리), 하위 메뉴를 줄 번호로 열기(구분선도 한 줄).
            _ if action.starts_with("@layer_menu=") => {
                let key = &action["@layer_menu=".len()..];
                if let Some(e) = self.left_layers.entries.iter().find(|e| e.layer_key == key) {
                    self.right_inspector.selected_layer_key = Some(e.label_ko.clone());
                }
                self.layer_context_menu_key = Some(key.to_owned());
                self.layer_context_menu_pos = [180.0, 260.0];
                self.layer_context_menu_open = true;
            }
            // 레이어 메뉴 줄을 누른 것처럼: layer_key;C++ 객체 이름;값(라벨 항목·글자 크기·투명도 %).
            _ if action.starts_with("@layer_action=") => {
                let mut parts = action["@layer_action=".len()..].splitn(3, ';');
                let (key, id, value) = (parts.next().unwrap_or_default(), parts.next().unwrap_or_default(), parts.next().unwrap_or_default());
                if let Some(id) = crate::layer_menu_model::MENU_IDS.iter().find(|i| **i == id) {
                    self.run_layer_menu_action(Some(key), &crate::menu_popup::MenuPick { id, value: value.to_owned() });
                }
            }
            // 옛 지도 → 대동여지도 (the menu row itself needs a pointer).
            "@old_map_daedong" => {
                self.add_reference_tile_layer(&crate::ref_tile_layers::DAEDONGYEOJIDO);
            }
            // 옛 지도 → 1919 조선지형도 1:5만, with the stored history-GIS key.
            "@old_map_1919" => self.add_history_map_1919(),
            // A table of a GeoPackage file as a reference row: file;table;title;lines | outlines | coloured.
            _ if action.starts_with("@ref_vector=") => self.snap_reference_vector(&action["@ref_vector=".len()..]),
            // 「유산」 with 「최근 받은 자료 다시 쓰기」, from archives kept under the folder named here:
            // folder;시·도;시·군·구. Nothing is asked of the intranet.
            _ if action.starts_with("@heritage=") => {
                let mut parts = action["@heritage=".len()..].split(';').map(str::to_owned);
                self.heritage.archive_root = parts.next().map(std::path::PathBuf::from);
                let (sido, city) = (parts.next().unwrap_or_default(), parts.next().unwrap_or_default());
                self.heritage.wanted = Some(crate::heritage_run::reuse::Wanted { sido, city, reuse: true });
            }
            // The received cadastral map from the table named here, without a run at the site.
            _ if action.starts_with("@cadastral=") => self.show_cadastral_layer(std::path::PathBuf::from(&action["@cadastral=".len()..]), None),
            // 「선 색·지번 표시」 as the menu row of the 지적 button opens it.
            "@cadastral_style" => self.open_cadastral_style(),
            // The check of the sheet: names of the sites in place of their numbers at 1:5,000 and larger.
            "@sheet_names" => self.drawing.sheet.show_site_names_under_5000 = true,
            // The check of the sheet: the legend and the title block as if the user had moved them (mm of paper).
            "@sheet_moved" => {
                self.drawing.item_offsets_mm.insert(crate::tab_drawing::SheetItem::Legend, [-95.0, 150.0]);
                self.drawing.item_offsets_mm.insert(crate::tab_drawing::SheetItem::TitleBlock, [-60.0, 0.0]);
            }
            // A coordinate point dx;dy metres from the middle of the sheet's map, as if clicked there.
            _ if action.starts_with("@coord=") => {
                let v: Vec<f64> = action["@coord=".len()..].split(';').filter_map(|s| s.trim().parse().ok()).collect();
                if let [dx, dy] = v[..] {
                    let c = self.sheet_map_center();
                    self.drawing.coord_callouts.push(crate::tab_drawing::CoordCallout { ground: [c[0] + dx, c[1] + dy] });
                }
            }
            // 「PDF 내보내기」 with the file named here in place of the save dialog (a temp folder only).
            _ if action.starts_with("@pdf=") => self.start_sheet_pdf(std::path::PathBuf::from(&action["@pdf=".len()..])),
            // Files dropped on the window, `;` between them (a drop cannot be made by a script otherwise).
            _ if action.starts_with("@drop=") => {
                let paths: Vec<std::path::PathBuf> = action["@drop=".len()..].split(';').map(std::path::PathBuf::from).collect();
                self.drop_paths(&paths);
            }
            // The DEM the survey folder has, without an order at the portal.
            "@dem" => self.show_survey_dem(),
            // 「DEM 표현」: the dialog as the menu row opens it, and 적용 with preset;relief;times;per cent.
            "@dem_dialog" => self.open_dem_look_dialog(),
            // 「국토지리원 DEM 불러오기」 with the file named here in place of the file dialog.
            _ if action.starts_with("@dem_import=") => {
                self.import_dem_file(std::path::Path::new(&action["@dem_import=".len()..]), |file| strata_engine::check_elevation_file(file).map_err(|e| e.0));
                eprintln!("[SNAP] DEM 불러오기: {:?}", self.home_map.draw_guide_message);
            }
            _ if action.starts_with("@dem_look=") => self.snap_dem_look(&action["@dem_look=".len()..]),
            _ if action.starts_with("@layer_menu_path=") => {
                self.home_map.layer_menu_path = action["@layer_menu_path=".len()..].split(',').filter_map(|s| s.trim().parse().ok()).collect();
            }
            _ => return false,
        }
        true
    }

    /// Offline screenshot of the real app runner, with a loopback fixture and a temporary
    /// survey only (`target/cpp-ref/tests/test_heritage_browser.cpp:25-35`, HER-CODEX-UI1).
    fn snap_heritage_fixture(&mut self, base: &str) {
        let local = base.strip_prefix("http://").and_then(|address| address.parse::<std::net::SocketAddr>().ok())
            .is_some_and(|address| address.ip().is_loopback());
        let temporary = self.survey_path.as_ref().is_some_and(|path| path.starts_with(crate::snap_shot_run::snap_temp_dir()));
        if self.snap.is_none() || !local || !temporary || strata_net::network::is_allowed() {
            eprintln!("[SNAP] heritage fixture requires offline loopback and a temporary survey");
            return;
        }
        self.accounts.config_dir = crate::snap_shot_run::snap_temp_dir().join("fixture-settings");
        self.accounts.heritage_configured = true;
        self.accounts.heritage_creds.username = "user1".into();
        self.accounts.heritage_creds.password = "pass1".into();
        self.heritage.archive_root = Some(crate::snap_shot_run::snap_temp_dir().join("fixture-archives"));
        self.modals.heritage_sido = "경상북도".into();
        self.modals.heritage_city = "경주시".into();
        crate::heritage_run_start::start_heritage_fetch(self);
        if let Some(download) = self.heritage.download.as_mut() {
            download.set_fixture_base(base.to_owned());
        }
        self.heritage.snap_capture_pending = true;
    }

    fn snap_dem_look(&mut self, text: &str) {
        use crate::topo_download::dem::look::{DemLook, DemPreset};
        let mut parts = text.split(';');
        let preset = match parts.next() {
            Some("lowland") => DemPreset::Lowland,
            Some("viewport") => DemPreset::Viewport,
            _ => DemPreset::National,
        };
        let relief = parts.next() != Some("0");
        let z_factor = parts.next().and_then(|s| s.parse().ok()).unwrap_or(1.0);
        let strength_pct = parts.next().and_then(|s| s.parse().ok()).unwrap_or(30);
        let said = self.apply_dem_look(DemLook { preset, relief, z_factor, strength_pct });
        eprintln!("[SNAP] DEM 표현 {text}: {said} -> {:?}", self.ref_vectors.look_of(crate::topo_download::dem::DEM_KEY));
    }

    /// The looks are those of the layers that use them: sheet lines (`KaTopographicImportDialog.cpp:204-208`),
    /// parcels, and polygons coloured by their `ka_color` column (`GeologyMapService.cpp:382-458`).
    fn snap_reference_vector(&mut self, text: &str) {
        use crate::ref_vector::data::{LoadSpec, Look, PolygonFill};
        let mut parts = text.splitn(4, ';');
        let (file, table, title) = (parts.next().unwrap_or_default(), parts.next().unwrap_or_default(), parts.next().unwrap_or_default());
        let look = match parts.next() {
            Some("coloured") => Look::Polygons { stroke: [80, 80, 80, 130], width_mm: 0.12, fill: PolygonFill::Column { name: "ka_color", alpha: 220, missing: [200, 200, 200] } },
            Some("outlines") => Look::Polygons { stroke: [80, 80, 80, 255], width_mm: 0.2, fill: PolygonFill::None },
            _ => Look::Lines { stroke: [128, 128, 128, 255], width_mm: 0.2 },
        };
        let spec = LoadSpec { file: file.into(), table: table.to_owned(), work_epsg: self.work_crs.epsg_code(), look, label: None };
        self.show_reference_vector_layer(&format!("ref_snap_{table}"), title, spec);
    }
}
