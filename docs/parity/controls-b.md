# Parity Audit: Heritage, Topographic, Download, Contour & Terrain Windows (UI2-b)

This document compares, control by control, 12 windows from the C++ reference (`target/cpp-ref/src/app/`) against the Rust/egui port (`crates/strata-app/src/`).

- **C++ reference**: `target/cpp-ref/src/app/` (read-only)
- **Rust target**: `crates/strata-app/src/`
- **Scope**: HB, HA, HR, TA, TB, TI, DP1, DP2, SC (+ progress), TG, DC, GT.

---

## 1. HB — 「주변유적 받기」 (`KaHeritageBrowser`)

### Header
- **C++ title & location**: `주변유적 받기` (`target/cpp-ref/src/app/KaHeritageBrowser.cpp:204`, inside `KaDownloadUi::configure` sets `주변유적 다운로드` on header banner `:209`, while `setWindowTitle("주변유적 받기")` is at `:204`)
- **How opened (C++)**: Ribbon button 「유산」 (`MainWindowRibbon.cpp:549`) → `MainWindow::fetchNearbyHeritage()` (`MainWindow.cpp:3071`) → `ensureHeritageBrowser()` + `m_heritageBrowser->show()` (`MainWindow.cpp:3102-3103`), then opens HR (`KaHeritageRegionDialog`) to confirm 시·군 before starting fetch.
- **How opened (Rust)**: Ribbon button 「유산」 (`gui_ribbon_click_a.rs:167-169`) opens `ActiveModalDialog::HeritageRegionSelect` (HR); clicking 「이 지역 받기」 in HR (`dlg_heritage_region_select.rs:155-156`) switches to `ActiveModalDialog::HeritageBrowser`.
- **Modality**: C++ non-modal (`setModal(false)` at `KaHeritageBrowser.cpp:205`, `.show()` at `MainWindow.cpp:3103`). Rust renders as an `egui::Window` overlay (`dlg_heritage_browser.rs:39`) blocking other modal slots via `ActiveModalDialog::HeritageBrowser` (`dialogs_download_print.rs:58`).
- **Size rules**:
  - **C++**: `KaWindowGeometry::placeDownload(this)` (`KaHeritageBrowser.cpp:206`, `KaWindowGeometry.h:34` → `680×330`, centered on parent, fitted to available screen); when 「상세 보기」 is toggled ON, `resize(980, 720)` + `KaWindowGeometry::fit(this)` (`KaHeritageBrowser.cpp:277-279`); resizable `QDialog`.
  - **Rust**: `.resizable(false).fixed_size([modal_w, modal_h])` where `modal_w = 940.0.min(vis_w - 32.0)`, `modal_h = 620.0.min(vis_h - 32.0)` (`dlg_heritage_browser.rs:33-43`).
- **Rust counterpart**: `crates/strata-app/src/dlg_heritage_browser.rs:23` (`render_heritage_browser`), `dlg_heritage_browser_progress.rs:12`, `dlg_heritage_browser_footer.rs:16`.

### Controls

| # | kind | C++ text (verbatim) | C++ tooltip / placeholder (verbatim) | C++ default, range, step, suffix | C++ enabled-when rule | C++ what it does | C++ file:line | Rust text | Rust tooltip / placeholder | Rust default, range, step, suffix | Rust enabled-when rule | Rust what it does | Rust file:line | same? |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 1 | Header title (QLabel) | `주변유적 다운로드` | — | — | Always | Header banner title via `KaDownloadUi::configure` | `KaHeritageBrowser.cpp:209`, `KaDownloadUi.cpp:20` | `주변유적 다운로드` | — | — | Always | `render_ka_download_header` title | `dlg_heritage_browser.rs:50` | yes |
| 2 | Header subtitle (QLabel) | `국가유산 공간정보 인트라넷 · 선택한 시·군의 자료를 순서대로 받습니다.` | — | — | Always | Header banner subtitle | `KaHeritageBrowser.cpp:210`, `KaDownloadUi.cpp:21` | `국가유산 공간정보 인트라넷 · 선택한 시·군의 자료를 순서대로 받습니다.` | — | — | Always | `render_ka_download_header` subtitle | `dlg_heritage_browser.rs:51` | yes |
| 3 | Stage label (QLabel) | `대기` (runtime: `[%1/%2] %3 — %4` or `단계` name) | — | Default `대기` | Always | Displays current stage name (`m_stageLabel`) | `KaHeritageBrowser.cpp:213,400-404` | `단계: <stage_name_ko>` | — | Default `대기` (`stage_idx = 0`) | Always | Displays `단계: {}` (`stage_enum.stage_name_ko()`) | `dlg_heritage_browser_progress.rs:40-44` | no (text format) |
| 4 | Detail label (QLabel) | `""` (runtime status messages) | — | Default empty | Always | Displays detailed status (`m_detailLabel`) | `KaHeritageBrowser.cpp:215` | `&app.modals.heritage_status_detail` | — | Default empty | Always | Displays `heritage_status_detail` | `dlg_heritage_browser_progress.rs:49-53` | yes |
| 5 | Notice label (QLabel) | `""` (hidden initially; retry/skip notes) | — | Hidden initially | Always (when shown) | Displays retry / skipped-city notice (`m_noticeLabel`) | `KaHeritageBrowser.cpp:220-223` | `MISSING` | — | — | — | — | — | MISSING |
| 6 | Progress bar (QProgressBar) | Accessible: `받기 진행률` | — | Range `0..0` (indeterminate); `0..100` on Done/Idle/Fail | Always | Shows fetch progress (`m_progress`) | `KaHeritageBrowser.cpp:225-227`, `KaDownloadUi.cpp:31-35` | `<pct>% · <sido> <city>` (painted bar) | — | `heritage_progress_pct` clamped `0.0..=1.0` | Always | Painted progress bar | `dlg_heritage_browser_progress.rs:56-84` | no (text/range) |
| 7 | Tab widget (QTabWidget) | Embedded web browser tabs (`m_tabs`) | — | Hidden (`setVisible(false)`) | When 「상세 보기」 toggled ON | Hosts `QWebEngineView` pages for intranet | `KaHeritageBrowser.cpp:252-257` | `MISSING` (no embedded webview) | — | — | — | — | — | MISSING |
| 8 | PlainTextEdit (QPlainTextEdit) | `""` (read-only outline) | Placeholder: `진행 중 문제가 발생한 경우 확인할 상세 내용입니다.` | Hidden initially; maxHeight `140` | Read-only | Shows page outline on stall (`m_outline`) | `KaHeritageBrowser.cpp:260-266` | Frame with monospace lines (`heritage_request_log`) | None | Hidden (`heritage_show_details == false`) | Always | Shows `heritage_request_log` lines | `dlg_heritage_browser_footer.rs:20-36` | no (placeholder/type) |
| 9 | PushButton (toggle) | `상세 보기` | Tooltip: `공식 사이트 화면을 펼쳐 봅니다. 막혔을 때만 쓰면 됩니다.` | Checkable, default `false` | Always | Toggles `m_tabs` + `m_outline` visibility and resizes window | `KaHeritageBrowser.cpp:269-283` | `상세 보기` | None | Toggle `heritage_show_details` | Always | Toggles log frame visibility | `dlg_heritage_browser_footer.rs:42-52` | no (tooltip/behavior) |
| 10 | PushButton | `취소` | None | — | Always | Calls `stop()` (`m_stopButton`) | `KaHeritageBrowser.cpp:286-289` | `취소` | None | — | Always | Closes modal (`app.modals.active = None`) | `dlg_heritage_browser_footer.rs:58-65` | no (what it does) |
| 11 | EXTRA: Window close / banner | — | — | — | — | — | — | `▲ 국가유산 인트라넷 아이디·비밀번호가 설정되지 않았습니다. [설정]을 눌러 계정을 입력해 주세요.` + `설정` button | — | Shown when `!heritage_configured` | Always | Opens `AccountSettings` | `dlg_heritage_browser.rs:56-79` | EXTRA |
| 12 | EXTRA: ComboBox + ComboBox + CheckBox | — | — | — | — | — | — | `시·도` combo, `시·군·구` combo, `최근 받은 자료 다시 쓰기` checkbox | — | Inline region & reuse controls | Always | Edits `heritage_sido`, `heritage_city`, `heritage_reuse_recent` | `dlg_heritage_browser.rs:89-123` | EXTRA |
| 13 | EXTRA: 7 CheckBoxes | — | — | — | — | — | — | `받을 항목:` + 7 dataset checkboxes (`국가등록문화유산`, `국가지정유산`, `국가지정유산보호구역`, `시도지정유산`, `시도지정유산보호구역`, `매장유산유존지역`, `문화유적분포지도`) | — | `heritage_dataset_checks[0..7]` | Always | Selects datasets to fetch | `dlg_heritage_browser.rs:127-144` | EXTRA |
| 14 | EXTRA: Label | — | — | — | — | — | — | `받기를 누르면 국가유산 인트라넷의 「원본자료 사용 서약서」에 자동으로 동의하고...` (shortened `preDisclosure`) | — | — | Always | Pledge note inside progress card | `dlg_heritage_browser_progress.rs:87-93` | EXTRA |
| 15 | EXTRA: PushButton | — | — | — | — | — | — | `설정` | — | — | Always | Opens `AccountSettings` | `dlg_heritage_browser_footer.rs:53-55` | EXTRA |
| 16 | EXTRA: PushButton | — | — | — | — | — | — | `받기` | Disabled tooltip: `아직 Rust 판에서 연결하지 않았습니다 (지시 WB1)` | — | Always disabled (`false`) | Deferred until WB1 | `dlg_heritage_browser_footer.rs:66-78` | EXTRA |

### Keyboard & Closing
- **Default button**: C++ none explicitly set (first focusable button). Rust none.
- **Enter / Esc**: C++ `QDialog` Esc calls `reject()` (closes dialog). Rust does not handle `Key::Enter` or `Key::Escape` (`dlg_heritage_browser.rs:39-154`).
- **Tab order**: C++ Qt default widget creation order (`detailsButton` → `m_stopButton`). Rust egui widget order.
- **Button box order**: C++ `[상세 보기]` left, stretch, `[취소]` right (`KaHeritageBrowser.cpp:284-289`). Rust `[상세 보기]` `[설정]` left, `[취소]` `[받기]` right-to-left (`dlg_heritage_browser_footer.rs:41-79`).
- **Window X close**: C++ `QDialog::reject()`. Rust `render_strata_modal_header` sets `*open = false` → `app.modals.active = None` (`dialogs_modal.rs:60,97-99`).

### Messages
- `받을 시·군이 정해지지 않았습니다.` (`KaHeritageBrowser.cpp:586`) → Rust: `MISSING`
- `저장할 폴더가 정해지지 않았습니다.` (`KaHeritageBrowser.cpp:590`) → Rust: `MISSING`
- `국가유산 인트라넷 계정이 없습니다. 더보기 → 국가유산 인트라넷 아이디·비밀번호에서 넣어 주세요.` (`KaHeritageBrowser.cpp:598`) → Rust: `▲ 국가유산 인트라넷 아이디·비밀번호가 설정되지 않았습니다. [설정]을 눌러 계정을 입력해 주세요.` (`dlg_heritage_browser.rs:67`)
- `로그인이 되지 않았습니다. 아이디·비밀번호를 확인해 주세요.` (`KaHeritageBrowser.cpp:930`) → Rust: `MISSING`
- `사용자가 중지했습니다.` (`KaHeritageBrowser.cpp:488`) → Rust: `MISSING`
- `HTTP 경로: %1` (`KaHeritageBrowser.cpp:233`), `「%1」 단계에서 화면이 예상과 달라 멈췄습니다. 아래 내용을 보고 알려 주세요.` (`KaHeritageBrowser.cpp:872`), `국가유산 사이트가 찾을 수 없는 페이지를 표시했습니다.` (`KaHeritageBrowser.cpp:953`), `시·도 목록에서 「%1」을 찾지 못했습니다.` (`KaHeritageBrowser.cpp:1032`), `시·군·구 목록 %1개에 「%2」이(가) 없습니다.` (`KaHeritageBrowser.cpp:1078`) → Rust: `MISSING`

