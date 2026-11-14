# Controls Survey — Group A (project, account, submit and location windows)

Read-only control-by-control comparison of C++ Strata (`target/cpp-ref/src/app`) and Rust (`crates/strata-app/src`) for windows A1–A14.

Note on C++ `paperLook`: C++ `KaTheme::displayOptions().paperLook` defaults to `false` (`target/cpp-ref/src/app/KaTheme.h:69`), and `qApp->property("kaPaperLook")` tracks it (`KaTheme.cpp:272`). Where a C++ dialog branches on `m_paper` / `paperLook`, both branches are noted; Rust targets the `paperLook = true` («조용한 도판») layout.

Note on Rust modal Esc/backdrop: In Rust, `ActiveModalDialog` windows use `egui::Window::new(...).title_bar(false)` without a modal backdrop (clicks behind the window are not blocked). Pressing `Esc` is handled globally in `gui_shell.rs:34-36` (`self.modals.active = None`) when `self.modal_window_open()` is `false` (which is true for `ActiveModalDialog` modals, unless a widget consumes `Esc`). `dlg_survey_area.rs` is rendered via `home_map.survey_area_setup` (`shell_workspace.rs:132`) and handles `Enter`/`Esc` directly (`dlg_survey_area.rs:26`).

---

## A1. 「새 조사」 (`KaNewSurveyDialog`)

- **C++ title & built at**: `새 조사` — `target/cpp-ref/src/app/KaNewSurveyDialog.cpp:48`
- **Opened in C++**: Ribbon «새 조사» / Home screen «새 조사» -> `MainWindow::newSurvey()` -> `KaNewSurveyDialog dlg(m_workCrs, mapLongitude, this)` (`target/cpp-ref/src/app/MainWindowSession.cpp:685`)
- **Opened in Rust**: Ribbon «새 조사» (`crates/strata-app/src/gui_ribbon_click_a.rs:48`), Home hero «새 조사» (`crates/strata-app/src/gui_tab_home_hero.rs:82`), Home recent (`crates/strata-app/src/gui_tab_home_recent.rs:135`), shortcut (`crates/strata-app/src/gui_shell.rs:126`) -> `ActiveModalDialog::NewSurvey`
- **Modality**: C++ `dlg.exec()` — application-modal (`target/cpp-ref/src/app/MainWindowSession.cpp:686`). Rust `egui::Window::new("ka_modal_new_survey")` without backdrop — non-modal (`crates/strata-app/src/dlg_new_survey.rs:19`).
- **Size rules**: C++ `setMinimumWidth(KaWindowSize::width(KaWindowSize::Medium, 460))`, resizable (`KaNewSurveyDialog.cpp:49`). Rust `.resizable(false).fixed_size([460.0_f32.min(ctx.screen_rect().width() - 32.0), 440.0_f32.min(ctx.screen_rect().height() - 32.0)])` (`dlg_new_survey.rs:21-26`).
- **Rust counterpart**: `crates/strata-app/src/dlg_new_survey.rs:18-206`, `crates/strata-app/src/dlg_new_survey_ruler.rs:12-126`

| # | kind | C++ text | C++ tooltip / placeholder | C++ default, range, step, suffix | C++ enabled-when | C++ what it does | C++ file:line | Rust text | Rust tooltip / placeholder | Rust default, range, step, suffix | Rust enabled-when | Rust what it does | Rust file:line | same? |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 1 | label | `조사명` | — | — | always | Form row label for `m_name` | `KaNewSurveyDialog.cpp:106` | `조사명` | — | — | always | Strong label above name input | `dlg_new_survey.rs:29` | yes |
| 2 | lineEdit | empty | placeholder `예: 병산동` | `""`, maxLength `100` (`kMaxNameLength + 20`) | always | `textChanged` -> `updateState()`, sets `surveyName()` | `KaNewSurveyDialog.cpp:55-59,153` | `app.modals.new_survey_name` | hint `예: 병산동` | `""`, no maxLength | always | Clears `new_survey_error` on change | `dlg_new_survey.rs:30-39` | no (maxLength) |
| 3 | label | dynamic `problem` | — | hidden when empty or trimmed input is empty | visible when `typed && !problem.isEmpty()` | Shows red validation error | `KaNewSurveyDialog.cpp:60-64,169-170` | dynamic `err_msg` | — | hidden when trimmed input is empty | visible when `!trimmed.is_empty()` and `Err` | Shows red validation error | `dlg_new_survey.rs:42-51` | yes |
| 4 | label | `작업 좌표계` (`m_paper` captionWord; stock form row label) | — | — | always | Section caption for CRS choice | `KaNewSurveyDialog.cpp:110,139` | `작업 좌표계` | — | — | always | Strong label on left | `dlg_new_survey.rs:54` | yes |
| 5 | label | `동경 128°를 경계로 나뉩니다` (`m_paper` only) | — | — | always (`m_paper`) | Right-aligned boundary note | `KaNewSurveyDialog.cpp:114` | `동경 128°를 경계로 나뉩니다` | — | — | always | Right-aligned muted label | `dlg_new_survey.rs:56-60` | yes |
| 6 | custom widget | `KaLongitudeRuler` (`중부원점` \| `동부원점`, 8 towns, 126°–130°, 0.1° ticks, center mark) (`m_paper` only) | tooltip `동경 128° 서쪽은 중부원점(5186), 동쪽은 동부원점(5187)\n눈금: 제주 · 광주 · 대전 · 춘천 · 대구 · 강릉 · 부산 · 울산` | height `88`, minWidth `320` | always (`m_paper`) | Visualizes longitude vs 128° boundary | `KaNewSurveyDialog.cpp:117-120`, `KaLongitudeRuler.cpp:36-47` | `paint_longitude_ruler` (`중부원점 \| 동부원점`, 8 towns, 126°–130°, 0.1° ticks, center mark) | none | `440×62` | always | Visualizes longitude vs 128° boundary | `dlg_new_survey_ruler.rs:12-126` | no (tooltip, size) |
| 7 | pushButton (checkable) | `5186  중부원점` (+ `추천` badge if `m_suggested == "EPSG:5186"` & `m_paper`) | — | checked if `!currentWorkCrs.contains("5187")` | always | Exclusive button group -> `updateState()` | `KaNewSurveyDialog.cpp:67,73-84,124-135` | `5186 중부원점` (+ `  추천` + `\n경도 128° 서쪽 · 서울·대전·전주·광주`) | — | selected if `new_survey_crs == Epsg5186` | always | Sets `app.modals.new_survey_crs = Epsg5186` | `dlg_new_survey.rs:73-111` | no (text) |
| 8 | pushButton (checkable) | `5187  동부원점` (+ `추천` badge if `m_suggested == "EPSG:5187"` & `m_paper`) | — | checked if `currentWorkCrs.contains("5187")` | always | Exclusive button group -> `updateState()` | `KaNewSurveyDialog.cpp:68,73-84,124-135` | `5187 동부원점` (+ `  추천` + `\n경도 128° 동쪽 · 강릉·대구·경주·부산`) | — | selected if `new_survey_crs == Epsg5187` | always | Sets `app.modals.new_survey_crs = Epsg5187` | `dlg_new_survey.rs:73-111` | no (text) |
| 9 | label | `지금 지도 중심(동경 %1°)은 %2 구역입니다.` (+ boundary/mismatch advice if `!m_paper`) | — | visible when `!m_suggested.isEmpty()` | visible when `m_suggested` non-empty | Shows zone of current map center | `KaNewSurveyDialog.cpp:93-96,172-184` | `지금 지도 중심(동경 {lon:.1}°)은 {zone} 구역입니다.` | — | visible when `map_lon` & `suggested` are `Some` | visible when `Some` | Shows zone of current map center | `dlg_new_survey.rs:115-129` | yes (`m_paper`) |
| 10 | label | `경계 가까운 곳은 발주처·측량 성과의 원점을 따르세요.` (`m_paper`; stock: `regionHint()`) | — | — | always | Boundary advice label | `KaNewSurveyDialog.cpp:88-90,214-219` | `경계 가까운 곳은 발주처·측량 성과의 원점을 따르세요.` | — | — | always | Muted label | `dlg_new_survey.rs:130-136` | yes (`m_paper`) |
| 11 | label | `작업 좌표계는 나중에 화면 아래에서 바꿀 수 있습니다.\n제출 파일은 언제나 5179로 바뀝니다.` (`m_paper`; stock: 2-sentence variant) | — | — | always | Bottom tip label | `KaNewSurveyDialog.cpp:97-102` | Split into 2 labels: `작업 좌표계는 나중에 화면 아래에서 바꿀 수 있습니다.` + `제출 파일은 언제나 5179로 바뀝니다.` | — | — | always | Two muted labels | `dlg_new_survey.rs:137-148` | yes (text) |
| 12 | pushButton (Ok) | `다음` | — | default button | `nameProblem(m_name->text()).isEmpty()` | `accepted` -> `accept()` if valid; caller opens `저장 폴더` picker | `KaNewSurveyDialog.cpp:145-146,149-151,171` | `다음` | — | — | `name_problem.is_ok()` | Opens `rfd::FileDialog::pick_folder("저장 폴더")` and creates `.gpkg` | `dlg_new_survey.rs:168-202` | no (default/Enter) |
| 13 | pushButton (Cancel) | `취소` | — | — | always | `rejected` -> `QDialog::reject()` | `KaNewSurveyDialog.cpp:147,152` | `취소` | — | — | always | Clears `new_survey_error`, closes modal | `dlg_new_survey.rs:161-167` | yes |
| 14 | label (EXTRA) | — | — | — | — | — | — | `app.modals.new_survey_error` | — | hidden unless `Some` | visible when `create_new_survey` fails | Shows creation error inline without closing | `dlg_new_survey.rs:150-157` | EXTRA |
| 15 | header (EXTRA) | — | — | — | — | — | — | `새 조사 - Strata` + `×` | — | — | always | Custom title bar (`render_strata_modal_header`) | `dlg_new_survey.rs:28` | EXTRA (title differs) |

- **Keyboard & closing**:
  - C++: Default button `다음` (`QDialogButtonBox::Ok`); `Enter` accepts if `nameProblem` is empty; `Esc` rejects; `m_name->setFocus()` at open (`KaNewSurveyDialog.cpp:144-155`); `X` rejects.
  - Rust: No default button / no `Enter` handler; no `request_focus()` on name edit; `Esc` closes via `gui_shell.rs:34-36`; `×` button sets `open = false`.
