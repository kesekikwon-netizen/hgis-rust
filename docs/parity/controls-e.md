# Main Window Parts (Outside Ribbon) — C++ vs Rust Control-by-Control Parity

Default C++ display state is `paperLook = true` and `highContrast = true` (`KaThemeOptions.cpp:100-104`). Comparisons below use this default C++ state ("새 모양") unless noted otherwise.

---

## 1. Home screen (`홈`)

- **C++ title (verbatim) & where built**: No standalone window title; embedded `KaStartPage` widget inside `MainWindow` `m_viewTabs` tab `"홈"` (`KaStartPage.cpp:74`, `MainWindow.cpp:224`).
- **How user opens it**:
  - **C++**: Shown on startup as tab index 0 (`MainWindow.cpp:224`, `KaStartPage.cpp:74`); clicking the `"홈"` tab (`MainWindow.cpp:1084-1091`) switches back and calls `m_startPage->reload()`.
  - **Rust**: Shown on startup (`MainTab::Home` default); clicking `"홈"` tab in ribbon tab bar (`gui_ribbon_tabbar.rs:76,87`).
- **Modality**: Modeless embedded page on both sides (`QWidget` inside `QTabWidget` in C++, `ScrollArea`/egui panel in Rust).
- **Size rules**:
  - **C++**: `KaStartHero` `setMinimumHeight(220)` (`KaStartHero.cpp:12`); hero height switches between `kHeroCompact = 184` and `kHeroTall = 236` at `kCompactWindowHeight = 820` (`KaStartPage.cpp:35-38,314-319`); right column `QScrollArea` width `kRightNarrow = 340` (`< 1500` px) or `kRightWide = 400` (`>= 1500` px) (`KaStartPage.cpp:32-34,307-313`).
  - **Rust**: Full available width inside `CentralPanel` with `ScrollArea::vertical()`; max content width `1380.0` (`gui_tab_home.rs:14-18`), hero card `min_height` `172.0` (`gui_tab_home_hero.rs:42`), two columns `3 : 2` (`gui_tab_home.rs:28-56`).
- **Rust counterpart**: `crates/strata-app/src/gui_tab_home.rs:11-58`, `gui_tab_home_hero.rs`, `gui_tab_home_recent.rs`, `gui_tab_home_recent_box.rs`, `gui_tab_home_status.rs`.

### Controls

| # | kind | C++ text (verbatim) | C++ tooltip / placeholder (verbatim) | C++ default, range, step, suffix | C++ enabled-when rule | C++ what it does | C++ file:line | Rust text | Rust tooltip / placeholder | Rust default, range, step, suffix | Rust enabled-when rule | Rust what it does | Rust file:line | same? |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 1 | Hero header labels | `Strata` / `필드고고학 GIS` / `오늘 현장을 새로 만들거나, 지난 조사를 이어서 여세요.` | — | — | Always visible | Displays app title, subtitle, and hero lead sentence | `KaStartPage.cpp:89-99` | `Strata` / `필드고고학 GIS` / `오늘 현장을 새로 만들거나, 지난 조사를 이어서 여세요.` | — | — | Always visible | Displays app title, subtitle, and hero lead sentence | `gui_tab_home_hero.rs:64-97` | yes |
| 2 | Primary CTA button | `새 조사` | — | — | Always enabled | Emits `newSurveyRequested()` to create a new survey | `KaStartPage.cpp:89-160` | `새 조사` | — | — | Always enabled | Calls `open_new_survey_dialog()` | `gui_tab_home_hero.rs:102-116` | yes |
| 3 | Secondary CTA button | `조사 열기` | — | — | Always enabled | Emits `openSurveyRequested()` to open survey picker | `KaStartPage.cpp:89-160` | `조사 열기` | — | — | Always enabled | Calls `open_project_dialog()` | `gui_tab_home_hero.rs:117-129` | yes |
| 4 | Continue block (`KaHomeTitleBlock`) | `이어서 작업` / `<name>` / `● 저장 안 됨` or `저장됨` | — | — | Visible when at least one recent survey exists; hidden if `m_recent.isEmpty()` | Shows most recent survey name, save state, folder, last opened, CRS, counts, inspection summary | `KaHomeTitleBlock.cpp:25-149`, `KaStartPage.cpp:224-230` | `이어서 작업` / `<name>` / `저장됨` | — | — | Visible when `!recent_projects.isEmpty()` | Shows most recent survey name and metadata, but hardcodes `"저장됨"` | `gui_tab_home_hero.rs:137-205` | no (text: hardcodes `저장됨`, never shows `● 저장 안 됨`; `KaHomeTitleBlock.cpp:133` vs `gui_tab_home_hero.rs:155`) |
| 5 | Continue action button | `이어서 열기  →` | — | — | Enabled when visible (`!m_recent.isEmpty()`) | Emits `openRecentRequested(m_path)` (opens recovery copy if user confirms or opens project) | `KaHomeTitleBlock.cpp:25-149` | `이어서 열기 →` | — | — | Always enabled when shown | Calls `load_project_from_path()` on most recent project | `gui_tab_home_hero.rs:163-173` | no (text: single space before `→` vs two spaces in C++; `KaHomeTitleBlock.cpp` vs `gui_tab_home_hero.rs:164`) |
| 6 | Recovery notice banner links | `저장하지 않은 편집의 복구 사본이 있습니다(%1). <a href="open">사본 열기</a> · <a href="folder">폴더 열기</a> · <a href="hide">숨기기</a>` | `사본에는 저장하지 않은 편집이 있던 레이어가 들어 있습니다. 원래 조사 파일은 바뀌지 않았습니다.` | — | Visible only when most recent survey has a newer recovery snapshot | Opens recovery copy, opens folder in Explorer, or hides recovery banner | `KaStartPage.cpp:124-129,233-253` | MISSING | MISSING | MISSING | MISSING | MISSING | MISSING | MISSING |
| 7 | Recent card header & count | `최근 조사` / `%1개` | — | — | Always visible | Displays section title and count of recent surveys | `KaHomeRecentCard.cpp:54-60,159` | `최근 조사` / `<N>개` | — | — | Always visible | Displays section title and count of recent surveys | `gui_tab_home_recent_box.rs:23-39` | yes |
| 8 | Recent filter input | `""` | Placeholder: `이름·폴더로 찾기` | `clearButtonEnabled = true`, width `260` | Always enabled | Filters recent survey rows by name or folder path | `KaHomeRecentCard.cpp:61-65` | `self.home_map.recent_filter` | Placeholder: `     이름·폴더로 찾기` | No clear button, width `260.0` | Always enabled | Filters recent survey rows by name or path | `gui_tab_home_recent.rs:34-38` | no (placeholder/clear button: 5 leading spaces in placeholder & no clear button; `KaHomeRecentCard.cpp:62-63` vs `gui_tab_home_recent.rs:35-38`) |
| 9 | Recent surveys table (6 columns in `paperLook`) | Headers: `조사`, `상태`, `조사구역`, `유구`, `검수`, `마지막 열림` | Cell tooltips: full name/folder, state tooltip, full date | Single row selection, read-only, double-click opens, `CustomContextMenu` | Always enabled; replaced by `아직 최근 조사가 없습니다. 새 조사 또는 조사 열기로 시작하세요.` when empty | Lists recent projects with live `SurveyFacts`; double-click opens project | `KaHomeRecentCard.cpp:69-85,169-212` | Headers: `조사`, `상태`, `조사구역`, `유구`, `검수`, `마지막 열림` | — | Hover + click/double-click opens, right-click opens context menu | Always enabled; shows `아직 최근 조사가 없습니다. 새 조사 또는 조사 열기로 시작하세요.` when empty | Lists recent projects (`상태` always `"저장됨"`); double-click or click opens | `gui_tab_home_recent_box.rs:42-159` | no (state column always `"저장됨"`, no tooltips, single click also opens; `KaHomeRecentCard.cpp:176-184` vs `gui_tab_home_recent_box.rs:97,138`) |
| 10 | Recent table context menu | `복구 사본 열기`, `복구 사본 폴더 열기`, `복구 사본 표시 숨기기`, `목록에서 제거` | — | — | Recovery items shown only if recovery exists | Opens/reveals/hides recovery copy or removes entry from recent list | `KaHomeRecentCard.cpp:214-237` | `목록에서 제거` | — | — | Always shown on right-click | Removes entry from recent list; recovery items MISSING | `gui_tab_home_recent_box.rs:144-154` | no (missing 3 recovery items; `KaHomeRecentCard.cpp:222-229` vs `gui_tab_home_recent_box.rs:145`) |
| 11 | Guide card (`작업 순서`) | `작업 순서` / `%1 기준` / Steps: `새 조사`, `조사구역 그리기`, `도면 · 제출` / `제출 준비: 오류 %1 · 주의 %2` | — | — | Always visible; step progress computed from `SurveyFacts` | Displays 3-step guide (`다음` chip on active step) and submission readiness summary | `KaHomeGuideCard.cpp:54-194` | `작업 순서` / `현재 조사 기준` / Steps: `새 조사`, `조사구역 그리기`, `도면 · 제출` / `제출 전 검수: ...` | — | Step states hardcoded (`1` done, `2` current, `3` pending) | Step titles are clickable links (`open_new_survey_dialog`, Map tab, Drawing tab) | `gui_tab_home_recent.rs:48-137` | no (default/what it does: hardcoded steps vs dynamic `SurveyFacts`, clickable links vs plain labels; `KaHomeGuideCard.cpp:161-194` vs `gui_tab_home_recent.rs:72-104`) |
| 12 | Connection card header | `연결 상태` / `비밀값은 표시되지 않습니다` | — | — | Always visible | Displays connection card header and security note | `KaHomeConnectionCard.cpp:64-73` | `연결 상태` / `비밀값은 표시되지 않습니다` | — | — | Always visible | Displays connection card header and security note | `gui_tab_home_status.rs:25-33` | yes |
| 13 | Internet probe chip & button | `인터넷 · 확인 안 함` (or `확인 중…` / `연결됨` / `연결 안 됨`) / `인터넷 확인` | Button tooltip: `VWorld 서버에 한 번 접속해 봅니다. 누를 때만 확인합니다.` | — | Button disabled while probe is running (`!probing`) | Probes VWorld server on click and updates status chip | `KaHomeConnectionCard.cpp:76-94,160-179` | `인터넷 · 확인 안 함` (or `확인 중…` / `연결됨` / `연결 안 됨`) / `인터넷 확인` | Button tooltip: `VWorld 서버에 한 번 접속해 봅니다. 누를 때만 확인합니다.` | — | Button disabled when probing | Probes VWorld server and updates chip | `gui_tab_home_status.rs:41-60`, `home_connection_model.rs:34-44` | yes |
| 14 | Account rows (5 rows) | `VWorld 키`, `역사지리정보DB 키`, `VWorld 지적도 계정`, `수치지형도 계정`, `문화재 인트라넷` + `설정됨`/`● 설정 필요` + `설정` | `설정` button tooltip: `entry.menuPath` | — | Always enabled | Opens corresponding settings/account dialog (`openSettingRequested(key)`) | `KaHomeConnectionCard.cpp:96-119,181-198` | `VWorld 키`, `역사지리정보DB 키`, `VWorld 지적도 계정`, `수치지형도 계정`, `문화재 인트라넷` + `설정됨`/`● 설정 필요` + `설정` | `설정` button tooltip: `entry.menu_path` | — | Always enabled | Dispatches `HomeConnectionAction` to open the settings dialog | `gui_tab_home_status.rs:64-97`, `home_connection_model.rs:46-95` | yes |