---

## 2. HA — 「국가유산 인트라넷 아이디·비밀번호」 (`KaHeritageAccountDialog`)

### Header
- **C++ title & location**: `국가유산 인트라넷 아이디·비밀번호` (`target/cpp-ref/src/app/KaHeritageSetupDialogs.cpp:24`)
- **How opened (C++)**: Ribbon 「더보기」 menu → 「국가유산 인트라넷 아이디·비밀번호」 (`MainWindowRibbon.cpp:680`), or automatically from `MainWindow::openHeritageBrowserFor` when credentials are missing (`MainWindow.cpp:3162` → `configureHeritageAccount()` `:3064-3066`), or from `MainWindowChrome.cpp:227`.
- **How opened (Rust)**: No dedicated dialog. Unified `ActiveModalDialog::AccountSettings` opened from ribbon 「더보기」 (`gui_ribbon_click_b.rs:147`), Home connection card (`gui_tab_home_status.rs:108`), or HB 「설정」 (`dlg_heritage_browser_footer.rs:54`).
- **Modality**: C++ modal (`dialog.exec()` at `MainWindow.cpp:3066`). Rust `AccountSettings` is an `egui::Window` overlay (`modal_paper_account.rs:148`).
- **Size rules**:
  - **C++**: `resize(480, 300)` bounded to `screen->availableGeometry().size() - QSize(24, 24)` (`KaHeritageSetupDialogs.cpp:75-78`), resizable `QDialog` with `QScrollArea`.
  - **Rust**: `.resizable(false).default_width(520.0)` (`modal_paper_account.rs:150-152`).
- **Rust counterpart**: Unified `render_account_settings` (`crates/strata-app/src/modal_paper_account.rs:147-292`); dedicated HA dialog is `MISSING`.

### Controls

| # | kind | C++ text (verbatim) | C++ tooltip / placeholder (verbatim) | C++ default, range, step, suffix | C++ enabled-when rule | C++ what it does | C++ file:line | Rust text | Rust tooltip / placeholder | Rust default, range, step, suffix | Rust enabled-when rule | Rust what it does | Rust file:line | same? |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 1 | Label (description) | `주변유적을 받을 때 사용할 국가유산 공간정보 인트라넷 계정을 이 PC에 저장합니다. 수치지형도 계정과는 별개입니다. 두 항목을 모두 비우고 저장하면 자동로그인 계정을 해제합니다.` | — | — | Always | Explains heritage intranet credential storage | `KaHeritageSetupDialogs.cpp:32-34` | `MISSING` (shows NGII description only, `:254`) | — | — | — | — | — | MISSING |
| 2 | Form label + LineEdit | `아이디` | None | Saved `HeritageIntranetSettings::credentials().username` | Always | Edits username (`heritageAccountUsername`) | `KaHeritageSetupDialogs.cpp:40-41,46,48` | `국가유산청 인트라넷 ID / PW:` (1st TextEdit) | None | Loaded `heritage_creds.username` | Always | Edits `app.accounts.heritage_creds.username` | `modal_paper_account.rs:232-239` | no (merged form label) |
| 3 | Form label + LineEdit | `비밀번호` | None | Saved password; `EchoMode::Password` | Always | Edits password (`heritageAccountPassword`) | `KaHeritageSetupDialogs.cpp:42-44,47,49` | `국가유산청 인트라넷 ID / PW:` (2nd TextEdit) | None | Loaded `heritage_creds.password`; masked unless `show_passwords` | Always | Edits `app.accounts.heritage_creds.password` | `modal_paper_account.rs:240-246` | no (merged form label) |
| 4 | Error label (QLabel) | `""` (hidden initially) | — | Hidden until validation/save error | Always | Shows validation/save error (`heritageAccountError`) | `KaHeritageSetupDialogs.cpp:51-55,71-72` | `&app.accounts.validation_error` (red label) | — | Shown when `Some(err)` | Always | Shows `validation_error` | `modal_paper_account.rs:260-262` | yes |
| 5 | PushButton (Save) | `저장` | None | — | Always | Validates both-filled-or-both-empty, saves credentials, `accept()` | `KaHeritageSetupDialogs.cpp:60-61,65-74` | `저장` | None | — | Always | Validates all 3 site accounts and saves all 5 entries to disk | `modal_paper_account.rs:270-288` | no (saves all accounts) |
| 6 | PushButton (Cancel) | `취소` | None | — | Always | Calls `QDialog::reject()` | `KaHeritageSetupDialogs.cpp:60,62,64` | `닫기` | None | — | Always | Sets `app.modals.active = None` | `modal_paper_account.rs:267-269` | no (text) |
| 7 | EXTRA: Unified AccountSettings controls | — | — | — | — | — | — | Status summary, `VWorld API 키`, `역사지리정보DB 키`, `VWorld 지적도 ID / PW`, `수치지형도(NGII) ID / PW`, NGII description, `비밀번호 표시` checkbox | Hints on API keys | — | Always | Edits/saves all 5 account entries in one window | `modal_paper_account.rs:155-265` | EXTRA |

### Keyboard & Closing
- **Default button**: C++ `QDialogButtonBox::Save` (default button in `QDialogButtonBox`). Rust none.
- **Enter / Esc**: C++ Enter triggers `Save`; Esc triggers `reject()`. Rust ignores both.
- **Button box order**: C++ `Save` (`저장`) | `Cancel` (`취소`). Rust left `비밀번호 표시`, right-to-left `닫기` | `저장`.
- **Window X close**: C++ `reject()`. Rust `*open = false` → `app.modals.active = None`.

### Messages
- `아이디와 비밀번호를 모두 입력하거나 두 항목을 모두 비워 주세요.` (`KaHeritageSetupDialogs.cpp:69`) → Rust: `아이디와 비밀번호를 모두 입력하거나 두 항목을 모두 비워 주세요.` (`dialogs_account.rs:95`)
- Save error from `HeritageIntranetSettings::saveCredentials(&why)` (`KaHeritageSetupDialogs.cpp:70-71`) → Rust: `StrataError::Secret(message)` or `"계정 설정을 기록하지 못했습니다. 저장 공간과 폴더 권한을 확인하세요."` (`dialogs_account.rs:157-160`).

---

## 3. HR — 「주변유적 받을 시·군 선택」 (`KaHeritageRegionDialog`)

### Header
- **C++ title & location**: `주변유적 받을 시·군 선택` (`target/cpp-ref/src/app/KaHeritageSetupDialogs.cpp:88`)
- **How opened (C++)**: `MainWindow::openHeritageBrowserFor` (`MainWindow.cpp:3155`) after `HeritageRegionResolver` finishes (or fails) when user clicks ribbon 「유산」.
- **How opened (Rust)**: Ribbon 「유산」 (`gui_ribbon_click_a.rs:167-169`) directly opens `ActiveModalDialog::HeritageRegionSelect`.
- **Modality**: C++ modal (`choice.exec()` at `MainWindow.cpp:3156`, parented to `m_heritageBrowser`). Rust renders two `egui::Window`s simultaneously: background `ka_modal_heritage_bg` (`Order::Middle`) and foreground `ka_modal_heritage_region` (`Order::Foreground`) (`dlg_heritage_region_select.rs:13-77`).
- **Size rules**:
  - **C++**: `resize(520, 300)` (`KaHeritageSetupDialogs.cpp:165`); `addNearby` calls `resize(width(), qMax(height(), sizeHint().height()))` (`:227`).
  - **Rust**: Foreground `.resizable(false).default_width(480.0)` (`dlg_heritage_region_select.rs:73-76`); background `.resizable(false).default_width(600.0)` (`:15-18`).
- **Rust counterpart**: `crates/strata-app/src/dlg_heritage_region_select.rs:12` (`render_heritage_region_select`).

### Controls

| # | kind | C++ text (verbatim) | C++ tooltip / placeholder (verbatim) | C++ default, range, step, suffix | C++ enabled-when rule | C++ what it does | C++ file:line | Rust text | Rust tooltip / placeholder | Rust default, range, step, suffix | Rust enabled-when rule | Rust what it does | Rust file:line | same? |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 1 | Label (description) | `조사구역이 속한 시·군을 확인하세요.\n자료는 선택한 시·군 단위로만 받습니다.` (or `<reason>\n조사구역이 속한 시·군을 아래에서 직접 선택하면 계속 받을 수 있습니다.`) | — | — | Always | Explains 시·군 selection (`heritageRegionText`) | `KaHeritageSetupDialogs.cpp:91-94` | `조사구역이 속한 시·군을 확인하세요.\n자료는 선택한 시·군 단위로만 받습니다.` | — | Fixed text (no `reason` variant) | Always | Static label | `dlg_heritage_region_select.rs:79` | no (missing `reason` variant) |
| 2 | Form label + ComboBox | `시·도` / `시·도 선택` + `KoreaRegionCatalog::sidoNames()` | — | Default `시·도 선택` or `initialSido` | Always | Selecting 시·도 repopulates `m_city` | `KaHeritageSetupDialogs.cpp:100-106,144-153` | `시·도` / `시·도 선택` + `KoreaRegionCatalog::all_sido()` | — | Default `app.modals.heritage_sido` | Always | Updates `heritage_sido` and clears invalid `heritage_city` | `dlg_heritage_region_select.rs:84-114` | yes |
| 3 | Form label + ComboBox | `시·군·구` / `시·군·구 선택` + `KoreaRegionCatalog::citiesOf(sido)` | — | Default `시·군·구 선택` or `initialCity` | Always | Selecting 시·군·구 updates OK enabled state & `refreshReuse()` | `KaHeritageSetupDialogs.cpp:102-107,145-154` | `시·군·구` / `시·군·구 선택` + `KoreaRegionCatalog::cities_of(&heritage_sido)` | — | Default `app.modals.heritage_city` | Always | Updates `heritage_city` | `dlg_heritage_region_select.rs:118-137` | yes |
| 4 | GroupBox + CheckBoxes (nearby) | `조사구역 주변 5km에 걸친 다른 시·군` + hint + `city.display()` checkboxes + `note` | — | Unchecked by default; hidden if `nearby` & `note` empty | Always | Lets user include 5 km neighbouring 시·군 (`addNearby`) | `KaHeritageSetupDialogs.cpp:174-228` | `MISSING` | — | — | — | — | — | MISSING |
| 5 | CheckBox | `최근 받은 자료 다시 쓰기` (or with `(%1 받음 · 없는 자료만 서버에서 받음)`) | Tooltip: `같은 시·군에서 최근에 받아 검사·적재까지 마친 자료를 다시 씁니다. 끄면 서버에서 새로 받습니다.` | Hidden when no recent download exists (`!newest.isValid()`) | Always (when shown) | Reuses recent validated download (`m_reuse`) | `KaHeritageSetupDialogs.cpp:122-126,248-253` | `최근 받은 자료 다시 쓰기` | None | Always visible | Always | Toggles `app.modals.heritage_reuse_recent` | `dlg_heritage_region_select.rs:140-142` | no (tooltip, always visible) |
| 6 | Label (reuse timestamp) | `%1 받음 · 없는 자료만 서버에서 받음` | — | Visible only in `m_paper` mode when recent exists | Always (when shown) | Shows timestamp of newest recent download (`m_reuseWhen`) | `KaHeritageSetupDialogs.cpp:128-131,251-253` | `MISSING` | — | — | — | — | — | MISSING |
| 7 | Label (pledge note) | `HeritagePledge::preDisclosure()`: `받기를 누르면 국가유산 인트라넷의 「원본자료 사용 서약서」에 자동으로 동의하고, 동의 시각과 서약서 원문을 조사폴더 주변유적/receipts 에 남깁니다. 받은 자료는 동국문화재연구원 소유입니다. 도면·보고서에 넣거나 밖으로 보내기 전에 서약 범위를 확인하세요.` | — | — | Always | Pre-disclosure of automatic pledge | `KaHeritageSetupDialogs.cpp:133-138`, `HeritagePledge.cpp:41-47` | Identical verbatim text | — | — | Always | Static label | `dlg_heritage_region_select.rs:144` | yes |
| 8 | PushButton (Ok) | `이 지역 받기` | None | — | Enabled when `!sido().isEmpty() && !city().isEmpty()` | Calls `accept()` | `KaHeritageSetupDialogs.cpp:139-141,149-157` | `이 지역 받기` | None | — | Enabled when `!heritage_sido.is_empty() && !heritage_city.is_empty()` | Switches to `ActiveModalDialog::HeritageBrowser` | `dlg_heritage_region_select.rs:152-157` | yes |
| 9 | PushButton (Cancel) | `취소` | None | — | Always | Calls `QDialog::reject()` | `KaHeritageSetupDialogs.cpp:142,158` | `취소` | None | — | Always | Sets `app.modals.active = None` | `dlg_heritage_region_select.rs:147-150` | yes |
| 10 | EXTRA: Background window | — | — | — | — | — | — | `ka_modal_heritage_bg` (`주변유적 다운로드 - Strata`, `준비 · 시·군 선택`, `조사구역이 속한 시·군을 확인하고 있습니다.`, dead `상세 보기` & `취소` buttons) | — | — | Always | Decorative background window with dead buttons | `dlg_heritage_region_select.rs:13-70` | EXTRA |