- **Messages**:
  - `조사명을 입력하세요.` (`KaNewSurveyDialog.cpp:189` — button disabled when empty, not shown in red) / Rust `validate_survey_name_ko` (hidden when empty, `dlg_new_survey.rs:43`).
  - `조사명에는 \ / : * ? " < > | 와 제어 문자를 쓸 수 없습니다.` (`KaNewSurveyDialog.cpp:193`) / Rust `validate_survey_name_ko`.
  - `조사명은 마침표(.)로 끝날 수 없습니다. Windows가 파일 이름 끝의 점을 지웁니다.` (`KaNewSurveyDialog.cpp:196`) / Rust `validate_survey_name_ko`.
  - `「%1」은(는) Windows가 장치 이름으로 쓰는 예약 이름이라 조사명으로 쓸 수 없습니다.` (`KaNewSurveyDialog.cpp:198-199`) / Rust `validate_survey_name_ko`.
  - `조사명은 80자 이하로 줄여 주세요.` (`KaNewSurveyDialog.cpp:201`) / Rust `validate_survey_name_ko`.
  - Creation failure: C++ `notify(Notice::Warning, "새 조사를 만들지 못했습니다", err)` after closing (`MainWindowSession.cpp:698`); Rust inline `new_survey_error_text(&e)` (`dlg_new_survey.rs:150-157, 210-215`).

---

## A2. 「조사 닫기」 (`KaClosePrompt`)

- **C++ title & built at**: `조사 닫기` — `target/cpp-ref/src/app/KaClosePrompt.cpp:26`
- **Opened in C++**: `MainWindow::closeEvent` when `surveyHasUnsavedChanges()` (`target/cpp-ref/src/app/MainWindow.cpp:342`)
- **Opened in Rust**: `StrataAppState::render_close_prompt` when `viewport().close_requested() && self.is_dirty` (`crates/strata-app/src/shell_dialogs.rs:67-72`)
- **Modality**: C++ `box.exec()` — application-modal `QMessageBox` (`KaClosePrompt.cpp:51`). Rust `egui::Window::new("ka_close_prompt")` without backdrop — non-modal (`shell_dialogs.rs:83`).
- **Size rules**: C++ `QMessageBox` auto-sized. Rust `.resizable(false).fixed_size([420.0_f32.min(full.width() - 32.0), 168.0_f32.min(full.height() - 32.0)])` (`shell_dialogs.rs:77-87`).
- **Rust counterpart**: `crates/strata-app/src/shell_dialogs.rs:67-141` (Note: `modal_controller_impl.rs:68-92` has `with_object_particle` and `format_close_prompt` matching C++, but `shell_dialogs.rs` does not call them.)

| # | kind | C++ text | C++ tooltip / placeholder | C++ default, range, step, suffix | C++ enabled-when | C++ what it does | C++ file:line | Rust text | Rust tooltip / placeholder | Rust default, range, step, suffix | Rust enabled-when | Rust what it does | Rust file:line | same? |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 1 | label (main text) | `m_paper`: `{name}을(를) 저장할까요?` (`withObjectParticle`); stock: `저장하지 않은 작업이 있습니다.\n\n{fileName}\n\n저장할까요?` | — | — | always | Heading question | `KaClosePrompt.cpp:32,41` | `저장하지 않은 편집이 있습니다` | — | — | always | Strong label | `shell_dialogs.rs:89-93` | no (text) |
| 2 | label (informative) | `m_paper`: `저장하지 않은 작업이 {N}건 있습니다.\n{folder} › {fileName}` (or `저장하지 않은 작업이 있습니다.\n...`) | — | — | always (`m_paper`) | Unsaved count and path | `KaClosePrompt.cpp:42-45` | `끝내기 전에 지금까지 그린 조사 내용을 저장할까요?` | — | — | always | Muted label | `shell_dialogs.rs:95-99` | no (text) |
| 3 | pushButton (Save) | `QMessageBox::Save` (Qt standard Save) | — | default button | always | Returns `Save`; caller runs `persistSurveyWork()` | `KaClosePrompt.cpp:27-28`, `MainWindow.cpp:347` | `저장하고 끝내기` | — | — | always | Calls `save_survey_project()`, closes if `!is_dirty` | `shell_dialogs.rs:102-115` | no (text, default) |
| 4 | pushButton (Discard) | `QMessageBox::Discard` (Qt standard Discard, `#closePromptDiscard` danger style in `m_paper`) | — | — | always | Returns `Discard`; caller closes without saving | `KaClosePrompt.cpp:27,36-39` | `저장하지 않고 끝내기` | — | — | always | Closes `close_prompt_open` and sends `ViewportCommand::Close` | `shell_dialogs.rs:116-127` | no (text) |
| 5 | pushButton (Cancel) | `QMessageBox::Cancel` (Qt standard Cancel) | — | — | always | Returns `Cancel`; `event->ignore()` | `KaClosePrompt.cpp:27`, `MainWindow.cpp:343` | `취소` | — | — | always | Sets `close_prompt_open = false` | `shell_dialogs.rs:128-138` | yes |

- **Keyboard & closing**:
  - C++: Default button `QMessageBox::Save`; `Enter` triggers `Save`; `Esc` and `X` trigger `Cancel`.
  - Rust: `.title_bar(false)` (no window title `조사 닫기` and no `×` button); no `Enter`/`Esc` key handling in `shell_dialogs.rs` (`gui_shell.rs:34` only clears `self.modals.active`, not `close_prompt_open`).
- **Messages**: Only the prompt text in rows 1–2 above.

---

## A3. 「API 키 입력」 (`KaApiKeyDialog`)

- **C++ title & built at**: `API 키 입력` — `target/cpp-ref/src/app/KaApiKeyDialog.cpp:18`
- **Opened in C++**: «더보기» menu -> `API 키 입력` (`target/cpp-ref/src/app/MainWindowRibbon.cpp:677`) or `MainWindowChrome.cpp:221` -> `MainWindow::configureVworldKey()` (`target/cpp-ref/src/app/MainWindowSession.cpp:1768`)
- **Opened in Rust**: Combined into `ActiveModalDialog::AccountSettings` (`crates/strata-app/src/modal_paper_account.rs:147`), opened from «더보기» menu `API 키 입력 · 계정 설정…` (`gui_ribbon_menus_b.rs:82,114`), Home status card (`gui_tab_home_status.rs:108`), etc.
- **Modality**: C++ `dlg.exec()` — application-modal (`MainWindowSession.cpp:1769`). Rust `egui::Window::new("ka_modal_account")` without backdrop — non-modal (`modal_paper_account.rs:148`).
- **Size rules**: C++ `setMinimumWidth(KaWindowSize::width(KaWindowSize::Small, 460))`, resizable (`KaApiKeyDialog.cpp:19`). Rust `.resizable(false).default_width(520.0)` (`modal_paper_account.rs:150-152`).
- **Rust counterpart**: `crates/strata-app/src/modal_paper_account.rs:147-292` (combined with A10 + NGII + Heritage accounts)

| # | kind | C++ text | C++ tooltip / placeholder | C++ default, range, step, suffix | C++ enabled-when | C++ what it does | C++ file:line | Rust text | Rust tooltip / placeholder | Rust default, range, step, suffix | Rust enabled-when | Rust what it does | Rust file:line | same? |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 1 | label | `VWorld API 키` | — | — | always | Label for `m_vworld` | `KaApiKeyDialog.cpp:48,70` | `VWorld API 키:` | — | — | always | Grid label (with colon) | `modal_paper_account.rs:166` | no (text) |
| 2 | lineEdit | `vworldKey` | placeholder `vworld.kr 인증키` | loaded `VworldSettings::loadApiKey()` | always | Edits VWorld API key | `KaApiKeyDialog.cpp:20-23` | `app.accounts.vworld_key` | hint `vworld.kr 인증키` | loaded from account settings | always | Edits VWorld API key | `modal_paper_account.rs:168-172` | yes |
| 3 | label | `위성 · 지적 · 주소 찾기에 함께 씁니다.` (`paper`; stock: `위성·지적·검색 공통 (SSOT: VWorld/ApiKey)`) | — | — | always | Hint under VWorld key | `KaApiKeyDialog.cpp:28-30` | `위성 · 지적 · 주소 찾기에 함께 씁니다.` | — | — | always | Muted label under VWorld key | `modal_paper_account.rs:173-177` | yes (`paper`) |
| 4 | label | `역사지리정보DB API 키` | — | — | always | Label for `m_history` | `KaApiKeyDialog.cpp:50,71` | `역사지리정보DB 키:` | — | — | always | Grid label (missing `API`, has colon) | `modal_paper_account.rs:180` | no (text) |
| 5 | lineEdit | `historyKey` | placeholder `hgis.history.go.kr 인증키` | loaded `VworldSettings::loadHistoryGisApiKey()` | always | Edits History GIS API key | `KaApiKeyDialog.cpp:24-27` | `app.accounts.history_gis_key` | hint `hgis.history.go.kr 인증키` | loaded from account settings | always | Edits History GIS API key | `modal_paper_account.rs:182-188` | yes |
| 6 | label | `1919 조선지형도 전용. VWorld 키로는 인증되지 않습니다.` (stock adds `\nhttps://hgis.history.go.kr/api/intro.do`) | — | — | always | Hint under History GIS key | `KaApiKeyDialog.cpp:33,43` | `1919 조선지형도 전용. VWorld 키로는 인증되지 않습니다.` | — | — | always | Muted label | `modal_paper_account.rs:189-195` | yes (`paper`) |
| 7 | pushButton (Ok) | `저장` | — | default button | always | `accept()` -> saves both keys, updates status bar | `KaApiKeyDialog.cpp:36-39`, `MainWindowSession.cpp:1770-1787` | `저장` | — | — | always | Validates credentials, `save_all_to_disk()`, closes modal | `modal_paper_account.rs:270-288` | yes |
| 8 | pushButton (Cancel) | `취소` | — | — | always | `reject()` | `KaApiKeyDialog.cpp:36,38,40` | `닫기` | — | — | always | Sets `app.modals.active = None` | `modal_paper_account.rs:267-269` | no (text) |
| 9 | label (EXTRA) | — | — | — | — | — | — | `app.accounts.status_summary_ko` | — | — | always | Combined account status summary | `modal_paper_account.rs:155-159` | EXTRA |
| 10 | grid rows (EXTRA) | — | — | — | — | — | — | `VWorld 지적도 ID / PW:`, `수치지형도(NGII) ID / PW:`, `국가유산청 인트라넷 ID / PW:` (6 `TextEdit`s) + NGII description label | — | — | always | Combined site credential inputs | `modal_paper_account.rs:198-258` | EXTRA |
| 11 | checkBox (EXTRA) | — | — | — | — | — | — | `비밀번호 표시` | — | `false` | always | Toggles password masking | `modal_paper_account.rs:265` | EXTRA |
| 12 | header (EXTRA) | — | — | — | — | — | — | `API 키 · 사이트 계정 설정 - Strata` + `×` | — | — | always | Custom title bar | `modal_paper_account.rs:154` | EXTRA (title differs) |

- **Keyboard & closing**:
  - C++: Default button `저장`; `Enter` accepts; `Esc` and `X` reject.
  - Rust: No `Enter` handler; `Esc` closes via `gui_shell.rs:34-36`; `×` closes.
- **Messages**:
  - C++ status bar after save: `API 키 삭제됨` or `{saved} 키 저장됨` (`MainWindowSession.cpp:1785-1787`).
  - Rust guide message after save: `API 키와 사이트 계정(DPAPI 암호화)을 저장했습니다.` (`modal_paper_account.rs:280`); validation/encryption error shown inline in red (`modal_paper_account.rs:260-262, 286`).