### Keyboard and closing behaviour
- **C++**: Embedded tab page; Tab order follows widget creation order (`새 조사`, `조사 열기`, `이어서 열기`, `m_filter`, `m_table`, `인터넷 확인`, 5x `설정` buttons). No Enter/Esc closing behaviour (not a dialog). `m_filter` has clear button (`KaHomeRecentCard.cpp:63`).
- **Rust**: Embedded tab page; standard egui Tab focus traversal. `home_recent_filter` (`TextEdit::singleline`) has no clear button (`gui_tab_home_recent.rs:34-38`). Not closable.

### Messages
| Condition | C++ verbatim + `file:line` | Rust text + `file:line` |
|---|---|---|
| No recent surveys | `아직 최근 조사가 없습니다. 새 조사 또는 조사 열기로 시작하세요.` (`KaHomeRecentCard.cpp:80-81`) | `아직 최근 조사가 없습니다. 새 조사 또는 조사 열기로 시작하세요.` (`gui_tab_home_recent_box.rs:17-18`) |
| Recent footer hint | `목록에서 빼려면 조사 위에서 오른쪽 단추를 누르세요.` (`KaHomeRecentCard.cpp:91`) | `목록에서 빼려면 조사 위에서 오른쪽 단추를 누르세요.` (`gui_tab_home_recent.rs:146-147`) |
| No survey open in Guide card | `조사를 열면 현재 상태가 표시됩니다.` (`KaHomeGuideCard.cpp:188`) | `조사를 열면 검수 상태가 표시됩니다.` (`gui_tab_home_recent.rs:116`) |
| Unsaved recovery copy banner | `저장하지 않은 편집의 복구 사본이 있습니다(%1). 사본 열기 · 폴더 열기 · 숨기기` (`KaStartPage.cpp:240-243`) | `MISSING` |

---

## 2. Tab bar and window title

- **C++ title (verbatim) & where built**: `MainWindow::setWindowTitle("Strata")` (`MainWindow.cpp:176`), updated by `MainWindow::refreshWindowTitle()` (`MainWindowSession.cpp:1327-1334`) via `KaSurveyBadge::windowTitleFor(m_title, m_dirty)` (`KaSurveyBadge.cpp:136-143`): `<name>[ *] - Strata` when a project is open, `Strata[ *]` otherwise. Tab bar is `m_viewTabs` (`QTabWidget` with `documentMode = true`, `tabsClosable = true`, `movable = false`) (`MainWindow.cpp:218-241`) and corner widget `viewTabsCorner` (`MainWindowChrome.cpp:137-166`).
- **How user opens it**:
  - **C++**: Always visible at top of `MainWindow` (`MainWindow.cpp:218-241`).
  - **Rust**: Always visible at top of window (`gui_ribbon_tabbar.rs:73-282`); window title updated via `ViewportCommand::Title` in `app_state_lifecycle.rs:125-160`.
- **Modality**: Non-modal main window chrome on both sides.
- **Size rules**:
  - **C++**: `MainWindow` initial `resize(1280, 900)` (`MainWindow.cpp:183`); `m_viewTabs` expands horizontally (`MainWindow.cpp:218`); corner bar holds `m_surveyBadge` + `테마 ▾` (`MainWindowChrome.cpp:142-165`); `KaTabClose` button `20x20` (`KaTabClose.cpp:43`).
  - **Rust**: Full window width top tab strip inside `TopBottomPanel::top` (`gui_ribbon_tabbar.rs:73-282`); tab strip height `32.0` (`gui_ribbon_tabbar.rs:74`); right corner holds survey badge + `테마 ▼` (`gui_ribbon_tabbar.rs:236-281`).
- **Rust counterpart**: `crates/strata-app/src/gui_ribbon_tabbar.rs:73-283`, `crates/strata-app/src/app_state_lifecycle.rs:125-160`.

### Controls

