# Editing Windows Parity Survey (C1–C11, C13–C14)

Read-only control-by-control comparison between the C++ Strata app (`target/cpp-ref/src/app`) and the Rust/egui port (`crates/strata-app/src`).
In the C++ app, `kaPaperLook` is the active theme property (`KaTheme.cpp:272`); where `KaControlPointDialog` or `KaControlCsvBox` branches on `kaPaperLook`, both are noted and the `kaPaperLook` layout is used as the primary reference.

---

## C1 「GPS 기준점」

- **C++ title & build**: `"GPS 기준점"` — `target/cpp-ref/src/app/KaControlPointDialog.cpp:59` (`KaControlPointDialog::KaControlPointDialog` at `:54-132`)
- **How user opens it**:
  - **C++**: Submit-check shortcut action `"add_control_point"` -> `MainWindow::addControlPoint()` (`KaSubmitFlow.cpp:251`, `MainWindowEditing.cpp:1392`)
  - **Rust**: Submit-check shortcut action `"add_control_point"` (`modal_export_table.rs:94-98`), or debug command (`debug_control.rs:108`) -> sets `ActiveModalDialog::ControlPoint` (`modal_control.rs:10`)
- **Modality**:
  - **C++**: Modal (`QDialog`, `dlg.exec()` at `MainWindowEditing.cpp:1395`)
  - **Rust**: Floating `egui::Window::new("ka_modal_control_point")` (`modal_control.rs:21`); `modal_window_open()` (`app_state_feature_record.rs:10-15`) does **not** include `modals.active`, so background shortcuts are only partially gated by `self.modals.active.is_none()` in `gui_shell.rs:30`.
- **Size rules**:
  - **C++**: `KaFormPlate::dress` (`KaFormPlate.cpp:13`) sets `minimumWidth(440)`; `m_x`, `m_y`, `m_z` have `setFixedWidth(176)` (`KaControlPointDialog.cpp:114`); margins `(24, 24, 24, 16)`, row spacing `12`; resizable by layout.
  - **Rust**: Non-resizable (`resizable(false)`), `fixed_size([modal_w, modal_h])` where `modal_w = 480.0.min(vis_w - 40.0)`, `modal_h = 560.0.min(vis_h - 40.0)` (`modal_control.rs:15-25`).
- **Rust counterpart**: `render_control_point` — `crates/strata-app/src/modal_control.rs:10-265`

| # | kind | C++ text (verbatim) | C++ tooltip / placeholder (verbatim) | C++ default, range, step, suffix | C++ enabled-when rule | C++ what it does | C++ file:line | Rust text | Rust tooltip / placeholder | Rust default, range, step, suffix | Rust enabled-when rule | Rust what it does | Rust file:line | same? |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 1 | label (hint) | `LayerOps::controlPointAxisHint()` (`"이 프로그램은 X를 동쪽, Y를 북쪽으로 읽습니다. 한국 측량 성과표는 흔히 X=북쪽, Y=동쪽입니다. 그 값이면 X·Y 교환을 고르세요."`, with `". "` -> `".\n"` in paper look) | — | — | always | Explains X=East, Y=North convention | `KaControlPointDialog.cpp:73-77`, `ControlPointCsv.cpp:552-556` | `"아래 동쪽·북쪽 값은 현재 조사 좌표계 기준 미터(m)입니다. (세계측지계 · GRS80 · 동부원점 (EPSG:5187))"` | — | — | always | Hardcoded note (ignores actual work CRS if 5186) | `modal_control.rs:30-36` | no (text) |
| 2 | group / section | `"위치"` | — | — | always | Section heading + hairline rule | `KaControlPointDialog.cpp:105` | `"위치"` (inside `Frame::group`) | — | — | always | Boxed group frame instead of hairline sectionHead | `modal_control.rs:40-43` | no (visual frame) |
| 3 | label + line edit | `"점ID"` | placeholder: none | `""` | always | Point ID input (`m_id`) | `KaControlPointDialog.cpp:60,109` | `"점ID"` | placeholder: `"예: GCP-01"` | `""` | always | Edits `app.modals.cp_id` | `modal_control.rs:49-55` | no (placeholder) |
| 4 | label + line edit | `"X (동쪽)"` | placeholder: none | `""` | always | X coordinate input (`m_x`), fixed width 176 | `KaControlPointDialog.cpp:61,110,114` | `"X (동쪽)"` (changes to `"X (북쪽·m)"` when swap checked) | — | `""` | always | Edits `app.modals.cp_x` | `modal_control.rs:57-67` | no (text changes on swap) |
| 5 | label + line edit | `"Y (북쪽)"` | placeholder: none | `""` | always | Y coordinate input (`m_y`), fixed width 176 | `KaControlPointDialog.cpp:62,111,114` | `"Y (북쪽)"` (changes to `"Y (동쪽·m)"` when swap checked) | — | `""` | always | Edits `app.modals.cp_y` | `modal_control.rs:69-79` | no (text changes on swap) |
| 6 | check box | `"X·Y 교환 (측량 X=북, Y=동)"` | — | unchecked (`false`) | always | Swaps X and Y on accept (`m_swap`) | `KaControlPointDialog.cpp:72,112` | `"X·Y 교환 (측량 X=북, Y=동)"` | — | unchecked (`false`) | always | Toggles `app.modals.cp_swap_axes` | `modal_control.rs:82-85` | yes |
| 7 | label + line edit | `"Z (표고 m, 선택)"` | placeholder: `"비워 두면 표고 없이 넣습니다"` | `""` | always | Optional elevation Z (`m_z`), fixed width 176 | `KaControlPointDialog.cpp:63-64,113-114` | `"Z (표고 m, 선택)"` | placeholder: `"비워 두면 표고 없이 넣습니다"` | `""` | always | Edits `app.modals.cp_z` | `modal_control.rs:88-94` | yes |
| 8 | section head | `"측량 기준"` | — | — | always | Section heading + hairline rule | `KaControlPointDialog.cpp:122` | `"측량 기준"` | — | — | always | Section heading + `Separator::horizontal` | `modal_control.rs:101-109` | yes |
| 9 | caption + line edit | `"측지기준계"` | — | `"세계측지계"` | always | Datum input (`m_datum`) | `KaControlPointDialog.cpp:65,124` | `"측지기준계"` | — | `"세계측지계"` | always | Edits `app.modals.cp_datum` | `modal_control.rs:113-121` | yes |
| 10 | caption + line edit | `"타원체"` | — | `"GRS80"` | always | Ellipsoid input (`m_ellipsoid`) | `KaControlPointDialog.cpp:66,124` | `"타원체"` | — | `"GRS80"` | always | Edits `app.modals.cp_ellipsoid` | `modal_control.rs:122-130` | yes |
| 11 | caption + line edit | `"투영"` | — | `"TM/UTM-K"` | always | Projection input (`m_projection`) | `KaControlPointDialog.cpp:67,125` | `"투영"` | — | `"TM/UTM-K"` | always | Edits `app.modals.cp_projection` | `modal_control.rs:131-139` | yes |
| 12 | caption + line edit (C++) / combo box (Rust) | `"원점"` | — | `""` (line edit) | always | Origin text input (`m_origin`) | `KaControlPointDialog.cpp:68,125` | `"원점"` | — | `""` (ComboBox: `"서부원점"`, `"중부원점"`, `"동부원점"`, `"동해원점"`, `"단일원점(UTM-K)"`) | always | Selects `app.modals.cp_origin` | `modal_control.rs:140-165` | no (kind: line edit vs combo box) |
| 13 | section head | `"수신"` | — | — | always | Section heading + hairline rule | `KaControlPointDialog.cpp:127` | `"수신"` | — | — | always | Section heading + `Separator::horizontal` | `modal_control.rs:170-178` | yes |
| 14 | caption + line edit | `attributeFieldLabelKo("accuracy_m")` = `"정확도(m)"` | — | `"1.0"` | always | Accuracy input (`m_accuracy`) | `KaControlPointDialog.cpp:69,129`, `MainWindowEditing.cpp:306` | `"정확도(m)"` | — | `"1.0"` | always | Edits `app.modals.cp_accuracy` | `modal_control.rs:182-190` | yes |
| 15 | caption + line edit | `"PDOP"` | — | `"1.5"` | always | PDOP input (`m_pdop`) | `KaControlPointDialog.cpp:70,129` | `"PDOP"` | — | `"1.5"` | always | Edits `app.modals.cp_pdop` | `modal_control.rs:191-199` | yes |
| 16 | caption + line edit (C++) / combo box (Rust) | `attributeFieldLabelKo("fix_type")` = `"수신상태"` | — | `"RTK"` (line edit) | always | Fix type input (`m_fix`) | `KaControlPointDialog.cpp:71,129`, `MainWindowEditing.cpp:308` | `"수신 상태"` (with space) | — | `"RTK"` (ComboBox: `"RTK"`, `"PPK"`, `"DGPS"`, `"단독측위"`) | always | Selects `app.modals.cp_fix` | `modal_control.rs:200-217` | no (text `"수신상태"` vs `"수신 상태"`, kind: line edit vs combo box) |
| 17 | button (Accept) | `"확인"` (`QDialogButtonBox::Ok`) | — | — | always | Accepts dialog -> validates & adds control point | `KaControlPointDialog.cpp:78-79` | `"지도에 추가 (확인)"` | — | — | always | Validates & pushes to `control_points_world` | `modal_control.rs:228-261` | no (text) |
| 18 | button (Reject) | `"취소"` (`QDialogButtonBox::Cancel`) | — | — | always | Rejects and closes dialog | `KaControlPointDialog.cpp:78,80` | `"취소"` | — | — | always | Sets `app.modals.active = None` | `modal_control.rs:225-227` | yes |

- **Keyboard & closing**:
  - **C++**: Default button is `Ok` (`"확인"`). `Enter` accepts; `Esc` rejects. Tab order: `m_id` -> `m_x` -> `m_y` -> `m_swap` -> `m_z` -> `m_datum` -> `m_ellipsoid` -> `m_projection` -> `m_origin` -> `m_accuracy` -> `m_pdop` -> `m_fix` (`KaControlPointDialog.cpp:115-116` explicitly orders `m_y -> m_swap -> m_z`). Button box order: `[확인] [취소]` (Windows `QDialogButtonBox`). Window X button rejects.
  - **Rust**: No Enter handler (`Enter` does not submit). `Esc` closes `app.modals.active = None` via `gui_shell.rs:34-36`. No explicit tab order. Button order (right-to-left): `[지도에 추가 (확인)] [취소]`. Header `×` button sets `open = false` (`dialogs_modal.rs:59-61`).