---

## A4. 「용지 설정」 (`KaPaperDialog`)

- **C++ title & built at**: `용지 설정` — `target/cpp-ref/src/app/KaPaperDialog.cpp:45`
- **Opened in C++**: Drawing studio / ribbon «도면» -> `KaPaperDialog::prompt(...)` (`target/cpp-ref/src/app/KaDrawingStudio.cpp:366`)
- **Opened in Rust**: Ribbon «도면» (`gui_ribbon_click_b.rs:130`), Home recent (`gui_tab_home_recent.rs:137`), Drawing toolbar (`gui_tab_drawing_toolbar.rs:117`), `gui_shell.rs:140` -> `ActiveModalDialog::PaperSetup`
- **Modality**: C++ `dialog.exec()` — application-modal (`KaPaperDialog.cpp:194`). Rust `egui::Window::new("ka_modal_paper_setup")` without backdrop — non-modal (`modal_paper_account.rs:46`).
- **Size rules**: C++ `setMinimumWidth(KaWindowSize::width(KaWindowSize::Small, 400))`, resizable (`KaPaperDialog.cpp:46`). Rust `.resizable(false).default_width(480.0)` (`modal_paper_account.rs:46`).
- **Rust counterpart**: `crates/strata-app/src/modal_paper_account.rs:39-145`

| # | kind | C++ text | C++ tooltip / placeholder | C++ default, range, step, suffix | C++ enabled-when | C++ what it does | C++ file:line | Rust text | Rust tooltip / placeholder | Rust default, range, step, suffix | Rust enabled-when | Rust what it does | Rust file:line | same? |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 1 | label | `용지` | — | — | always | Form row label | `KaPaperDialog.cpp:138` | `용지` | — | — | always | Row label | `modal_paper_account.rs:50` | yes |
| 2 | toolButton (checkable) | `A4` | — | checked `true` | always | Exclusive `paperGroup` (id 0) -> `applyPreset()` | `KaPaperDialog.cpp:64,69,177` | `A4` | — | selected `PaperChoice::A4` | always | Sets `draft.choice = A4`, `apply_preset()` | `modal_paper_account.rs:51-59` | yes |
| 3 | toolButton (checkable) | `A3` | — | checked `false` | always | Exclusive `paperGroup` (id 1) -> `applyPreset()` | `KaPaperDialog.cpp:65,70,177` | `A3` | — | — | always | Sets `draft.choice = A3`, `apply_preset()` | `modal_paper_account.rs:51-59` | yes |
| 4 | toolButton (checkable) | `직접 입력` | — | checked `false` | always | Exclusive `paperGroup` (id 2) -> `applyPreset()` | `KaPaperDialog.cpp:66,71,177` | `직접 입력` | — | — | always | Sets `draft.choice = Custom`, `apply_preset()` | `modal_paper_account.rs:51-59` | yes |
| 5 | label | `방향` | — | — | always | Form row label | `KaPaperDialog.cpp:139` | `방향` | — | — | always | Row label | `modal_paper_account.rs:64` | yes |
| 6 | toolButton (checkable, icon) | `세로` | — | checked `true` | always | Exclusive `orientGroup` (id 0) -> `applyPreset()` | `KaPaperDialog.cpp:80,84,178` | `세로` | — | selected `SheetOrientation::Portrait` | always | Sets `draft.orientation = Portrait`, `apply_preset()` | `modal_paper_account.rs:65-82` | yes |
| 7 | toolButton (checkable, icon) | `가로` | — | checked `false` | always | Exclusive `orientGroup` (id 1) -> `applyPreset()` | `KaPaperDialog.cpp:81,85,178` | `가로` | — | — | always | Sets `draft.orientation = Landscape`, `apply_preset()` | `modal_paper_account.rs:65-82` | yes |
| 8 | label | `가로` | — | — | always | Form row label for width | `KaPaperDialog.cpp:140` | `가로` | — | — | always | Row label for width | `modal_paper_account.rs:88-90` | yes |
| 9 | doubleSpinBox / reading line | `210.0 mm` (or `210 mm` reading line in `plate`) | — | default `210.0`, range `50.0..=2000.0`, step `1.0`, decimals `1`, suffix `" mm"` | enabled/editable when `직접 입력` (`custom`) | Stores `widthMm()` | `KaPaperDialog.cpp:90-100,154-175` | `DragValue` (when `!paper_look \|\| custom`) or dotted reading line (`plain_mm` + `mm`) | — | default `210.0`, range `50.0..=2000.0`, speed `1.0`, decimals `1`, suffix `" mm"` | `add_enabled(custom, ...)` | Edits `draft.width_mm` | `modal_paper_account.rs:91-101` | yes |
| 10 | label | `세로` | — | — | always | Form row label for height | `KaPaperDialog.cpp:141` | `세로` | — | — | always | Row label for height | `modal_paper_account.rs:88-90` | yes |
| 11 | doubleSpinBox / reading line | `297.0 mm` (or `297 mm` reading line in `plate`) | — | default `297.0`, range `50.0..=2000.0`, step `1.0`, decimals `1`, suffix `" mm"` | enabled/editable when `직접 입력` (`custom`) | Stores `heightMm()` | `KaPaperDialog.cpp:90-100,154-175` | `DragValue` (when `!paper_look \|\| custom`) or dotted reading line (`plain_mm` + `mm`) | — | default `297.0`, range `50.0..=2000.0`, speed `1.0`, decimals `1`, suffix `" mm"` | `add_enabled(custom, ...)` | Edits `draft.height_mm` | `modal_paper_account.rs:91-101` | yes |
| 12 | label | `기본은 A4 세로입니다. A4 가로는 좌·우·위 1cm를 더 줍니다.` (or A4 landscape note) | — | updates dynamically | always | Explains A4 landscape field sheet size | `KaPaperDialog.cpp:135,149-153` | same text via `draft.scale_note()` | — | updates dynamically | always | Same text | `modal_paper_account.rs:33-36,105` | yes |
| 13 | pushButton (Ok) | `용지 만들기` | — | default button | always | `accept()` | `KaPaperDialog.cpp:180-184` | `용지 만들기` | — | — | always | Applies paper to `app.drawing`, opens `Drawing` tab, closes | `modal_paper_account.rs:113-137` | no (default/Enter) |
| 14 | pushButton (Cancel) | `취소` | — | — | always | `reject()` | `KaPaperDialog.cpp:180,185` | `취소` | — | — | always | Clears draft, closes modal | `modal_paper_account.rs:108-112` | yes |
| 15 | header (EXTRA) | — | — | — | — | — | — | `용지 설정` + `×` | — | — | always | Custom title bar (`render_strata_modal_header`) | `modal_paper_account.rs:48` | EXTRA |

- **Keyboard & closing**:
  - C++: Default button `용지 만들기`; `Enter` accepts; `Esc` and `X` reject.
  - Rust: No `Enter` handler; `Esc` closes via `gui_shell.rs:34-36`; `×` closes and clears draft.
- **Messages**:
  - A4 landscape note: `A4 가로는 좌·우·위 1cm를 더 준 현장 용지(317×220mm)입니다. A4 종이 한 장에 찍으면 약 90%로 줄어 종이 위 축척이 달라집니다. 축척대로 찍으려면 인쇄 창에서 A3 또는 여러 장 나눠 찍기를 고르세요.` (`KaPaperDialog.cpp:150-152` / `modal_controller_impl.rs:100`).

---

## A5. 「조사구역 그리기 설정」 (`KaSurveyAreaDialog`)

- **C++ title & built at**: `조사구역 그리기 설정` — `target/cpp-ref/src/app/KaSurveyAreaDialog.cpp:81`
- **Opened in C++**: Ribbon «그리기» -> 조사구역 -> `MainWindow::startEditSurveyArea()` when no existing `survey_area` layer to continue (`target/cpp-ref/src/app/MainWindowEditing.cpp:1100-1103`)
- **Opened in Rust**: `start_edit_survey_area` when no existing `survey_area` layer (`crates/strata-app/src/app_state_draw_entry.rs:34`) -> `self.home_map.survey_area_setup = Some(...)` -> `render_survey_area_setup` (`crates/strata-app/src/dlg_survey_area.rs:24`). *(Note: an older unused stub `ActiveModalDialog::SurveyArea` also exists at `crates/strata-app/src/modal_survey.rs:10`, only reachable via `debug_control.rs:112`.)*
- **Modality**: C++ `setModal(true)` + `exec()` (`KaSurveyAreaDialog.cpp:82`, `MainWindowEditing.cpp:1104`). Rust `egui::Window::new("조사구역 그리기 설정")` without backdrop — non-modal (`dlg_survey_area.rs:27`).
- **Size rules**: C++ `if (m_paper) setMinimumWidth(440)`, resizable (`KaSurveyAreaDialog.cpp:96`). Rust `.resizable(false).min_width(440.0)` (`dlg_survey_area.rs:29,31`).
- **Rust counterpart**: `crates/strata-app/src/dlg_survey_area.rs:24-104`, `crates/strata-app/src/survey_area_setup.rs:14-65`