### Keyboard & Closing
- **Default button**: C++ `QDialogButtonBox::Ok` (`이 지역 받기`). Rust none.
- **Enter / Esc**: C++ Enter triggers `이 지역 받기` if enabled; Esc triggers `reject()`. Rust ignores both.
- **Button box order**: C++ `이 지역 받기` | `취소`. Rust right-to-left `취소` | `이 지역 받기`.
- **Window X close**: C++ `reject()`. Rust `*open = false` → `app.modals.active = None`.

### Messages
- Pre-disclosure pledge text (`HeritagePledge::preDisclosure()`, `HeritagePledge.cpp:41-47`) → Rust: identical (`dlg_heritage_region_select.rs:144`).
- `reason` fallback header (`KaHeritageSetupDialogs.cpp:94`) → Rust: `MISSING`.

---

## 4. TA — 「수치지형도 아이디·비밀번호」 (`KaTopographicAccountDialog`)

### Header
- **C++ title & location**: `수치지형도 아이디·비밀번호` (`target/cpp-ref/src/app/KaTopographicAccountDialog.cpp:14`)
- **How opened (C++)**: Ribbon 「더보기」 menu → 「수치지형도 아이디·비밀번호」 (`MainWindowRibbon.cpp:679`) → `MainWindow::configureTopographicAccount()` (`MainWindowTopographic.cpp:74-77`), or `MainWindowChrome.cpp:225`.
- **How opened (Rust)**: No dedicated dialog. Unified `ActiveModalDialog::AccountSettings` (`modal_paper_account.rs:147`).
- **Modality**: C++ modal (`dialog.exec()` at `MainWindowTopographic.cpp:77`). Rust `AccountSettings` is an `egui::Window` overlay.
- **Size rules**:
  - **C++**: `resize(480, 300)` bounded to `screen->availableGeometry().size() - QSize(24, 24)` (`KaTopographicAccountDialog.cpp:65-68`), resizable `QDialog` with `QScrollArea`.
  - **Rust**: `.resizable(false).default_width(520.0)` (`modal_paper_account.rs:150-152`).
- **Rust counterpart**: Unified `render_account_settings` (`crates/strata-app/src/modal_paper_account.rs:147-292`); dedicated TA dialog is `MISSING`.

### Controls

| # | kind | C++ text (verbatim) | C++ tooltip / placeholder (verbatim) | C++ default, range, step, suffix | C++ enabled-when rule | C++ what it does | C++ file:line | Rust text | Rust tooltip / placeholder | Rust default, range, step, suffix | Rust enabled-when rule | Rust what it does | Rust file:line | same? |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 1 | Label (description) | `국토지리정보원 계정을 이 PC에 저장하면 수치지형도를 받을 때 자동 로그인에 사용합니다. 두 항목을 모두 비우고 저장하면 저장된 자동로그인 계정을 해제합니다.` | — | — | Always | Explains NGII account storage | `KaTopographicAccountDialog.cpp:25` | Identical verbatim text | — | — | Always | Static label | `modal_paper_account.rs:252-258` | yes |
| 2 | Form label + LineEdit | `아이디` | None | Saved `TopographicSettings::credentials().username` | Always | Edits username (`topographicAccountUsername`) | `KaTopographicAccountDialog.cpp:32-34,39,43` | `수치지형도(NGII) ID / PW:` (1st TextEdit) | None | Loaded `ngii_creds.username` | Always | Edits `app.accounts.ngii_creds.username` | `modal_paper_account.rs:215-222` | no (merged form label) |
| 3 | Form label + LineEdit | `비밀번호` | None | Saved password; `EchoMode::Password` | Always | Edits password (`topographicAccountPassword`) | `KaTopographicAccountDialog.cpp:35-38,40,43` | `수치지형도(NGII) ID / PW:` (2nd TextEdit) | None | Loaded `ngii_creds.password`; masked unless `show_passwords` | Always | Edits `app.accounts.ngii_creds.password` | `modal_paper_account.rs:223-229` | no (merged form label) |
| 4 | Error label (QLabel) | `""` (hidden initially) | — | Hidden until error | Always | Shows validation/save error (`topographicAccountError`) | `KaTopographicAccountDialog.cpp:44-47,63` | `&app.accounts.validation_error` | — | Shown when `Some(err)` | Always | Shows `validation_error` | `modal_paper_account.rs:260-262` | yes |
| 5 | PushButton (Save) | `저장` | None | — | Always | Validates both-filled-or-both-empty, saves credentials, `accept()` | `KaTopographicAccountDialog.cpp:50-52,57-64` | `저장` | None | — | Always | Validates all 3 site accounts and saves all 5 entries | `modal_paper_account.rs:270-288` | no (saves all accounts) |
| 6 | PushButton (Cancel) | `취소` | None | — | Always | Calls `QDialog::reject()` | `KaTopographicAccountDialog.cpp:53-56` | `닫기` | None | — | Always | Sets `app.modals.active = None` | `modal_paper_account.rs:267-269` | no (text) |

### Keyboard & Closing
- **Default button**: C++ `QDialogButtonBox::Save`. Rust none.
- **Enter / Esc**: C++ Enter triggers `Save`; Esc triggers `reject()`. Rust ignores both.
- **Button box order**: C++ `저장` | `취소`. Rust right-to-left `닫기` | `저장`.
- **Window X close**: C++ `reject()`. Rust `*open = false` → `app.modals.active = None`.

### Messages
- `아이디와 비밀번호를 모두 입력하거나 두 항목을 모두 비워 주세요.` (`KaTopographicAccountDialog.cpp:61`) → Rust: identical (`dialogs_account.rs:95`).
- Save error from `TopographicSettings::saveCredentials(&error)` (`KaTopographicAccountDialog.cpp:62-63`) → Rust: `StrataError::Secret(message)` or `"계정 설정을 기록하지 못했습니다. 저장 공간과 폴더 권한을 확인하세요."` (`dialogs_account.rs:157-160`).

---

## 5. TB — 「수치지형도 내려받기」 (`KaTopographicBrowser`)

### Header
- **C++ title & location**: `수치지형도 내려받기` (`target/cpp-ref/src/app/KaTopographicBrowser.cpp:120`; `KaDownloadUi::configure` sets `수치지형도 다운로드` on banner `:126`)
- **How opened (C++)**: Ribbon 「수치지형」 button → `MainWindow::openTopographicDownload()` (`MainWindowTopographic.cpp:18,68-70`).
- **How opened (Rust)**: Ribbon 「수치지형」 button (`gui_ribbon_click_a.rs:163-165`) sets `ActiveModalDialog::NgiiTopoDownload` → `render_ngii_topo_download` (`dlg_ngii_topo_download.rs:10`) → `render_topographic_browser` (`dlg_topographic_browser.rs:33`).
- **Modality**: C++ non-modal (`setModal(false)` at `KaTopographicBrowser.cpp:121`, `setWindowModality(Qt::NonModal)` at `MainWindowTopographic.cpp:68`). Rust `egui::Window` overlay.
- **Size rules**:
  - **C++**: Compact mode uses `KaWindowGeometry::placeDownload(this)` (`680×330`, `KaTopographicBrowser.cpp:308`); expanded mode resizes to `QSize(1100, 780).boundedTo(clientLimit)` (`:311-312`), `m_details->setMinimumSize(680, 480)` (`:137`); resizable `QDialog`.
  - **Rust**: `.resizable(false).default_width(620.0)` (`dlg_topographic_browser.rs:43-45`).
- **Rust counterpart**: `crates/strata-app/src/dlg_topographic_browser.rs:33`, `dlg_topo_browser_rail.rs:10`, `dlg_topo_browser_body.rs:13`, `ngii_topographic_state.rs:19`.

### Controls