| # | kind | C++ text (verbatim) | C++ tooltip / placeholder (verbatim) | C++ default, range, step, suffix | C++ enabled-when rule | C++ what it does | C++ file:line | Rust text | Rust tooltip / placeholder | Rust default, range, step, suffix | Rust enabled-when rule | Rust what it does | Rust file:line | same? |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 1 | Window title | `<name>[ *] - Strata` or `Strata[ *]` | — | Default: `Strata` | Always set on `MainWindow` | Shows active project name and dirty `*` marker | `MainWindow.cpp:176`, `MainWindowSession.cpp:1327-1334`, `KaSurveyBadge.cpp:136-143` | `<name>[ *] - Strata` or `Strata[ *]` | — | Default: `Strata` | Updated on project/dirty state change | Shows active project name and dirty `*` marker | `app_state_lifecycle.rs:125-160` | yes |
| 2 | Home tab | `홈` | — | Default active tab on launch; not closable | Always enabled | Switches to `KaStartPage` and refreshes home data | `MainWindow.cpp:224,230-231,1084-1091` | `홈` | — | Default active tab; not closable | Always enabled | Switches to `MainTab::Home` | `gui_ribbon_tabbar.rs:76,87` | yes |
| 3 | Map tab | `지도` | — | Not closable | Always enabled | Switches to map canvas workspace | `MainWindow.cpp:225,230-231,1093` | `지도` | — | Not closable | Always enabled | Switches to `MainTab::Map` | `gui_ribbon_tabbar.rs:77,88` | yes |
| 4 | Dynamic closable tabs | `도면`, `입체지형`, `입체지형 도면`, or topographic browser | Close button tooltip: `탭 닫기` (`KaTabClose.cpp:45`) | Added on demand; closable via `KaTabClose` | Always enabled when open | Switches to respective workspace tab; close button closes tab | `MainWindowSession.cpp:645-665`, `KaTabClose.cpp:35-63` | `도면`, `고지형` (`MainTab::Paleo`), `입체지형`, `입체지형 도면` | Close `×` has no tooltip | `도면` and `고지형` always shown/closable? (`close_tab` removes dynamic tabs) | Always enabled | Switches to tab; `×` closes closable tab | `gui_ribbon_tabbar.rs:78-115` | no (EXTRA `고지형` tab; missing close button tooltip `탭 닫기`; `KaTabClose.cpp:45` vs `gui_ribbon_tabbar.rs:81,107-113`) |
| 5 | Survey badge (`KaSurveyBadge`) | `<name> ● 저장 안 됨` or `<name> 저장됨` (in `paperLook`) | `열려 있는 조사입니다. 누르면 지도 탭으로 갑니다.` | Hidden when no project open | Always enabled & Tab-focusable when visible | Clicking (or Space/Enter) switches to `지도` tab | `KaSurveyBadge.cpp:26,105-135`, `MainWindowChrome.cpp:154-159` | `<name> ● 저장 안 됨` or `<name> 저장됨` | `열려 있는 조사입니다. 누르면 지도 탭으로 갑니다.` | Hidden when no project open | Non-interactive `ui.label` | Displays text and hover tooltip only; clicking does NOT switch to Map tab | `gui_ribbon_tabbar.rs:272-279` | no (what it does: non-clickable label vs clickable button to Map tab; `KaSurveyBadge.cpp:133` vs `gui_ribbon_tabbar.rs:274`) |
| 6 | Theme button & menu (`테마 ▾`) | `테마 ▾` | `화면 색을 고릅니다. 지도와 저장·내보내는 파일의 색은 바뀌지 않습니다` | 4 color themes (`종이` default, etc.) | Always enabled | Opens menu of 4 color themes only | `KaThemeOptions.cpp:157-188`, `MainWindowChrome.cpp:160-165` | `테마 ▼` | `화면 색을 고릅니다. 지도와 저장·내보내는 파일의 색은 바뀌지 않습니다` | Color themes + `새 모양`, `고대비 화면`, `큰 글씨`, sound level options | Always enabled | Opens popup mixing color themes, display toggles, and sound options | `gui_ribbon_tabbar.rs:150-245` | no (what it does: mixes display toggles & sound into `테마 ▼` menu vs C++ separate `화면` options; `KaThemeOptions.cpp:168-181` vs `gui_ribbon_tabbar.rs:150-230`) |

### Keyboard and closing behaviour
- **C++**: `KaSurveyBadge` accepts `Qt::TabFocus` and emits `clicked()` on Space/Return (`KaSurveyBadge.cpp:26,133`). `QTabBar` supports arrow keys when focused; `KaTabClose` buttons close dynamic tabs. Closing main window checks dirty state before exit.
- **Rust**: Tab strip buttons use standard egui click/focus; `KaSurveyBadge` is not focusable/clickable (`gui_ribbon_tabbar.rs:274`).

### Messages
- No direct validation/error messages inside the tab bar itself; dirty prompt on close is handled by `MainWindow` close handler (outside tab bar).

---

## 3. Status bar

- **C++ title (verbatim) & where built**: `KaStatusBar` installed as `MainWindow::statusBar()` (`MainWindow.cpp:242-255`, `KaStatusBar.cpp:16-148`).
- **How user opens it**:
  - **C++**: Always visible at bottom of `MainWindow` (`MainWindow.cpp:242-255`).
  - **Rust**: Always visible at bottom of `shell_workspace.rs` (`TopBottomPanel::bottom`) (`shell_workspace.rs:37-253`).
- **Modality**: Modeless status bar on both sides.
- **Size rules**:
  - **C++**: `KaStatusBar` height fits contents; `xyReadout` `setMinimumWidth(240)` (`KaStatusBar.cpp:62`); `scaleCombo` `setMinimumWidth(120)` (`KaStatusBar.cpp:79`).
  - **Rust**: Fixed height `28.0` (`shell_workspace.rs:39`).
- **Rust counterpart**: `crates/strata-app/src/shell_workspace.rs:37-253`, `crates/strata-app/src/status_bar_text.rs:1-284`, `crates/strata-app/src/app_state_status_bar.rs:1-122`.

### Controls

| # | kind | C++ text (verbatim) | C++ tooltip / placeholder (verbatim) | C++ default, range, step, suffix | C++ enabled-when rule | C++ what it does | C++ file:line | Rust text | Rust tooltip / placeholder | Rust default, range, step, suffix | Rust enabled-when rule | Rust what it does | Rust file:line | same? |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 1 | Status hint label (`m_nextAction`) | Dynamic text (`updateNextActionStatus`) | — | Default: `""` | Always visible | Shows next recommended action or brief status message | `MainWindow.cpp:393-413`, `KaStatusBar.cpp:42-46` | Dynamic text via `status_bar_text` | — | Default: `""` | Always visible | Shows next action or brief status message | `shell_workspace.rs:42-51,126-135`, `status_bar_text.rs` | yes |
| 2 | Snap chip (`snapChip`) | `자석 끔` / `자석 켬` | `그리기 도구 줄의 자석 설정에서 바꿉니다.` | Default: `자석 끔` | Visible on map tab (`onMap`) | Displays whether snapping is enabled | `KaStatusBar.cpp:48-52,155-158,217` | MISSING | MISSING | MISSING | MISSING | MISSING | MISSING | MISSING |
| 3 | Unsaved chip (`unsavedChip`) | `저장 안 됨 %1건` or `저장 안 됨` | `아직 파일에 쓰지 않은 편집입니다. 「저장」(Ctrl+S)으로 씁니다.` | Hidden when clean | Visible when project has unsaved edits | Displays count of unsaved edits | `KaStatusBar.cpp:54-58,160-169` | `저장 안 됨 <N>건` or `저장 안 됨` | `아직 파일에 쓰지 않은 편집입니다. 「저장」(Ctrl+S)으로 씁니다.` | Hidden when clean | Visible when dirty | Displays count of unsaved edits | `shell_workspace.rs:144-156` | yes |
| 4 | Coordinate readout (`xyReadout`) | `X —    Y —` or `X <x>  Y <y>` | `지도 위 커서의 작업 좌표입니다.` | Min width `240` | Visible on map tab (`onMap`) | Displays cursor coordinates in current work CRS | `KaStatusBar.cpp:60-66,171-180,217` | `X —    Y —` or `X <x>  Y <y>` | `지도 위 커서의 작업 좌표입니다.` | Auto width | Visible on map tab | Displays cursor coordinates in current work CRS | `shell_workspace.rs:141-143,240-250` | yes |
| 5 | Scale label & combo (`scaleLabel` / `scaleCombo`) | `축척` + `1:25000` (editable combo) | Placeholder: `예: 1000`; Tooltip: `축척을 고르거나 직접 입력하고 Enter를 누르세요. 1000 · 1:1000 둘 다 됩니다.` | Default `1:25000`; 13 presets (`100`..`500000`), minWidth `120` | Visible on map tab (`onMap`) | Selects preset or parses typed scale on Enter and zooms canvas | `KaStatusBar.cpp:74-98,182-189,217-220` | `축척` + `1:<denom>` button + popup | No tooltip on button | Button opens popup with preset list & text input | Visible on non-Home tabs (`render_status_right_cluster`) | Opens popup to pick preset or enter scale | `shell_workspace.rs:182-227` | no (kind/tooltip/enabled: editable `QComboBox` with tooltip & `onMap` visibility vs popup button without tooltip shown on Drawing/other tabs too; `KaStatusBar.cpp:77-98,217` vs `shell_workspace.rs:182-227`) |
| 6 | Work CRS button (`crsButton`) | `작업 %1 · %2 ▾` (e.g. `작업 5187 · 동부원점 ▾`) | `지금 그리는 좌표계는 %1입니다. 눌러서 바꿀 수 있습니다.` | Default `EPSG:5187` | Always enabled on map/non-home | Opens menu: `중부원점 (EPSG:5186)`, `동부원점 (EPSG:5187)`, disabled `제출용 파일은 항상 EPSG:5179로 나갑니다` | `KaStatusBar.cpp:104-110,191-196`, `MainWindow.cpp:700-721` | `작업 <code > · <name> ▾` | No tooltip | Default `EPSG:5187` | Always enabled on non-home | Directly toggles between EPSG:5187 and EPSG:5186 on click (no menu) | `shell_workspace.rs:160-174` | no (tooltip/what it does: direct toggle without menu or tooltip vs 2-item menu + tooltip; `MainWindow.cpp:700-721`, `KaStatusBar.cpp:194` vs `shell_workspace.rs:160-174`) |
| 7 | Submit CRS chip (`uploadCrsChip`) | `→ 제출 5179` | `제출용 파일은 작업 좌표계와 상관없이 항상 %1로 변환되어 나갑니다. 누르면 「검수·제출」을 엽니다.` | Always `5179` | Always enabled when shown | Clicking emits `uploadCrsClicked()` to open Inspect & Submit | `KaStatusBar.cpp:114-121,198-204` | `→ 제출 5179` | `제출용 파일은 작업 좌표계와 상관없이 항상 EPSG:5179로 변환되어 나갑니다. 누르면 「검수·제출」을 엽니다.` | Always `5179` | Always enabled when shown | Opens Inspect & Submit panel | `shell_workspace.rs:175-181` | yes |
| 8 | Render toggle (`renderToggle`) | `지도갱신 켜짐` / `지도갱신 멈춤` | `지도 다시 그리기를 잠시 멈춥니다. 레이어를 여러 개 켜고 끌 때 편합니다.` | Checkable, default `true` (`지도갱신 켜짐`) | Visible on map tab (`onMap`) | Pauses or resumes canvas rendering | `KaStatusBar.cpp:125-134,206-221`, `MainWindow.cpp:250-253` | `지도갱신 켜짐` / `지도갱신 멈춤` | `지도 다시 그리기를 잠시 멈춥니다. 레이어를 여러 개 켜고 끌 때 편합니다.` | Toggle bool, default `true` | Visible on map tab | Toggles map canvas rendering freeze | `shell_workspace.rs:229-239` | yes |