| # | kind | C++ text | C++ tooltip / placeholder | C++ default, range, step, suffix | C++ enabled-when | C++ what it does | C++ file:line | Rust text | Rust tooltip / placeholder | Rust default, range, step, suffix | Rust enabled-when | Rust what it does | Rust file:line | same? |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 1 | label | `이름` | — | — | always | Form label | `KaSurveyAreaDialog.cpp:105` | `이름` | — | — | always | Fixed-width `96.0` label | `dlg_survey_area.rs:36` | yes |
| 2 | lineEdit | `조사구역` | — | default `"조사구역"` | always | Sets `layerName()` (falls back to `"조사구역"` if empty) | `KaSurveyAreaDialog.cpp:104,262-265` | `setup.name` | — | default `"조사구역"` | always | Edits `setup.name` | `dlg_survey_area.rs:37` | yes |
| 3 | label | `외곽선 색` + chosen color name (`m_colorName`, e.g. `주황`) | — | default `주황` (index 1) | always | Shows selected swatch name | `KaSurveyAreaDialog.cpp:130-135,225` | `외곽선 색` + `setup.color_name()` | — | default `주황` (index 1) | always | Shows selected swatch name | `dlg_survey_area.rs:42-44` | yes |
| 4..11 | 8 color buttons | empty in `m_paper` (named in stock: `빨강`, `주황`, `노랑`, `초록`, `파랑`, `남색`, `보라`, `갈색`) | tooltip color name (`m_paper`) | `52×32` (`m_paper`), 4 per row, default index 1 (`주황`) | always | `selectColorIndex(i)` | `KaSurveyAreaDialog.cpp:111-123` | 8 color swatches (`52×32`, 4 per row) | hover `name` | default index 1 (`주황`) | always | Sets `setup.color = idx` | `dlg_survey_area.rs:48-65` | yes (`m_paper`) |
| 12 | label | `선 굵기` | — | — | always | Form label | `KaSurveyAreaDialog.cpp:181,193` | `선 굵기` | — | — | always | Fixed-width `96.0` label | `dlg_survey_area.rs:69` | yes |
| 13..16 | 4 preset buttons (checkable) | `보통 1.0`, `기본 1.5`, `굵게 2.0`, `강조 3.0` | — | default `기본 1.5` checked | always | `selectPresetWidth(w)` updates spinbox | `KaSurveyAreaDialog.cpp:147-167,241-248` | `보통 1.0`, `기본 1.5`, `굵게 2.0`, `강조 3.0` | — | selected when `chosen_preset() == Some(i)` | always | Sets `setup.width_mm = *w` | `dlg_survey_area.rs:71-77` | yes |
| 17 | label | `직접` (`m_paper` only) | — | — | always (`m_paper`) | Form label for custom spinbox | `KaSurveyAreaDialog.cpp:182` | `직접` | — | — | always | Fixed-width `96.0` label | `dlg_survey_area.rs:80` | yes |
| 18 | doubleSpinBox | `1.5 mm` | — | default `1.5`, range `0.5..=10.0`, step `0.2`, decimals `1`, suffix `" mm"` | always | Updates `strokeWidthMm()`, unchecks presets | `KaSurveyAreaDialog.cpp:168-178` | `DragValue` | — | default `1.5`, range `0.5..=10.0`, speed `0.2`, decimals `1`, suffix `" mm"` | always | Edits `setup.width_mm` | `dlg_survey_area.rs:81-84` | yes |
| 19 | pushButton (default) | `그리기 시작 (Enter)` | — | default button | always | `accept()` | `KaSurveyAreaDialog.cpp:200-206` | `그리기 시작 (Enter)` | — | — | always | Calls `accept_survey_area_setup()` | `dlg_survey_area.rs:92-95,101-102` | yes |
| 20 | pushButton | `취소` | — | — | always | `reject()` | `KaSurveyAreaDialog.cpp:207-210` | `취소` | — | — | always | Sets `survey_area_setup = None` | `dlg_survey_area.rs:89-91,99-100` | yes |

- **Keyboard & closing**:
  - C++: Default button `그리기 시작 (Enter)`; `Enter` accepts; `Esc` and `X` reject.
  - Rust: `Enter` starts drawing (`dlg_survey_area.rs:26,101-102`), `Esc` cancels (`dlg_survey_area.rs:26,99-100`); `.collapsible(false)` standard title bar has no `×` close button (unlike `QDialog`).
- **Messages**: None inside the dialog.

---

## A6. 「검수·제출」 (`KaSubmitDialog` + `KaSubmitDialogPlate` + `KaSubmitFlow` sub-dialogs)

- **C++ title & built at**: `검수·제출` — `target/cpp-ref/src/app/KaSubmitDialog.cpp:25`, `KaSubmitDialogPlate.cpp:12,64`, `KaSubmitFlow.cpp:145,158,181,198`
- **Opened in C++**: Ribbon «검수·제출» (`MainWindowRibbon.cpp:577`) or status bar (`MainWindowChrome.cpp:158`) -> `MainWindow::openSubmitReview()` (`KaSubmitFlow.cpp:54`)
- **Opened in Rust**: Ribbon «검수·제출» (`gui_ribbon_click_b.rs:146`), shortcut (`gui_shell.rs:148`), `shell_workspace.rs:158` -> `ActiveModalDialog::ExportSubmitPackage`
- **Modality**: C++ `dialog.exec()` — application-modal (`KaSubmitFlow.cpp:70`). Rust `egui::Window::new("ka_modal_submit")` without backdrop — non-modal (`modal_export.rs:24`).
- **Size rules**: C++ `resize(820, 580)`, resizable (`KaSubmitDialog.cpp:26`). Rust `.resizable(false).fixed_size([820.0_f32.min((vis_w - 40.0).max(640.0)), 540.0_f32.min((vis_h - 60.0).max(420.0))])` (`modal_export.rs:18-28`).
- **Rust counterpart**: `crates/strata-app/src/modal_export.rs:13-160`, `crates/strata-app/src/modal_export_table.rs:19-191`, `crates/strata-app/src/modal_export_footer.rs:11-43`

| # | kind | C++ text | C++ tooltip / placeholder | C++ default, range, step, suffix | C++ enabled-when | C++ what it does | C++ file:line | Rust text | Rust tooltip / placeholder | Rust default, range, step, suffix | Rust enabled-when | Rust what it does | Rust file:line | same? |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 1 | label (headline) | `검수 규칙 파일을 찾지 못했습니다` / `오류 %1건을 확인하세요` / `오류가 없습니다` / `오류와 주의 항목이 없습니다` (`m_plate`) | — | — | always (`m_plate`) | Serif headline | `KaSubmitDialogPlate.cpp:19,75-78` | `오류 {err_cnt}건을 확인하세요 ({status_txt})` / `오류가 없습니다 ({status_txt})` / `오류와 주의 항목이 없습니다` | — | — | always | Strong headline (appends `({status_txt})`) | `modal_export.rs:45-57` | no (text) |
| 2 | label (subline) | `{m_rulesTitle} · {next}` (`m_plate`) | — | — | always (`m_plate`) | Rules title + next step | `KaSubmitDialogPlate.cpp:22,81-87` | `{engine.title()} · 고치지 않아도 꾸러미는 만들 수 있고, README_submit.txt에 적힙니다.` | — | — | always | Muted subline (fixed text even when 0 errors) | `modal_export.rs:58-65` | no (text) |
| 3 | count cards (3) | `오류` `{N}` `건` \| `주의` `{N}` `건` \| `통과` `{N}` `건` (`m_plate`) | — | — | always (`m_plate`) | Mono counts | `KaSubmitDialogPlate.cpp:33-60,88-92` | `오류` `{err_cnt}` `건` \| `주의` `{warn_cnt}` `건` \| `통과` `{pass_cnt}` `건` | — | — | always | 3 colored count cards | `modal_export.rs:67-111` | yes |
| 4 | drawing strip | `KaSubmitDrawingStrip`: 4 tiles (`조사구역도`, `유적위치도`, `유구배치도`, `개별유구실측도`, `없음` if missing) (`m_plate`) | — | height `108`, visible if rules loaded | always (`m_plate`) | Shows required 4 drawings status | `KaSubmitDialog.cpp:49-52`, `KaSubmitDrawingStrip.cpp:9-69` | `필수 도면 4종:` + 4 tags `● {name} ({id})` / `▲ {name} 없음 ({id})` | — | — | always | Inline status strip (no tile boxes) | `modal_export.rs:115-147` | no (text, layout) |
| 5 | treeWidget / table | 3 cols: `항목`, `고치는 법`, ``; 3 groups: `막는 항목(오류) %1건`, `확인할 항목(주의) %1건`, `통과 %1건`; child target rows `%1 · %2` (max 30 + `…외 %1건`); row buttons `위치 보기` + `actionLabel` | row col 0: `{id}\n근거: {basisKo}`; col 1: `fixKo`; target child: `두 번 누르면 지도에서 이 도형으로 이동합니다.` | Error/Warn expanded, Pass collapsed | always | Double-click child -> `GoToTargets`; `위치 보기` -> `GoToTargets`; action btn -> `RunAction` | `KaSubmitDialog.cpp:54-77,153-226` | Flat 27-rule list; header `위치 이동 · 순번 · JSON 규칙 ID (27개)` \| `심각도 · 검수 항목 및 고치는 법 (근거)`; row buttons `위치 보기` + `actionLabel` | none | flat list, no target child rows | always | `위치 보기` -> `go_to_check_targets`; action btn -> tab/tool (`set_work_crs` opens `AccountSettings`) | `modal_export_table.rs:28-164` | no (structure, headers, tooltips, `set_work_crs`) |
| 6 | label (package line) | `EPSG:5179 SHP · 조사도면.pdf · MANIFEST.sha256` (`m_plate`) | `제출 꾸러미에 들어가는 것입니다. 검수 오류가 남아 있어도 만들 수 있고, 도면 용지가 없으면 조사도면.pdf만 빠집니다.` | — | always (`m_plate`) | Package contents summary | `KaSubmitDialogPlate.cpp:65-71` | `EPSG:5179 SHP · 조사도면.pdf · MANIFEST.sha256 · README_submit.txt` + right `status_txt` | none | — | always | Adds `README_submit.txt` & `status_txt` | `modal_export_table.rs:168-189` | no (text, tooltip) |
| 7 | label (`m_blockReason`) | Missing rules path / errors / ready message | — | hidden in `m_plate` unless `!m_rulesLoaded \|\| m_results.isEmpty()` | conditional | Explains missing rules path | `KaSubmitDialog.cpp:80-83,228-239`, `KaSubmitDialogPlate.cpp:93` | MISSING | — | — | — | — | — | MISSING |
| 8 | pushButton | `다시 검수` | — | — | always | Emits `recheckRequested` -> re-evaluates & rebuilds | `KaSubmitDialog.cpp:86-88`, `KaSubmitFlow.cpp:69` | `다시 검수` | — | — | always | Sets `draw_guide_message = Some(status_txt)` (already re-evaluated every frame) | `modal_export_footer.rs:19-21` | yes |
| 9 | pushButton | `레이어 5179 변환…` | empty: `지도 목록에서 레이어 하나를 고르면 그 레이어만 EPSG:5179 SHP 파일로 바꿉니다.`; selected: `「%1」만 EPSG:5179 SHP 파일로 저장합니다. 검수·PDF·MANIFEST는 없습니다.` | — | `!layerName.isEmpty()` | `choose(Choice::ConvertLayer)` -> `convertSelectedTo5179()` | `KaSubmitDialog.cpp:89-91,139-145` | `레이어 5179 변환…` | `아직 Rust 판에서 연결하지 않았습니다 (지시 R2)` | — | always disabled (`false`) | Disabled placeholder | `modal_export_footer.rs:23-24` | no (enabled, tooltip, action) |
| 10 | pushButton (default) | `제출 꾸러미 만들기…` | `검수 결과와 함께 5179 SHP·조사도면.pdf·MANIFEST를 새 폴더에 만듭니다. 검수 오류가 남아 있으면 한 번 묻고 README에 적습니다.` | default button | `packageAllowed()` (`true`) | `choose(Choice::MakePackage)` -> `makeSubmitPackage()` | `KaSubmitDialog.cpp:92-97,241-243` | `제출 꾸러미 만들기…` | `아직 Rust 판에서 연결하지 않았습니다 (지시 R2)` | — | always disabled (`false`) | Disabled placeholder | `modal_export_footer.rs:29-40` | no (enabled, tooltip, action) |
| 11 | pushButton | `닫기` | — | — | always | `QDialog::reject()` | `KaSubmitDialog.cpp:98-99` | `닫기` | — | — | always | Sets `app.modals.active = None` | `modal_export_footer.rs:26-28` | yes |
| 12 | flow sub-dialog 1 | `QMessageBox` (`검수·제출`): error/no-rules confirmation; buttons `그래도 만들기`, `취소` | — | default `그래도 만들기` | `!rules \|\| errors > 0` | Confirms package creation despite errors | `KaSubmitFlow.cpp:88-99` | MISSING | — | — | — | — | — | MISSING |
| 13 | flow sub-dialog 2 | `QMessageBox` (`검수·제출`): unsaved edits confirmation; buttons `저장하고 만들기`, `저장하지 않고 만들기`, `취소` | — | default `저장하고 만들기` | `unsaved` | Asks to save before packaging | `KaSubmitFlow.cpp:104-125` | MISSING | — | — | — | — | — | MISSING |
| 14 | flow sub-dialog 3 | `QInputDialog::getItem` (`SHP 인코딩`): `SHP 글자 인코딩 (모르면 UTF-8)`, items `{"UTF-8", "EUC-KR"}` | — | default `UTF-8`, non-editable | always in flow | Picks SHP text encoding | `KaSubmitFlow.cpp:128-131` | MISSING | — | — | — | — | — | MISSING |
| 15 | flow sub-dialog 4 | `QFileDialog::getExistingDirectory` (`제출 꾸러미를 만들 위치`) | — | `preferredSurveyDir()` | always in flow | Picks output parent folder | `KaSubmitFlow.cpp:132-134` | MISSING | — | — | — | — | — | MISSING |
| 16 | flow sub-dialog 5 | `QProgressDialog` (`검수·제출`): `제출 꾸러미를 만들고 있습니다…`, button `취소`, `KaProgressPlate::dress`, `KaProgressPlate::packageLabel(step)` | — | `0..=total`, modal | during export | Shows export progress and allows cancel | `KaSubmitFlow.cpp:144-169` | MISSING | — | — | — | — | — | MISSING |
| 17 | header (EXTRA) | — | — | — | — | — | — | `검수·제출 - Strata` + `×` | — | — | always | Custom title bar | `modal_export.rs:32` | EXTRA (title differs) |