- **Messages**:
  - Validation error (empty `point_id` or non-numeric `x`/`y`/`z`):
    - C++: `KaUserError::warn` modal (`MainWindowEditing.cpp:1404-1409`): title `"GPS 기준점"`, heading `"기준점을 넣지 않았습니다."`, body `"점ID와 X·Y는 꼭 적어야 하고, X·Y·Z는 숫자여야 합니다."`, action `"점ID와 좌표를 숫자로(소수점은 .) 적은 뒤 다시 넣으세요."`
    - Rust: Writes combined string to `app.home_map.draw_guide_message` (status bar, not modal) (`modal_control.rs:242-245`): `"기준점을 넣지 않았습니다. 점ID와 X·Y는 꼭 적어야 하고, X·Y·Z는 숫자여야 합니다. 점ID와 좌표를 숫자로(소수점은 .) 적은 뒤 다시 넣으세요."`
  - Add failure:
    - C++: `KaUserError::warn` (`MainWindowEditing.cpp:1456-1461`): `"기준점 「%1」을 넣지 못했습니다."`
    - Rust: `MISSING` (pushes directly to `Vec`)
  - Success:
    - C++: Status bar (`MainWindowEditing.cpp:1467-1470`): `"기준점 %1을 넣었습니다 (모두 %2개). Ctrl+Z로 되돌리고, 「저장」(Ctrl+S)으로 파일에 씁니다."`
    - Rust: `draw_guide_message` (`modal_control.rs:256-259`): `"GPS 기준점 「{id}」 (X={east_x:.3}, Y={north_y:.3}, {} · {}) 지도에 추가됨"`

---

## C2 「기준점 미리보기」

- **C++ title & build**: `"기준점 미리보기"` — `target/cpp-ref/src/app/KaControlCsvBox.cpp:10` (built in `MainWindow::importControlCsv()` at `MainWindow.cpp:2557-2563`)
- **How user opens it**:
  - **C++**: Ribbon `더보기` menu -> `"CSV 기준점"` (`MainWindowRibbon.cpp:641-642`) -> `QFileDialog` -> `MainWindow::importControlCsv()` (`MainWindow.cpp:2538`)
  - **Rust**: Ribbon `더보기` menu -> `"CSV 기준점"` (`gui_ribbon_menus_b.rs:33`), but clicking it falls through to `else { self.modals.active = Some(ActiveModalDialog::AccountSettings); }` (`gui_ribbon_menus_b.rs:113-115`)
- **Modality**:
  - **C++**: Modal (`QMessageBox::exec()`, `MainWindow.cpp:2563`)
  - **Rust**: `MISSING`
- **Size rules**:
  - **C++**: Standard `QMessageBox` auto-sized (`kaPlate` property set at `KaControlCsvBox.cpp:22`)
  - **Rust**: `MISSING`
- **Rust counterpart**: `MISSING` (no CSV control-point preview dialog; clicking `"CSV 기준점"` opens `AccountSettings` at `gui_ribbon_menus_b.rs:114`)

| # | kind | C++ text (verbatim) | C++ tooltip / placeholder (verbatim) | C++ default, range, step, suffix | C++ enabled-when rule | C++ what it does | C++ file:line | Rust text | Rust tooltip / placeholder | Rust default, range, step, suffix | Rust enabled-when rule | Rust what it does | Rust file:line | same? |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 1 | label (main text) | If `swapSuggested`: `"X·Y가 바뀐 것 같습니다."`; else `"기준점 %1개를 읽었습니다."` (arg: `count`) | — | — | always | Headline of preview box | `KaControlCsvBox.cpp:16,25` | `MISSING` | — | — | — | — | — | MISSING |
| 2 | label (informative text) | `summary` lines (encoding·count, skipped rows, no-header note, Z column note, duplicate/same-name notes, axis hint or EPSG:4326 note, sample rows, distance to survey extent, swap reason) | — | — | always | Detailed CSV preview summary | `KaControlCsvBox.cpp:17-26`, `ControlPointCsv.cpp:527-548` | `MISSING` | — | — | — | — | — | MISSING |
| 3 | button (AcceptRole) | `"이대로 가져오기"` | — | default if `!preview.swapSuggested` | always | Imports CSV without swapping X/Y | `MainWindow.cpp:2559,2562` | `MISSING` | — | — | — | — | — | MISSING |
| 4 | button (ActionRole) | `"X·Y 바꿔서 가져오기"` | — | default if `preview.swapSuggested` | always | Imports CSV with X and Y swapped | `MainWindow.cpp:2560,2562` | `MISSING` | — | — | — | — | — | MISSING |
| 5 | button (RejectRole) | `"취소"` | — | — | always | Cancels CSV import | `MainWindow.cpp:2561` | `MISSING` | — | — | — | — | — | MISSING |

- **Keyboard & closing**:
  - **C++**: Default button is `"X·Y 바꿔서 가져오기"` when `preview.swapSuggested` is true, else `"이대로 가져오기"` (`MainWindow.cpp:2562`). `Enter` clicks default button; `Esc` / window X clicks `"취소"` (`RejectRole`). Button order: `AcceptRole`, `ActionRole`, `RejectRole`.
  - **Rust**: `MISSING`
- **Messages**:
  - CSV read failure: `KaUserError::warn` (`MainWindow.cpp:2549-2554`): title `"CSV"`, heading `"기준점 CSV를 읽지 못했습니다."`, detail `err` or `"파일 형식이나 좌표 칸을 확인하지 못했습니다."`, action `"CSV에 X·Y(또는 경도·위도) 칸이 있는지 확인한 뒤 다시 가져오세요."` -> Rust `MISSING`.
  - CSV import failure: `KaUserError::warn` (`MainWindow.cpp:2567`): `"기준점 CSV를 불러오지 못했습니다"` -> Rust `MISSING`.
  - Success: Status bar (`MainWindow.cpp:2573`): `"CSV 기준점 %1개 저장 (합 %2)"` -> Rust `MISSING`.

---

## C3 「기준점 축」

- **C++ title & build**: `"기준점 축"` — `target/cpp-ref/src/app/MainWindowEditing.cpp:1418` (built in `MainWindow::addControlPoint()` at `:1416-1427`)
- **How user opens it**:
  - **C++**: Shown automatically after accepting `KaControlPointDialog` (C1) when `LayerOps::suggestControlPointAxisSwap` returns `ok && !summary.isEmpty()` (`MainWindowEditing.cpp:1414-1415`)
  - **Rust**: `MISSING`
- **Modality**:
  - **C++**: Modal (`QMessageBox::exec()`, `MainWindowEditing.cpp:1424`)
  - **Rust**: `MISSING`
- **Size rules**:
  - **C++**: Standard `QMessageBox` auto-sized
  - **Rust**: `MISSING`
- **Rust counterpart**: `MISSING` (`modal_control.rs:248-260` applies `cp_swap_axes` directly without checking survey extent or showing `"기준점 축"`)

| # | kind | C++ text (verbatim) | C++ tooltip / placeholder (verbatim) | C++ default, range, step, suffix | C++ enabled-when rule | C++ what it does | C++ file:line | Rust text | Rust tooltip / placeholder | Rust default, range, step, suffix | Rust enabled-when rule | Rust what it does | Rust file:line | same? |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 1 | icon + label | Icon: `Warning` if `swapSuggested`, else `Information`. Text: `suggestion.summary` (axis hint + `"입력  X=%1  Y=%2"` + distance to survey extent or no-extent note + optional `"X·Y가 바뀐 것 같습니다. 교환한 쪽이 조사구역에 더 가깝습니다."`) | — | — | always | Shows axis-swap check & distances | `MainWindowEditing.cpp:1417,1419`, `ControlPointCsv.cpp:568-583` | `MISSING` | — | — | — | — | — | MISSING |
| 2 | button (AcceptRole) | `"이대로 넣기"` | — | default if `!(suggestion.swapSuggested || swap)` | always | Keeps X, Y as entered (`swap = false`) | `MainWindowEditing.cpp:1420,1423` | `MISSING` | — | — | — | — | — | MISSING |
| 3 | button (ActionRole) | `"X·Y 교환"` | — | default if `suggestion.swapSuggested || swap` | always | Swaps X and Y (`swap = true`) | `MainWindowEditing.cpp:1421,1423` | `MISSING` | — | — | — | — | — | MISSING |
| 4 | button (RejectRole) | `"취소"` | — | — | always | Aborts adding control point | `MainWindowEditing.cpp:1422` | `MISSING` | — | — | — | — | — | MISSING |

- **Keyboard & closing**:
  - **C++**: Default button is `"X·Y 교환"` if `suggestion.swapSuggested || swap`, else `"이대로 넣기"` (`MainWindowEditing.cpp:1423`). `Enter` clicks default button; `Esc` / window X clicks `"취소"` (`RejectRole`).
  - **Rust**: `MISSING`
- **Messages**: None beyond the box text itself.

---

## C4 「이름·번호」

- **C++ title & build**: `"이름·번호"` — `target/cpp-ref/src/app/KaFeatureFormDialog.cpp:70` (`KaFeatureFormDialog::KaFeatureFormDialog` at `:64-159`)
- **How user opens it**:
  - **C++**: Opens automatically after finishing drawing a feature on a layer where `KaFeatureFormDialog::canOffer(layer)` is true and `!m_continuousDraw` (`MainWindowEditing.cpp:853-857`)
  - **Rust**: Opens automatically via `StrataAppState::after_capture` when `can_offer` is true and `!self.ribbon.continuous_draw` (`app_state_feature_form.rs:12-20`)
- **Modality**:
  - **C++**: Modal (`QDialog`, `form.exec()` at `MainWindowEditing.cpp:858`)
  - **Rust**: `egui::Window::new("이름·번호")` (`dlg_feature_form.rs:73`); `modal_window_open()` (`app_state_feature_record.rs:10-15`) returns `true` when `feature_form.is_some()`, blocking map drawing and main-window shortcuts (`gui_shell.rs:22-24`). No backdrop dim; other UI panels remain clickable if not blocked.
- **Size rules**:
  - **C++**: `setMinimumWidth(KaWindowSize::width(KaWindowSize::Small, 380))` -> `440` in `kaPaperLook` (`KaFeatureFormDialog.cpp:71`, `KaWindowSize.h:15`); margins `(24, 24, 24, 16)`, row spacing `12`, label width `96` (`KaFeatureFormDialog.cpp:147-153`).
  - **Rust**: `.collapsible(false).resizable(false).min_width(380.0)` (`dlg_feature_form.rs:74-77`), margins `(24, 24, 24, 16)`, `item_spacing.y = 12.0`, `WORD_W = 96.0` (`dlg_feature_form.rs:9,78-80`).
- **Rust counterpart**: `render_feature_form` — `crates/strata-app/src/dlg_feature_form.rs:65-139`