| # | kind | C++ text (verbatim) | C++ tooltip / placeholder (verbatim) | C++ default, range, step, suffix | C++ enabled-when rule | C++ what it does | C++ file:line | Rust text | Rust tooltip / placeholder | Rust default, range, step, suffix | Rust enabled-when rule | Rust what it does | Rust file:line | same? |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 1 | Header title (QLabel) | `수치지형도 다운로드` | — | — | Always | Header banner title | `KaTopographicBrowser.cpp:126` | `수치지형도 다운로드` | — | — | Always | `render_ka_download_header` title | `dlg_topographic_browser.rs:52` | yes |
| 2 | Header subtitle (QLabel) | `국토정보플랫폼 · 조사 주변 도엽을 확인하고 지도 자료를 받습니다.` | — | — | Always | Header banner subtitle | `KaTopographicBrowser.cpp:127` | `국토정보플랫폼 · 조사 주변 도엽을 확인하고 지도 자료를 받습니다.` | — | — | Always | `render_ka_download_header` subtitle | `dlg_topographic_browser.rs:53` | yes |
| 3 | Stage rail (KaStageRail) | `도엽 확인 → 로그인 → 신청 → 파일 준비 → 다운로드 → 지도에 올리기` | — | Step 0..5 | Always | 6-step visual stage rail (`m_stageRail`) | `KaTopographicBrowser.cpp:176-179` | Painted 6-circle rail + `도엽 확인 → 로그인 → 신청 → 파일 준비 → 다운로드 → 지도에 올리기` + `[n/6] <stage>` | — | `stage = topo_browser_stage.min(5)` | Always | Draws 6-step rail and stage label | `dlg_topographic_browser.rs:61-76`, `dlg_topo_browser_rail.rs:10-77` | yes |
| 4 | Status labels (card) | `m_currentStage`, `m_compactStatus`, `m_orderSheets`, `m_failureOutline` | — | Hidden initially | Always | Displays stage, compact status, ordered sheets, failure outline | `KaTopographicBrowser.cpp:180-190` | Last 3 lines of `topo_browser_log` | — | — | Always | Shows `· <line>` | `dlg_topographic_browser.rs:100-106` | no (log vs structured labels) |
| 5 | Progress bar | `m_progress` | — | Range `0..1`, `textVisible(true)` | Always | Overall progress bar | `KaTopographicBrowser.cpp:193-196` | Painted progress bar `<n> / 6 — <stage>` (compact mode only) | — | `(stage + 1) / 6` | Shown when `!detail` | Painted stage fraction bar | `dlg_topo_browser_body.rs:145-170` | no (hidden in detail mode) |
| 6 | ToolBar actions (6) | `뒤로`, `앞으로`, `새로고침`, `국토정보맵`, `받은 폴더 열기`, `받은 자료 불러오기…` | None | Hidden in compact mode | Always (when expanded) | Web navigation, open download folder, or `importFolderRequested` | `KaTopographicBrowser.cpp:139-151` | 6 buttons with identical labels | None | Shown when `detail == true` | Always | `받은 자료 불러오기…` opens TI; others push log message | `dlg_topo_browser_body.rs:23-47` | no (5 buttons are log stubs) |
| 7 | Label (hint) | `현재 조사 주변의 1:25,000 DXF 도엽을 선택합니다. 로그인·인증이 필요하면 공식 화면의 안내를 따라 주세요.` | — | Hidden in compact mode | Always | Instruction hint | `KaTopographicBrowser.cpp:152` | Identical verbatim text | — | Always visible | Always | Static label | `dlg_topographic_browser.rs:87-93` | yes |
| 8 | Label (transferSummary) | `m_transferSummary` | — | Hidden in compact mode | Always | Transfer summary label | `KaTopographicBrowser.cpp:191-192` | `MISSING` | — | — | — | — | — | MISSING |
| 9 | Label (status) | `수치지형도를 준비하고 있습니다.` | — | Hidden in compact mode | Always | Activity status (`m_status`) | `KaTopographicBrowser.cpp:201-202` | `MISSING` (replaced by `ngii_topographic_ui::render_sheet_lookup`) | — | — | — | — | — | MISSING |
| 10 | LineEdit (loginId) | `""` | Placeholder: `국토지리정보원 아이디` | Set from `TopographicSettings` | Always | NGII login ID (`m_loginId`) | `KaTopographicBrowser.cpp:155-156` | `""` | Hint: `국토지리정보원 아이디` | `topo_browser_login_id` | Always (when `detail`) | Edits `topo_browser_login_id` | `dlg_topo_browser_body.rs:56-62` | yes |
| 11 | LineEdit (loginPassword) | `""` | Placeholder: `비밀번호 — 이번 로그인에만 사용` | `EchoMode::Password` | Always | NGII login password (`m_loginPassword`) | `KaTopographicBrowser.cpp:157-159` | `""` | Hint: `비밀번호 — 이번 로그인에만 사용` | `password(true)` | Always (when `detail`) | Edits `topo_browser_login_pw` | `dlg_topo_browser_body.rs:63-70` | yes |
| 12 | PushButton | `로그인 다시 시도` | None | — | Always | Increments `m_loginAttempt`, clears last automation message | `KaTopographicBrowser.cpp:160-161` | `로그인 다시 시도` | None | — | Always (when `detail`) | Sets stage=1, logs message | `dlg_topo_browser_body.rs:71-77` | no (stub) |
| 13 | LineEdit (birthDate) | `""` | Placeholder: `생년월일 (공식 신청서 형식)` | `maxWidth(220)` | Always | Birth date for official form (`m_birthDate`) | `KaTopographicBrowser.cpp:164-165` | `""` | Hint: `생년월일 (공식 신청서 형식)` | `topo_browser_birth_date` | Always (when `detail`) | Edits `topo_browser_birth_date` | `dlg_topo_browser_body.rs:80-86` | yes |
| 14 | LineEdit (purpose) | `고고학 발굴조사 현장 도면 작성 및 지형 확인` | Placeholder: `사용목적 — 이번 실행 동안만 사용` | Default `고고학 발굴조사 현장 도면 작성 및 지형 확인` | Always | Purpose for official form (`m_purpose`) | `KaTopographicBrowser.cpp:166-168` | `topo_browser_purpose` | None | `topo_browser_purpose` | Always (when `detail`) | Edits `topo_browser_purpose` | `dlg_topo_browser_body.rs:87-92` | no (placeholder) |
| 15 | PushButton | `자동 진행 중지` | None | — | Always | Calls `cancelFromUser()` | `KaTopographicBrowser.cpp:169-170` | `자동 진행 중지` / `자동 진행 재개` | None | — | Always (when `detail`) | Toggles `topo_browser_auto_stopped` | `dlg_topo_browser_body.rs:93-103` | no (text/stub) |
| 16 | TabWidget (m_tabs) | Embedded web browser tabs (`m_tabs`, closable) | — | Hidden in compact mode | Always | Hosts `QWebEngineView` pages | `KaTopographicBrowser.cpp:172-174` | `MISSING` (no embedded webview) | — | — | — | — | — | MISSING |
| 17 | TreeWidget (m_downloads) | Columns: `파일`, `진행`, `상태`, `취소` | — | `maxHeight(150)`, hidden in compact mode | Always | Lists active/completed file transfers | `KaTopographicBrowser.cpp:197-200` | Grid with 4 columns `파일`, `진행`, `상태`, `취소` | — | Hardcoded 3 sample rows (`37704055.dxf`, `37704056.dxf`, `37704057.dxf`) | Always (when `detail`) | Displays fake sample rows + dead `취소` buttons | `dlg_topo_browser_body.rs:109-141` | no (fake rows) |
| 18 | PushButton (expand) | `상세 보기` / `상세 접기` | Tooltip: `도엽·파일별 진행 상황과 공식 로그인·인증 화면을 확인합니다.` | Default `상세 보기` | Always | Toggles `setCompactMode(!m_compactMode)` | `KaTopographicBrowser.cpp:205-207,211,256` | `상세 보기` / `상세 접기` | None | Default `상세 보기` | Always | Toggles `topo_browser_detail` | `dlg_topographic_browser.rs:113-125` | no (tooltip) |
| 19 | PushButton (import fallback) | `받은 자료 불러오기…` | Tooltip: `공식 화면에서 직접 받은 DXF·ZIP 폴더를 골라 지도에 올립니다. 자동 신청은 다시 하지 않습니다.` | Hidden initially (`m_importFallback->hide()`) | Always (when shown) | Emits `importFolderRequested` | `KaTopographicBrowser.cpp:213-217` | `MISSING` (only in expanded toolbar) | — | — | — | — | — | MISSING |
| 20 | PushButton (cancel) | `취소` | None | — | Always | Calls `KaTopographicBrowser::reject()` | `KaTopographicBrowser.cpp:208-212` | `취소` | None | — | Always | Sets `app.modals.active = None` | `dlg_topographic_browser.rs:142-149` | yes |
| 21 | EXTRA: Sheet lookup row | — | — | — | — | — | — | `도엽 확인` button (disabled, tooltip `아직 Rust 판에서 연결하지 않았습니다 (지시 WB2)`) + `IDLE_STATUS` + `login_stage_blocked_message` | — | — | Always disabled | Deferred until WB2 | `ngii_topographic_state.rs:19-45` | EXTRA |
| 22 | EXTRA: PushButton | — | — | — | — | — | — | `다음 단계` | — | — | Always | Advances `topo_browser_stage` and opens TI at step 5 | `dlg_topographic_browser.rs:126-140` | EXTRA |

### Keyboard & Closing
- **Default button**: C++ none explicitly set. Rust none.
- **Enter / Esc**: C++ Esc calls `reject()` → `cancelFromUser()` + `QDialog::reject()` (`KaTopographicBrowser.cpp:334-338`). Rust ignores both.
- **Button box order**: C++ `[상세 보기]` `[받은 자료 불러오기…]` | stretch | `[취소]`. Rust `[상세 보기]` `[다음 단계]` | right-to-left `[취소]`.
- **Window X close**: C++ `reject()` cancels automation and downloads. Rust `*open = false` → `app.modals.active = None`.

### Messages
- `60초 동안 새 파일 데이터가 수신되지 않았습니다. 연결 상태를 확인한 뒤 다시 시도하세요.` (`KaTopographicBrowser.cpp:231`) → Rust: `MISSING`
- `이 단계의 서버 응답 시간이 초과되었습니다. 계정 설정과 연결 상태를 확인하세요. 신청은 자동으로 반복하지 않습니다.` (`KaTopographicBrowser.cpp:232`) → Rust: `MISSING`
- `공식 사이트 페이지를 불러오지 못했습니다. 인터넷 연결을 확인한 뒤 다시 시도하세요.` (`KaTopographicBrowser.cpp:582`) → Rust: `MISSING`
- `파일 크기가 1GB를 넘어서 자동 내려받기를 중단했습니다. 공식 목록에서 나누어 내려받아 주세요.` (`KaTopographicBrowser.cpp:942`) → Rust: `MISSING`
- `요청한 파일과 도착한 파일이 달라 전송을 중단했습니다.` (`KaTopographicBrowser.cpp:932`) → Rust: `MISSING`

---

## 6. TI — 「수치지형도 — 좌표계 확인과 지도 추가」 (`KaTopographicImportDialog`)

### Header
- **C++ title & location**: `수치지형도 — 좌표계 확인과 지도 추가` (`target/cpp-ref/src/app/KaTopographicImportDialog.cpp:289`)
- **How opened (C++)**: `MainWindow::importTopographicFolder()` (`MainWindowTopographic.cpp:102-109`) or `MainWindow::showTopographicFiles(folder)` (`MainWindowTopographic.cpp:111-120`) from TB toolbar/fallback or file download completion.
- **How opened (Rust)**: From TB 「받은 자료 불러오기…」 (`dlg_topo_browser_body.rs:37-40`) or TB 「다음 단계」 reaching step 5 (`dlg_topographic_browser.rs:137-138`).
- **Modality**: C++ `QDialog` (`show()` via `scanFolder`). Rust `egui::Window` overlay (`dlg_topographic_import.rs:50`).
- **Size rules**:
  - **C++**: `resize(1050, 720)` (`KaTopographicImportDialog.cpp:290`), resizable `QDialog`.
  - **Rust**: `.resizable(false).fixed_size([modal_w, modal_h])` where `modal_w = 960.0.min(vis_w - 32.0)`, `modal_h = 620.0.min(vis_h - 32.0)` (`dlg_topographic_import.rs:36-54`).
- **Rust counterpart**: `crates/strata-app/src/dlg_topographic_import.rs:24`, `dlg_topo_import_table.rs:12`, `dlg_topo_import_preview.rs:14`, `dlg_topo_import_actions.rs:12`.

### Controls