- **Keyboard & closing**:
  - C++: Default button `제출 꾸러미 만들기…`; `Enter` activates default; `Esc` and `X` reject.
  - Rust: No `Enter` handler; `Esc` closes via `gui_shell.rs:34-36`; `×` closes.
- **Messages**:
  - Flow error confirmation: `검수 오류 %1건이 남아 있습니다.` / `검수 규칙 파일을 찾지 못해 검수하지 못했습니다.` + `\n그래도 EPSG:5179 제출 꾸러미를 만들까요? 꾸러미 안내 파일(README)에 적힙니다.` (`KaSubmitFlow.cpp:90-92`) — Rust: MISSING.
  - Flow unsaved confirmation: `저장하지 않은 편집이 있습니다.\n제출 꾸러미는 지금 화면의 도형으로 만들어져 조사 파일(마지막 저장본)과 다를 수 있습니다.` (`KaSubmitFlow.cpp:106-107`) — Rust: MISSING.
  - Flow save failure: `저장하지 못해 제출 꾸러미를 만들지 않았습니다. 저장 문제를 해결한 뒤 다시 만드세요.` (`KaSubmitFlow.cpp:119`) — Rust: MISSING.
  - Flow export failure: `제출 꾸러미를 만들지 못했습니다.` (`KaSubmitFlow.cpp:175`) — Rust: MISSING.
  - Flow export success: `제출 꾸러미를 만들었습니다. 인트라넷에는 SHP를 하나씩 올리세요.` (or with `도면 용지가 없어 조사도면.pdf는 빠졌습니다.`) (`KaSubmitFlow.cpp:185-186`) — Rust: MISSING.

---

## A7. 「자료 준비 상태」 (`KaReferenceStatusDialog`)

- **C++ title & built at**: `자료 준비 상태` — `target/cpp-ref/src/app/KaReferenceStatusDialog.cpp:35`
- **Opened in C++**: «더보기» menu -> `자료 준비 상태` (`MainWindowRibbon.cpp:683`) -> `MainWindow::showReferenceStatus()` (`target/cpp-ref/src/app/MainWindowOffline.cpp:342-356`)
- **Opened in Rust**: «더보기» menu has item `자료 준비 상태` (`gui_ribbon_menus_b.rs:82`), **but** clicking it falls through to `ActiveModalDialog::AccountSettings` (`gui_ribbon_menus_b.rs:114`)! `ActiveModalDialog::ReferenceStatus` is only opened by `debug_control.rs:124`.
- **Modality**: C++ `setModal(false)` + `dialog->show()` — non-modal (`KaReferenceStatusDialog.cpp:36`, `MainWindowOffline.cpp:353`). Rust `egui::Window::new("ka_modal_reference_status")` without backdrop — non-modal (`modal_reference.rs:23`).
- **Size rules**: C++ `resize(860, 420)`, resizable (`KaReferenceStatusDialog.cpp:37`). Rust `.resizable(false).fixed_size([880.0_f32.min(vis_w - 40.0), 460.0_f32.min(vis_h - 40.0)])` (`modal_reference.rs:17-27`).
- **Rust counterpart**: `crates/strata-app/src/modal_reference.rs:12-230`, `crates/strata-app/src/modal_reference_data.rs`, `crates/strata-app/src/modal_reference_rows.rs`

| # | kind | C++ text | C++ tooltip / placeholder | C++ default, range, step, suffix | C++ enabled-when | C++ what it does | C++ file:line | Rust text | Rust tooltip / placeholder | Rust default, range, step, suffix | Rust enabled-when | Rust what it does | Rust file:line | same? |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 1 | label (`m_summary`) | `ReferenceInventory::summary(items)` (e.g. `현장 사용 가능 N개 · 인터넷 필요 M개 · 파일 없음 K개`) | — | — | always | Shows live inventory summary | `KaReferenceStatusDialog.cpp:45-50,95` | `현장 사용 가능 {offline_cnt}개 · 인터넷 필요 {online_cnt}개 · 파일 없음 {missing_cnt}개` | — | `online_cnt` hardcoded `2` | always | Summary from 7 fixed rows | `modal_reference.rs:44-56` | no (data source) |
| 2 | label (`hint`) | `조사에 올린 참조 자료가 현장에서 인터넷 없이 열리는지 확인하는 표입니다. 이 창은 자료를 새로 받거나 지우지 않습니다.\n인터넷이 필요한 배경은 레이어를 오른쪽 클릭해 「오프라인 저장」으로 직접 받아 두세요.` | — | — | always | Explains read-only purpose | `KaReferenceStatusDialog.cpp:39-44` | same 2-line text | — | — | always | Muted label | `modal_reference.rs:58-64` | yes |
| 3 | table (`m_table`) | 6 cols: `자료`, `종류`, `범위`, `받은 날짜`, `크기`, `현장(오프라인)`; rows from `ReferenceInventory::collect(m_project)`; non-offline has `warnMark()` icon | col 0 & col 5: `item.location` | single selection, no edit | always | Lists project reference layers | `KaReferenceStatusDialog.cpp:51-67,96-111` | 7 cols: `자료`, `종류`, `범위`, `받은 날짜`, `크기`, `현장(오프라인)`, `받기 / 설정`; 7 fixed rows (`● ` / `▲ `) | none | — | always | Displays 7 fixed rows | `modal_reference.rs:68-209` | no (extra col, tooltips, hardcoded rows) |
| 4 | pushButton | `다시 확인` (`QDialogButtonBox::ActionRole`) | — | `autoDefault = false` (`m_paper`) | always | Calls `refresh()` | `KaReferenceStatusDialog.cpp:81-83` | `다시 확인` | — | — | always | Only sets `draw_guide_message` | `modal_reference.rs:219-222` | no (action) |
| 5 | pushButton | `닫기` (`QDialogButtonBox::Close`) | — | `autoDefault = false` (`m_paper`) | always | `QDialog::close()` | `KaReferenceStatusDialog.cpp:80,84` | `닫기` | — | — | always | Sets `app.modals.active = None` | `modal_reference.rs:224-226` | yes |
| 6 | buttons (EXTRA) | — | — | — | — | — | — | `받기` and `설정` small buttons in 7th column of each row | — | — | always | Opens download or settings modal (contradicts C++ «새로 받거나 지우지 않습니다») | `modal_reference.rs:197-204` | EXTRA |
| 7 | header (EXTRA) | — | — | — | — | — | — | `자료 준비 상태` + `×` | — | — | always | Custom title bar | `modal_reference.rs:31` | EXTRA |

- **Keyboard & closing**:
  - C++: No default button (`autoDefault(false)` in `m_paper`); `Esc` and `X` close (`reject`/`close`).
  - Rust: No `Enter` handler; `Esc` closes via `gui_shell.rs:34-36`; `×` closes.
- **Messages**: Only summary/hint/status texts above; guide message on `다시 확인` in Rust (`자료 준비 상태 다시 확인 완료: {summary_txt}`, `modal_reference.rs:221`).

---

## A8. Start notice (`KaStartupSplash`)

- **C++ title & built at**: `Strata · 필드고고학 GIS 시작 안내` — `target/cpp-ref/src/app/KaStartupSplash.cpp:104`
- **Opened in C++**: Application startup in `KaApplication` (`target/cpp-ref/src/app/KaApplication.cpp:766`)
- **Opened in Rust**: **MISSING** (app starts directly into the main window / home tab in `crates/strata-app/src/main.rs`)
- **Modality**: C++ `Qt::SplashScreen | Qt::FramelessWindowHint` modeless widget (`KaStartupSplash.cpp:98`). Rust: **MISSING**.
- **Size rules**: C++ `kCardWidth + 2*kSide = 732` × `kCardHeight + kTopMargin + kBottomMargin = 434` scaled to fit screen (`KaStartupSplash.cpp:30-31,123-126`). Rust: **MISSING**.
- **Rust counterpart**: **MISSING**

| # | kind | C++ text | C++ tooltip / placeholder | C++ default, range, step, suffix | C++ enabled-when | C++ what it does | C++ file:line | Rust text | Rust tooltip / placeholder | Rust default, range, step, suffix | Rust enabled-when | Rust what it does | Rust file:line | same? |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 1 | painted card | Title, version, icon, animated strata section (`KaSplashStrata`), progress bar, credits/notices (`KaSplashCredits`), status (`앱을 준비하고 있습니다` -> `잠시 후 홈 화면이 열립니다` -> `준비 완료`) | — | min motion `3000 ms` (`m_strata`), `16 ms` timer | always | Displays startup attribution & loading progress, emits `readyToShow` | `KaStartupSplash.cpp:33-35,159-250` | MISSING | — | — | — | — | — | MISSING |

- **Keyboard & closing**: C++ frameless splash closes automatically when `readyToShow()` fires (`KaStartupSplash.cpp:195`, `KaApplication.cpp:949`). Rust: **MISSING**.
- **Messages**: `앱을 준비하고 있습니다`, `잠시 후 홈 화면이 열립니다`, `준비 완료` (`KaStartupSplash.cpp:33-34,174`). Rust: **MISSING**.

---

## A9. 「위치 검색」 progress, notice and 「위치 선택」