### Keyboard and closing behaviour
- **C++**: `scaleCombo` accepts Enter (`returnPressed`) on `scaleEdit` to parse and apply scale (`KaStatusBar.cpp:94-97`). Status bar cannot be closed independently.
- **Rust**: Scale popup text edit applies on Enter or Apply (`shell_workspace.rs:202-226`). Status bar cannot be closed independently.

### Messages
| Condition | C++ verbatim + `file:line` | Rust text + `file:line` |
|---|---|---|
| Scale input invalid | Ignores invalid input and restores current scale (`KaStatusBar.cpp:234-256`) | Ignores invalid input (`shell_workspace.rs:212-220`) |
| Work CRS menu note | `제출용 파일은 항상 EPSG:5179로 나갑니다` (`MainWindow.cpp:714`) | `MISSING` (no menu) |

---

## 4. Left layer panel (`레이어` + `표시 설정` + `파일함`)

- **C++ title (verbatim) & where built**: Dock/panel card titled `레이어` (`MainWindow.cpp:810-818`) inside `m_leftSplit` (`QSplitter`, vertical) inside `m_mainSplit` (`QSplitter`, horizontal) (`MainWindow.cpp:900-920,1031-1039`), containing `KaLayerListChrome`, `KaLayerInformationPanel`, and `KaFileBrowserPanel`.
- **How user opens it**:
  - **C++**: Always visible on left side of `지도` tab (`MainWindow.cpp:905`).
  - **Rust**: Always visible on left side of `지도` tab (`gui_layer_panel_tree.rs:12-71`).
- **Modality**: Modeless side panel on both sides.
- **Size rules**:
  - **C++**: Resizable horizontal `m_mainSplit` initialized to `{348, 932}` (`MainWindow.cpp:1031-1039`), plus vertical `m_leftSplit` between layers and `KaFileBrowserPanel`.
  - **Rust**: Fixed-width `egui::SidePanel::left` `.resizable(false).exact_width(UX_MAP_LEFT_WIDTH)` = `351.0` (`gui_layer_panel_tree.rs:15-18`, `gui_tab_map_consts.rs:5`).
- **Rust counterpart**: `crates/strata-app/src/gui_layer_panel.rs`, `gui_layer_panel_card.rs`, `gui_layer_panel_rows.rs`, `gui_layer_panel_tree.rs`, `gui_layer_panel_context_menu.rs`.

### Controls