| # | kind | C++ text (verbatim) | C++ tooltip / placeholder (verbatim) | C++ default, range, step, suffix | C++ enabled-when rule | C++ what it does | C++ file:line | Rust text | Rust tooltip / placeholder | Rust default, range, step, suffix | Rust enabled-when rule | Rust what it does | Rust file:line | same? |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 1 | Label (hint) | `도엽의 좌표계를 확인한 뒤 추가하세요. 좌표 숫자만으로 구·신 측지계나 원점대를 확정할 수 없습니다.\n미리보기에서 도로·건물·하천이 기존 지도와 맞는지 확인하세요. 원본 파일은 변경하지 않습니다.` | — | — | Always | Top guidance label | `KaTopographicImportDialog.cpp:292-294` | Identical verbatim text | — | — | Always | Static label | `dlg_topographic_import.rs:65-71` | yes |
| 2 | EXTRA: Folder picker button + label | (Opened via `QFileDialog` before showing dialog) | — | — | — | — | `MainWindowTopographic.cpp:107` | `받은 폴더 고르기…` + folder path label (`아직 받은 폴더를 고르지 않았습니다.`) | — | — | Always | Opens `rfd::FileDialog::pick_folder()` and calls `rescan_topo_folder` | `dlg_topographic_import.rs:76-102` | EXTRA |
| 3 | TableWidget (4 cols) | Headers: `추가 / 레이어`, `파일`, `확인할 원본 좌표계`, `좌표 범위` | Col 0 tooltip: categoryName + reviewReason; Col 1 tooltip: `record.source` | Checkboxes per row; CRS combo: `좌표계 미확인 — 선택 필요`, optional `파일 정보: <crs>`, 12 EPSG presets (`5185..5188, 5173..5179, 4326`) | Always | Displays scanned layers, checkable, CRS selection combo, extent | `KaTopographicImportDialog.cpp:295-303,460-492` | Headers + `ScrollArea` rows with checkbox, label, file name, CRS combo (`row.choices`), extent | None on cells | Same 12 EPSG presets (`TOPO_CRS_CANDIDATES`) | Always | Selects row, checks/unchecks, chooses CRS | `dlg_topographic_import.rs:135-158`, `dlg_topo_import_table.rs:18-169`, `modal_topo_rows.rs:18-43` | no (tooltips missing) |
| 4 | MapCanvas (m_preview) | Embedded `QgsMapCanvas` (`minHeight(180)`) | — | White background | Always | Renders real transformed preview layer over canvas layers | `KaTopographicImportDialog.cpp:304-306,516-521` | Custom painted schematic box (`125 px` high) | — | Schematic only (checks `choice_is_work_crs_family`) | Always | Draws schematic green/red lines | `dlg_topo_import_preview.rs:36-105` | no (schematic vs real map) |
| 5 | CheckBox | `수치지형도 표시` | None | Checked (`true`) | Always | Toggles loaded topographic layer visibility (`setMapsEnabled`) | `KaTopographicImportDialog.cpp:308-311` | `수치지형도 표시` | None | `topo_maps_enabled` | Always | Toggles `layer_visible["수치지형도 1:5,000 (합침)"]` | `dlg_topo_import_actions.rs:23-31` | yes |
| 6 | PushButton | `선택 도엽 정합 미리보기` | None | — | Always | Calls `previewCurrent()` | `KaTopographicImportDialog.cpp:312,327` | `선택 도엽 정합 미리보기` | None | — | Always | Sets `topo_status_text` | `dlg_topo_import_actions.rs:32-44` | no (stub text only) |
| 7 | PushButton | `좌표계 확인 완료 · 지도에 추가` | None | — | Disabled initially; enabled when `!m_records.isEmpty()` (`:314,493`) | Calls `importSelected()` | `KaTopographicImportDialog.cpp:313-314,328,493` | `좌표계 확인 완료 · 지도에 추가` | None | — | Enabled when `!rows.is_empty()` | Checks blocking message / picked rows and updates `topo_status_text` | `dlg_topo_import_actions.rs:55-97` | no (does not load real layers) |
| 8 | PushButton | `색인 취소` | None | — | Always | Cancels `m_task` (`CatalogTask`) | `KaTopographicImportDialog.cpp:315,330` | `색인 취소` | None | — | Always | Calls `cancel_topo_scan()` and closes modal | `dlg_topo_import_actions.rs:50-53` | no (also closes modal) |
| 9 | PushButton | `닫기` | None | — | Always | Calls `QDialog::hide()` | `KaTopographicImportDialog.cpp:316,329` | `닫기` | None | — | Always | Sets `app.modals.active = None` | `dlg_topo_import_actions.rs:47-49` | yes |
| 10 | ProgressBar | `m_progress` | — | Range `0..100` | Always | Folder scan / import progress | `KaTopographicImportDialog.cpp:319` | Painted progress bar | — | `topo_progress_pct` `0.0..=1.0` | Always | Painted progress bar | `dlg_topo_import_actions.rs:103-117` | yes |
| 11 | Label (status) | `m_status` | — | Empty initially | Always | Displays scan summary, warnings, preview/import errors | `KaTopographicImportDialog.cpp:320-321` | `&app.modals.topo_status_text` | — | — | Always | Displays `topo_status_text` | `dlg_topo_import_actions.rs:118-122` | yes |

### Keyboard & Closing
- **Default button**: C++ none explicitly set. Rust none.
- **Enter / Esc**: C++ `QDialog` Esc calls `reject()`. Rust ignores both.
- **Button box order**: C++ `[수치지형도 표시]` `[선택 도엽 정합 미리보기]` `[좌표계 확인 완료 · 지도에 추가]` `[색인 취소]` `[닫기]`. Rust left `[수치지형도 표시]` `[선택 도엽 정합 미리보기]`, right-to-left `[닫기]` `[색인 취소]` `[좌표계 확인 완료 · 지도에 추가]`.
- **Window X close**: C++ `QDialog::reject()`. Rust `*open = false` → `app.modals.active = None`.

### Messages
- `검토 필요: %1. 자동으로 지도에 추가하지 않습니다.` (`KaTopographicImportDialog.cpp:325`) → Rust: identical (`dlg_topo_import_table.rs:107`).
- `먼저 미리 볼 원본 좌표계를 선택하세요.` (`KaTopographicImportDialog.cpp:503`) → Rust: identical (`dlg_topo_import_actions.rs:38`).
- `선택 좌표계의 미리보기입니다. 위성·지적과 정합을 확인한 뒤 추가하세요.` (`KaTopographicImportDialog.cpp:521`) → Rust: `선택 좌표계({sel_crs_lbl})의 미리보기입니다. 위성·지적과 정합을 확인한 뒤 추가하세요.` (`dlg_topo_import_actions.rs:41`).
- `이 레이어를 열 수 없습니다.` (`:506`), `도엽의 유효한 범위를 확인하지 못했습니다.` (`:514`), `선택 좌표계로 지도 위치를 계산하지 못했습니다.` (`:522`), `유효한 도형 범위가 없는 항목은 지도에 추가할 수 없습니다. 해당 체크를 해제하세요.` (`:530`), `좌표계가 확인되지 않은 레이어가 있습니다. 해당 좌표계를 선택하거나 추가 체크를 해제하세요.` (`:535`) → Rust: check `topo_blocking_message()` / `MISSING` for preview errors.

---

## 7. DP1 — 「지적도 다운로드」 (`KaDownloadProgressDialog`)

### Header
- **C++ title & location**: `지적도 다운로드` (`target/cpp-ref/src/app/KaDownloadUi.cpp:13,38`, opened by `MainWindow::downloadCadastral()` at `MainWindowCadastral.cpp:122-123`)
- **How opened (C++)**: Ribbon 「지적」 button or menu → `MainWindow::downloadCadastral()` (`MainWindowCadastral.cpp:82,122-173`) starts download immediately and shows non-modal `KaDownloadProgressDialog`.
- **How opened (Rust)**: Ribbon 「지적」 button (`gui_ribbon_click_a.rs:158-161`) opens `ActiveModalDialog::CadastralDownload`.
- **Modality**: C++ non-modal (`setModal(false)` at `KaDownloadUi.cpp:40`, `dialog->show()` at `MainWindowCadastral.cpp:173`). Rust `egui::Window` overlay (`cadastral_download_ui_render.rs:29`).
- **Size rules**:
  - **C++**: `KaWindowGeometry::placeDownload(this)` (`KaDownloadUi.cpp:56`, `680×330`), resizable `QDialog`.
  - **Rust**: `.resizable(false).default_width(540.0)` (`cadastral_download_ui_render.rs:31-33`).
- **Rust counterpart**: `crates/strata-app/src/dlg_cadastral_download.rs:9` → `cadastral_download_ui_render.rs:17` (`render_cadastral_download`).

### Controls

| # | kind | C++ text (verbatim) | C++ tooltip / placeholder (verbatim) | C++ default, range, step, suffix | C++ enabled-when rule | C++ what it does | C++ file:line | Rust text | Rust tooltip / placeholder | Rust default, range, step, suffix | Rust enabled-when rule | Rust what it does | Rust file:line | same? |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 1 | Header title (QLabel) | `지적도 다운로드` | — | — | Always | Header banner title | `MainWindowCadastral.cpp:122`, `KaDownloadUi.cpp:20` | `지적도 다운로드` (`CADASTRAL_DIALOG_TITLE`) | — | — | Always | Title label | `cadastral_download_ui_render.rs:38-41` | yes |
| 2 | Header subtitle (QLabel) | `VWorld · 조사구역 경계에서 주변 5km의 지적도를 준비합니다.` | — | — | Always | Header banner subtitle | `MainWindowCadastral.cpp:123`, `KaDownloadUi.cpp:21` | `VworldClient::CADASTRAL_DIALOG_SUBTITLE` | — | — | Always | Subtitle label | `cadastral_download_ui_render.rs:44-46` | yes |
| 3 | Label (stage) | `자료 준비 및 다운로드` | — | — | Always | Bold card heading | `KaDownloadUi.cpp:43-44` | `자료 준비 및 다운로드` | — | — | Always | Bold card heading | `cadastral_download_ui_render.rs:56-60` | yes |
| 4 | Label (detail) | `조사 범위를 확인하고 있습니다.` (updated by `setLabelText`) | — | — | Always | Shows current phase (`m_detail`) | `KaDownloadUi.cpp:45,60` | `&state.status` (idle: `조사구역 경계에서 주변 5km의 시·군·구 지적도를 준비합니다.`) | — | — | Always | Shows `state.status` | `cadastral_download_ui_render.rs:62`, `cadastral_download_ui_state.rs:35` | no (idle text) |
| 5 | ProgressBar | Accessible: `받기 진행률` | — | Range `0..100`, value `0` (`MainWindowCadastral.cpp:124`) | Always | Shows download progress | `KaDownloadUi.cpp:47-48`, `MainWindowCadastral.cpp:124` | Painted bar `<pct>%` | — | `0.0..=1.0` | Always | Painted progress bar | `cadastral_download_ui_render.rs:73-101` | yes |
| 6 | Label (hint) | `다운로드 중에도 지도 작업을 계속할 수 있습니다.` | — | — | Always | Non-modal work hint | `KaDownloadUi.cpp:51` | `MISSING` | — | — | — | — | — | MISSING |
| 7 | PushButton | `취소` | None | — | Always | Calls `reject()` → emits `canceled()` (`job->cancel()`) | `KaDownloadUi.cpp:53-55,61-63` | `취소` | None | — | Enabled only when `state.running` | Sets `state.cancel = true` | `cadastral_download_ui_render.rs:131-136,163-166` | no (enabled rule, does not close) |
| 8 | EXTRA: Key warning label | — | — | — | — | — | — | `VworldClient::NO_KEY_MESSAGE` | — | Shown when `!key_present` | Always | Warns missing VWorld key | `cadastral_download_ui_render.rs:64-71` | EXTRA |
| 9 | EXTRA: Downloaded files list | — | — | — | — | — | — | Up to 4 `· <path>` labels | — | Shown when `!downloaded.is_empty()` | Always | Lists downloaded ZIPs | `cadastral_download_ui_render.rs:102-111` | EXTRA |
| 10 | EXTRA: PushButton | — | — | — | — | — | — | `지적도 받기` | Hover: `조사구역 주변 5km 시·군·구 지적도 ZIP 을 조사 폴더 지적도/원본 에 받습니다.` | — | Enabled when `!state.running` | Starts `start_download` | `cadastral_download_ui_render.rs:115-130` | EXTRA |
| 11 | EXTRA: PushButton | — | — | — | — | — | — | `계정 설정` | None | — | Always | Opens `AccountSettings` | `cadastral_download_ui_render.rs:137-139` | EXTRA |
| 12 | EXTRA: PushButton | — | — | — | — | — | — | `닫기` | None | — | Always | Cancels and closes modal | `cadastral_download_ui_render.rs:141-156,167-170` | EXTRA |

### Keyboard & Closing
- **Default button**: C++ `취소` (only button). Rust none.
- **Enter / Esc**: C++ Esc calls `reject()` (cancels job and closes). Rust ignores both.
- **Button box order**: C++ `[hint]` | `[취소]`. Rust `[지적도 받기]` `[취소]` `[계정 설정]` | right-to-left `[닫기]`.
- **Window X close**: C++ `reject()` → cancels job. Rust `*open = false` → `app.modals.active = None` (note: header X does NOT set `state.cancel`!).

### Messages
- `먼저 조사를 열거나 저장하고 조사구역을 그려 주세요.` (`MainWindowCadastral.cpp:86`) → Rust: `VworldClient::NO_SURVEY_MESSAGE` (`cadastral_download_ui_state.rs:58`).
- `조사구역을 먼저 그려 주세요. 경계에서 주변 5km의 지적도를 받습니다.` (`MainWindowCadastral.cpp:114`) → Rust: `NO_SURVEY_AREA_MESSAGE` (`cadastral_download_ui_state.rs:45,68`).
- `지적도 받기를 취소했습니다.` (`MainWindowCadastral.cpp:139`) → Rust: `지적도 받기를 취소하는 중입니다.` (`cadastral_download_ui_render.rs:165`).

---

## 8. DP2 — 「<title> 내려받기」 (`MainWindowDownloads::createDownloadProgress`)

### Header
- **C++ title & location**: `<title> 내려받기` (`target/cpp-ref/src/app/MainWindowDownloads.cpp:27`, where `<title>` is `DEM`, `토양도`, `지질도`, `수계도`, or `오프라인 지도`)
- **How opened (C++)**:
  - DEM: Ribbon 「DEM」 (`MainWindowDownloads.cpp:133`)
  - 토양도 / 지질도 / 수계도: Ribbon 「토양」 / 「지질」 / 「수계」 (`MainWindowOverlay.cpp:612`)
  - 오프라인 지도: Offline download (`MainWindowOffline.cpp:319`)