- **C++ titles & built at**:
  1. Progress: `위치 검색` (`QProgressDialog`) — `target/cpp-ref/src/app/KaShellUi.cpp:71`
  2. Notice: `위치 검색` (`QMessageBox::information`) — `target/cpp-ref/src/app/MainWindow.cpp:2786`
  3. Picker: `위치 선택` (`QInputDialog::getItem`) — `target/cpp-ref/src/app/MainWindow.cpp:2808`
- **Opened in C++**: App bar search `returnPressed` or region popup «찾기» -> `MainWindow::searchLocation` -> `onLocationFailed` / `onLocationResults` (`MainWindow.cpp:2757-2815`)
- **Opened in Rust**: `run_app_bar_search` / `run_region_popup_search` -> `execute_interactive_location_search` (`crates/strata-app/src/gui_ribbon_search.rs:88-107`) -> `render_region_locator_popup_b` (`crates/strata-app/src/gui_ribbon_region_b.rs:15-100`)
- **Modality**:
  - Progress: C++ `Qt::NonModal` (`KaShellUi.cpp:72`); Rust **MISSING** (search runs synchronously on UI thread).
  - Notice: C++ `QMessageBox::information` (modal); Rust `egui::Window::new("위치 검색")` without backdrop (non-modal, `gui_ribbon_region_b.rs:77`).
  - Picker: C++ `QInputDialog::getItem` (modal); Rust `egui::Window::new("위치 선택")` without backdrop (non-modal, `gui_ribbon_region_b.rs:20`).
- **Size rules**: C++ Qt standard auto-sized dialogs. Rust `.collapsible(false).resizable(false)`, Notice `default_width(380.0)`, Picker `default_width(460.0)`.
- **Rust counterpart**: `crates/strata-app/src/gui_ribbon_region_b.rs:15-100`

| # | kind | C++ text | C++ tooltip / placeholder | C++ default, range, step, suffix | C++ enabled-when | C++ what it does | C++ file:line | Rust text | Rust tooltip / placeholder | Rust default, range, step, suffix | Rust enabled-when | Rust what it does | Rust file:line | same? |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 1 | progressDialog (label + cancel) | `「%1」 위치를 찾는 중입니다. 지도 작업을 계속할 수 있습니다.` (or `위치를 찾는 중입니다...`), cancel `취소` | — | `0..=0` (indeterminate) | during search | Cancels `m_locator`, sets status bar `위치 검색을 취소했습니다.` | `KaShellUi.cpp:60-76`, `MainWindow.cpp:2770-2777` | MISSING | — | — | — | Search blocks UI thread synchronously | `gui_ribbon_search.rs:90` | MISSING |
| 2 | messageBox (label + Ok) | `위치 검색`: `message` (or `검색 결과 없음`) + Ok | — | Information icon | on search fail or 0 hits | Acknowledges failure | `MainWindow.cpp:2786,2793` | `위치 검색`: `err_msg` (or `검색 결과 없음`) + `확인` | — | no icon | when `search_error_modal.is_some()` | Clears `search_error_modal` | `gui_ribbon_region_b.rs:75-99` | no (icon, modal) |
| 3 | inputDialog (label) | `검색 결과 %1건 — 이동할 위치를 선택하세요` | — | — | `hits.size() > 1` | Prompt label | `MainWindow.cpp:2809` | `검색 결과 {len}건 — 이동할 위치를 선택하세요` | — | — | `!search_results_modal.is_empty()` | Prompt label | `gui_ribbon_region_b.rs:26-29` | yes |
| 4 | inputDialog (combo/list) | `labels` (`{title}  —  {detail}` or `{title}`) | — | index `0`, non-editable | always | Selects hit index | `MainWindow.cpp:2801-2810` | `ScrollArea` of `selectable_label`s | — | `selected_hit_idx = 0` | always | Sets `selected_hit_idx = idx` | `gui_ribbon_region_b.rs:31-45` | no (widget kind) |
| 5 | inputDialog (Ok) | Qt standard OK | — | default button | always | Accepts & calls `zoomToLocation` | `MainWindow.cpp:2807-2814` | `확인` | — | — | always | Calls `zoom_to_location_hit` & clears list | `gui_ribbon_region_b.rs:48-55,64-68` | no (default/Enter) |
| 6 | inputDialog (Cancel) | Qt standard Cancel | — | — | always | Rejects | `MainWindow.cpp:2807-2811` | `취소` | — | — | always | Clears `search_results_modal` | `gui_ribbon_region_b.rs:56-61,69-71` | yes |

- **Keyboard & closing**:
  - C++: `QMessageBox` and `QInputDialog` accept on `Enter`, reject on `Esc` and `X`.
  - Rust: `.collapsible(false)` standard title bar has no `×` button; no `Enter` or `Esc` handlers in `gui_ribbon_region_b.rs:15-100` (nor `gui_shell.rs:34` since these use `ribbon.app_bar` fields, not `modals.active`).
- **Messages**: `검색 결과 없음` (`MainWindow.cpp:2793` / `gui_ribbon_search.rs:99`), `위치 검색을 취소했습니다.` (`MainWindow.cpp:2775` — Rust MISSING).

---

## A10. 「VWorld 지적도 계정」 (`MainWindow::configureCadastralAccount`)

- **C++ title & built at**: `VWorld 지적도 계정` — `target/cpp-ref/src/app/MainWindowCadastral.cpp:36`
- **Opened in C++**: Ribbon «지적» menu -> `VWorld 계정 설정` (`MainWindowRibbon.cpp:477`), «더보기» menu -> `VWorld 지적도 아이디·비밀번호` (`MainWindowRibbon.cpp:678`), or during `downloadCadastral()` when credentials empty/rejected (`MainWindowCadastral.cpp:118,152`)
- **Opened in Rust**: Ribbon «지적» menu -> `VWorld 계정 설정` (`gui_ribbon_menus_a.rs:180-181`), `cadastral_download_ui_render.rs:138` -> opens combined `ActiveModalDialog::AccountSettings` (`modal_paper_account.rs:147`). Dedicated dialog is **MISSING**.
- **Modality**: C++ `dialog.exec()` — application-modal (`MainWindowCadastral.cpp:47`). Rust combined `AccountSettings` is non-modal (`modal_paper_account.rs:148`).
- **Size rules**: C++ `KaFormPlate::dress(&dialog, form)` (440 frame, `MainWindowCadastral.cpp:44`). Rust `AccountSettings` `default_width(520.0)`, `resizable(false)`.
- **Rust counterpart**: Combined into `crates/strata-app/src/modal_paper_account.rs:198-213` (dedicated dialog **MISSING**)

| # | kind | C++ text | C++ tooltip / placeholder | C++ default, range, step, suffix | C++ enabled-when | C++ what it does | C++ file:line | Rust text | Rust tooltip / placeholder | Rust default, range, step, suffix | Rust enabled-when | Rust what it does | Rust file:line | same? |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 1 | label | `아이디` | — | — | always | Label for `id` | `MainWindowCadastral.cpp:40` | `VWorld 지적도 ID / PW:` (combined row) | — | — | always | Combined label in `AccountSettings` | `modal_paper_account.rs:198` | no (text, layout) |
| 2 | lineEdit | `previous.username` | — | loaded `CadastralPortal::credentials().username` | always | Edits username | `MainWindowCadastral.cpp:38` | `app.accounts.cadastral_creds.username` | — | loaded from account settings | always | Edits username | `modal_paper_account.rs:200-205` | yes |
| 3 | label | `비밀번호` | — | — | always | Label for `password` | `MainWindowCadastral.cpp:40` | (shared with row 1) | — | — | — | — | `modal_paper_account.rs:198` | no (layout) |
| 4 | lineEdit (Password) | `previous.password` | — | `QLineEdit::Password` | always | Edits password | `MainWindowCadastral.cpp:39` | `app.accounts.cadastral_creds.password` | — | masked unless `show_passwords` | always | Edits password | `modal_paper_account.rs:206-212` | yes |
| 5 | label | `이 PC의 개인 설정에 저장하여 다음 다운로드에 사용합니다.` | — | — | always | Storage note | `MainWindowCadastral.cpp:41` | MISSING (replaced by NGII description in `AccountSettings`) | — | — | — | — | `modal_paper_account.rs:252-258` | no (text) |
| 6 | pushButton (Save) | `QDialogButtonBox::Save` | — | default button | always | `accept()` -> `CadastralPortal::saveCredentials` | `MainWindowCadastral.cpp:42,45,49` | `저장` (saves all 5 accounts) | — | — | always | `save_all_to_disk()` | `modal_paper_account.rs:270-288` | no (scope) |
| 7 | pushButton (Cancel) | `QDialogButtonBox::Cancel` | — | — | always | `reject()` | `MainWindowCadastral.cpp:42,46` | `닫기` | — | — | always | Closes `AccountSettings` | `modal_paper_account.rs:267-269` | no (text) |

- **Keyboard & closing**: C++ `Enter` accepts (`Save`), `Esc`/`X` reject. Rust: see A3.
- **Messages**: C++ `KaUserError::warn` `"계정을 저장하지 못했습니다"` on save failure (`MainWindowCadastral.cpp:50`). Rust: inline `validation_error` (`modal_paper_account.rs:260-262`).

---

## A11. 「지적도 선 색·지번」 (`MainWindow::configureCadastralStyle`)

- **C++ title & built at**: `지적도 선 색·지번` — `target/cpp-ref/src/app/MainWindowCadastral.cpp:61`
- **Opened in C++**: Ribbon «지적» menu -> `선 색·지번 표시` (`MainWindowRibbon.cpp:478`), or layer context menu on cadastral layer (`MainWindowEditing.cpp:444`)
- **Opened in Rust**: Ribbon «지적» menu has item `선 색·지번 표시` (`gui_ribbon_menus_a.rs:164`), **but** clicking it opens `ActiveModalDialog::CadastralDownload` (`gui_ribbon_menus_a.rs:183`)!
- **Modality**: C++ `dialog.exec()` — application-modal (`MainWindowCadastral.cpp:76`). Rust: **MISSING**.
- **Size rules**: C++ auto-sized `QDialog`. Rust: **MISSING**.
- **Rust counterpart**: **MISSING** (`modal_style.rs:11` is `KaLineColorDialog`, not `configureCadastralStyle`)

