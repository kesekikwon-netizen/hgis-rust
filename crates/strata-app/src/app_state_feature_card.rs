//! 오른쪽 카드가 보이는 도형과 그 고침 — C++ `KaFeatureCard::showSelection`·`setFeature`·`commitTyping`·`setValue`
//! (KaFeatureCard.cpp:155-210, KaFeatureCardEdit.cpp:50-100)와 `MainWindow::syncFeatureCard`(MainWindowFeatureCard.cpp:70-85).
//! 카드에 적는 중인 글(draft)은 Enter·칸을 떠날 때, 그리고 다른 도형으로 바뀌기·새 도형 넣기·「도형 속성」 열기·저장 전에 쓴다.

use std::collections::BTreeMap;

use crate::app_state::StrataAppState;
use crate::feature_card::{card_value, card_view, CardView};

/// 카드가 보이는 도형. `follows_selection` 이면 고른 것이 없어질 때 카드도 비운다(그린 도형의 카드는 남는다).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CardTarget {
    pub(crate) table: String,
    pub(crate) fid: i64,
    pub(crate) follows_selection: bool,
}

impl StrataAppState {
    /// 카드에 그 도형을 보인다 — 적던 글은 먼저 지금 도형에 쓴다(showSelection·setFeature 앞의 commitTyping).
    pub(crate) fn set_card_target(&mut self, table: &str, fid: i64, follows_selection: bool) {
        self.commit_card_typing();
        self.home_map.card = Some(CardTarget { table: table.to_owned(), fid, follows_selection });
        self.home_map.card_failure = None;
    }

    pub(crate) fn clear_card(&mut self) {
        self.commit_card_typing();
        self.home_map.card = None;
        self.home_map.card_failure = None;
    }

    /// syncFeatureCard — 그리기 도구가 켜져 있으면 그대로, 도형 하나를 고르면 그것, 고른 것이 없으면 따라가던 카드만 비운다.
    pub(crate) fn sync_card_with_selection(&mut self) {
        if self.active_tool == strata_canvas2d::map_tools::MapToolKind::DrawPolygon {
            return;
        }
        match self.home_map.selected_shape.clone() {
            Some((table, fid)) => self.set_card_target(&table, fid, true),
            None if self.home_map.card.as_ref().is_some_and(|c| c.follows_selection) => self.clear_card(),
            None => {}
        }
    }

    /// 도형이 없어졌으면(되돌리기·저장으로 번호가 바뀜·지움) 카드를 비운다 — 다른 도형을 보이지 않게.
    pub(crate) fn drop_card_if_gone(&mut self) {
        let gone = self.home_map.card.as_ref().is_some_and(|c| {
            !self.survey_tables.iter().any(|t| t.table == c.table && t.layer.features.iter().any(|f| f.fid == c.fid))
        });
        if gone {
            self.home_map.card = None;
            self.home_map.card_drafts.clear();
            self.home_map.card_failure = None;
        }
    }

    /// 카드가 지금 보일 값.
    pub(crate) fn card_view_now(&self) -> Option<CardView> {
        let c = self.home_map.card.as_ref()?;
        let t = self.survey_tables.iter().find(|t| t.table == c.table)?;
        let name = self.left_layers.entries.iter().find(|e| e.layer_key == c.table).map(|e| e.label_ko.clone()).unwrap_or_default();
        card_view(t, c.fid, &name, self.home_map.card_failure.as_deref())
    }

    /// 카드 칸에 적는 중인 글.
    pub(crate) fn card_draft(&mut self, field: &str, text: String) {
        self.home_map.card_drafts.insert(field.to_owned(), text);
    }

    /// commitTyping — 적던 글을 모두 지금 도형에 쓴다.
    pub(crate) fn commit_card_typing(&mut self) {
        let drafts: BTreeMap<String, String> = std::mem::take(&mut self.home_map.card_drafts);
        for (field, text) in drafts {
            self.card_set_value(&field, &text);
        }
    }

    /// setValue — 바뀌었으면 「기록 고치기」 한 단계(값 + updated_at), 같으면 아무것도. 창(모달)이 열려 있으면 받지 않는다
    /// (C++ 은 창 뒤의 카드를 누를 수 없다). 못 바꾸면 상태 칩에 「바꾸지 못했습니다: …」. 상태줄 글은 없다(C++ edited).
    pub(crate) fn card_set_value(&mut self, field: &str, text: &str) -> bool {
        if self.modal_window_open() {
            return false;
        }
        let Some(c) = self.home_map.card.clone() else { return false };
        let Some(idx) = self.survey_tables.iter().position(|t| t.table == c.table) else { return false };
        match card_value(&self.survey_tables[idx], c.fid, field, text) {
            Ok(None) => false,
            Ok(Some(values)) => {
                if !self.survey_tables[idx].layer.change_attributes(c.fid, &values) {
                    self.home_map.card_failure = Some("도형을 찾지 못했습니다.".to_owned());
                    return false;
                }
                self.redo_tables.clear();
                self.undo_tables.push(c.table.clone());
                self.is_dirty = true;
                self.home_map.card_failure = None;
                self.after_layer_edit(&c.table);
                true
            }
            Err(e) => {
                self.home_map.card_failure = Some(e);
                false
            }
        }
    }
}

#[cfg(test)]
#[path = "app_state_feature_card_tests.rs"]
mod tests;
