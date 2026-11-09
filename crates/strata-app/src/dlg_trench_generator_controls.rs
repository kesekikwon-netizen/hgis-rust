//! 「시굴격자 속성」 대화상자 — 조사 종류·사면·좌표격자·행/열·둑·방위 다이얼.
//!
//! `dlg_trench_generator.rs` 가 300줄 규칙에 맞춰 나눈 파일이다 (`KaTrenchDialog.cpp:27-155`).
//! 원래 창 안의 앞부분을 **그대로** 옮겼고, 위젯 순서·수치·문구는 하나도 바꾸지 않았다.
//! 뒤쪽 요약이 같은 `manual`/`kind` 를 쓰므로 이 함수가 둘을 돌려준다.

use eframe::egui::{self, Color32, Pos2, Stroke, Vec2};

use crate::app_state::StrataAppState;
use strata_geo::trench_grid::{TrenchShapeKind, TrenchStandardLength};

pub(crate) const KIND_LABELS: [&str; 3] = [
    "시굴조사 — 전체 면적의 10%",
    "표본조사 — 전체 면적의 2%",
    "직접 지정 — 규격·둑을 내가 정함",
];

/// 앞부분을 그리고 `(manual, kind)` 를 돌려준다 — 요약이 같은 두 값을 그대로 쓴다.
pub(crate) fn render_trench_controls(
    app: &mut StrataAppState,
    ui: &mut egui::Ui,
) -> (bool, usize) {
// KaTrenchDialog.cpp:27-155 조사 종류 3단 순환

ui.horizontal(|ui| {
    ui.label(egui::RichText::new("조사 종류").size(13.0));
    if ui
        .add_sized(
            [260.0, 28.0],
            egui::Button::new(format!(
                "{}  ▼",
                KIND_LABELS[app.modals.trench_survey_kind_idx.min(2)]
            )),
        )
        .clicked()
    {
        app.modals.trench_survey_kind_idx =
            (app.modals.trench_survey_kind_idx + 1) % 3;
    }
})
.response
.on_hover_text(
    "시굴은 조사구역의 10%, 표본은 2%로 배치합니다. 폭 2 m·길이 20 m 이내에서 길이와 둑 간격을 맞추며, 모든 격자가 구역 안에 들어갑니다.",
);

ui.checkbox(
    &mut app.modals.trench_slope_align,
    egui::RichText::new("지형 사면 방향으로 넣기(등고선 직교)").size(13.0),
)
.on_hover_text(
    "트렌치 장축을 오르막 쪽으로 돌립니다. 등고선과 나란히 넣으면 같은 층만 따라가 층서를 못 읽습니다. DEM을 올려야 씁니다.",
);
// KaTrenchDialog.cpp:54-57,227-237 — m_terrainInfo
ui.label(
    egui::RichText::new(
        if app.modals.trench_slope_align {
            "지형: 오르막 135° · 경사 4.2% — 트렌치 장축을 이 방향으로 넣습니다."
        } else {
            "지형 자료가 없어 남북으로 깝니다(DEM·수치지형도 등고선 없음). 방위 칸에 다른 값을 넣으면 그 방향으로 깝니다."
        },
    )
    .size(11.5)
    .color(Color32::from_rgb(95, 94, 88)),
);

ui.horizontal(|ui| {
    ui.label(egui::RichText::new("모양").size(13.0));
    let shape_lbl = format!("{}  ▼", app.modals.trench_shape.label_ko());
    if ui
        .add_sized([286.0, 28.0], egui::Button::new(shape_lbl))
        .on_hover_text(
            "ㄱ·ㄴ자는 팔 15 m 그대로 구역 안에 온전히 들어가는 것만 놓아, 비율이 목표를 조금 넘습니다.",
        )
        .clicked()
    {
        let idx = TrenchShapeKind::ALL
            .iter()
            .position(|&k| k == app.modals.trench_shape)
            .unwrap_or(0);
        app.modals.trench_shape =
            TrenchShapeKind::ALL[(idx + 1) % TrenchShapeKind::ALL.len()];
    }
});
if app.modals.trench_shape == TrenchShapeKind::Straight {
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("트렌치 길이").size(13.0));
        for len_opt in [
            TrenchStandardLength::M20,
            TrenchStandardLength::M15,
            TrenchStandardLength::M10,
        ] {
            ui.radio_value(
                &mut app.modals.trench_length,
                len_opt,
                egui::RichText::new(len_opt.label_ko()).size(13.0),
            );
        }
    });
}
ui.checkbox(
    &mut app.modals.trench_grid_snap,
    egui::RichText::new("좌표격자에 맞추기").size(13.0),
)
.on_hover_text(
    "ㄱ·ㄴ자 시굴·표본에서 꺾인 바깥 모서리를 좌표격자 20 m 교점에, 방향을 좌표격자 방향에 맞춥니다. 트렌치끼리는 1 m 이상 띄웁니다.",
);