| # | kind | C++ text | C++ tooltip / placeholder | C++ default, range, step, suffix | C++ enabled-when | C++ what it does | C++ file:line | Rust text | Rust tooltip / placeholder | Rust default, range, step, suffix | Rust enabled-when | Rust what it does | Rust file:line | same? |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 1 | label | `경계선 0.2mm` | — | — | always | Form row label | `MainWindowCadastral.cpp:69` | MISSING | — | — | — | — | — | MISSING |
| 2 | pushButton | `color.name()` (default `#000000`) | — | layer's `ka_hgis/cadastral_color` or `#000000` | always | Opens `QColorDialog::getColor(..., "경계선 색")` | `MainWindowCadastral.cpp:63-68` | MISSING | — | — | — | — | — | MISSING |
| 3 | checkBox | `지번 표시` | — | `layers.first()->labelsEnabled()` | always | Toggles parcel number labels | `MainWindowCadastral.cpp:70-71` | MISSING | — | — | — | — | — | MISSING |
| 4 | label | `지번은 1:10,000보다 확대했을 때 표시됩니다.` | — | — | always | Scale threshold hint | `MainWindowCadastral.cpp:72` | MISSING | — | — | — | — | — | MISSING |
| 5 | pushButton (Ok) | `QDialogButtonBox::Ok` | — | default button | always | `accept()` -> `CadastralImport::applyStyle` + refresh | `MainWindowCadastral.cpp:73-79` | MISSING | — | — | — | — | — | MISSING |
| 6 | pushButton (Cancel) | `QDialogButtonBox::Cancel` | — | — | always | `reject()` | `MainWindowCadastral.cpp:73,75` | MISSING | — | — | — | — | — | MISSING |

- **Keyboard & closing**: C++ `Enter` accepts, `Esc`/`X` reject. Rust: **MISSING**.
- **Messages**: C++ pre-check `QMessageBox::information(this, "지적도", "지적도를 먼저 받아 주세요.")` (`MainWindowCadastral.cpp:60`). Rust: **MISSING**.

---

## A12. 「레이어 추가」 (`MainWindow::addUserLayer`)

- **C++ title & built at**: `레이어 추가` — `target/cpp-ref/src/app/MainWindow.cpp:2445`
- **Opened in C++**: Layer panel context menu `참고 자료 불러오기…` / `레이어 불러오기 (SHP·DXF·GPKG)` (`MainWindowContextMenus.cpp:214,368` -> `addUserLayer()`)
- **Opened in Rust**: **MISSING** (searched `crates/strata-app/src/*.rs` for `레이어 추가`)
- **Modality**: C++ `dlg.exec()` — application-modal (`MainWindow.cpp:2500`). Rust: **MISSING**.
- **Size rules**: C++ `Qt::MSWindowsFixedSizeDialogHint`, `QLayout::SetFixedSize` (`MainWindow.cpp:2446,2450`). Rust: **MISSING**.
- **Rust counterpart**: **MISSING**

| # | kind | C++ text | C++ tooltip / placeholder | C++ default, range, step, suffix | C++ enabled-when | C++ what it does | C++ file:line | Rust text | Rust tooltip / placeholder | Rust default, range, step, suffix | Rust enabled-when | Rust what it does | Rust file:line | same? |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 1 | label | `이름` | — | — | always | Caption for `nameEdit` | `MainWindow.cpp:2454` | MISSING | — | — | — | — | — | MISSING |
| 2 | lineEdit | empty | placeholder `예: 조사구역, 1호 주거지` | `""` | always | Layer name input | `MainWindow.cpp:2451-2453` | MISSING | — | — | — | — | — | MISSING |
| 3 | label | `좌표계` | — | — | always | Caption for CRS buttons | `MainWindow.cpp:2480` | MISSING | — | — | — | — | — | MISSING |
| 4 | pushButton (checkable) | `5186  중부` | — | checked if `!m_workCrs.contains("5187")` | always | Mutually exclusive with `btn5187` | `MainWindow.cpp:2459-2473` | MISSING | — | — | — | — | — | MISSING |
| 5 | pushButton (checkable) | `5187  동부` | — | checked if `m_workCrs.contains("5187")` | always | Mutually exclusive with `btn5186` | `MainWindow.cpp:2460-2477` | MISSING | — | — | — | — | — | MISSING |
| 6 | checkBox | `채우기 없음 (외곽선만)` | — | unchecked | always | Hides `fillBox` & `dlg.adjustSize()` when checked | `MainWindow.cpp:2481,2490-2493` | MISSING | — | — | — | — | — | MISSING |
| 7 | label | `면 색` | — | hidden when `noFill` checked | `!noFill->isChecked()` | Caption for `fillBtn` | `MainWindow.cpp:2485` | MISSING | — | — | — | — | — | MISSING |
| 8 | pushButton | `#22C55E  ·  클릭해서 색 고르기` | — | `QColor(34, 197, 94, 140)` | `!noFill->isChecked()` | Opens `QColorDialog` (`면 색`, alpha enabled) | `MainWindow.cpp:2405,2415-2424,2483-2484` | MISSING | — | — | — | — | — | MISSING |
| 9 | label | `외곽선 색` | — | — | always | Caption for `strokeBtn` | `MainWindow.cpp:2489` | MISSING | — | — | — | — | — | MISSING |
| 10 | pushButton | `#15803D  ·  클릭해서 색 고르기` | — | `QColor(21, 128, 61)` | always | Opens `QColorDialog` (`외곽선 색`, alpha enabled) | `MainWindow.cpp:2405,2415-2424,2487-2488` | MISSING | — | — | — | — | — | MISSING |
| 11 | pushButton (Ok) | `추가` | — | default button | always | `accept()` -> validates name/survey, creates user polygon layer | `MainWindow.cpp:2494-2526` | MISSING | — | — | — | — | — | MISSING |
| 12 | pushButton (Cancel) | `취소` | — | — | always | `reject()` | `MainWindow.cpp:2494,2496,2499` | MISSING | — | — | — | — | — | MISSING |

- **Keyboard & closing**: C++ `Enter` accepts, `Esc`/`X` reject. Rust: **MISSING**.
- **Messages**:
  - Empty name after Ok: `QMessageBox::information(this, "레이어 추가", "이름을 입력하세요.")` (`MainWindow.cpp:2503`).
  - No open survey: `QMessageBox::question(this, "레이어", "먼저 새 조사가 필요합니다. 지금 만들까요?")` (`MainWindow.cpp:2507-2508`).
  - Creation failure: `notify(Notice::Warning, "레이어 추가", "레이어를 만들지 못했습니다.", err)` (`MainWindow.cpp:2519-2520`).
  - Rust: **MISSING**.

---

## A13. 「토양도 불러오기 설정」 (`MainWindow::importSoilShapefile`)

- **C++ title & built at**: `토양도 불러오기 설정` — `target/cpp-ref/src/app/MainWindow.cpp:2671`
- **Opened in C++**: Ribbon «토양» menu -> `importSoilShapefile()` (`MainWindowRibbon.cpp:452,640,706`) -> `QFileDialog::getOpenFileName(..., "토양도 SHP 불러오기", ..., "Shapefile (*.shp)")` (`MainWindow.cpp:2645`) -> opens dialog
- **Opened in Rust**: **MISSING** (clicking `토양도 파일 불러오기(SHP·GPKG)` in `gui_ribbon_menus_a.rs:136,153` just toggles the `토양도(흙토람)` overlay via `handle_ribbon_click("토양")`).
- **Modality**: C++ `dlg.exec()` — application-modal (`MainWindow.cpp:2719`). Rust: **MISSING**.
- **Size rules**: C++ auto-sized `QDialog`. Rust: **MISSING**.
- **Rust counterpart**: **MISSING**

| # | kind | C++ text | C++ tooltip / placeholder | C++ default, range, step, suffix | C++ enabled-when | C++ what it does | C++ file:line | Rust text | Rust tooltip / placeholder | Rust default, range, step, suffix | Rust enabled-when | Rust what it does | Rust file:line | same? |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 1 | label | `좌표계` | — | — | always | Form row label | `MainWindow.cpp:2686` | MISSING | — | — | — | — | — | MISSING |
| 2 | comboBox (`crsBox`) | optional `파일에 기록된 좌표계 사용 — %1`; `EPSG:2097 — 중부원점(Bessel) · 흙토람 고시 좌표계`; `EPSG:5174 — 중부원점(Bessel, 10.405″ 보정)`; `EPSG:5186 — 중부원점(GRS80)`; `EPSG:5187 — 동부원점(GRS80)`; `EPSG:5179 — UTM-K`; `EPSG:4326 — 경위도(WGS84)` | — | index `0` | always | Selects source CRS of SHP | `MainWindow.cpp:2674-2685` | MISSING | — | — | — | — | — | MISSING |
| 3 | label | `색 구분 필드` | — | — | always | Form row label | `MainWindow.cpp:2702` | MISSING | — | — | — | — | — | MISSING |
| 4 | comboBox (`fieldBox`) | `(단색 — 구분 없음)` + string/int fields from SHP | — | auto-selects first field matching `(분포\|지형\|토양\|tpgrp\|topo\|dist\|soil\|sltp\|sym)`, else `0` | always | Selects classification field | `MainWindow.cpp:2688-2701` | MISSING | — | — | — | — | — | MISSING |
| 5 | label (`note`) | `흙토람 → 토양도 신청에서 무료로 받은 SHP를 그대로 불러옵니다.\n지도가 엉뚱한 위치에 뜨면 좌표계를 EPSG:2097 ↔ 5174로 바꿔 다시 불러오세요.\n불러온 뒤에는 작업 좌표계(%1)로 자동 재투영되어 지적·위성과 겹쳐 보입니다.` | — | — | always | Explains CRS reprojection | `MainWindow.cpp:2704-2711` | MISSING | — | — | — | — | — | MISSING |
| 6 | pushButton (Ok) | `불러오기` | — | default button | always | `accept()` -> `LayerOps::addSoilShapefile` | `MainWindow.cpp:2713-2714,2716,2722` | MISSING | — | — | — | — | — | MISSING |
| 7 | pushButton (Cancel) | `취소` | — | — | always | `reject()` | `MainWindow.cpp:2713,2715,2717` | MISSING | — | — | — | — | — | MISSING |

- **Keyboard & closing**: C++ `Enter` accepts, `Esc`/`X` reject. Rust: **MISSING**.
- **Messages**:
  - Open failure: `notify(Notice::Critical, "토양도", "SHP를 열 수 없습니다: %1")` (`MainWindow.cpp:2656-2657`).
  - Add failure: `notify(Notice::Critical, "토양도", err)` or `"토양도를 불러오지 못했습니다."` (`MainWindow.cpp:2726-2727`).
  - Success: `notify(Notice::Success, "토양도", "%1 · %2 · 색 구분: %3 — 참조 지도 그룹")` (`MainWindow.cpp:2734-2737`).
  - Rust: **MISSING**.

---

## A14. Address popup of the region button (`KaRegionLocator::openAddressPopup`)

- **C++ title & built at**: Frameless popup (`#regionAddressPopup`) — `target/cpp-ref/src/app/KaRegionLocator.cpp:129-286`
- **Opened in C++**: Clicking any 시·도 chip in the region dropdown (`KaRegionLocator.cpp:124`)
- **Opened in Rust**: Clicking any 시·도 chip in `render_region_locator_popup` (`crates/strata-app/src/gui_ribbon_region.rs:107-115`) -> `paint_region_address_popup` (`crates/strata-app/src/gui_ribbon_region_b.rs:103-282`)
- **Modality**: C++ `Qt::Tool | Qt::FramelessWindowHint`, closes on `WindowDeactivate`, `Esc`, `✕`, or `취소` (`KaRegionLocator.cpp:116-119,129,196-197`). Rust `egui::Area` (`Order::Foreground`), closes on `Esc` (`gui_ribbon_region.rs:21-24`), `✕`, or `취소` (no outside-click close).
- **Size rules**: C++ `placeAddressPopup` tries 6/3/2/1 columns and clamps to screen (`KaRegionLocator.cpp:239-262`). Rust fixed `popup_w = 570.0_f32` single horizontal row (`gui_ribbon_region_b.rs:112`).
- **Rust counterpart**: `crates/strata-app/src/gui_ribbon_region_b.rs:103-282`