| # | kind | C++ text (verbatim) | C++ tooltip / placeholder (verbatim) | C++ default, range, step, suffix | C++ enabled-when rule | C++ what it does | C++ file:line | Rust text | Rust tooltip / placeholder | Rust default, range, step, suffix | Rust enabled-when rule | Rust what it does | Rust file:line | same? |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 1 | label (tip) | `"이름과 번호는 나중에 넣어도 됩니다.\n건너뛰어도 그린 도형은 그대로 남습니다."` | — | — | always | Explains form is optional | `KaFeatureFormDialog.cpp:73-79` | `"이름과 번호는 나중에 넣어도 됩니다.\n건너뛰어도 그린 도형은 그대로 남습니다."` | — | — | always | Same tip | `dlg_feature_form.rs:81` | yes |
| 2 | label + editable combo box (when `nameField == "kind"`) | `"종류"` | placeholder: `"예: 주거지, 수혈 — 목록에 없으면 직접 적으세요"` | `""`, max 14 visible items, popup `MatchContains` completer | shown when `nameIndex >= 0 && m_nameField == "kind"` | Selects/types `kind`, triggers `updateNumberHint()` | `KaFeatureFormDialog.cpp:36,42-61,82-85,139` | `"종류"` | placeholder: `"예: 주거지, 수혈 — 목록에 없으면 직접 적으세요"` | `""`, ScrollArea max 14*22 px, no typing completer | shown when `form.name_is_kind` | Edits/picks `form.name`, calls `form.update_number_hint()` | `dlg_feature_form.rs:33-60,82-89` | no (no typing completer) |
| 3 | label + line edit (when `nameField != "kind"`) | `"이름"` | placeholder: `"예: 조사명"` | `""` | shown when `nameIndex >= 0 && m_nameField != "kind"` | Edits feature name | `KaFeatureFormDialog.cpp:86-90` | `"이름"` | placeholder: `"예: 조사명"` | `""` | shown when `!form.name_is_kind && form.fields.name_field.is_some()` | Edits `form.name` | `dlg_feature_form.rs:90-95` | yes |
| 4 | label + editable combo box | `"시대"` | placeholder: `"예: 청동기, 원삼국"` | `""`, max 14 visible items, popup `MatchContains` completer | shown when `featureId != FID_NULL` and layer has `period` | Selects/types `period` | `KaFeatureFormDialog.cpp:37,42-61,92-96` | `"시대"` | placeholder: `"예: 청동기, 원삼국"` | `""`, ScrollArea max 14*22 px, no typing completer | shown when `form.has_period` | Edits/picks `form.period` | `dlg_feature_form.rs:96-102` | no (no typing completer) |
| 5 | label + line edit + hint label | `"번호"` + `"다음 번호를 미리 넣었습니다"` | placeholder: `"예: 1호"` | Auto-filled next number (e.g. `"1호"`) until typed | shown when `numberIndex >= 0`; hint visible when `!m_numberTyped && !m_number->text().isEmpty()` | Edits `m_number`, sets `m_numberTyped = true`, calls `updateNumberHint()` | `KaFeatureFormDialog.cpp:38,97-111,135-140,171` | `"번호"` + `"다음 번호를 미리 넣었습니다"` | placeholder: `"예: 1호"` | Auto-filled next number | shown when `form.fields.number_field.is_some()`; hint visible when `form.number_filled_visible()` | Edits `form.number`, sets `form.number_typed = true`, calls `form.update_number_hint()` | `dlg_feature_form.rs:103-115` | yes |
| 6 | label (duplicate warning) | `"같은 종류에 이미 있는 번호입니다(%2). 그래도 저장할 수 있습니다."` or `"이 레이어에 이미 있는 번호입니다(%2). 그래도 저장할 수 있습니다."` (color `#B45309`) | — | — | visible when normalized number already exists in group/layer | Non-blocking duplicate-number warning | `KaFeatureFormDialog.cpp:112-117,172-180` | Same format string, color `#B45309` | — | — | visible when `form.number_note().is_some()` | Same warning | `dlg_feature_form.rs:116-121`, `feature_form_state.rs:100-107` | yes |
| 7 | button (Save) | `"저장"` | — | — | always | Accepts dialog -> `applyTo` | `KaFeatureFormDialog.cpp:142-143,157` | `"저장"` | — | — | always | Calls `save_feature_form()` | `dlg_feature_form.rs:127-130` | yes |
| 8 | button (Cancel) | `"건너뛰기"` | — | — | always | Rejects dialog -> shape stays without attributes | `KaFeatureFormDialog.cpp:142,144,158` | `"건너뛰기"` | — | — | always | Calls `skip_feature_form()` | `dlg_feature_form.rs:124-126` | yes |

- **Keyboard & closing**:
  - **C++**: Default button is `"저장"` (`QDialogButtonBox::Save`). `Enter` accepts; `Esc` rejects (skip). Window X button rejects (skip).
  - **Rust**: `Enter` saves (`save_feature_form`), `Esc` skips (`skip_feature_form`), armed from frame 2 (`keys_armed`) and suppressed while a dropdown popup is open (`dlg_feature_form.rs:67-72`). No window X button (`.collapsible(false)` without `.open(...)`).
- **Messages**:
  - Status bar on open (`MainWindowEditing.cpp:855-856`): `"도형은 넣었습니다. 이름·번호를 적거나 Esc로 건너뛰세요."` -> Rust matches (`app_state_feature_form.rs:18`).
  - Save error (`MainWindowEditing.cpp:863-868`): `KaUserError::warn` modal: title `"속성"`, heading `"이름·번호를 넣지 못했습니다. 도형은 그대로 있습니다."`, body `formError`, action `"도형을 우클릭해 「이 도형 기록 입력」으로 다시 적으세요."` -> Rust writes to status bar instead of modal (`app_state_feature_form.rs:38-40`).

---

## C5 「시굴격자 속성」

- **C++ title & build**: `"시굴격자 속성"` — `target/cpp-ref/src/app/KaTrenchDialog.cpp:27` (`KaTrenchDialog::KaTrenchDialog` at `:26-182`, `addShapeRows` at `KaTrenchDialogPlan.cpp:24-41`, summary at `KaTrenchDialogPlan.cpp:110-197`)
- **How user opens it**:
  - **C++**: Ribbon `"시굴격자"` button (`MainWindowRibbon.cpp:365`) or `survey_area` layer context menu `"규격 지정·격자 만들기…"` (`MainWindowContextMenus.cpp:430`) -> `MainWindow::startTrenchGrid()` (`MainWindow.cpp:1816`)
  - **Rust**: Ribbon `"시굴격자"` button (`gui_ribbon_click_a.rs:89-97`), or `debug_control.rs:77` -> sets `ActiveModalDialog::TrenchGenerator`
- **Modality**:
  - **C++**: Modeless (`setModal(false)` at `KaTrenchDialog.cpp:28`, `show()`/`raise()` at `MainWindow.cpp:1860-1861`)
  - **Rust**: Floating `egui::Window::new("ka_modal_trench")` (`dlg_trench_generator.rs:26`), modeless (`modal_window_open()` does not include `modals.active`)
- **Size rules**:
  - **C++**: Auto-sized `QFormLayout`, calls `adjustSize()` when `m_afterPlace` visibility changes (`KaTrenchDialog.cpp:274`)
  - **Rust**: `.resizable(false).default_width(380.0)` (`dlg_trench_generator.rs:28,30`)
- **Rust counterpart**: `render_trench_generator` — `crates/strata-app/src/dlg_trench_generator.rs:21-91`, `dlg_trench_generator_controls.rs:19-190`, `dlg_trench_generator_summary.rs:13-134`

