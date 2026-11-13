//! `--snap` 의 창·카드·메뉴 단계 — snap_shot_run.rs 가 300줄을 넘어 옮겼다. 처리했으면 true.

use crate::app_state::StrataAppState;

impl StrataAppState {
    pub(crate) fn snap_apply_window_step(&mut self, action: &str) -> bool {
        match action {
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
            _ if action.starts_with("@layer_menu_path=") => {
                self.home_map.layer_menu_path = action["@layer_menu_path=".len()..].split(',').filter_map(|s| s.trim().parse().ok()).collect();
            }
            _ => return false,
        }
        true
    }
}