| # | kind | C++ text (verbatim) | C++ tooltip / placeholder (verbatim) | C++ default, range, step, suffix | C++ enabled-when rule | C++ what it does | C++ file:line | Rust text | Rust tooltip / placeholder | Rust default, range, step, suffix | Rust enabled-when rule | Rust what it does | Rust file:line | same? |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 1 | Header title | `레이어` | — | — | Always visible | Panel title | `MainWindow.cpp:812` | `레이어` | — | — | Always visible | Panel title | `gui_layer_panel_tree.rs:78` | yes |
| 2 | File drawer toggle (`sidebarFilesToggle`) | `파일함` | — | Checkable, default `true` | Always enabled | Shows/hides `KaFileBrowserPanel` in `m_leftSplit` | `MainWindow.cpp:821-829,1272-1288` | `파일함` | — | Toggle bool `files_open` | Always enabled | Sets `files_open`, but `paint_file_drawer` is called unconditionally so panel never hides | `gui_layer_panel_tree.rs:70,79-86` | no (what it does: toggling does not hide file drawer; `MainWindow.cpp:1272-1288` vs `gui_layer_panel_tree.rs:70,84`) |
| 3 | Check-all toggle (`layerCheckAllBtn`) | `전체 끄기` / `전체 켜기` | `레이어 체크를 한 번에 모두 끄거나 켭니다. 하나라도 켜져 있으면 전부 끕니다.` | — | Disabled when layer list is empty | Checks all layers on, or all off if any is checked | `MainWindow.cpp:832-842` | `전체 끄기` / `전체 켜기` | No tooltip | — | Disabled when layer list is empty | Checks/unchecks all layers | `gui_layer_panel_tree.rs:88-96` | no (tooltip missing; `MainWindow.cpp:839-840` vs `gui_layer_panel_tree.rs:88-96`) |
| 4 | Layer filter input (`layerFilterEdit`) | `""` | Placeholder: `레이어 찾기` | `clearButtonEnabled = true`, minHeight `30` | Always enabled | Filters layer tree by text; Esc clears | `KaLayerListChrome.cpp:36-46` | `layer_filter` | Placeholder: `레이어 찾기` | No clear button | Always enabled | Filters layer list | `gui_layer_panel_tree.rs:99-103` | no (clear button missing; `KaLayerListChrome.cpp:41` vs `gui_layer_panel_tree.rs:99-103`) |
| 5 | Layer tree & sections | Sections: `조사 데이터`, `지적도`, `주변·참조`, `배경 지도` + per-layer check, swatch, name, pencil (`편집 중 · 저장하지 않은 변경이 있습니다 (「저장」 Ctrl+S)`) | Pencil tooltip above | Drag-drop reorderable, multi-select, right-click context menu | Always enabled | Toggles visibility, selects active layer, opens context menu | `KaLayerSectionDelegate.cpp:20-55`, `KaLayerListChrome.cpp:80-120`, `MainWindow.cpp:855-895` | Sections: `조사 데이터`, `지적도`, `주변·참조`, `배경 지도` + per-layer check, swatch, name, pencil | — | Click selects, checkbox toggles visibility, right-click opens context menu | Always enabled | Toggles visibility, selects active layer, opens context menu | `gui_layer_panel_rows.rs:1-242` | no (tooltip on pencil missing, no drag-drop reorder; `KaLayerListChrome.cpp:95-102` vs `gui_layer_panel_rows.rs:100-140`) |
| 6 | Layer context menu | `전체 보기 (확대)`, `스타일 꾸미기…`, `속성표 열기`, `필터…`, `위로`/`아래로`, `파일로 내보내기…`, `이름 바꾸기`, `이 레이어만 켜기`, `지우기` | — | — | Items enabled/hidden by layer type & editability | Performs layer actions (zoom, style, table, filter, reorder, export, rename, solo, remove) | `MainWindowContextMenus.cpp:176-448` | `전체 보기`, `이 레이어만 켜기`, `이름 바꾸기`, `지우기` (subset) | — | — | Enabled by layer state | Subset of context actions | `gui_layer_panel_context_menu.rs:1-118` | no (text/what it does: missing `스타일 꾸미기…`, `속성표 열기`, `필터…`, `위로`/`아래로`, `파일로 내보내기…`; `MainWindowContextMenus.cpp:176-448` vs `gui_layer_panel_context_menu.rs:1-118`) |
| 7 | `표시 설정` header toggle (`layerInformationToggle`) | `표시 설정` | — | Collapsible, default closed (`false`) | Always enabled | Expands/collapses per-layer label & info card | `KaLayerInformationPanel.cpp:42-55` | `표시 설정` | — | Collapsible, default closed | Always enabled | Expands/collapses label settings card | `gui_layer_panel_card.rs:16-30` | yes |
| 8 | Label layer title (`m_title`) & `글자 표시` (`layerLabelVisible`) | `<layer name>` / `글자 표시` (or `<caption > 표시`) | — | Checkbox | Disabled when selected layer is not vector/editable | Enables/disables layer labels | `KaLayerInformationPanel.cpp:58-70,120-150` | MISSING | MISSING | MISSING | MISSING | MISSING (`내용`/`크기`/`색`/`흰 테두리` shown directly without `m_title`, `글자 표시`, or `m_reason`) | `gui_layer_panel_card.rs:32-89` | MISSING |
| 9 | Label field (`내용`) | `표시할 내용 선택…` / `현재 설정 유지` + layer fields | — | `QComboBox` | Enabled when vector layer selected | Picks attribute field for map labels | `KaLayerInformationPanel.cpp:72-76,152-165` | Cycling button with 3 hardcoded strings | — | Cycling button | Enabled when vector layer selected | Cycles 3 hardcoded field labels instead of layer fields | `gui_layer_panel_card.rs:42-55` | no (kind/default: `QComboBox` of layer fields vs cycling button with 3 hardcoded strings; `KaLayerInformationPanel.cpp:72-76` vs `gui_layer_panel_card.rs:42-55`) |
| 10 | Show area (`면적(㎡) 함께 표시`) | `면적(㎡) 함께 표시` | — | `QCheckBox`, default `false` | Enabled on polygon layers | Appends area in m² to label | `KaLayerInformationPanel.cpp:78-80` | `면적(㎡) 함께 표시` | — | Checkbox | Always enabled when shown | Toggles area in label | `gui_layer_panel_card.rs:56-60` | yes |
| 11 | Label size (`크기`) | `크기` | — | `QDoubleSpinBox`, `1.0..72.0`, 1 decimal, ` pt` | Enabled when vector layer selected | Sets label font size in points | `KaLayerInformationPanel.cpp:82-86` | `크기` | — | `DragValue`, `6..=24`, integer `pt` | Enabled when vector layer selected | Sets label font size | `gui_layer_panel_card.rs:61-68` | no (range/step: `1.0..72.0` float vs `6..=24` int; `KaLayerInformationPanel.cpp:83-85` vs `gui_layer_panel_card.rs:64`) |
| 12 | Label color (`글자 색`) & halo (`흰 테두리`) | `글자 색` (`기본 (먹색)`, `흰색`, `진한 파랑`, `진한 빨강`, `갈색`, `진한 초록`) / `흰 테두리 (위성·지적 위에서 읽기)` | — | `QComboBox` + `QCheckBox` | Enabled when vector layer selected | Sets label text color and white buffer halo | `KaLayerInformationPanel.cpp:87-95` | `글자 색` (cycling button) / `흰 테두리 (위성·지적 위에서 읽기)` | — | Cycling button + checkbox | Enabled when vector layer selected | Cycles colors and toggles white halo | `gui_layer_panel_card.rs:70-88` | no (kind: `QComboBox` vs cycling button for color; `KaLayerInformationPanel.cpp:87-91` vs `gui_layer_panel_card.rs:70-80`) |
| 13 | File drawer (`파일함`) shortcuts & list | `파일함` + 6 buttons: `내 PC`, `바탕화면`, `문서`, `내려받기`, `폴더 직접 찾기`, `상위` + `fileBrowser` list | `fileBrowser`: `파일을 지도에 끌어 넣으면 레이어가 됩니다.` | List of folders & supported spatial files | Always enabled (`상위` disabled at root) | Navigates folders and drags/double-clicks files onto map | `KaFileBrowserPanel.cpp:42-180` | `파일함` + 6 buttons: `내 PC`, `바탕화면`, `문서`, `내려받기`, `폴더 직접 찾기`, `상위` + file list | — | List of folders & files | Always enabled | `내 PC` navigates to user home dir instead of drives; double-click adds layer | `gui_layer_panel_card.rs:95-225` | no (what it does/tooltip: `내 PC` goes to home dir instead of drives; list tooltip missing; `KaFileBrowserPanel.cpp:58,105` vs `gui_layer_panel_card.rs:106,117`) |

### Keyboard and closing behaviour
- **C++**: `layerFilterEdit` clears on Esc (`KaLayerListChrome.cpp:45`). `m_mainSplit` and `m_leftSplit` can collapse/resize panes (`MainWindow.cpp:1031-1039`).
- **Rust**: Panel width is fixed `351.0` and not collapsible/resizable (`gui_layer_panel_tree.rs:15-18`). `파일함` button does not collapse the file drawer (`gui_layer_panel_tree.rs:70,79-86`).

### Messages
| Condition | C++ verbatim + `file:line` | Rust text + `file:line` |
|---|---|---|
| No layers in project | `레이어가 없습니다.\n파일함에서 SHP·DXF·DWG를 끌어 넣거나\n위성·지적 배경을 올리세요.` (`MainWindow.cpp:860-862`) | `레이어가 없습니다.` (`gui_layer_panel_rows.rs:28`) |
| Filter matches nothing | `「%1」에 맞는 레이어가 없습니다.` (`KaLayerListChrome.cpp:62`) | `「<query>」에 맞는 레이어가 없습니다.` (`gui_layer_panel_rows.rs:42`) |

---

## 5. Right panel (`선택한 유구` / `속성` / `스타일` / `배경 지도`)

- **C++ title (verbatim) & where built**: `KaInspectorPanel` inside `MainWindow` right splitter pane (`KaInspectorPanel.cpp:25-174`, `MainWindowChrome.cpp:85-135`).
- **How user opens it**:
  - **C++**: Visible on right side of `지도` tab; collapsible via `inspectorCollapse` (`F10`) (`KaInspectorPanel.cpp:58-65`, `MainWindowChrome.cpp:95`).
  - **Rust**: Visible on right side of `지도` tab (`right_inspector.rs:245-260`), collapsible via `F10` / button.
- **Modality**: Modeless side panel on both sides.
- **Size rules**:
  - **C++**: `preferredWidth()` is `272` (`width < 1500`) or `300` (`width >= 1500`), resizable in `QSplitter` (`KaInspectorPanel.cpp:34-35`).
  - **Rust**: Fixed width `egui::SidePanel::right(...).resizable(false).exact_width(UX_MAP_RIGHT_WIDTH)` = `236.0` (`right_inspector.rs:249-251`, `gui_tab_map_consts.rs:7`).
- **Rust counterpart**: `crates/strata-app/src/right_inspector.rs`, `ri_body.rs`, `ri_body_cards.rs`, `ri_body_bottom.rs`, `ri_attr_card.rs`, `ri_attr_card_rows.rs`, `ri_layer_card.rs`.

### Controls