| # | kind | C++ text (verbatim) | C++ tooltip / placeholder (verbatim) | C++ default, range, step, suffix | C++ enabled-when rule | C++ what it does | C++ file:line | Rust text | Rust tooltip / placeholder | Rust default, range, step, suffix | Rust enabled-when rule | Rust what it does | Rust file:line | same? |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 1 | check box | `"지금 그린(또는 선택한) 조사구역에 자동 배치"` | has area: `"조사구역이 여러 개면 선택한 곳만, 선택이 없으면 마지막에 그린 곳만 깝니다."`; no area: `"조사구역을 먼저 그리면 그 구역 안에 자동 배치를 쓸 수 있습니다."` | checked if `hasArea`, else unchecked | enabled when `!wkb.isEmpty()` | Toggles auto-fill vs manual placement (`m_auto`) | `KaTrenchDialog.cpp:32-33,196-206` | `"지금 그린(또는 선택한) 조사구역에 자동 배치"` | `"조사구역이 여러 개면 선택한 곳만, 선택이 없으면 마지막에 그린 곳만 깝니다."` | `true` (`trench_auto_place`) | always enabled | Toggles `app.modals.trench_auto_place` | `dlg_trench_generator.rs:33-39`, `modal_controller_state.rs:165` | no (enabled-when & no-area tooltip) |
| 2 | label + combo box (C++) / cycling button (Rust) | `"조사 종류"` — entries: `"시굴조사 — 전체 면적의 10%"`, `"표본조사 — 전체 면적의 2%"`, `"직접 지정 — 규격·둑을 내가 정함"` | `"시굴은 조사구역의 10%, 표본은 2%로 배치합니다. 폭 2 m·길이 20 m 이내에서 길이와 둑 간격을 맞추며, 모든 격자가 구역 안에 들어갑니다."` | `"시굴조사 — 전체 면적의 10%"` (index 0) | always | Sets target ratio (10% / 2% / manual) (`m_kind`) | `KaTrenchDialog.cpp:36-46` | `"조사 종류"` + `"{label}  ▼"` button | same tooltip | index 0 | always | Cycles `trench_survey_kind_idx` 0->1->2->0 on click | `dlg_trench_generator_controls.rs:25-44` | no (kind: ComboBox vs cycling Button) |
| 3 | check box | `"지형 사면 방향으로 넣기(등고선 직교)"` | `"트렌치 장축을 오르막 쪽으로 돌립니다. 등고선과 나란히 넣으면 같은 층만 따라가 층서를 못 읽습니다. DEM을 올려야 씁니다."` | checked when `aspect.valid`, else unchecked | enabled when `aspect.valid` | Uses terrain uphill azimuth (`m_terrain`) | `KaTrenchDialog.cpp:48-52,228-239` | `"지형 사면 방향으로 넣기(등고선 직교)"` | same tooltip | `false` (`trench_slope_align`) | always enabled | Toggles `trench_slope_align` | `dlg_trench_generator_controls.rs:46-52`, `modal_controller_state.rs:169` | no (enabled-when & default) |
| 4 | label | No terrain: `"지형 자료가 없어 남북으로 깝니다(DEM·수치지형도 등고선 없음). 방위 칸에 다른 값을 넣으면 그 방향으로 깝니다."`; Flat: `"평지(평균 경사 %1%)라 남북으로 깝니다. 방위 칸에 다른 값을 넣으면 그 방향으로 깝니다."`; Valid: `"지형: 오르막 %1° · 경사 %2% — 트렌치 장축을 이 방향으로 넣습니다."` | — | — | always | Shows terrain slope/aspect status (`m_terrainInfo`) | `KaTrenchDialog.cpp:53-56,232-243` | Checked: hardcoded `"지형: 오르막 135° · 경사 4.2% — 트렌치 장축을 이 방향으로 넣습니다."`; unchecked: `"지형 자료가 없어 남북으로 깝니다(DEM·수치지형도 등고선 없음). 방위 칸에 다른 값을 넣으면 그 방향으로 깝니다."` | — | — | always | Static hardcoded text | `dlg_trench_generator_controls.rs:54-64` | no (hardcoded 135°/4.2%, no flat case) |
| 5 | label + combo box (C++) / cycling button (Rust) | `"모양"` — entries: `"곧은 트렌치"`, `"ㄴ자 트렌치 (15 m + 15 m)"`, `"ㄱ자 트렌치 (15 m + 15 m)"`, `"ㄴ자 좌우 반대 트렌치 (15 m + 15 m)"`, `"ㄱ자 좌우 반대 트렌치 (15 m + 15 m)"` | `"ㄱ·ㄴ자는 팔 15 m 그대로 구역 안에 온전히 들어가는 것만 놓아, 비율이 목표를 조금 넘습니다."` | `"곧은 트렌치"` (index 0) | always | Selects straight or L-shaped trench (`m_shape`) | `KaTrenchDialogPlan.cpp:26-34` | `"모양"` + `"{label}  ▼"` button | same tooltip | `Straight` | always | Cycles `trench_shape` through 5 variants | `dlg_trench_generator_controls.rs:66-83` | no (kind: ComboBox vs cycling Button) |
| 6 | label + combo box (C++) / radio buttons (Rust) | `"규격"` — entries: `"2 × 20 m"` (20), `"2 × 10 m"` (10) | — | `"2 × 20 m"` | visible when `!ratio && m_shape == 0` | Sets trench length (`m_size`) | `KaTrenchDialog.cpp:60-63`, `KaTrenchDialogPlan.cpp:69` | `"트렌치 길이"` — 3 radio buttons: `"20 m"`, `"15 m"`, `"10 m"` | — | `M20` (`"20 m"`) | visible when `trench_shape == Straight` (even in ratio mode!) | Sets `trench_length` | `dlg_trench_generator_controls.rs:84-99`, `trench_types.rs:22-28` | no (label, kind, options, visibility) |
| 7 | check box | `"좌표격자에 맞추기"` | Grid on: `"ㄱ·ㄴ자 시굴·표본에서 꺾인 바깥 모서리를 좌표격자 %1 m 교점에, 방향을 좌표격자 방향에 맞춥니다. 트렌치끼리는 1 m 이상 띄웁니다."`; Grid off: `"지도에 미터 좌표격자를 켜면 쓸 수 있습니다."` | unchecked (`false`) | enabled when `m_gridAvailable && ratio && m_shape > 0` | Snaps L-trench corners to coordinate grid (`m_gridSnap`) | `KaTrenchDialogPlan.cpp:37-39,74-78` | `"좌표격자에 맞추기"` | hardcoded `"20 m"` tooltip | `false` | always enabled | Toggles `trench_grid_snap` | `dlg_trench_generator_controls.rs:100-106` | no (enabled-when & tooltip) |
| 8 | label + double spin box | `"둑(간격)"` | `"트렌치 사이 간격입니다. 간격을 키우면 시굴 비율이 내려갑니다."` | `10.0`, `0..200`, step `1.0`, `" m"` | visible when `!ratio` | Sets balk width (`m_balk`) | `KaTrenchDialog.cpp:65-70`, `KaTrenchDialogPlan.cpp:70` | `"둑(간격)"` (`DragValue`) | same tooltip | `10.0`, `0.0..=200.0`, speed `0.5`, `" m"` | visible when `manual || kind == 2` | Edits `trench_balk_m` | `dlg_trench_generator_controls.rs:130-140` | yes |
| 9 | label + spin box | `"행"` | — | `2`, `1..200`, step `1` | visible when `!autoFill()` | Sets manual row count (`m_rows`) | `KaTrenchDialog.cpp:72-75`, `KaTrenchDialogPlan.cpp:67` | `"행"` (`DragValue`) | — | `2`, `1..=200`, speed `0.2` | visible when `manual` | Edits `trench_rows` | `dlg_trench_generator_controls.rs:112-119` | yes |
| 10 | label + spin box | `"열"` | — | `2`, `1..200`, step `1` | visible when `!autoFill()` | Sets manual column count (`m_cols`) | `KaTrenchDialog.cpp:77-80`, `KaTrenchDialogPlan.cpp:68` | `"열"` (`DragValue`) | — | `2`, `1..=200`, speed `0.2` | visible when `manual` | Edits `trench_cols` | `dlg_trench_generator_controls.rs:121-127` | yes |
| 11 | label + double spin box + dial + note | `"회전(방위)"`, dial `KaAzimuthDial`, note `"위가 북쪽입니다"` / `"북에서 시계 방향 %1°"` | `"격자 회전(북 기준 시계 방향). 「구역에 깔기」를 누르면 회전된 격자로 다시 배치합니다."` | `0.0`, `0..360`, step `1.0`, `" °"` | spin box enabled when `!useTerrainAzimuth() && !gridLock()` | Sets grid azimuth (`m_az`) | `KaTrenchDialog.cpp:82-98`, `KaTrenchDialogPlan.cpp:79-83`, `KaAzimuthDial.cpp:24-28` | `"회전(방위)"` (`DragValue`) + custom painted dial + note | same tooltip | `0.0`, `0.0..=360.0`, speed `1.0`, `" °"` | always enabled | Edits `trench_azimuth_deg` | `dlg_trench_generator_controls.rs:151-188` | no (enabled-when) |
| 12 | label + line edit | `"이름 접두"` | — | `"Tr-"` | visible when `!ratio` | Sets trench name prefix (`m_prefix`) | `KaTrenchDialog.cpp:103-104`, `KaTrenchDialogPlan.cpp:71` | `"이름 접두"` | — | `"Tr-"` | visible when `manual || kind == 2` | Edits `trench_prefix` | `dlg_trench_generator_controls.rs:142-146` | yes |
| 13 | label (summary) | Manual: `"수동 배치: %1개 · 총 %2㎡ — 「맵에 찍기」로 맵에서 위치를 정하세요."`; Computing: `"배치를 계산하는 중입니다…"`; Ratio: `"%1 · 트렌치 %2개 … 비율 %6% (목표 %7%) · 방위 %8°"`; Fill: `"트렌치 %1개 · 총 %2㎡ · 시굴 비율 %3% (기준: 시굴 10% · 표본 2%)%4"` | — | — | always | Shows live plan summary (`m_ratio`) | `KaTrenchDialog.cpp:107-110`, `KaTrenchDialogPlan.cpp:98-196` | Green-framed summary box with manual or auto text (reads `real_survey.trenches` instead of computing live plan) | — | — | always | Shows static/current trench summary | `dlg_trench_generator_summary.rs:21-76` | no (reads existing trenches, no live plan calculation) |
| 14 | meter (`KaTargetMeter`) | Bar `0` .. `20%` (or `4%`), middle tick + `"목표 %1%"` | — | — | visible when `percent >= 0.0` | Visual ratio meter against target | `KaTrenchDialog.cpp:111-115,277-283`, `KaTargetMeter.cpp:31-60` | Painted bar + `"목표 {target:.0}%"` below | — | — | visible when `!manual` | Bar without `0`, max `%`, or middle tick line | `dlg_trench_generator_summary.rs:78-95` | no (missing `0`, max `%`, tick) |
| 15 | label + 2 buttons (`m_afterPlace`) | `"깐 격자는 맵에서 끌어 옮깁니다. 개별 편집은 트렌치 하나 이동·삭제입니다."`, `"개별 편집"` (tip: `"그래픽처럼 트렌치를 하나씩 선택·이동·삭제합니다."`), `"전체 이동"` (tip: `"격자를 끌어다 놓아 전체를 옮깁니다. Ctrl+Z로 되돌립니다."`) | tooltips as shown | — | visible when grid is placed (`setGridPlaced(true)`) | Starts single-trench edit or whole-grid move | `KaTrenchDialog.cpp:118-138,271-275` | Same text & buttons | same tooltips | — | visible when `!manual && !trenches.is_empty()` | Sets `draw_guide_message` only (no tool switch) | `dlg_trench_generator_summary.rs:97-132` | no (handler only sets status message; hidden in manual mode) |
| 16 | button (default) | `"구역에 깔기"` | `"조사구역 위에 격자를 다시 깝니다. 깐 뒤 맵에서 끌어 옮기세요."` | default button | always | Emits `applyRequested` -> places grid on area | `KaTrenchDialog.cpp:141-143,153` | `"구역에 깔기"` | same tooltip | — | always | Shows `시굴격자` layer & sets `draw_guide_message` (does not generate grid) | `dlg_trench_generator.rs:47-72` | no (what it does) |
| 17 | button | `"맵에 찍기"` | `"맵을 한 번 찍어 그 점을 원점으로 놓습니다."` | — | always | Emits `manualPlaceRequested` -> pick origin on map | `KaTrenchDialog.cpp:144-145,154` | `"맵에 찍기"` | same tooltip | — | always | Sets `trench_auto_place = false`, `draw_guide_message`, closes dialog | `dlg_trench_generator.rs:73-83` | no (no map-pick tool) |
| 18 | button | `"닫기"` | — | — | always | Hides dialog (`QDialog::hide`) | `KaTrenchDialog.cpp:146,157` | `"닫기"` | — | — | always | Sets `*open = false` | `dlg_trench_generator.rs:85-87` | yes |

- **Keyboard & closing**:
  - **C++**: Default button is `"구역에 깔기"` (`apply->setDefault(true)` at `KaTrenchDialog.cpp:142`). `Enter` triggers `"구역에 깔기"`; `Esc` / window X hides the modeless dialog. Button row order: `[구역에 깔기] [맵에 찍기] <stretch> [닫기]`.
  - **Rust**: No `Enter` handler. `Esc` closes `app.modals.active = None` via `gui_shell.rs:34-36`. Header `×` button sets `*open = false`. Button row order: `[구역에 깔기] [맵에 찍기] <right> [닫기]`.
- **Messages**:
  - No survey open (`MainWindow.cpp:1821-1822`): `notify(Info, "시굴격자", "먼저 「새 조사」를 만들거나 「열기」로 조사를 여세요.")` -> Rust sets `draw_guide_message` (`gui_ribbon_click_a.rs:90-93`).
  - Leftover survey areas (`MainWindow.cpp:1808-1811`): status bar `"조사구역 %1곳이 남아 있어 마지막에 그린 구역에만 시굴격자를 놓습니다. 예전 구역에 놓으려면 그 구역을 선택한 뒤 다시 누르세요."` -> Rust `MISSING`.

---

## C6 Trial-trench message boxes