// KaTrenchDialogPlan.cpp:80-92 행/열 은 수동 배치에서만, 둑/접두 는
// 수동 배치이거나 '직접 지정'(idx 2)일 때 보인다.
let manual = !app.modals.trench_auto_place;
let kind = app.modals.trench_survey_kind_idx.min(2);
if manual {
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("행").size(13.0));
        ui.add(
            egui::DragValue::new(&mut app.modals.trench_rows)
                .range(1..=200)
                .speed(0.2),
        );
        ui.add_space(10.0);
        ui.label(egui::RichText::new("열").size(13.0));
        ui.add(
            egui::DragValue::new(&mut app.modals.trench_cols)
                .range(1..=200)
                .speed(0.2),
        );
    });
}
if manual || kind == 2 {
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("둑(간격)").size(13.0));
        ui.add(
            egui::DragValue::new(&mut app.modals.trench_balk_m)
                .range(0.0..=200.0)
                .speed(0.5)
                .suffix(" m"),
        )
        .on_hover_text(
            "트렌치 사이 간격입니다. 간격을 키우면 시굴 비율이 내려갑니다.",
        );
        ui.add_space(10.0);
        ui.label(egui::RichText::new("이름 접두").size(13.0));
        ui.add(
            egui::TextEdit::singleline(&mut app.modals.trench_prefix)
                .desired_width(90.0),
        );
    });
}

// KaAzimuthDial.cpp:24-49 회전(방위) 다이얼
ui.horizontal(|ui| {
    ui.label(egui::RichText::new("회전(방위)").size(13.0));
    ui.add(
        egui::DragValue::new(&mut app.modals.trench_azimuth_deg)
            .range(0.0..=360.0)
            .speed(1.0)
            .suffix(" °"),
    )
    .on_hover_text(
        "격자 회전(북 기준 시계 방향). 「구역에 깔기」를 누르면 회전된 격자로 다시 배치합니다.",
    );
    let (cr, _) =
        ui.allocate_exact_size(Vec2::new(22.0, 22.0), egui::Sense::hover());
    let cp = ui.painter_at(cr);
    cp.circle_stroke(
        cr.center(),
        9.0,
        Stroke::new(1.2, Color32::from_rgb(120, 118, 110)),
    );
    let rad = (app.modals.trench_azimuth_deg as f32).to_radians();
    let tip = Pos2::new(
        cr.center().x + rad.sin() * 8.0,
        cr.center().y - rad.cos() * 8.0,
    );
    cp.line_segment([cr.center(), tip], Stroke::new(1.8, Color32::BLACK));
    cp.circle_filled(cr.center(), 2.0, Color32::BLACK);
    let turn = app.modals.trench_azimuth_deg.rem_euclid(360.0);
    let dial_note = if turn < 0.5 || (360.0 - turn) < 0.5 {
        "위가 북쪽입니다".to_owned()
    } else {
        format!("북에서 시계 방향 {turn:.0}°")
    };
    ui.label(
        egui::RichText::new(dial_note)
            .size(12.0)
            .color(Color32::from_rgb(95, 94, 88)),
    );
});
    (manual, kind)
}