- **How opened (Rust)**:
  - **DEM**: Ribbon 「DEM」 (`gui_ribbon_click_a.rs:155`) opens `ActiveModalDialog::DemDownload` (`dem_download_ui_render.rs:16`), a dedicated modal window (`국토지리정보원 DEM 다운로드 - Strata`).
  - **토양도 / 지질도 / 수계도**: Ribbon clicks (`gui_ribbon_click_b.rs:38-85`) toggle layer visibility directly and show a status bar message without opening a download window.
  - **오프라인 지도**: `MISSING` (no download window).
  - *Note*: Rust also has a status-bar `download_gauge` (`download_gauge.rs`, `download_gauge_popup.rs`) per `docs/RUST_PLAN.md` §2.8, but it is not yet wired to real download jobs.
- **Modality**: C++ non-modal `QProgressDialog` (`Qt::NonModal` at `MainWindowDownloads.cpp:28`). Rust `DemDownload` is an `egui::Window` overlay (`dem_download_ui_render.rs:25`).
- **Size rules**: C++ `QProgressDialog` default sizeHint, resizable. Rust `DemDownload`: `.resizable(false).default_width(510.0)` (`dem_download_ui_render.rs:27-29`).
- **Rust counterpart**: `crates/strata-app/src/dem_download_ui_render.rs:16` (for DEM only); `MISSING` for 토양도, 지질도, 수계도, 오프라인 지도.

### Controls (comparing C++ `QProgressDialog` with Rust `DemDownload`)

| # | kind | C++ text (verbatim) | C++ tooltip / placeholder (verbatim) | C++ default, range, step, suffix | C++ enabled-when rule | C++ what it does | C++ file:line | Rust text | Rust tooltip / placeholder | Rust default, range, step, suffix | Rust enabled-when rule | Rust what it does | Rust file:line | same? |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 1 | Label (progress text) | `<title> 자료를 내려받는 중입니다. 지도 작업을 계속할 수 있습니다.` | — | — | Always | `QProgressDialog` label | `MainWindowDownloads.cpp:23-24` | `조사구역 주변 공개 DEM 30m(Copernicus)를 받아 조사 폴더 DEM/원본 에 한 장으로 저장합니다.` + `&state.status` | — | — | Always | Description + status label (DEM only) | `dem_download_ui_render.rs:32-38,69-73` | no (DEM-only modal) |
| 2 | ProgressBar | `QProgressDialog` bar | — | Range `0..0` initially, then `0..100` (`:25,86-87`) | Always | Shows download progress | `MainWindowDownloads.cpp:25,86-87` | Painted bar `<pct>%` | — | `0.0..=1.0` | Always | Painted progress bar | `dem_download_ui_render.rs:40-67` | yes (for DEM) |
| 3 | PushButton (Cancel) | `취소` | None | — | Always | Emits `QProgressDialog::canceled` → `job->cancel()` | `MainWindowDownloads.cpp:25,83` | `내려받기 취소` | None | — | Enabled when `state.running` | Sets `state.cancel = true` | `dem_download_ui_render.rs:107-112` | no (text, enabled rule) |
| 4 | EXTRA: Tile count label | — | — | — | — | — | — | `받을 칸 <n>개 (1° 격자) · 한 번에 최대 <max>칸` | — | Shown when `tile_total > 0` | Always | Shows tile count | `dem_download_ui_render.rs:74-84` | EXTRA |
| 5 | EXTRA: PushButton (Start) | — | — | — | — | — | — | `DEM 받기` / `다시 받기` | Hover: `지금 화면 범위를 덮는 1° 공개 DEM 타일을 받습니다.` | — | Enabled when `!state.running` | Starts `start_download` | `dem_download_ui_render.rs:87-106` | EXTRA |
| 6 | EXTRA: PushButton (Close) | — | — | — | — | — | — | `닫기` | None | — | Always | Cancels and closes modal | `dem_download_ui_render.rs:115-130,141-145` | EXTRA |

### Keyboard & Closing
- **Default button**: C++ `취소`. Rust none.
- **Enter / Esc**: C++ Esc triggers `canceled` and closes. Rust ignores both.
- **Window X close**: C++ `canceled` signal. Rust `*open = false` (header X does not set `state.cancel`, unlike `닫기` button).

### Messages
- `<title> 내려받기를 취소했습니다. 기존 자료는 그대로입니다.` (`MainWindowDownloads.cpp:54`) → Rust: `DEM 내려받기를 취소하는 중입니다.` (`dem_download_ui_render.rs:139`).
- `<title> 내려받기 실패` / `자료를 받지 못했습니다. 인터넷 연결과 저장 공간을 확인하고 다시 시도하세요.` (`MainWindowDownloads.cpp:58-59`) → Rust: `MISSING` (no generic DP2 failure popup).

---

## 9. SC — 「측량 등고선」 & Progress (`KaSurveyContourDialog` + `contourProgress`)

### Header
- **C++ title & location**: `측량 등고선` (`target/cpp-ref/src/app/KaSurveyContourDialog.cpp:25`) + progress `QProgressDialog` (`MainWindowSurveyContour.cpp:132-134`)
- **How opened (C++)**: Ribbon 「등고선」 button (`MainWindowRibbon.cpp:435`) or 「DEM」 menu → 「측량점으로 등고선…」 (`MainWindowRibbon.cpp:418-419`) → `MainWindow::createSurveyContours()` (`MainWindowSurveyContour.cpp:104,122`).
- **How opened (Rust)**: Ribbon 「등고선」 button (`gui_ribbon_click_b.rs:31-33`) or 「DEM」 menu → 「측량점으로 등고선…」 (`gui_ribbon_menu_actions.rs:88-90`) → `ActiveModalDialog::SurveyContour`.
- **Modality**: C++ `KaSurveyContourDialog` is modal (`dialog.exec()` at `MainWindowSurveyContour.cpp:124`); `contourProgress` is non-modal (`Qt::NonModal` at `:135`). Rust `SurveyContour` is an `egui::Window` overlay (`dlg_survey_contour.rs:11`).
- **Size rules**:
  - **C++**: `setMinimumWidth(KaWindowSize::width(KaWindowSize::Medium, 460))` (`KaSurveyContourDialog.cpp:26`), resizable `QDialog` (expands when 「고급 설정」 is opened).
  - **Rust**: `.resizable(false).fixed_size([440.0_f32.min(w - 32.0), 290.0_f32.min(h - 32.0)])` (`dlg_survey_contour.rs:13-18`).
- **Rust counterpart**: `crates/strata-app/src/dlg_survey_contour.rs:10` (`render_survey_contour`); `contourProgress` is `MISSING`.

### Controls

| # | kind | C++ text (verbatim) | C++ tooltip / placeholder (verbatim) | C++ default, range, step, suffix | C++ enabled-when rule | C++ what it does | C++ file:line | Rust text | Rust tooltip / placeholder | Rust default, range, step, suffix | Rust enabled-when rule | Rust what it does | Rust file:line | same? |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 1 | EXTRA: Label | — | — | — | — | — | — | `엑셀·CSV·DXF의 측량 좌표(X, Y, Z)로 등고선과 표고점 레이어를 만듭니다.` | — | — | Always | Extra intro label | `dlg_survey_contour.rs:24-30` | EXTRA |
| 2 | Form label + Label/Edit + Button | `좌표 파일` / `파일을 고르세요` + `찾기…` | None | Default `파일을 고르세요` | Always | `browse()` opens `QFileDialog` (`*.dxf *.xlsx *.xls *.csv`), reads file | `KaSurveyContourDialog.cpp:28-34,142-147` | `좌표 파일` + `TextEdit` (`contour_file_path`) + `찾기…` button | None | Default empty `TextEdit` | Always | `TextEdit` edits path; `찾기…` button is dead (`let _ = ...`) | `dlg_survey_contour.rs:32-44` | no (TextEdit vs Label, dead button) |
| 3 | Form label + DoubleSpinBox | `등고선 간격` | None | Default `0.5`, range `0.1..100`, step `0.1`, decimals `1`, suffix ` m` | Always | Sets contour interval (`m_interval`) | `KaSurveyContourDialog.cpp:35-42` | `등고선 간격` + `DragValue` | None | Range `0.1..=100.0`, speed `0.1`, suffix ` m` | Always | Edits `contour_interval_m` | `dlg_survey_contour.rs:45-52` | yes |
| 4 | EXTRA: Label | — | — | — | — | — | — | `(계곡선 5줄마다 굵게 표시)` | — | — | Always | Extra note beside interval | `dlg_survey_contour.rs:53-57` | EXTRA |
| 5 | Form label + CheckBox | `축 순서` / `X를 북쪽, Y를 동쪽으로 읽기` | Tooltip: `측량 성과표는 보통 X가 북쪽입니다. 파일을 고르면 자동으로 정하고, 등고선이 대각선으로 뒤집혀 놓이면 이 칸을 바꾸세요.` | Auto-set from file | Always | Toggles axis swap (`m_swap`) | `KaSurveyContourDialog.cpp:43-47` | `축 순서` / `X를 북쪽, Y를 동쪽으로 읽기` | None | `contour_swap_axes` | Always | Edits `contour_swap_axes` | `dlg_survey_contour.rs:61-67` | no (tooltip) |
| 6 | Form label + CheckBox | `이상한 점` / `의심점 빼기` (with count: `의심점 %1개 빼기`) | Tooltip: `표고가 0이거나 999 m 이상인 점은 오측일 때가 많습니다. 빼면 삼각망에 넣지 않습니다.` | Unchecked | Disabled initially (`setEnabled(false)`); enabled when `suspiciousCount > 0` (`:190`) | Excludes suspicious points (`m_excludeSuspicious`) | `KaSurveyContourDialog.cpp:48-53,190-193` | `이상한 점` / `의심점 빼기` | None | `contour_exclude_suspicious` | Always enabled | Edits `contour_exclude_suspicious` | `dlg_survey_contour.rs:68-71` | no (tooltip, enabled rule) |
| 7 | ToolButton (toggle) | `고급 설정` (RightArrow / DownArrow) | None | Collapsed (`false`) | Always | Shows/hides `advanced` form (`more`) | `KaSurveyContourDialog.cpp:55-63,114-117` | `▸ 고급 설정` / `▾ 고급 설정` | None | `contour_advanced_open` | Always | Toggles `contour_advanced_open` | `dlg_survey_contour.rs:72-79` | yes |
| 8 | Form label + ComboBox | `파일 좌표계` / `작업 좌표계와 같음 (%1)` + `EPSG:5185 서부원점`, `5186 중부원점`, `5187 동부원점`, `5179 UTM-K`, `4326 경위도 WGS84` (excluding `m_crsAuthId`) | Tooltip: `측량 파일이 작업 좌표계와 다른 원점으로 되어 있을 때만 고르세요. 점을 작업 좌표계로 바꿔 놓습니다.` | Default index `0` | Always (inside `advanced`) | Re-arranges points with selected source CRS (`m_fileCrs`) | `KaSurveyContourDialog.cpp:66-79` | `파일 좌표계` + cycling `Button` (`<label>  ▼`) | None | Hardcoded `EPSG:5187` in index 0 | Always (when expanded) | Cycles `contour_file_crs_idx` `(idx + 1) % 5` | `dlg_survey_contour.rs:81-100` | no (button vs combo, tooltip) |
| 9 | Form label + DoubleSpinBox | `최대 삼각형 변` | Tooltip: `변이 이보다 긴 삼각형은 비워 둡니다. 트렌치·구덩이 사이 빈 곳을 가로지르는 가짜 사면을 막습니다.` | Default `0`, range `0..1000`, decimals `1`, suffix ` m`, special `쓰지 않음` | Always (inside `advanced`) | Sets max triangle edge (`m_maxEdge`) | `KaSurveyContourDialog.cpp:80-88` | `최대 삼각형 변` + `DragValue` + separate `쓰지 않음` label | None | Range `0.0..=1000.0`, speed `1.0`, suffix ` m` | Always (when expanded) | Edits `contour_max_edge_m` | `dlg_survey_contour.rs:101-116` | no (tooltip) |
| 10 | Form label + CheckBox | `단절선` / `높이가 있는 DXF 선을 단절선으로 쓰기` (with count: `...선 %1개를...`) | None | Unchecked | Disabled initially (`setEnabled(false)`); enabled when `breaklines.size() > 0` (`:196`) | Uses 3D DXF polylines as breaklines (`m_breaklines`) | `KaSurveyContourDialog.cpp:89-92,196-199` | `단절선` / `높이가 있는 DXF 선을 단절선으로 쓰기` | None | `contour_use_breaklines` | Always enabled | Edits `contour_use_breaklines` | `dlg_survey_contour.rs:117-123` | no (enabled rule) |
| 11 | Form label + DoubleSpinBox | `계산 격자` | Tooltip: `등고선을 뽑는 표고 격자 크기입니다. 자동은 점 간격의 절반입니다.` | Default `0`, range `0..5`, step `0.05`, decimals `2`, suffix ` m`, special `자동` | Always (inside `advanced`) | Sets grid cell size (`m_cell`) | `KaSurveyContourDialog.cpp:93-101` | `계산 격자` + `DragValue` + separate `자동` label | None | Range `0.0..=5.0`, speed `0.05`, suffix ` m` | Always (when expanded) | Edits `contour_cell_size_m` | `dlg_survey_contour.rs:124-139` | no (tooltip) |
| 12 | Label (preview) | `DXF·엑셀의 점번호, X(북), Y(동), 표고는 자동으로 읽습니다.` | Tooltip: `m_report.issues.join('\n')` | — | Always | Shows point count, elevation range, estimated lines, or error | `KaSurveyContourDialog.cpp:105-108,211-235` | Same initial text; if path non-empty shows fake `점 248개, 표고 24.5–48.2 m...` | None | — | Always | Shows summary or `좌표 파일을 고르세요.` | `dlg_survey_contour.rs:145-169` | no (fake numbers, tooltip) |
| 13 | PushButton (Ok) | `등고선 만들기` | None | — | Always | Validates file/points; calls `accept()` | `KaSurveyContourDialog.cpp:109-110,126-134` | `등고선 만들기` | None | — | Always | Sets `contour_empty_rejected` if empty, else sets fake guide message & closes | `dlg_survey_contour.rs:176-199` | no (fake result) |
| 14 | PushButton (Cancel) | `취소` | None | — | Always | Calls `QDialog::reject()` | `KaSurveyContourDialog.cpp:111,125` | `취소` | None | — | Always | Clears flag and closes | `dlg_survey_contour.rs:172-175` | yes |
| 15 | ProgressDialog (`contourProgress`) | `측량점으로 등고선을 계산하고 있습니다.` + `취소` button + progress bar (`0..100`) | None | `Qt::NonModal`, `0..100` | Always | Shows contour computation progress & cancels task | `MainWindowSurveyContour.cpp:132-137,191-195` | `MISSING` | — | — | — | — | — | MISSING |