Two `QMessageBox` dialogs in C++:
1. **`confirmReplaceAdjusted`** (`KaTrenchDialogPlan.cpp:200-213`):
   - **C++ title & build**: `"시굴격자"` — `KaTrenchDialogPlan.cpp:202`
   - **How user opens it**: Called when `TrenchLayerEdit::placeGrid` is about to replace a hand-adjusted trench grid (`MainWindow.cpp:2126`).
   - **Modality & size**: Modal `QMessageBox::exec()`, `Question` icon, auto-sized.
   - **Rust counterpart**: `MISSING`
2. **Multi-survey-area placement prompt** (`MainWindow.cpp:2065-2081`):
   - **C++ title & build**: `"시굴격자"` — `MainWindow.cpp:2066`
   - **How user opens it**: Layer context menu `"시굴 · 조사구역의 10%"` or `"표본 · 조사구역의 2%"` (`MainWindowContextMenus.cpp:428-429`) -> `MainWindow::applyTrenchByRatio` when `!pick.usedSelection && pick.totalCount > 1`.
   - **Modality & size**: Modal `QMessageBox::exec()`, auto-sized.
   - **Rust counterpart**: `MISSING`

| # | kind | C++ text (verbatim) | C++ tooltip / placeholder (verbatim) | C++ default, range, step, suffix | C++ enabled-when rule | C++ what it does | C++ file:line | Rust text | Rust tooltip / placeholder | Rust default, range, step, suffix | Rust enabled-when rule | Rust what it does | Rust file:line | same? |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 1 | icon + text (`confirmReplaceAdjusted`) | `"이미 깔린 시굴격자를 새 격자로 바꿀까요?"` + informative: `"지금 격자(트렌치 %1개)는 손으로 옮기거나 지운 배치까지 새 배치로 바뀝니다.\n바꾼 뒤에도 Ctrl+Z로 지금 격자를 되살릴 수 있습니다."` | — | — | always | Warns before replacing hand-adjusted grid | `KaTrenchDialogPlan.cpp:201-207` | `MISSING` | — | — | — | — | — | MISSING |
| 2 | button (AcceptRole) | `"새 격자로 바꾸기"` | — | — | always | Proceeds with grid replacement | `KaTrenchDialogPlan.cpp:208` | `MISSING` | — | — | — | — | — | MISSING |
| 3 | button (RejectRole) | `"그대로 두기"` | — | default & escape button | always | Keeps existing grid | `KaTrenchDialogPlan.cpp:209-211` | `MISSING` | — | — | — | — | — | MISSING |
| 4 | text (multi-area) | `"조사구역이 %1곳입니다. 시굴격자를 어디에 놓을까요?"` + informative: `"「전체」를 고르면 %1곳을 합친 면적으로 비율을 계산해 모든 구역에 시굴격자를 놓습니다.\n예전 조사의 구역이 남아 있다면 「마지막 구역만」을 고르세요."` | — | — | always | Asks whether to cover all areas or last area | `MainWindow.cpp:2067-2071` | `MISSING` | — | — | — | — | — | MISSING |
| 5 | button (AcceptRole) | `"전체 %1곳"` | — | default button | always | Places trenches across all survey areas | `MainWindow.cpp:2072-2073,2076` | `MISSING` | — | — | — | — | — | MISSING |
| 6 | button (RejectRole) | `"마지막 구역만"` | — | — | always | Places trenches on last area only | `MainWindow.cpp:2074` | `MISSING` | — | — | — | — | — | MISSING |
| 7 | button (DestructiveRole) | `"취소"` | — | — | always | Cancels operation | `MainWindow.cpp:2075` | `MISSING` | — | — | — | — | — | MISSING |

- **Keyboard & closing**:
  - **C++**: `confirmReplaceAdjusted`: default & Esc = `"그대로 두기"`; multi-area: default = `"전체 %1곳"`, Esc/X = reject (`!= all && != last` returns).
  - **Rust**: `MISSING`
- **Messages**: None beyond the box texts.

---

## C7 「선 색」

- **C++ title & build**: `"선 색"` — `target/cpp-ref/src/app/KaLineColorDialog.cpp:56` (`KaLineColorDialog::edit` at `:46-102`)
- **How user opens it**:
  - **C++**: Layer context menu -> `"면·외곽선 색"` (`editCurrentLayerStyle` -> `KaLineColorDialog::edit` when `isCadastralLayer || isReferenceLayer`, `MainWindowEditing.cpp:443-445`)
  - **Rust**: Layer context menu `"면·외곽선 색"` (`layer.style` at `gui_layer_panel_context_menu.rs:64`) or debug command (`debug_control.rs:116`) -> sets `ActiveModalDialog::LineColor`
- **Modality**:
  - **C++**: Modal (`QDialog::exec()`, `KaLineColorDialog.cpp:95`)
  - **Rust**: Floating `egui::Window::new("ka_modal_line_color")` (`modal_style.rs:22`)
- **Size rules**:
  - **C++**: Auto-sized `QVBoxLayout` + `QFormLayout`
  - **Rust**: `.resizable(false)`, `fixed_size([modal_w, modal_h])` where `modal_w = 430.0.min(vis_w - 40.0)`, `modal_h = 310.0.min(vis_h - 40.0)` (`modal_style.rs:16-26`)
- **Rust counterpart**: `render_line_color` — `crates/strata-app/src/modal_style.rs:11-181`

| # | kind | C++ text (verbatim) | C++ tooltip / placeholder (verbatim) | C++ default, range, step, suffix | C++ enabled-when rule | C++ what it does | C++ file:line | Rust text | Rust tooltip / placeholder | Rust default, range, step, suffix | Rust enabled-when rule | Rust what it does | Rust file:line | same? |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 1 | label | `layer->name()` | — | — | always | Shows target layer name | `KaLineColorDialog.cpp:58-60` | `"대상 레이어: {target_layer}"` | — | — | always | Shows `"대상 레이어: ..."` | `modal_style.rs:36-40` | no (text prefix) |
| 2 | label + push button | `"선 색"` + `"{HEX}  ·  클릭해서 색 고르기"` | — | `LineMapStyle::currentColor(layer)` | always | Opens `QColorDialog` (`"선 색 고르기"`, `DontUseNativeDialog`) | `KaLineColorDialog.cpp:28,62-69,77` | `"선 색"` + `"#{r:02X}{g:02X}{b:02X} · 클릭해서 색 고르기"` | — | `[0xEA, 0x58, 0x0C]` | always | Cycles `KA_STYLE_PALETTE` 0..7 (no color picker) | `modal_style.rs:65-83` | no (text spacing, what it does) |
| 3 | EXTRA: swatch grid | — | — | — | — | — | — | `"색 팔레트"` + 8 color swatch buttons | `"{sname} (#{sr:02X}{sg:02X}{sb:02X})"` | `KA_STYLE_PALETTE` | always | Selects palette color | `modal_style.rs:85-114` | EXTRA |
| 4 | label + double spin box | `"선 굵기"` | — | `currentWidthMm(layer)`, `0.1..5.0`, step `0.1`, decimals `2`, `" mm"` | always | Sets line width | `KaLineColorDialog.cpp:70-78` | `"선 굵기"` (`DragValue`) + EXTRA 5 quick buttons (`0.5mm`, `1.0mm`, `1.5mm`, `2.0mm`, `3.0mm`) | — | `1.5`, `0.1..=5.0`, speed `0.1`, decimals `2`, `" mm"` | always | Sets `app.modals.line_width_mm` | `modal_style.rs:116-137` | no (EXTRA quick width buttons) |
| 5 | button (ResetRole) | `"원래 색으로"` | — | — | enabled when `LineMapStyle::hasOriginal(layer)` | Restores original style & accepts | `KaLineColorDialog.cpp:84-86,90-93` | `"원래 색으로"` | — | — | always enabled | Resets to `#EA580C`, `1.5 mm` & closes | `modal_style.rs:144-154` | no (enabled-when & hardcoded reset) |
| 6 | button (Ok) | `"적용"` | — | — | always | Applies color & width, marks project dirty | `KaLineColorDialog.cpp:81-82,97-100` | `"적용"` | — | — | always | Saves to `right_inspector` & closes | `modal_style.rs:159-177` | yes |
| 7 | button (Cancel) | `"취소"` | — | — | always | Rejects and closes | `KaLineColorDialog.cpp:81,83,88` | `"취소"` | — | — | always | Sets `app.modals.active = None` | `modal_style.rs:156-158` | yes |

- **Keyboard & closing**:
  - **C++**: Default button `"적용"` (`QDialogButtonBox::Ok`). `Enter` accepts; `Esc` / window X rejects.
  - **Rust**: No `Enter` handler. `Esc` closes via `gui_shell.rs:34-36`. Header `×` closes.
- **Messages**:
  - Non-line-map layer (`KaLineColorDialog.cpp:47-53`): `KaUserError::warn` prevents opening dialog: title `"선 색"`, heading `"「%1」은 색을 바꾸지 않습니다."`, body `"면을 색으로 채운 지도(토양도·지질도)와 점·글자는 공식 범례·도면 표시 그대로 둡니다."`, action `"도면 선·받은 지적도·등고선처럼 선으로 된 지도나 조사 데이터 레이어를 고르세요."`
  - **Rust**: Opens dialog anyway and renders inline orange text (`modal_style.rs:43-52`).

---

## C8 「도형 색」

- **C++ title & build**: `"도형 색"` — `target/cpp-ref/src/app/MainWindowEditing.cpp:467` (`MainWindow::editCurrentLayerStyle` at `:434-614`)
- **How user opens it**:
  - **C++**: Layer context menu -> `"면·외곽선 색"` on a non-reference, non-cadastral vector layer (`MainWindowEditing.cpp:434-450`)
  - **Rust**: `MISSING` (`layer.style` always opens `ActiveModalDialog::LineColor` at `gui_layer_panel_context_menu.rs:64`)
- **Modality**:
  - **C++**: Modal (`QDialog::exec()`, `MainWindowEditing.cpp:601`)
  - **Rust**: `MISSING`
- **Size rules**:
  - **C++**: `Qt::MSWindowsFixedSizeDialogHint`, `QLayout::SetFixedSize`, margins `(16, 14, 16, 12)`, spacing `8` (`MainWindowEditing.cpp:468-473`)
  - **Rust**: `MISSING`
- **Rust counterpart**: `MISSING`