| # | kind | C++ text (verbatim) | C++ tooltip / placeholder (verbatim) | C++ default, range, step, suffix | C++ enabled-when rule | C++ what it does | C++ file:line | Rust text | Rust tooltip / placeholder | Rust default, range, step, suffix | Rust enabled-when rule | Rust what it does | Rust file:line | same? |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 1 | Header title & count chip | `선택한 유구` (or feature header) / `%1개 선택` | — | Hidden count chip if `<= 1` | Always visible | Shows panel title and multi-select count | `KaInspectorPanel.cpp:48-57,140-150` | `선택한 유구` / `<N>개 선택` | — | Hidden count chip if `<= 1` | Always visible | Shows panel title and multi-select count | `ri_body.rs:22-45` | yes |
| 2 | Collapse button (`inspectorCollapse`) | `▸` / `▾` | `패널 접기 (F10)` | — | Always enabled | Collapses/expands right inspector panel | `KaInspectorPanel.cpp:58-65` | `›` / `‹` | `패널 접기 (F10)` | — | Always enabled | Collapses/expands right inspector panel | `ri_body.rs:30-42` | yes |
| 3 | Inspector tabs | `속성` / `스타일` | — | Default tab `속성` | Always enabled | Switches between `KaFeatureCard` (`속성`) and `KaInspectorStyle` (`스타일`) | `KaInspectorPanel.cpp:72-82` | `속성` / `스타일` | — | Default tab `속성` | Always enabled | Switches between attribute card and layer style card | `ri_body.rs:50-68` | yes |
| 4 | Feature card: auto measures (`자동 계산`) | `자동 계산`: `면적` (`m²`), `둘레`/`길이` (`m`) | — | Read-only computed values | Shown when single feature selected | Shows live area and perimeter/length | `KaFeatureCard.cpp:120-145,280-282` | `자동 계산`: `면적`, `둘레`/`길이` | — | Read-only | Shown when feature selected | Shows area and perimeter/length | `ri_attr_card_rows.rs:30-60` | yes |
| 5 | Feature card: number & next button | `번호` (`m_number`) + `다음 번호` | `다음 번호` tooltip: `%1 넣기` | Editable text + button | Enabled on editable layer; `다음 번호` disabled if `next.isEmpty()` | Edits `number`/`feat_no` or inserts `FeatureNumbering::suggestNext` | `KaFeatureCard.cpp:150-168,290-294` | `번호` + `다음 번호` | Tooltip on `다음 번호` | Editable text + button | Enabled on editable layer | Edits number or inserts next number | `ri_attr_card_rows.rs:65-95` | yes |
| 6 | Feature card: attributes | `조사명`/`유적명`/`이름`, `종류` (`kind`), `시대` (`period` + `KaPeriodBand`), `상태` (`status`), `조사자` (`surveyor`), `조사일` (`surv_date`), `현장 메모 (두 줄)` (`note`) | `surv_date` placeholder: `예: 2026-10-03` | Combos/line edits + 2-row note | Editable only on editable survey layers; locked on cadastral/reference | Edits feature attributes directly into edit buffer | `KaFeatureCardEditors.cpp:1-275`, `KaFeatureCard.cpp:258-279` | `조사명`/`유적명`/`이름`, `종류`, `시대`, `상태`, `조사자`, `조사일`, `현장 메모` | `예: 2026-10-03` | Text edits / combos | Editable on editable layers | Edits feature attributes | `ri_attr_card_rows.rs:96-198` | yes |
| 7 | Feature card: state chip & full card button | `저장된 값입니다` / `변경됨 · 아직 저장 안 됨` + `조사카드 열기` | `조사카드 열기`: `이 도형의 전체 조사카드(모든 칸)를 창으로 엽니다.` | — | Enabled when single feature selected | Opens full feature card dialog (`KaFeatureDialog`) | `KaFeatureCard.cpp:215-245,295-298` | State label + `조사카드 열기` | `이 도형의 전체 조사카드(모든 칸)를 창으로 엽니다.` | — | Enabled when feature selected | Opens full feature card window | `ri_attr_card.rs:80-125` | yes |
| 8 | Style tab: layer swatch & 8 color swatches | Layer name + 8 color swatches (4x2 grid, `44x24`) | — | 8 palette colors | Enabled when vector layer selected | Applies fill/stroke color to selected layer | `KaInspectorStyle.cpp:45-105` | Layer name + 8 color swatches | — | 8 palette colors | Enabled when vector layer selected | Applies color to selected layer | `ri_layer_card.rs:45-115` | yes |
| 9 | Style tab: no-fill / no-stroke & thickness | `채우기 없음 (선만)` / `선 없음` (hidden on Line) + `점 크기`/`선 굵기`/`외곽선 굵기` `QDoubleSpinBox` | — | `0.2..12.0`, step `0.2`, 1 decimal, ` mm` | Enabled on vector layer (`선 없음` hidden for Line) | Sets fill/stroke toggles and symbol size/width in mm | `KaInspectorStyle.cpp:106-160` | `채우기 없음 (선만)` / `선 없음` + thickness `DragValue` | — | `0.2..=12.0`, step `0.2`, `mm` | Enabled on vector layer | Sets fill/stroke toggles and width | `ri_layer_card.rs:120-210` | yes |
| 10 | Basemap quick card (`KaBasemapQuickCard`) | `배경 지도` / `조사를 열면 위성과 지적이 올라옵니다.` + 3 checkable buttons: `위성`, `지형`, `옛 지도 ▾` | Note tooltip: `아래 단추는 리본의 「배경 지도」와 같은 단추입니다.` | Checkable buttons synced to active basemap | Always enabled | Toggles satellite/topo basemaps or opens `옛 지도 ▾` menu | `KaBasemapQuickCard.cpp:52-75` | `배경 지도` + `위성`, `지형`, `옛 지도 ▼` | No tooltip on note/header | Plain buttons (not checkable/synced state) | Always enabled | Switches basemap / opens old-map menu | `ri_body_bottom.rs:13-35` | no (tooltip/default: no note tooltip, buttons are not checkable/synced; `KaBasemapQuickCard.cpp:55,63-71` vs `ri_body_bottom.rs:13-34`) |

### Keyboard and closing behaviour
- **C++**: `F10` toggles panel collapse (`KaInspectorPanel.cpp:60`). Tab navigates fields inside `속성` and `스타일`. Splitter resizable.
- **Rust**: `F10` toggles panel collapse; panel width fixed at `236.0` (`right_inspector.rs:249-251`, `gui_tab_map_consts.rs:7`).

### Messages
| Condition | C++ verbatim + `file:line` | Rust text + `file:line` |
|---|---|---|
| Nothing selected (`속성`) | Empty selection prompt (`KaInspectorPanel.cpp:105-120`) | Selection prompt (`ri_body_cards.rs:1-19`) |
| Duplicate feature number | `같은 종류에 같은 번호가 %1개 더 있습니다. 제출 전 검수에서도 알려 드립니다.` (`KaFeatureCard.cpp:285-286`) | Same warning text (`ri_attr_card_rows.rs`) |

---

## 6. Map overlays (`지도` canvas HUD, zoom, scale bar, grid, opacity, measure)

- **C++ title (verbatim) & where built**: Floating child widgets over `QgsMapCanvas`: `KaMapControls`, `KaMapScaleBar`, `KaDrawGuideBand`, `KaLayerOpacityRail`, `KaShellGridControls`, `KaGridOriginButton`, `KaMeasureMapTool` HUD (`MainWindow.cpp:1016-1030`).
- **How user opens it**:
  - **C++**: Shown automatically on `지도` tab (`MainWindow.cpp:1016-1030`); `measureHud` opens when Measure tool is active (`KaMeasureMapTool.cpp:358-410`).
  - **Rust**: Shown automatically on `지도` tab (`gui_tab_map_canvas_overlay.rs:1-105`); measure HUD opens when Measure tool is active (`gui_tab_map_hud.rs:276-278`).
- **Modality**: Modeless floating HUD overlays on both sides.
- **Size rules**:
  - **C++**: Zoom buttons `36x36` (`KaMapControls.cpp:58`); `KaDrawGuideBand` height `36` (`KaDrawGuideBand.cpp:34`); `measureHud` width `300` (`KaMeasureMapTool.cpp:100`); `KaLayerOpacityRail` draggable & remembers position (`KaLayerOpacityRail.cpp:97-105`).
  - **Rust**: Zoom buttons `32x32` (`gui_tab_map_overlay.rs:22-60`); `KaLayerOpacityRail` fixed at `(12, opacity_top_y)` (`gui_tab_map_opacity.rs`, `gui_tab_map_hud.rs:240-255`); measure HUD floating card (`gui_tab_map_measure.rs`).