### Keyboard & Closing
- **Default button**: C++ `QDialogButtonBox::Ok` (`등고선 만들기`). Rust none.
- **Enter / Esc**: C++ Enter triggers `등고선 만들기`; Esc triggers `reject()`. Rust ignores both.
- **Button box order**: C++ `등고선 만들기` | `취소`. Rust right-to-left `취소` | `등고선 만들기`.
- **Window X close**: C++ `reject()`. Rust `*open = false` → `app.modals.active = None`.

### Messages
- `좌표 파일을 고르세요.` (`KaSurveyContourDialog.cpp:129`) → Rust: identical (`dlg_survey_contour.rs:149`).
- `점 %1개, 표고 %2–%3 m. 등고선 약 %4줄.` (`KaSurveyContourDialog.cpp:227`) → Rust: fake hardcoded `점 248개, 표고 24.5–48.2 m. 등고선 약 {line_est}줄 (간격 {:.1} m). 의심점 3개 제외.` (`dlg_survey_contour.rs:154`).
- `줄이 많습니다. 간격을 넓히세요.` (`KaSurveyContourDialog.cpp:232`) → Rust: `MISSING`.
- `조사를 연 뒤에 등고선을 만들 수 있습니다.` (`MainWindowSurveyContour.cpp:108`) → Rust: `MISSING`.

---

## 10. TG — 「지형분석」 (`MainWindow::runDemHillshade`)

### Header
- **C++ title & location**: `지형분석` (`target/cpp-ref/src/app/MainWindow.cpp:1715`)
- **How opened (C++)**: Ribbon 「DEM」 dropdown menu → 「DEM 파일로 음영 만들기…」 (`MainWindowRibbon.cpp:414-415`) → `QFileDialog("DEM GeoTIFF")` (`MainWindow.cpp:1709-1713`) → `QDialog dlg` (`:1714-1740`).
- **How opened (Rust)**: `MISSING` — clicking 「DEM 파일로 음영 만들기…」 in `gui_ribbon_menu_actions.rs:85` opens `ActiveModalDialog::DemClass` (DC) instead of TG.
- **Modality**: C++ modal (`dlg.exec()` at `MainWindow.cpp:1739`). Rust: `MISSING`.
- **Size rules**: C++ default `QFormLayout` sizeHint, resizable `QDialog`. Rust: `MISSING`.
- **Rust counterpart**: `MISSING`.

### Controls

| # | kind | C++ text (verbatim) | C++ tooltip / placeholder (verbatim) | C++ default, range, step, suffix | C++ enabled-when rule | C++ what it does | C++ file:line | Rust text | Rust tooltip / placeholder | Rust default, range, step, suffix | Rust enabled-when rule | Rust what it does | Rust file:line | same? |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 1 | Form label + ComboBox | `방식` / `다중광원 음영 (유구·분묘)`, `단방향 음영` | None | Default index `0` (`Multi`) | Always | Selects `DemAnalyzer::HillshadeMode` | `MainWindow.cpp:1717-1719,1731` | `MISSING` | — | — | — | — | — | MISSING |
| 2 | Form label + DoubleSpinBox | `방위각` | None | Default `315`, range `0..360`, suffix ` °` | Always | Sets `opt.azimuthDeg` | `MainWindow.cpp:1720-1723,1732` | `MISSING` | — | — | — | — | — | MISSING |
| 3 | Form label + DoubleSpinBox | `고도각` | None | Default `45`, range `1..90`, suffix ` °` | Always | Sets `opt.altitudeDeg` | `MainWindow.cpp:1724-1727,1733` | `MISSING` | — | — | — | — | — | MISSING |
| 4 | Form label + DoubleSpinBox | `Z계수` | None | Default `1.0`, range `0.01..50` | Always | Sets `opt.zFactor` | `MainWindow.cpp:1728-1730,1734` | `MISSING` | — | — | — | — | — | MISSING |
| 5 | DialogButtonBox | `Ok` / `Cancel` | None | — | Always | Accepts or rejects dialog | `MainWindow.cpp:1735-1738` | `MISSING` | — | — | — | — | — | MISSING |

### Keyboard & Closing
- **C++**: `QDialogButtonBox::Ok | QDialogButtonBox::Cancel`; Enter accepts, Esc rejects.
- **Rust**: `MISSING`.

### Messages
- `음영기복을 만들지 못했습니다.` (`MainWindow.cpp:1757`), `음영 지도를 지도 화면에 올리지 못했습니다.` (`:1762`), `음영기복을 참조 지도에 올렸습니다.` (`:1772`) → Rust: `MISSING`.

---

## 11. DC — 「DEM 표현」 (`KaDemClassDialog`)

### Header
- **C++ title & location**: `DEM 표현` (`target/cpp-ref/src/app/KaDemClassDialog.cpp:21`)
- **How opened (C++)**: Ribbon 「DEM」 dropdown menu → 「DEM 표현…」 (`MainWindowRibbon.cpp:416-417`) → `MainWindow::editDemElevationClasses()` (`MainWindowOverlay.cpp:291-293`).
- **How opened (Rust)**: Ribbon 「DEM」 dropdown menu → 「DEM 표현…」 (or 「DEM 파일로 음영 만들기…」) (`gui_ribbon_menu_actions.rs:85-87`) → `ActiveModalDialog::DemClass`.
- **Modality**: C++ non-modal (`setModal(false)` at `KaDemClassDialog.cpp:22`, `dlg->show()` at `MainWindowOverlay.cpp:293`). Rust `egui::Window` overlay (`modal_style_demclass.rs:19`).
- **Size rules**:
  - **C++**: `setMinimumWidth(KaWindowSize::width(KaWindowSize::Small, 380))` (`KaDemClassDialog.cpp:23`), resizable `QDialog`.
  - **Rust**: `.resizable(false).fixed_size([modal_w, modal_h])` where `modal_w = 500.0.min(vis_w - 40.0)`, `modal_h = 430.0.min(vis_h - 40.0)` (`modal_style_demclass.rs:13-23`).
- **Rust counterpart**: `crates/strata-app/src/modal_style_demclass.rs:8` (`render_dem_class`).

### Controls

| # | kind | C++ text (verbatim) | C++ tooltip / placeholder (verbatim) | C++ default, range, step, suffix | C++ enabled-when rule | C++ what it does | C++ file:line | Rust text | Rust tooltip / placeholder | Rust default, range, step, suffix | Rust enabled-when rule | Rust what it does | Rust file:line | same? |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 1 | Label (hint) | `표고에 따라 색이 부드럽게 이어집니다. 유적처럼 높이차가 작은 곳은 「현재 화면에 색 맞추기」로 화면 안의 높이만으로 색을 나누세요.` | — | — | Always | Top guidance label | `KaDemClassDialog.cpp:25-27` | Identical verbatim text | — | — | Always | Static label | `modal_style_demclass.rs:30-36` | yes |
| 2 | EXTRA: Radio buttons (`DEM 모드`) | — | — | — | — | — | — | `DEM 모드`: `등고`, `음영`, `고도` | None | `dem_mode_idx` | Always | Sets `dem_mode_idx` & `dem_applied_mode_idx` | `modal_style_demclass.rs:44-55` | EXTRA |
| 3 | Form label + ComboBox | `색 표현` / `전국 표준 (0–2000m)`, `저지대 강조`, `현재 화면 맞춤` | None | Default from layer property or `0` (`national`) | Always | Selects colour ramp preset (`m_preset`) | `KaDemClassDialog.cpp:29-35,44` | `색 표현` / `전국 표준 (0–2000m)`, `저지대 강조`, `현재 화면 맞춤` | None | `dem_preset_idx` | Always | Sets `dem_preset_idx` | `modal_style_demclass.rs:58-77` | yes |
| 4 | PushButton | `현재 화면에 색 맞추기` | Tooltip: `지금 지도 화면에 보이는 가장 낮은 곳과 높은 곳으로 색띠를 다시 나눕니다. 지도를 옮기면 따라 바뀝니다.` | — | Enabled when `canvas != nullptr` | Calls `fitToView()` (sets `viewport` preset & applies) | `KaDemClassDialog.cpp:36-40,70,89-92` | `현재 화면에 색 맞추기` | None | — | Always | Sets preset=2, mode=2, fake status range `32.4 m ~ 74.8 m` | `modal_style_demclass.rs:78-84` | no (tooltip, fake range) |
| 5 | EXTRA: Painted colour ramp preview | — | — | — | — | — | — | `색 띠 미리보기` + painted 24-step bar | — | — | Always | Paints colour ramp preview | `modal_style_demclass.rs:89-163` | EXTRA |
| 6 | CheckBox | `지형 음영 합성` | None | Default `true` (or layer property) | Always | Toggles hillshade blend (`m_relief`) | `KaDemClassDialog.cpp:45-48` | `지형 음영 합성` | None | `dem_relief_enabled` | Always | Toggles `dem_relief_enabled` | `modal_style_demclass.rs:167` | yes |
| 7 | Form label + DoubleSpinBox | `수직과장` | None | Default `1.0`, range `0.1..5.0`, step `0.1`, suffix ` 배` | Always | Sets Z-factor (`m_exaggeration`) | `KaDemClassDialog.cpp:49-55` | `수직과장` + `DragValue` | None | Range `0.1..=5.0`, speed `0.1`, suffix ` 배` | Always | Edits `dem_exaggeration` | `modal_style_demclass.rs:171-178` | yes |
| 8 | Form label + SpinBox | `음영 세기` | None | Default `30`, range `0..80`, suffix ` %` | Always | Sets relief strength (`m_strength`) | `KaDemClassDialog.cpp:56-61` | `음영 세기` + `DragValue` | None | Range `0..=80`, speed `5.0`, suffix ` %` | Always | Edits `dem_relief_strength_pct` | `modal_style_demclass.rs:181-188` | yes |
| 9 | Label (status) | `m_status` (populated by `suggestViewport` or `applyStyle`) | — | Empty or viewport hint | Always | Shows status/error text | `KaDemClassDialog.cpp:62-65` | `&app.modals.dem_status_text` | — | — | Always | Shows `dem_status_text` | `modal_style_demclass.rs:193-196` | yes |
| 10 | PushButton (Apply) | `적용` | None | — | Always | Calls `applyStyle()` | `KaDemClassDialog.cpp:66-69` | `적용` | None | — | Always | Copies draft to applied, shows `지형 음영` layer, sets status | `modal_style_demclass.rs:202-220` | yes |
| 11 | PushButton (Close) | `닫기` | None | — | Always | Calls `QDialog::close()` | `KaDemClassDialog.cpp:66,68,71` | `취소` (plus EXTRA `확인` button) | None | — | Always | `확인` applies & closes; `취소` closes without applying | `modal_style_demclass.rs:221-254` | no (buttons `확인`/`취소` vs `닫기`) |