| # | kind | C++ text (verbatim) | C++ tooltip / placeholder (verbatim) | C++ default, range, step, suffix | C++ enabled-when rule | C++ what it does | C++ file:line | Rust text | Rust tooltip / placeholder | Rust default, range, step, suffix | Rust enabled-when rule | Rust what it does | Rust file:line | same? |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 1 | label (title) | `"도형 색"` | — | — | always | Header label | `MainWindowEditing.cpp:475` | `MISSING` | — | — | — | — | — | MISSING |
| 2 | label (subtitle) | `layer->name()` | — | — | always | Target layer name | `MainWindowEditing.cpp:476-479` | `MISSING` | — | — | — | — | — | MISSING |
| 3 | check box | `"채우기 없음 (외곽선만)"` | — | `noFill` | shown when `isPoly || isPoint` | Hides fill box & applies live | `MainWindowEditing.cpp:484-487,574-583` | `MISSING` | — | — | — | — | — | MISSING |
| 4 | label + color button | `"면 색"` + `"{HEX}  ·  클릭해서 색 고르기"` | — | `fill` (fallback `QColor(22,163,74,160)`) | shown when `(isPoly || isPoint) && !noFill` | Opens `QColorDialog` (`"면 색"`, alpha) & applies live | `MainWindowEditing.cpp:84-99,488-491,576` | `MISSING` | — | — | — | — | — | MISSING |
| 5 | check box | `"외곽선 없음"` | — | `noStroke` | always | Hides stroke/width/dash & applies live | `MainWindowEditing.cpp:494-496,575-584` | `MISSING` | — | — | — | — | — | MISSING |
| 6 | check box | `"점선"` | — | `dashed` | shown when `(isLine || isPoly) && !noStroke` | Toggles dashed line & applies live | `MainWindowEditing.cpp:498-503,579` | `MISSING` | — | — | — | — | — | MISSING |
| 7 | label + color button | `isLine ? "선 색" : "외곽선 색"` + `"{HEX}  ·  클릭해서 색 고르기"` | — | `stroke` (fallback `QColor(21,128,61)`) | shown when `!noStroke` | Opens `QColorDialog` (`"선 색"` or `"외곽선 색"`, alpha) & applies live | `MainWindowEditing.cpp:84-99,505-510,577` | `MISSING` | — | — | — | — | — | MISSING |
| 8 | label + double spin box | `isLine ? "선 굵기" : "외곽선 굵기"` | — | `widthMm`, `0.2..12.0`, step `0.2`, decimals `1`, `" mm"` | shown when `!noStroke` | Sets stroke width & applies live | `MainWindowEditing.cpp:113-131,512-516,578` | `MISSING` | — | — | — | — | — | MISSING |
| 9 | label + double spin box | `"점 크기"` | — | `markerMm`, `1.0..20.0`, step `0.5`, decimals `1`, `" mm"` | shown when `isPoint` | Sets point marker size & applies live | `MainWindowEditing.cpp:518-523` | `MISSING` | — | — | — | — | — | MISSING |
| 10 | check box | `"종류별 자동 색"` | — | `LayerStyleKinds::isByKind(layer)` | shown when `layerKey == "feature_poly"` | Applies kind-based colors (mutually exclusive with preset) | `MainWindowEditing.cpp:525-531,541-542` | `MISSING` | — | — | — | — | — | MISSING |
| 11 | check box | `"시대색·종류 무늬"` | `"시대는 밝기 순서 색으로, 종류는 빗금·점선·기호 모양으로 나눕니다. 흑백 인쇄에서도 구분됩니다."` | `FeaturePresets::isPresetStyled(layer)` | shown when `FeaturePresets::canStyle(layer)` | Applies period/kind preset renderer | `MainWindowEditing.cpp:533-544` | `MISSING` | — | — | — | — | — | MISSING |
| 12 | button (Save) | `"적용"` | — | — | always | Accepts & keeps live style | `MainWindowEditing.cpp:565-566,609` | `MISSING` | — | — | — | — | — | MISSING |
| 13 | button (Cancel) | `"취소"` | — | — | always | Rejects & restores original style/renderer | `MainWindowEditing.cpp:565,567,601-608` | `MISSING` | — | — | — | — | — | MISSING |

- **Keyboard & closing**:
  - **C++**: Default button `"적용"` (`Save`). `Enter` accepts; `Esc` / window X rejects and restores original style.
  - **Rust**: `MISSING`
- **Messages**:
  - No vector layer selected (`MainWindowEditing.cpp:439-440`): `QMessageBox::information("모양", "벡터 레이어를 선택한 뒤 다시 실행하세요.")` -> `MISSING`.
  - Success (`MainWindowEditing.cpp:610`): status bar `"면·선 색 적용: %1"` -> `MISSING`.

---

## C9 「도형 속성 — <layer>」

- **C++ title & build**: `"도형 속성 — %1"` (layer name) — `target/cpp-ref/src/app/MainWindowEditing.cpp:643` (`editFeatureAttributes` at `:616-776`, arranged by `KaRecordPlate::arrange` at `KaRecordPlate.cpp:180-266`)
- **How user opens it**:
  - **C++**: (1) Attribute edit map tool (`KaAttributeMapTool::featurePicked`, `MainWindowEditing.cpp:321-324`), (2) Layer context menu `"도형 속성"` -> `editCurrentLayerAttributes` (`MainWindowEditing.cpp:332,406`), (3) Right inspector feature card `"조사카드 열기"` (`formRequested`)
  - **Rust**: Right inspector feature card `"조사카드 열기"` -> `open_feature_record` (`ri_attr_card.rs:110-115`, `app_state_feature_record.rs:18`). Note: layer context menu `"도형 속성"` (`layer.attributes`) is disabled/deferred (`layer_menu_model.rs:187`).
- **Modality**:
  - **C++**: Modal (`QDialog::exec()`, `MainWindowEditing.cpp:700`)
  - **Rust**: `egui::Window::new(form.title())` (`dlg_feature_record.rs:123`); `modal_window_open()` (`app_state_feature_record.rs:10-15`) returns `true`, blocking map drawing and main-window shortcuts (`gui_shell.rs:22-24`).
- **Size rules**:
  - **C++**: `setFixedWidth(560)` (`KaRecordPlate.cpp:210` + `KaWindowSize.h:15`), scrollable body (`QScrollArea`), margins `(24, 24, 8, 16)`
  - **Rust**: `.collapsible(false).resizable(false).min_width(528.0).max_width(528.0)` (`dlg_feature_record.rs:124-128`), margins `(24, 24, 8, 16)`, `ScrollArea::vertical().max_height((screen_h - 180.0).max(200.0))`
- **Rust counterpart**: `render_feature_record` — `crates/strata-app/src/dlg_feature_record.rs:114-169`

| # | kind | C++ text (verbatim) | C++ tooltip / placeholder (verbatim) | C++ default, range, step, suffix | C++ enabled-when rule | C++ what it does | C++ file:line | Rust text | Rust tooltip / placeholder | Rust default, range, step, suffix | Rust enabled-when rule | Rust what it does | Rust file:line | same? |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 1 | label (tip) | `"그린 도형의 속성입니다. 종류·시대 등을 입력한 뒤 저장하세요."` | — | — | always | Form instruction | `MainWindowEditing.cpp:647-650` | `RECORD_TIP` (same text) | — | — | always | Same tip | `dlg_feature_record.rs:134`, `feature_record.rs` | yes |
| 2 | section heads | `"식별"`, `"층위"`, `"표고"`, `"조사"`, `"사진 · 비고"` | — | — | shown if section has fields | Section heading + hairline rule | `KaRecordPlate.cpp:25,49-60,224-227` | `RECORD_SECTIONS` (same 5 titles) | — | — | shown if section has fields | Same section headings + rule | `dlg_feature_record.rs:137-146` | yes |
| 3 | field editors (non-auto fields) | `attributeFieldLabelKo(name)` (e.g. `"조사명"`, `"유적명"`, `"유구종류"`, `"시대"`, `"유구번호"`, `"유물번호"`, `"단면번호"`, `"점ID"`, `"비고"`, etc.) | `kind`: `"예: 주거지, 수혈 — 목록에 없으면 직접 적으세요"`; `period`: `"예: 청동기, 원삼국"`; `feature_no`: `"예: 1호"`; elided labels & combos have full text as tooltip | Current feature attribute value; `kind`/`period` = editable `QComboBox` (with completer); `note` = `QPlainTextEdit` (72 px); elevation fields = right-aligned mono `QLineEdit` with `"m"` suffix; others = `QLineEdit` | non-`fid`, non-`ogc_*`, non-auto fields | Edits attribute values | `MainWindowEditing.cpp:660-686`, `KaFeatureFormDialog.cpp:31-61,219-232`, `KaRecordPlate.cpp:35-39,63-113` | Same labels, placeholders, tooltips, mono `"m"` boxes, 72 px `multiline` for `note`, `Choice` dropdown for `kind`/`period` (no typing completer) | same | Current feature attribute value | same | Edits `row.text` | `dlg_feature_record.rs:25-110,174-219` | no (no typing completer on `kind`/`period`) |
| 4 | custom widget (`KaElevationFigure`) | Cross-section diagram next to `top_el` & `bottom_el` | — | — | shown in `"표고"` section when `top_el` and `bottom_el` exist | Live-updates section drawing from `top_el`, `bottom_el`, `depth_m` | `KaRecordPlate.cpp:142-175` | `elevation_figure(ui, ...)` | — | — | shown when `top_el` and `bottom_el` exist | Same 244×76 section drawing | `dlg_feature_record.rs:180-192`, `dlg_feature_record_figure.rs:1` | yes |
| 5 | button (Save) | `"저장"` | — | — | always | Validates numbers, writes attributes + `updated_at` | `MainWindowEditing.cpp:694-695,700-752` | `RECORD_BUTTONS[0]` (`"저장"`) | — | — | always | Calls `save_feature_record()` | `dlg_feature_record.rs:156-158` | yes |
| 6 | button (Cancel) | `"취소"` | — | — | always | Rejects and discards edits | `MainWindowEditing.cpp:694,698` | `RECORD_BUTTONS[1]` (`"취소"`) | — | — | always | Calls `cancel_feature_record()` | `dlg_feature_record.rs:153-155` | yes |

- **Keyboard & closing**:
  - **C++**: Default button `"저장"`. `Enter` accepts (except inside `QPlainTextEdit` `note` where `Enter` inserts newline and `Tab` changes focus via `setTabChangesFocus(true)`). `Esc` / window X rejects. `KaRecordPlate.cpp:223-245` sets explicit `setTabOrder` in screen order.
  - **Rust**: `Enter` saves (`save_feature_record`) except when `note` (`lines_focus`) has focus or a dropdown popup is open; `Esc` cancels (`cancel_feature_record`) when no popup is open; `keys_armed` prevents closing on open frame (`dlg_feature_record.rs:116-120,161-168`). No window X button.