- **Rust counterpart**: `crates/strata-app/src/gui_tab_map_canvas_overlay.rs`, `gui_tab_map_overlay.rs`, `gui_tab_map_hud.rs`, `gui_tab_map_gridctl.rs`, `gui_tab_map_opacity.rs`, `gui_tab_map_measure.rs`, `gui_tab_map_undo.rs`.

### Controls

| # | kind | C++ text (verbatim) | C++ tooltip / placeholder (verbatim) | C++ default, range, step, suffix | C++ enabled-when rule | C++ what it does | C++ file:line | Rust text | Rust tooltip / placeholder | Rust default, range, step, suffix | Rust enabled-when rule | Rust what it does | Rust file:line | same? |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 1 | Zoom buttons (`KaMapControls`) | `+` (`지도 확대`), `-` (`지도 축소`), fit icon (`조사구역 맞춤`) | `확대 (마우스 휠을 앞으로)`, `축소 (마우스 휠을 뒤로)`, `조사구역이 화면에 꽉 차게 봅니다` | `36x36` each | Always enabled | Zooms in, zooms out, or zooms to survey area | `KaMapControls.cpp:52-92` | `+`, `-`, `□` | `확대 (마우스 휠을 앞으로)`, `축소 (마우스 휠을 뒤로)`, `조사구역이 화면에 꽉 차게 봅니다` | `32x32` each | Always enabled | Zooms in, zooms out, or zooms to survey area | `gui_tab_map_overlay.rs:22-60` | no (size & text: `32x32` with `"□"` text vs `36x36` with fit icon; `KaMapControls.cpp:58,78-86` vs `gui_tab_map_overlay.rs:22-60`) |
| 2 | Scale bar (`KaMapScaleBar`) | `▲ 도북` + 4-quarter bar + distance labels | — | Dynamic metric scale | Always visible on map | Displays grid north arrow and metric scale bar | `KaMapControls.cpp:150-288` | `▲ 도북` + scale bar + distance labels | — | Dynamic metric scale | Always visible on map | Displays grid north arrow and metric scale bar | `gui_tab_map_overlay.rs:65-138` | yes |
| 3 | Draw guide band + undo/redo (`KaDrawGuideBand`) | Guide pill + `되돌리기 (Ctrl+Z)` + `다시 실행 (Ctrl+Y)` | — | Height `36`, shown when drawing/measuring | Undo/Redo enabled when stack has items | Shows active drawing/measure hint and undo/redo buttons | `KaDrawGuideBand.cpp:28-140` | Guide pill + `되돌리기` + `다시 실행` | Undo/Redo tooltips | Shown when drawing/opacity idle | Enabled when stack has items | Shows active hint and undo/redo buttons | `gui_tab_map_hud.rs:249-255`, `gui_tab_map_undo.rs:1-120` | yes |
| 4 | Opacity/brightness rail (`KaLayerOpacityRail`) | `투명도·밝기: 조절할 레이어를 고르세요` (idle) / `투명도`, `밝기` | `빈 곳을 잡고 끌면 옮길 수 있습니다. 놓은 자리는 다음에도 그대로입니다.` | `투명도`: `0..100` (`%`), `밝기`: `-255..255`; draggable | Sliders shown when raster/layer active | Adjusts active layer opacity & brightness; card is draggable | `KaLayerOpacityRail.cpp:40-150` | `투명도·밝기: 조절할 레이어를 고르세요` (idle) / `투명도`, `밝기` | No drag tooltip | `투명도`: `0..=100` (`%`), `밝기`: `-255..=255`; fixed position | Sliders shown when layer selected | Adjusts opacity & brightness; card is NOT draggable | `gui_tab_map_opacity.rs:1-149`, `gui_tab_map_hud.rs:245` | no (what it does/tooltip: fixed position, not draggable, no drag tooltip; `KaLayerOpacityRail.cpp:97-105` vs `gui_tab_map_opacity.rs`) |
| 5 | Grid toggle (`좌표격자`) | `좌표격자` | `지도에 좌표 격자를 켭니다. 켠 뒤 미터·경위도 격자를 고릅니다.` | Default `false` | Always enabled | Toggles coordinate grid overlay | `KaShellGridControls.cpp:35-55` | `좌표격자` | `지도에 좌표 격자를 켭니다. 켠 뒤 미터·경위도 격자를 고릅니다.` | Default `false` | Always enabled | Toggles coordinate grid overlay | `gui_tab_map_gridctl.rs:15-35` | yes |
| 6 | Grid kind (`m_kind`) | `미터 격자`, `경위도 격자` | — | Default `미터 격자` | Enabled when `좌표격자` checked | Switches metric vs geographic graticule; disables step & rotation on geographic | `KaShellGridControls.cpp:58-68,181-186` | `미터 격자` / `경위도 격자` | — | Default `미터 격자` | Enabled when `좌표격자` checked | Switches metric vs geographic grid | `gui_tab_map_gridctl.rs:36-55` | yes |
| 7 | Grid step, rotation, width, dash | `간격 <v> m`, `회전 <v>°`, `굵기 <v>`, `실선`/`점선`/`점` | — | `step`: `0.1..10000.0` (1 dec, ` m`); `rotation`: `0.0..360.0` (1 dec, `°`); `width`: `0.5..5.0` (step `0.1`, 1 dec) | `step` & `rotation` disabled when `경위도 격자` (`KaShellGridControls.cpp:181-186`) | Updates grid spacing, rotation, stroke width, and dash style | `KaShellGridControls.cpp:70-98,181-186` | `간격`, `회전`, `굵기`, `실선`/`점선`/`점` | — | `step`: `0.1..=10000.0` (`m`), `rotation`: `0.0..=360.0` (`°`), `width`: `0.5..=5.0` | `step` & `rotation` disabled on `경위도 격자` | Updates grid settings | `gui_tab_map_gridctl.rs:56-114` | yes |
| 8 | Grid colors & `…` button | 3 color swatches + `…` | `…`: `그 밖의 색을 직접 고릅니다` | 3 presets + `QColorDialog` | Enabled when grid checked | Sets preset color or opens `QColorDialog` | `KaShellGridControls.cpp:99-115` | 3 color swatches + `…` | `그 밖의 색을 직접 고릅니다` | 3 presets | Enabled when grid checked | `…` cycles the 3 preset colors instead of opening color picker | `gui_tab_map_gridctl.rs:115-125` | no (what it does: `…` cycles presets instead of color picker; `KaShellGridControls.cpp:101-110` vs `gui_tab_map_gridctl.rs:115-121`) |
| 9 | Grid origin button (`KaGridOriginButton`) | `원점` | `격자 원점: 좌표계 원점(0, 0). 눌러서 조사지 격자 원점을 정합니다.` (or `X(동) %1 m, Y(북) %2 m`) | Default `(0, 0)`, dialog `X`/`Y` `-10000000.0..10000000.0` (`3` dec, ` m`) | Enabled when grid checked (`MainWindow.cpp:1027`) | Opens modal `격자 원점` dialog (`지도 가운데`, `좌표계 원점 (0, 0)`, `적용`/`취소`) | `KaGridOriginButton.cpp:31-112`, `MainWindow.cpp:1027` | MISSING | MISSING | MISSING | MISSING | MISSING | MISSING | MISSING |
| 10 | Measure HUD (`KaMeasureMapTool`) | `거리`, `면적`, `<total>`, `<detail>`, `마침`, `복사`, `한 점 취소`, `다시`, `클릭 점 · 우클릭에서 마침 · Backspace 한 점 취소 · Esc 지우기` | — | Width `300`, default `거리` | Visible when Measure tool active | Switches Distance/Area, finishes, copies result, undoes vertex, clears | `KaMeasureMapTool.cpp:85-175,357-411` | `거리`, `면적`, `<total>`, `<detail>`, `마침`, `복사`, `한 점 취소`, `다시`, hint | — | Default `거리` | Visible when Measure tool active | Switches Distance/Area, finishes, copies result, undoes vertex, clears | `gui_tab_map_measure.rs:1-243` | yes |