### Keyboard & Closing
- **Default button**: C++ `QDialogButtonBox::Close` / `Apply`. Rust none.
- **Enter / Esc**: C++ Esc closes (`QDialog::close()`). Rust ignores both.
- **Button box order**: C++ `적용` | `닫기`. Rust left `적용`, right-to-left `취소` | `확인`.
- **Window X close**: C++ `close()`. Rust `*open = false` → `app.modals.active = None`.

### Messages
- `먼저 DEM을 켜 주세요. 표고 자료를 불러오면 색 표현과 음영을 조절할 수 있습니다.` (`MainWindowOverlay.cpp:288`) → Rust: `MISSING` (opens without DEM).
- `지금 화면의 높이차는 약 %1 m뿐이라 이 색 표현으로는 거의 한 색으로 보입니다. 「현재 화면에 색 맞추기」를 누르면 화면 안의 높이로 색을 나눕니다.` (`KaDemClassDialog.cpp:104-106`) → Rust: `MISSING`.
- `지도 화면에서 DEM 표현을 다시 열어 주세요.` (`:114`), `화면 좌표를 변환하지 못했습니다. 작업 좌표계를 확인하세요.` (`:116`), `이 화면에서 표고값을 읽지 못했습니다. DEM이 있는 곳으로 이동한 뒤 적용하세요.` (`:120`) → Rust: `MISSING`.

---

## 12. GT — 「GeoTIFF 저장」 progress (`MainWindowExport.cpp:80`)

### Header
- **C++ title & location**: `GeoTIFF 저장` (`target/cpp-ref/src/app/MainWindowExport.cpp:78-84`)
- **How opened (C++)**: Ribbon 「GeoTIFF」 button (`MainWindowRibbon.cpp:567-569`) → `MainWindow::exportMapGeoTiff()` (`MainWindowExport.cpp:57`) → `QFileDialog("현재 지도 GeoTIFF 저장")` (`:68-75`) → `QProgressDialog` (`:78-84`).
- **How opened (Rust)**: `MISSING` — ribbon 「GeoTIFF」 is deferred (`gui_ribbon_click_b.rs:145`, tooltip `아직 Rust 판에서 연결하지 않았습니다 (지시 R7)` at `gui_ribbon_draw.rs:61`).
- **Modality**: C++ `Qt::ApplicationModal` (`MainWindowExport.cpp:81`). Rust: `MISSING`.
- **Size rules**: C++ `QProgressDialog` default sizeHint. Rust: `MISSING`.
- **Rust counterpart**: `MISSING`.

### Controls

| # | kind | C++ text (verbatim) | C++ tooltip / placeholder (verbatim) | C++ default, range, step, suffix | C++ enabled-when rule | C++ what it does | C++ file:line | Rust text | Rust tooltip / placeholder | Rust default, range, step, suffix | Rust enabled-when rule | Rust what it does | Rust file:line | same? |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 1 | Label | `현재 지도를 GeoTIFF로 저장하고 있습니다…` | — | — | Always | `QProgressDialog` label | `MainWindowExport.cpp:78` | `MISSING` | — | — | — | — | — | MISSING |
| 2 | ProgressBar | `QProgressDialog` bar | — | Range `0..0` (indeterminate) | Always | Busy indicator | `MainWindowExport.cpp:79` | `MISSING` | — | — | — | — | — | MISSING |
| 3 | PushButton (Cancel) | `취소` | None | — | Always | Sets `progress.wasCanceled()` | `MainWindowExport.cpp:79,87` | `MISSING` | — | — | — | — | — | MISSING |

### Keyboard & Closing
- **C++**: `Qt::ApplicationModal` `QProgressDialog`; Esc or `취소` sets `wasCanceled()`.
- **Rust**: `MISSING`.

### Messages
- `지도에 저장할 레이어가 없습니다.` (`MainWindowExport.cpp:65`), `GeoTIFF 저장 완료 · %1 · %2` (`:90-91`), `GeoTIFF 저장을 취소했습니다.` (`:93`), `GeoTIFF 저장 실패` / `GeoTIFF를 저장하지 못했습니다` (`:95`) → Rust: `MISSING`.

---

## Differences to fix

Most visible to the user first:

1. **TG 「지형분석」**: Entire dialog & flow missing; clicking 「DEM 파일로 음영 만들기…」 opens `DemClass` instead (`MainWindow.cpp:1715-1739` -> `gui_ribbon_menu_actions.rs:85-87`).
2. **GT 「GeoTIFF 저장」**: Progress dialog and export flow missing (`MainWindowExport.cpp:78-84` -> `gui_ribbon_click_b.rs:145`).
3. **DP2 (<title> 내려받기)**: `토양도`, `지질도`, `수계도`, `오프라인 지도` download progress windows missing; ribbon clicks only toggle visibility (`MainWindowDownloads.cpp:22-33`, `MainWindowOverlay.cpp:612` -> `gui_ribbon_click_b.rs:38-85`).
4. **SC 「측량 등고선」 progress**: Non-modal `contourProgress` `QProgressDialog` missing (`MainWindowSurveyContour.cpp:132-137` -> `MISSING`).
5. **SC 「측량 등고선」 file picker & fake preview**: `찾기…` button is dead and preview fabricates `점 248개, 표고 24.5–48.2 m` (`KaSurveyContourDialog.cpp:31,142-147,227-230` -> `dlg_survey_contour.rs:43,154`).
6. **HB 「주변유적 받기」 fetch button**: `받기` is permanently disabled (`DEFERRED_WB1`), embedded webview (`m_tabs`) missing (`KaHeritageBrowser.cpp:252-257` -> `dlg_heritage_browser_footer.rs:66-78`).
7. **TB 「수치지형도 내려받기」**: `도엽 확인` disabled (`DEFERRED_WB2`), embedded webview missing, and `m_downloads` table shows 3 hardcoded fake rows (`KaTopographicBrowser.cpp:172-200` -> `ngii_topographic_state.rs:30-31`, `dlg_topo_browser_body.rs:123-140`).
8. **HA / TA account dialogs**: Separate `KaHeritageAccountDialog` and `KaTopographicAccountDialog` merged into `AccountSettings`, which lacks HA's description (`KaHeritageSetupDialogs.cpp:32-34` -> `modal_paper_account.rs:252-258`) and uses `닫기` instead of `취소` (`KaHeritageSetupDialogs.cpp:62` -> `modal_paper_account.rs:267`).
9. **HR 「주변유적 받을 시·군 선택」**: `addNearby` 5 km neighbouring 시·군 group, `m_reuseWhen` timestamp, and `m_reuse` tooltip missing; decorative `ka_modal_heritage_bg` window has dead `상세 보기`/`취소` buttons (`KaHeritageSetupDialogs.cpp:124-131,174-228` -> `dlg_heritage_region_select.rs:13-70,140-142`).
10. **TI 「수치지형도 — 좌표계 확인과 지도 추가」**: Real `QgsMapCanvas` preview replaced by schematic box, `선택 도엽 정합 미리보기` / `지도에 추가` are stubs, and cell tooltips missing (`KaTopographicImportDialog.cpp:304-306,471-474,498-522` -> `dlg_topo_import_preview.rs:36-105`, `dlg_topo_import_actions.rs:32-97`).
11. **DC 「DEM 표현」**: Opens without DEM layer, `현재 화면에 색 맞추기` uses fake `32.4 m ~ 74.8 m` range and lacks tooltip, bottom buttons are `[적용] [확인] [취소]` instead of `[적용] [닫기]`, and `DEM 모드` + `색 띠 미리보기` are EXTRA (`KaDemClassDialog.cpp:38-39,66-71` -> `modal_style_demclass.rs:44-163,200-254`).
12. **SC 「측량 등고선」 controls**: `의심점 빼기` and `단절선` are always enabled instead of disabled until detected, `파일 좌표계` is a cycling button instead of ComboBox, and 5 tooltips are missing (`KaSurveyContourDialog.cpp:45-100,190-199` -> `dlg_survey_contour.rs:63-139`).
13. **DP1 「지적도 다운로드」**: `취소` button disabled when idle and does not close dialog; `다운로드 중에도 지도 작업을 계속할 수 있습니다.` hint missing; extra `지적도 받기` / `계정 설정` / `닫기` buttons added (`KaDownloadUi.cpp:51-55` -> `cadastral_download_ui_render.rs:115-156`).
14. **Keyboard (Enter / Esc) across all 12 windows**: None of the Rust dialogs handle `Key::Enter` (default button) or `Key::Escape` (reject/close), whereas all C++ `QDialog`s do.
15. **Window resizability**: All Rust modal windows use `.resizable(false)` (and fixed sizes on HB, TI, SC, DC), whereas C++ dialogs are resizable.

---

## Could not determine

- **DP2 (오프라인 지도)**: Searched `crates/strata-app/src` for `오프라인 지도` and `download_jobs.`; no Rust UI trigger or dialog for offline map downloading was found (`download_jobs` is only wired to `apply_snap_preset` and unit tests).
- **TI (`topo_blocking_message`)**: Looked in `dlg_topo_import_actions.rs:67`; the implementation of `topo_blocking_message()` in `modal_controller_impl.rs` was not audited line-by-line for every error string variant.

---

## Counts

- **Windows audited**: 12 (HB, HA, HR, TA, TB, TI, DP1, DP2, SC+progress, TG, DC, GT)
- **C++ controls**: 101 (HB: 10, HA: 6, HR: 9, TA: 6, TB: 20, TI: 10, DP1: 7, DP2: 3, SC: 13, TG: 5, DC: 9, GT: 3)
- **Rust controls**: 105 (82 matching/modified + 23 EXTRA)
  - `EXTRA` by window: HB: 6, HA: 1, HR: 1, TA: 0, TB: 2, TI: 1, DP1: 5, DP2: 3, SC: 2, TG: 0, DC: 2, GT: 0
- **Differences by `same?` column** (across the 101 C++ controls + 23 EXTRA = 124 rows):
  - `yes`: 39 (including `yes (for DEM)` on DP2 row 2)
  - `no`: 43
  - `MISSING`: 19
  - `EXTRA`: 23
- **Verification**: Recounted all 12 tables against source (`39 + 43 + 19 = 101` C++ controls; `30 / 30` random citations verified exact).