- **Messages**:
  - Read-only layer (`MainWindowEditing.cpp:621-628`): `KaUserError::warn` `"「%1」의 속성은 고칠 수 없습니다."` -> Rust `MISSING` (card doesn't open for read-only).
  - Feature vanished (`MainWindowEditing.cpp:632-638`): `KaUserError::warn` `"고른 도형을 다시 읽지 못했습니다."` -> Rust status bar (`app_state_feature_record.rs:25`).
  - No editable fields (`MainWindowEditing.cpp:688-691`): `QMessageBox::information("속성", "이 레이어에 편집할 속성 필드가 없습니다.")` -> Rust status bar (`app_state_feature_record.rs:25`).
  - Invalid number (`MainWindowEditing.cpp:722-728`): `KaUserError::warn` `"「%1」에 넣은 값이 %2가 아닙니다: %3"` -> Rust writes to status bar (`app_state_feature_record.rs:41-44`).
  - Save error (`MainWindowEditing.cpp:745-750`): `KaEditErrors::show` `"속성을 바꾸지 못했습니다."` -> Rust status bar (`app_state_feature_record.rs:51`).
  - Success (`MainWindowEditing.cpp:770-774`): status bar `"속성을 바꿨습니다: %1 (#%2). Ctrl+Z로 되돌리고, 「저장」(Ctrl+S)으로 파일에 씁니다."` -> Rust matches (`app_state_feature_record.rs:58-61`).

---

## C10 「겹치는 구간 분리 (클립)」

- **C++ title & build**: `"겹치는 구간 분리 (클립)"` — `target/cpp-ref/src/app/MainWindowEditing.cpp:1306` (`MainWindow::clipOverlappingLayers` at `:1282-1389`)
- **How user opens it**:
  - **C++**: Draw sub-toolbar `"구간 분리"` button -> `MainWindow::clipOverlappingLayers()`
  - **Rust**: `MISSING` (`"구간 분리"` button in `gui_ribbon_draw_row.rs:205,210` is disabled/not wired)
- **Modality**:
  - **C++**: Modal (`QDialog::exec()`, `MainWindowEditing.cpp:1348`)
  - **Rust**: `MISSING`
- **Size rules**:
  - **C++**: `dlg.resize(420, 200)` (`MainWindowEditing.cpp:1307`), resizable
  - **Rust**: `MISSING`
- **Rust counterpart**: `MISSING`

| # | kind | C++ text (verbatim) | C++ tooltip / placeholder (verbatim) | C++ default, range, step, suffix | C++ enabled-when rule | C++ what it does | C++ file:line | Rust text | Rust tooltip / placeholder | Rust default, range, step, suffix | Rust enabled-when rule | Rust what it does | Rust file:line | same? |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 1 | label | `"기준 바운더리 레이어와 겹치는 구간만 잘라내어 새 레이어로 분리합니다."` | — | — | always | Dialog description | `MainWindowEditing.cpp:1310-1312` | `MISSING` | — | — | — | — | — | MISSING |
| 2 | label + combo box | `"자를 대상 레이어:"` — all valid vector layers | — | current layer in tree (or index 0) | always | Selects target layer to clip (`targetCombo`) | `MainWindowEditing.cpp:1315,1320-1325,1337` | `MISSING` | — | — | — | — | — | MISSING |
| 3 | label + combo box | `"기준 바운더리:"` — all valid polygon vector layers | — | first polygon layer whose name contains `"바운더리"`, `"구역"`, or `"허가"` (or index 0) | always | Selects boundary polygon layer (`boundaryCombo`) | `MainWindowEditing.cpp:1316,1327-1335,1338` | `MISSING` | — | — | — | — | — | MISSING |
| 4 | button (Ok) | `"분리 실행"` | — | — | always | Accepts dialog -> runs clip & adds new layer | `MainWindowEditing.cpp:1341-1342` | `MISSING` | — | — | — | — | — | MISSING |
| 5 | button (Cancel) | `"취소"` | — | — | always | Rejects and closes | `MainWindowEditing.cpp:1341,1343` | `MISSING` | — | — | — | — | — | MISSING |

- **Keyboard & closing**:
  - **C++**: Default button `"분리 실행"` (`Ok`). `Enter` accepts; `Esc` / window X rejects.
  - **Rust**: `MISSING`
- **Messages**:
  - Fewer than 2 vector layers or 0 polygon layers (`MainWindowEditing.cpp:1299-1302`): `QMessageBox::information("구간 분리 (클립)", "구간을 분리하려면 최소 1개의 폴리곤(바운더리) 레이어와 대상 레이어가 필요합니다.")` -> `MISSING`.
  - Same layer chosen (`MainWindowEditing.cpp:1354-1360`): `KaUserError::warn("구간 분리", "같은 레이어로는 구간을 나눌 수 없습니다.", ...)` -> `MISSING`.
  - Success (`MainWindowEditing.cpp:1383-1386`): status bar + `notify(Success, "구간 분리 완료", ...)` -> `MISSING`.

---

## C11 「격자 원점」

- **C++ title & build**: `"격자 원점"` — `target/cpp-ref/src/app/KaGridOriginButton.cpp:62` (`KaGridOriginButton::editOrigin` at `:55-113`)
- **How user opens it**:
  - **C++**: Map grid detail bar -> `"원점"` `QToolButton` (`KaGridOriginButton` at `MainWindow.cpp:1027`, `KaGridOriginButton.cpp:35-37`)
  - **Rust**: `MISSING` (`gui_tab_map_canvas_overlay.rs:40` explicitly notes: `"「원점」 단추는 아직 없다."`)
- **Modality**:
  - **C++**: Modal (`QDialog::exec()`, `KaGridOriginButton.cpp:112`)
  - **Rust**: `MISSING`
- **Size rules**:
  - **C++**: `dialog.setMinimumWidth(440)` in `kaPaperLook` (`KaGridOriginButton.cpp:85`), margins `(24, 24, 24, 16)`, row spacing `12`, label width `96`, spin box fixed width `176` (`:86-92`)
  - **Rust**: `MISSING`
- **Rust counterpart**: `MISSING`

| # | kind | C++ text (verbatim) | C++ tooltip / placeholder (verbatim) | C++ default, range, step, suffix | C++ enabled-when rule | C++ what it does | C++ file:line | Rust text | Rust tooltip / placeholder | Rust default, range, step, suffix | Rust enabled-when rule | Rust what it does | Rust file:line | same? |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 1 | label (hint) | `"격자선 하나가 이 점을 지납니다. 회전한 격자는 이 점을\n중심으로 돌고, 눈금은 이 점에서 잰 거리로 적습니다."` | — | — | always | Explains grid origin | `KaGridOriginButton.cpp:64-71` | `MISSING` | — | — | — | — | — | MISSING |
| 2 | label + double spin box | `"X (동쪽)"` | — | `current.originX` (default `0.0`), `-10000000.0..10000000.0`, step `1.0`, decimals `3`, `" m"` | always | Sets easting origin (`east`) | `KaGridOriginButton.cpp:18-27,72,74` | `MISSING` | — | — | — | — | — | MISSING |
| 3 | label + double spin box | `"Y (북쪽)"` | — | `current.originY` (default `0.0`), `-10000000.0..10000000.0`, step `1.0`, decimals `3`, `" m"` | always | Sets northing origin (`north`) | `KaGridOriginButton.cpp:18-27,73,75` | `MISSING` | — | — | — | — | — | MISSING |
| 4 | push button | `"지도 가운데"` | — | — | always | Sets `east`/`north` to `m_canvas->extent().center()` | `KaGridOriginButton.cpp:77,100-105` | `MISSING` | — | — | — | — | — | MISSING |
| 5 | push button | `"좌표계 원점 (0, 0)"` | — | — | always | Sets `east = 0.0`, `north = 0.0` | `KaGridOriginButton.cpp:78,106-109` | `MISSING` | — | — | — | — | — | MISSING |
| 6 | button (Ok) | `"적용"` | — | — | always | Applies origin via `setOrigin(east, north)` | `KaGridOriginButton.cpp:96-97,112` | `MISSING` | — | — | — | — | — | MISSING |
| 7 | button (Cancel) | `"취소"` | — | — | always | Rejects and closes | `KaGridOriginButton.cpp:96,98,111` | `MISSING` | — | — | — | — | — | MISSING |

- **Keyboard & closing**:
  - **C++**: Default button `"적용"` (`Ok`). `Enter` accepts; `Esc` / window X rejects.
  - **Rust**: `MISSING`
- **Messages**: None (button tooltip updates to `"격자 원점: X(동) %1 m, Y(북) %2 m. 눌러서 바꿉니다."` at `KaGridOriginButton.cpp:50-52`).

---

## C13 Colour pickers (「면 색」, 「선 색」, 「외곽선 색」)

- **C++ title & build**:
  - `QColorDialog` with `DontUseNativeDialog` and `ShowAlphaChannel`:
    - `"면 색"` — `MainWindowEditing.cpp:489` (inside C8 「도형 색」) and `MainWindow.cpp:2484` (inside `addUserLayer`)
    - `"선 색"` — `MainWindowEditing.cpp:507` (inside C8 「도형 색」)
    - `"외곽선 색"` — `MainWindowEditing.cpp:507` (inside C8 「도형 색」) and `MainWindow.cpp:2488` (inside `addUserLayer`)
    - `"선 색 고르기"` — `KaLineColorDialog.cpp:67` (inside C7 「선 색」, without `ShowAlphaChannel`)
  - Built via `kaMakeColorButton` (`MainWindowEditing.cpp:84-100`, `MainWindow.cpp:2410-2426`)
- **How user opens it**:
  - **C++**: Clicking any `"{HEX}  ·  클릭해서 색 고르기"` swatch button in C7, C8, or `addUserLayer`
  - **Rust**: `MISSING` (Rust has no `QColorDialog`-style HSV/RGB/Alpha picker; `modal_style.rs:77-82` cycles 8 fixed palette colors, and `modal_survey.rs:97` / `dlg_survey_area.rs:42` use an 8-color swatch grid)
- **Modality**:
  - **C++**: Modal (`QColorDialog::exec()` / `QColorDialog::getColor`)
  - **Rust**: `MISSING`
- **Size rules**:
  - **C++**: Standard Qt `QColorDialog` size
  - **Rust**: `MISSING`
- **Rust counterpart**: `MISSING`

| # | kind | C++ text (verbatim) | C++ tooltip / placeholder (verbatim) | C++ default, range, step, suffix | C++ enabled-when rule | C++ what it does | C++ file:line | Rust text | Rust tooltip / placeholder | Rust default, range, step, suffix | Rust enabled-when rule | Rust what it does | Rust file:line | same? |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 1 | `QColorDialog` | Titles: `"면 색"`, `"선 색"`, `"외곽선 색"`, `"선 색 고르기"` | — | Initial color = current button `kaColor`; `ShowAlphaChannel` enabled in `kaMakeColorButton` | always | Full Qt color picker (basic/custom colors, HSV/RGB/HTML hex/alpha) | `MainWindowEditing.cpp:89-97`, `MainWindow.cpp:2415-2423`, `KaLineColorDialog.cpp:66-68` | `MISSING` | — | — | — | — | — | MISSING |

- **Keyboard & closing**:
  - **C++**: Standard `QColorDialog` (`Enter` accepts selected color, `Esc` / X cancels).
  - **Rust**: `MISSING`
- **Messages**: None.

---

## C14 Confirm & info boxes of the editing tools (`MainWindowEditing.cpp` and `KaVertexEditTool.cpp`)

`KaVertexEditTool.cpp` has **0** `QMessageBox` calls.
`MainWindowEditing.cpp` has **14** active `QMessageBox` calls (plus 3 `#else` non-QGIS stub calls at `:408`, `:430`, `:612`, and 1 already covered as C3 at `:1416`):

| # | kind | C++ text (verbatim) | C++ tooltip / placeholder (verbatim) | C++ default, range, step, suffix | C++ enabled-when rule | C++ what it does | C++ file:line | Rust text | Rust tooltip / placeholder | Rust default, range, step, suffix | Rust enabled-when rule | Rust what it does | Rust file:line | same? |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 1 | `QMessageBox::information` | Title: `"속성"`, text: `"벡터 레이어(조사구역·유구 등)를 선택한 뒤 다시 실행하세요."` | — | Ok | when no valid vector layer in `editCurrentLayerAttributes` | Informs user to select a vector layer | `MainWindowEditing.cpp:339-340` | `MISSING` | — | — | — | — | — | MISSING |
| 2 | `QMessageBox::information` | Title: `"속성"`, text: `"「%1」에 도형이 없습니다.\n먼저 그리기로 도형을 만든 뒤 속성을 입력하세요."` | — | Ok | when `layer->featureCount() <= 0` | Informs user layer is empty | `MainWindowEditing.cpp:344-347` | `MISSING` | — | — | — | — | — | MISSING |
| 3 | `QMessageBox::information` | Title: `"속성"`, text: `"피처를 읽을 수 없습니다. 레이어 파일을 확인하세요."` | — | Ok | when `feats.isEmpty()` after iteration | Informs user features cannot be read | `MainWindowEditing.cpp:363-364` | `MISSING` | — | — | — | — | — | MISSING |
| 4 | `QMessageBox::information` | Title: `"모양"`, text: `"벡터 레이어를 선택한 뒤 다시 실행하세요."` | — | Ok | when no valid vector layer in `editCurrentLayerStyle` | Informs user to select a vector layer | `MainWindowEditing.cpp:439-440` | `MISSING` | — | — | — | — | — | MISSING |
| 5 | `QMessageBox::information` | Title: `"속성"`, text: `"이 레이어에 편집할 속성 필드가 없습니다."` | — | Ok | when `rows.isEmpty()` in `editFeatureAttributes` | Informs user there are no editable fields | `MainWindowEditing.cpp:689-690` | Status bar text via `FeatureRecordForm::new` (`Err(...)`) | — | — | — | Writes to status bar instead of modal | `app_state_feature_record.rs:23-25` | no (status bar vs modal) |
| 6 | `QMessageBox::information` | Title: `"쉽게그리기"`, text: `"먼저 「새 조사」로 저장 위치를 만드세요."` | — | Ok | when `m_surveyPath.isEmpty()` in `startEasyDraw` | Informs user to create a survey first | `MainWindowEditing.cpp:1040-1041` | `MISSING` | — | — | — | — | — | MISSING |
| 7 | `QMessageBox::question` | Title: `"레이어"`, text: `"먼저 「새 조사」로 저장 경로를 만드세요.\n\n지금 「새 조사」를 만들까요?"` | — | `Yes \| No`, default `Yes` | when `m_surveyPath.isEmpty()` in `startEditSurveyArea` | Offers to open `newSurvey()` (`Yes`) or abort (`No`) | `MainWindowEditing.cpp:1090-1095` | `MISSING` (`app_state_draw_entry.rs:7` explicitly notes this prompt is absent) | — | — | — | — | — | MISSING |
| 8 | `QMessageBox::information` | Title: `title`, text: `"「%1」은(는) 보기만 하는 참고 자료(참조 지도·지적)라 고칠 수 없습니다.\n조사 도형만 골라 다시 누르세요."` | — | Ok | when `isReferenceOrBasemapLayer(layer)` in `refuseReadOnlyLayer` | Blocks editing read-only reference/cadastral layers | `MainWindowEditing.cpp:1181-1184` | `MISSING` | — | — | — | — | — | MISSING |
| 9 | `QMessageBox::information` | Title: `"폴리곤 나누기"`, text: `"도형을 Shift+클릭으로 선택하거나, 나눌 폴리곤 레이어를 좌측 목록에서 선택해 주세요."` | — | Ok | when no valid polygon layer in `startSplitPolygonTool` | Informs user how to pick polygon to split | `MainWindowEditing.cpp:1265-1266` | `MISSING` | — | — | — | — | — | MISSING |
| 10 | `QMessageBox::information` | Title: `"구간 분리 (클립)"`, text: `"구간을 분리하려면 최소 1개의 폴리곤(바운더리) 레이어와 대상 레이어가 필요합니다."` | — | Ok | when `allVecs.size() < 2 \|\| polyVecs.isEmpty()` | Blocks clip when insufficient layers | `MainWindowEditing.cpp:1300-1301` | `MISSING` | — | — | — | — | — | MISSING |
| 11 | `QMessageBox::question` | Title: `"그린 도형 삭제"`, text: `"%1의 도형 %2개를 지웁니다. Ctrl+Z로 복원할 수 있습니다.\n계속할까요?"` | — | `Yes \| No`, default `No` | when clearing all features of current survey domain layer | Confirms deleting all features of the layer | `MainWindowEditing.cpp:1491-1496` | `MISSING` | — | — | — | — | — | MISSING |

---

## Differences to fix

Most visible to the user first:

1. **C2 「기준점 미리보기」**: Entire dialog missing; clicking `"CSV 기준점"` in the `더보기` menu opens `AccountSettings` instead (`MainWindow.cpp:2557-2563` -> `gui_ribbon_menus_b.rs:114`).
2. **C3 「기준점 축」**: Entire axis-swap confirmation dialog missing after adding a control point (`MainWindowEditing.cpp:1416-1427` -> `modal_control.rs:248-260`).
3. **C8 「도형 색」**: Entire dialog missing; right-clicking `"면·외곽선 색"` on a polygon/line/point survey layer opens `"선 색"` instead (`MainWindowEditing.cpp:467-610` -> `gui_layer_panel_context_menu.rs:64`).
4. **C10 「겹치는 구간 분리 (클립)」**: Entire dialog missing; ribbon `"구간 분리"` button is disabled (`MainWindowEditing.cpp:1306-1348` -> `gui_ribbon_draw_row.rs:205,210`).
5. **C11 「격자 원점」**: Entire dialog and `"원점"` button missing (`KaGridOriginButton.cpp:62-112` -> `gui_tab_map_canvas_overlay.rs:40`).
6. **C13 Colour pickers (`QColorDialog`)**: Missing; `"클릭해서 색 고르기"` in `"선 색"` only cycles 8 palette colors instead of opening a color/alpha picker (`MainWindowEditing.cpp:89-97`, `KaLineColorDialog.cpp:66-68` -> `modal_style.rs:77-82`).
7. **C6 Trial-trench message boxes**: Both `confirmReplaceAdjusted` (`KaTrenchDialogPlan.cpp:200-213`) and the multi-survey-area prompt (`MainWindow.cpp:2065-2081`) are missing.
8. **C14 Editing confirm/info boxes**: 10 of 11 active `QMessageBox` prompts in `MainWindowEditing.cpp` are missing (including `"그린 도형 삭제"` at `:1491` and `"지금 「새 조사」를 만들까요?"` at `:1090`); the 11th (`:689`) is status-bar-only (`app_state_feature_record.rs:25`).
9. **C7 「선 색」 — non-line-map guard & extra controls**: C++ refuses to open the dialog and shows `KaUserError::warn` (`KaLineColorDialog.cpp:47-53`), whereas Rust opens the dialog with an inline warning (`modal_style.rs:43-52`), adds EXTRA swatch & quick-width rows (`:85-137`), and always enables `"원래 색으로"` (`:144`).
10. **C5 「시굴격자 속성」 — title, widgets, terrain, live plan, and actions**: Title `"시굴격자 속성"` -> `"시굴격자 속성 - Strata"` (`KaTrenchDialog.cpp:27` -> `dlg_trench_generator.rs:32`); `m_kind` and `m_shape` are cycling buttons instead of `ComboBox` (`dlg_trench_generator_controls.rs:27-39,69-82`); `m_size` (`"규격"`, 20m/10m) became `"트렌치 길이"` 3 radio buttons (`20 m`, `15 m`, `10 m`) (`:84-99`); `m_terrain` is always enabled with hardcoded `135° · 4.2%` text (`:54-64`); `"구역에 깔기"` does not generate a grid (`dlg_trench_generator.rs:56-72`).
11. **C1 「GPS 기준점」 — hint, labels, combos, button text, Enter, and error display**: Hint hardcoded to EPSG:5187 (`modal_control.rs:32`); `"X (동쪽)"`/`"Y (북쪽)"` mutate when swap is checked (`:57-73`); `"원점"` and `"수신상태"` (`"수신 상태"`) are `ComboBox` instead of `QLineEdit` (`:140-165,200-217`); `"확인"` -> `"지도에 추가 (확인)"` (`:229`); `Enter` does not submit; validation error goes to status bar (`:242-245`).
12. **C4 「이름·번호」 — modality, width, completer, and error display**: `min_width` `380` vs C++ `440` (`KaFeatureFormDialog.cpp:71`, `KaWindowSize.h:15` -> `dlg_feature_form.rs:77`); no typing completer on `kind`/`period`; save error goes to status bar instead of `KaUserError::warn` (`app_state_feature_form.rs:38-40`).
13. **C9 「도형 속성 — <layer>」 — opener, completer, and error display**: Layer context menu `"도형 속성"` is disabled (`layer_menu_model.rs:187`); no typing completer on `kind`/`period`; validation/save errors go to status bar instead of modal (`app_state_feature_record.rs:41-52`).

## Could not determine

- Nothing left undetermined for C1–C11, C13–C14: every C++ file and Rust counterpart (or explicit absence) was inspected directly in the source. (Note: `MainWindowEditing.cpp:408`, `:430`, `:612` are `#else` non-QGIS stub `QMessageBox::information(..., "QGIS 빌드 필요")` and were not given rows in C14 since they are unreachable in QGIS builds.)

## Counts

- **Windows audited**: 13 (C1, C2, C3, C4, C5, C6, C7, C8, C9, C10, C11, C13, C14)
- **Table rows**: 110 total
- **C++ controls**: 109 (C1: 18, C2: 5, C3: 4, C4: 8, C5: 18, C6: 7, C7: 6, C8: 13, C9: 6, C10: 5, C11: 7, C13: 1, C14: 11)
- **Rust controls**: 58 (57 matching/modified + 1 `EXTRA` row in C7; 52 `MISSING` rows across C2, C3, C6, C8, C10, C11, C13, C14)
- **Differences by `same?` status**:
  - `yes`: 28
  - `no`: 29 (text/title/tooltip/placeholder, kind/options/default/enabled-when/handler, plus 1 inline extra control in C7 row #4)
  - `MISSING`: 52
  - `EXTRA`: 1
- **Verification**: Recounted all 110 rows across all 13 windows; checked 30 random citations across C++ and Rust files — 30/30 exact.