### Keyboard and closing behaviour
- **C++**: `KaMeasureMapTool` handles `Backspace` (undo last vertex), `Esc` (clear), right-click (`showTapeMenu`), `Ctrl+Z`/`Ctrl+Y` for undo/redo (`KaMeasureMapTool.cpp:413-425`). `KaGridOriginButton` opens modal `QDialog` (`exec()`) with `적용`/`취소`.
- **Rust**: `gui_tab_map_measure.rs` supports `Backspace`, `Esc`, right-click finish. `KaGridOriginButton` is missing.

### Messages
| Condition | C++ verbatim + `file:line` | Rust text + `file:line` |
|---|---|---|
| Measure distance (<2 points) | `두 점 이상 찍고 우클릭 → 마침` (`KaMeasureMapTool.cpp:388`) | Same prompt (`gui_tab_map_measure.rs`) |
| Measure area (<3 points) | `세 점 이상 찍고 우클릭 → 마침` (`KaMeasureMapTool.cpp:381`) | Same prompt (`gui_tab_map_measure.rs`) |
| Opacity rail idle | `투명도·밝기: 조절할 레이어를 고르세요` (`KaLayerOpacityRail.cpp`) | `투명도·밝기: 조절할 레이어를 고르세요` (`gui_tab_map_hud.rs:245`) |

---

## Differences to fix

1. Home screen, `복구 사본` banner links (`사본 열기 · 폴더 열기 · 숨기기`) and context menu items, missing in Rust (`KaStartPage.cpp:240-243`, `KaHomeRecentCard.cpp:222-229` vs `gui_tab_home_hero.rs`, `gui_tab_home_recent_box.rs:145`).
2. Home screen, `이어서 작업` save badge and recent table `상태` column, hardcodes `"저장됨"` instead of checking unsaved recovery copy (`● 저장 안 됨`) (`KaHomeTitleBlock.cpp:133`, `KaHomeRecentCard.cpp:176-184` vs `gui_tab_home_hero.rs:155`, `gui_tab_home_recent_box.rs:97`).
3. Home screen, `작업 순서` (`KaHomeGuideCard`), hardcoded step progress and clickable step links instead of dynamic `SurveyFacts` and plain labels (`KaHomeGuideCard.cpp:161-194` vs `gui_tab_home_recent.rs:72-104`).
4. Tab bar, `KaSurveyBadge`, non-clickable `ui.label` instead of clickable/Tab-focusable control that switches to `지도` tab (`KaSurveyBadge.cpp:26,133` vs `gui_ribbon_tabbar.rs:274`).
5. Tab bar, `테마 ▼` menu, mixes display toggles (`새 모양`, `고대비 화면`, `큰 글씨`) and sound options into color theme menu (`KaThemeOptions.cpp:168-181` vs `gui_ribbon_tabbar.rs:150-230`).
6. Tab bar, `고지형` (`MainTab::Paleo`) extra tab and missing `탭 닫기` tooltip on dynamic tab close buttons (`KaTabClose.cpp:45` vs `gui_ribbon_tabbar.rs:81,107-113`).
7. Status bar, `snapChip` (`자석 끔`/`자석 켬`), missing in Rust (`KaStatusBar.cpp:48-52` vs `shell_workspace.rs:139-252`).
8. Status bar, `crsButton`, toggles directly on click with no tooltip instead of opening 2-option menu + `EPSG:5179` note (`MainWindow.cpp:700-721`, `KaStatusBar.cpp:194` vs `shell_workspace.rs:160-174`).
9. Status bar, `scaleCombo`, popup button without tooltip shown on non-Map tabs too instead of editable `QComboBox` (`minWidth 120`) shown only `onMap` (`KaStatusBar.cpp:77-98,217` vs `shell_workspace.rs:182-227`).
10. Left layer panel, `파일함` toggle button, sets `files_open` but `paint_file_drawer` runs unconditionally so panel never hides (`MainWindow.cpp:1272-1288` vs `gui_layer_panel_tree.rs:70,79-86`).
11. Left layer panel, `표시 설정` (`KaLayerInformationPanel`), missing `m_title`, `글자 표시` checkbox, `m_reason`; `내용` and `글자 색` are cycling buttons (3 hardcoded fields) instead of `QComboBox`; `크기` is `6..=24` int instead of `1.0..72.0` float (`KaLayerInformationPanel.cpp:58-95` vs `gui_layer_panel_card.rs:16-89`).
12. Left layer panel, `내 PC` button in `파일함`, opens user home folder instead of drives list, and file list has no tooltip (`KaFileBrowserPanel.cpp:58,105` vs `gui_layer_panel_card.rs:106,117`).
13. Left & Right panels, fixed widths (`351.0` and `236.0`) instead of resizable `QSplitter` (`348` and `272`/`300`) (`MainWindow.cpp:1031-1039`, `KaInspectorPanel.cpp:34-35` vs `gui_layer_panel_tree.rs:15-18`, `right_inspector.rs:249-251`).
14. Left layer panel, context menu, missing `스타일 꾸미기…`, `속성표 열기`, `필터…`, `위로`/`아래로`, `파일로 내보내기…` (`MainWindowContextMenus.cpp:176-448` vs `gui_layer_panel_context_menu.rs:1-118`).
15. Right panel, `배경 지도` quick card, missing header tooltip and checkable/synced button states (`KaBasemapQuickCard.cpp:52-108` vs `ri_body_bottom.rs:13-35`).
16. Map overlays, `KaGridOriginButton` (`원점` button & `격자 원점` modal dialog), missing in Rust (`KaGridOriginButton.cpp:31-112`, `MainWindow.cpp:1027` vs `gui_tab_map_canvas_overlay.rs:40`).
17. Map overlays, `KaLayerOpacityRail`, fixed position and no drag tooltip instead of draggable card (`KaLayerOpacityRail.cpp:97-105` vs `gui_tab_map_opacity.rs`, `gui_tab_map_hud.rs:245`).
18. Map overlays, `KaMapControls` zoom buttons (`32x32` + `"□"` text vs `36x36` + fit icon) and `KaShellGridControls` `…` button (cycles 3 presets instead of color picker) (`KaMapControls.cpp:58,78-86`, `KaShellGridControls.cpp:101-110` vs `gui_tab_map_overlay.rs:22-60`, `gui_tab_map_gridctl.rs:115-121`).
19. Home screen & Left panel, `m_filter` / `layerFilterEdit` missing clear button (`KaHomeRecentCard.cpp:63`, `KaLayerListChrome.cpp:41` vs `gui_tab_home_recent.rs:34-38`, `gui_layer_panel_tree.rs:99-103`).
20. Home screen, `이어서 열기` button text spacing (`"이어서 열기  →"` vs `"이어서 열기 →"`) (`KaHomeTitleBlock.cpp:36` vs `gui_tab_home_hero.rs:164`).

---

## Could not determine

- Exact line numbers inside `KaLayerOpacityRail.cpp` for the idle label text `"투명도·밝기: 조절할 레이어를 고르세요"` (observed in `gui_tab_map_hud.rs:245`; `KaLayerOpacityRail.cpp` lines 40-150 were summarized without the exact line of that string).
- Exact line numbers in `gui_tab_map_measure.rs` for the `"두 점 이상 찍고 우클릭 → 마침"` / `"세 점 이상 찍고 우클릭 → 마침"` strings (marked without line range in table; only `gui_tab_map_measure.rs:1-243` checked at file level).
- Exact line numbers in `ri_attr_card_rows.rs` for the duplicate feature number warning `"같은 종류에 같은 번호가 %1개 더 있습니다..."` and `KaInspectorPanel.cpp:105-120` verbatim empty-selection string.

---

## Counts

- **Windows compared**: 6 (`Home screen`, `Tab bar and window title`, `Status bar`, `Left layer panel`, `Right panel`, `Map overlays`)
- **C++ control rows**: 61 total (14 Home + 6 Tab bar/title + 8 Status bar + 13 Left panel + 10 Right panel + 10 Map overlays)
- **Rust counterparts**: 57 implemented (4 `MISSING`: Home recovery banner, Status `snapChip`, Left `표시 설정` `m_title`/`글자 표시`, Map `KaGridOriginButton`) + 1 `EXTRA` (`고지형` tab)
- **Differences by column**:
  - `text`: 7
  - `tooltip / placeholder`: 9
  - `default, range, step, suffix`: 7
  - `enabled-when rule`: 3
  - `what it does`: 11
  - `MISSING`: 4
  - `EXTRA`: 1