| # | kind | C++ text | C++ tooltip / placeholder | C++ default, range, step, suffix | C++ enabled-when | C++ what it does | C++ file:line | Rust text | Rust tooltip / placeholder | Rust default, range, step, suffix | Rust enabled-when | Rust what it does | Rust file:line | same? |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 1 | label (`m_sidoLabel`) | `{sido}` | — | selected 시·도 | always | Shows selected 시·도 | `KaRegionLocator.cpp:137,205` | `{canon_sido}` | — | selected 시·도 | always | Shows canonical 시·도 | `gui_ribbon_region_b.rs:139-144` | yes |
| 2 | toolButton (`closeX`) | `✕` | tooltip `닫기 (Esc)` | — | always | `closePanel()` | `KaRegionLocator.cpp:183-189,196` | `✕` | hover `닫기 (Esc)` | — | always | Sets `address_popup_open = false` | `gui_ribbon_region_b.rs:148-154` | yes |
| 3 | comboBox (`m_city`) | `시·군·구` + `KoreaRegionCatalog::citiesOf(sido)` | — | index `0` (`시·군·구`), minWidth `120` | always | `currentIndexChanged` -> `fillDongs()` | `KaRegionLocator.cpp:143-145,201,207-210` | `시·군·구` + `cities_of(&canon_sido)` | — | `""` (`시·군·구`), width `120.0` | always | Sets `city_input`, clears `dong_input` | `gui_ribbon_region_b.rs:162-189` | yes |
| 4 | editable comboBox (`m_dong`) | `동·읍·면` + `KoreaRegionCatalog::dongsOf(sido, city)` | lineEdit placeholder `법정동·읍·면·리` | empty lineEdit, minWidth `110` | always | `returnPressed` -> `emitSearch()` | `KaRegionLocator.cpp:146-160,200,264-271` | Split into `TextEdit` (`dong_input`, width `112.0`) + separate `ComboBox` (`width 24.0`, shown when `!dongs.is_empty()`) | hint `법정동·읍·면·리` | `""` | always | `TextEdit` `Enter` -> `run_search`; `ComboBox` sets `dong_input` | `gui_ribbon_region_b.rs:191-222` | no (split widget) |
| 5 | lineEdit (`m_lot`) | empty | placeholder `산 12-3`; tooltip `번지 (예: 12, 12-3, 산 12-3). 산번지는 ‘산’을 함께 입력하세요.` | `""`, maxWidth `120` | always | `returnPressed` -> `emitSearch()` | `KaRegionLocator.cpp:161-167,199` | `lot_input` | hint `산 12-3`; hover `번지 (예: 12, 12-3, 산 12-3). 산번지는 ‘산’을 함께 입력하세요.` | `""`, width `96.0` | always | `Enter` (`lost_focus`) -> `run_search` | `gui_ribbon_region_b.rs:224-237` | yes |
| 6 | pushButton (`go`) | `찾기` | — | default button | always | `emitSearch()` | `KaRegionLocator.cpp:168-169,198` | `찾기` | — | — | always | `run_region_popup_search()` | `gui_ribbon_region_b.rs:239-250` | yes |
| 7 | pushButton (`cancel`) | `취소` | — | `autoDefault(false)` | always | `closePanel()` | `KaRegionLocator.cpp:171-173,197` | `취소` | — | — | always | Sets `address_popup_open = false` | `gui_ribbon_region_b.rs:252-263` | yes |
| 8 | label (`hint`) | `번지 입력 시 정확한 지번만 찾습니다. 읍·면은 리까지 입력하세요.\n예: 고아읍 봉한리 / 산 12-3` | — | — | always | Bottom input hint | `KaRegionLocator.cpp:192-194` | `REGION_BOTTOM_HINT` (same text) | — | — | always | Muted label | `gui_ribbon_region_b.rs:267-271` | yes |

- **Keyboard & closing**:
  - C++: `m_dong->setFocus()` on open; `Enter` in `m_dong`/`m_lot` or default `찾기` calls `emitSearch()`; `Esc`, `WindowDeactivate` (outside click), `✕`, `취소` close popup.
  - Rust: No auto-focus on `dong_input`; `Enter` on `dong_input`/`lot_input` searches; `Esc`, `✕`, `취소` close popup; no outside-click close.
- **Messages**: None inside popup (search errors/results go to A9).

---

## Differences to fix (most visible first)

1. **A7 (`자료 준비 상태`) — Broken menu entry**: «더보기» -> `자료 준비 상태` opens `AccountSettings` instead of `ReferenceStatus` (`MainWindowOffline.cpp:342-356` vs `crates/strata-app/src/gui_ribbon_menus_b.rs:82,114`).
2. **A11 (`지적도 선 색·지번`) — Wrong dialog & missing**: Clicking «선 색·지번 표시» opens `CadastralDownload` instead of `지적도 선 색·지번`; the entire dialog (`경계선 0.2mm`, `지번 표시`, `지번은 1:10,000보다 확대했을 때 표시됩니다.`, `지적도를 먼저 받아 주세요.`) is missing (`MainWindowCadastral.cpp:56-80` vs `crates/strata-app/src/gui_ribbon_menus_a.rs:164,183`).
3. **A13 (`토양도 불러오기 설정`) — Missing dialog**: Clicking «토양도 파일 불러오기(SHP·GPKG)» toggles overlay instead of opening SHP picker + `토양도 불러오기 설정` (`MainWindow.cpp:2643-2737` vs `crates/strata-app/src/gui_ribbon_menus_a.rs:136,153`).
4. **A12 (`레이어 추가`) — Missing dialog**: Entire `레이어 추가` dialog (`이름`, `5186 중부`/`5187 동부`, `채우기 없음`, `면 색`, `외곽선 색`, `추가`) is missing (`MainWindow.cpp:2441-2530` vs not found in `crates/strata-app/src`).
5. **A8 (Start notice) — Missing window**: `KaStartupSplash` (`Strata · 필드고고학 GIS 시작 안내`) is missing (`KaStartupSplash.cpp:104` vs `crates/strata-app/src/main.rs`).
6. **A6 (`검수·제출`) — Export & convert buttons disabled, 5 flow sub-dialogs missing**: `레이어 5179 변환…` and `제출 꾸러미 만들기…` are disabled with R2 tooltip, and all 5 `KaSubmitFlow` sub-dialogs (error confirm, unsaved confirm, `SHP 인코딩`, folder picker, progress dialog) are missing (`KaSubmitDialog.cpp:89-97`, `KaSubmitFlow.cpp:88-169` vs `crates/strata-app/src/modal_export_footer.rs:23-40`).
7. **A9 (`위치 검색` progress) — Missing progress dialog & synchronous search**: `KaShellUi::createSearchProgress` (`위치 검색` non-modal progress with `취소`) is missing because search runs synchronously (`KaShellUi.cpp:68-77` vs `crates/strata-app/src/gui_ribbon_search.rs:90`).
8. **A2 (`조사 닫기`) — Wrong texts, missing title/count/path**: Rust uses `저장하지 않은 편집이 있습니다` / `끝내기 전에 지금까지 그린 조사 내용을 저장할까요?` / `저장하고 끝내기` / `저장하지 않고 끝내기` instead of `KaClosePrompt` (`{name}을(를) 저장할까요?`, `저장하지 않은 작업이 {N}건 있습니다.\n{folder} › {fileName}`, `Save`/`Discard`), even though `format_close_prompt` already exists (`KaClosePrompt.cpp:26-46` vs `crates/strata-app/src/shell_dialogs.rs:83-138`).
9. **A3 (`API 키 입력`) & A10 (`VWorld 지적도 계정`) — Merged into one window**: C++ has two separate dialogs (`API 키 입력` and `VWorld 지적도 계정`), whereas Rust merges them with NGII and Heritage accounts into `API 키 · 사이트 계정 설정 - Strata` (`KaApiKeyDialog.cpp:18`, `MainWindowCadastral.cpp:36` vs `crates/strata-app/src/modal_paper_account.rs:147-292`).
10. **A7 (`자료 준비 상태`) — Hardcoded rows & extra `받기 / 설정` column**: Rust uses 7 hardcoded rows (with `online_cnt = 2`) and adds a 7th `받기 / 설정` column with `받기`/`설정` buttons, contradicting C++ «이 창은 자료를 새로 받거나 지우지 않습니다» (`KaReferenceStatusDialog.cpp:39-67,92-111` vs `crates/strata-app/src/modal_reference.rs:42-209`).
11. **A6 (`검수·제출`) — Tree vs flat list & `set_work_crs` action**: C++ uses a 3-column collapsible `QTreeWidget` with target child rows and double-click jump; Rust uses a flat 27-rule list without child rows, adds `README_submit.txt` to the package line, and routes `set_work_crs` to `AccountSettings` (`KaSubmitDialog.cpp:54-77,153-226` vs `crates/strata-app/src/modal_export_table.rs:28-164`).
12. **A1 (`새 조사`) — Origin button text, title, ruler tooltip, maxLength**: Rust adds city subtitles inside origin buttons and ` - Strata` to title, omits ruler tooltip and `maxLength(100)` (`KaNewSurveyDialog.cpp:48,59,67-68`, `KaLongitudeRuler.cpp:42-43` vs `crates/strata-app/src/dlg_new_survey.rs:28,33,73-95`).
13. **A14 (Address popup) — Split `동·읍·면` widget & no outside-click close**: Rust splits editable `QComboBox` into `TextEdit` + 24 px `ComboBox` and does not close on outside click (`KaRegionLocator.cpp:116-119,146-160` vs `crates/strata-app/src/gui_ribbon_region_b.rs:191-222`).
14. **A1, A2, A3, A4, A6, A9 — Missing `Enter` default-button handling & non-modal windows**: C++ dialogs are modal `QDialog::exec()` (except A7, A8, A9 progress) with `Enter` activating the default button; Rust `egui::Window`s are non-modal and lack `Enter` handlers (except A5).

---

## Could not determine

- None: all 14 C++ windows and their Rust counterparts (or absence thereof) were located and inspected directly in source.

---

## Counts

- **Windows**: 14 (A1–A14)
- **C++ controls**: 128 (118 table rows; A5 rows `4..11` and `13..16` expand to 8 color swatches and 4 width presets, +10)
- **Rust controls**: 105 (95 matching/differing + 10 `EXTRA`, including custom modal headers)
- **Differences by `same?` column** (expanded):
  - `yes`: 63
  - `no`: 32
  - `MISSING`: 33
  - `EXTRA`: 10 (includes 3 custom headers marked `EXTRA (title differs)`)
