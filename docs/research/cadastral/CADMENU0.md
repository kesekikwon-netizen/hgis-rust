# CADMENU0 — Surrounding UI, Menus, Legend Context Menu & Persistence of `지적도 · 조사 주변 5km`

- **Order**: `CADMENU0` (`docs/senior/research/cadastral/ORDER-CADMENU0.yaml`)
- **Checkout (`HEAD`)**: `8739aff` on `main` (Note: `ORDER-CADMENU0.yaml:5` named `845f290`; while read-only research was in progress, the director landed `6b350b6` `CAD1` and `8739aff` `CAD1b` onto `main`. Per `ORDER-CADMENU0.yaml:7, 74`, all Rust `file:line` citations below match the live `crates/` files of THIS checkout at `8739aff`, with notes where `8739aff` changed `845f290`.)
- **C++ reference**: `target/cpp-ref/` (cited relative to `target/cpp-ref/`, e.g. `src/app/MainWindowCadastral.cpp:34`)
- **Rust source**: `crates/` at `8739aff` (cited as `crates/.../<file>:line`)
- **Order line-number & title notes (`not_stop`)**:
  - `src/app/MainWindowCadastral.cpp` ends at line `174` (order line `24` says `:134-175`).
  - Order lines `15-16` refer to the two dialogs by their ribbon menu labels (`「VWorld 계정 설정」`, `「선 색·지번 표시」`); in C++ their actual `windowTitle()` strings are `「VWorld 지적도 계정」` (`src/app/MainWindowCadastral.cpp:36`) and `「지적도 선 색·지번」` (`src/app/MainWindowCadastral.cpp:61`).
  - Order line `28` cites `CAD0`'s note that `"선 색·지번 표시"` was misrouted: at `845f290` `crates/strata-app/src/gui_ribbon_menus_a.rs:183` opened `ActiveModalDialog::CadastralDownload`; commit `8739aff` (`CAD1b`) changed `:183` to `self.open_cadastral_style()`.

---

## Part 1 — The `지적` Ribbon Button and Its Menu

### 1.1 Ribbon Button Construction and Tooltip
- **Placement**: Added to ribbon group `"fetch"` (`"자료 받기"`, `src/app/MainWindowRibbon.cpp:250, 471`) before `수치지형` (`topographic`, `:484`) and `옛 지도` (`oldMaps`, `:485`).
- **Button & primary action**: `addIcon("fetch", "cadastral", "지적", "조사구역 주변 5km 지적도를 받아 경계선과 지번을 표시합니다. 우클릭: 계정·선 색 설정", &MainWindow::downloadCadastral)` (`src/app/MainWindowRibbon.cpp:471-474`); `actCadastral->setObjectName("actionCadastralDownload")` (`:475`).
- **Effective tooltip**: `addIcon` (`src/app/MainWindowRibbon.cpp:256-264`) sets `a->setToolTip(tip)` (`:259`) and delegates button creation to `KaBeginnerRibbon::addAction` (`src/app/KaBeginnerRibbon.cpp:111-123`), keeping the tooltip verbatim:
  - `「조사구역 주변 5km 지적도를 받아 경계선과 지번을 표시합니다. 우클릭: 계정·선 색 설정」` (`src/app/MainWindowRibbon.cpp:473`).
- **Left-click on main button body**: Triggers `MainWindow::downloadCadastral()` (`src/app/MainWindowRibbon.cpp:474`, `src/app/MainWindowCadastral.cpp:82`).

### 1.2 How the Menu Opens
The `cadastralMenu` (`new QMenu(btnCadastral)`, `src/app/MainWindowRibbon.cpp:476`) opens in **both** ways:
1. **Dropdown arrow click**: `btnCadastral->setMenu(cadastralMenu)` + `btnCadastral->setPopupMode(QToolButton::MenuButtonPopup)` (`src/app/MainWindowRibbon.cpp:479-480`) draws a split-button arrow beside `지적`; clicking the arrow opens `cadastralMenu` without running `downloadCadastral()`.
2. **Right-click anywhere on `btnCadastral`**: `btnCadastral->setContextMenuPolicy(Qt::CustomContextMenu)` + `connect(btnCadastral, &QToolButton::customContextMenuRequested, ...)` executes `cadastralMenu->exec(btnCadastral->mapToGlobal(pos))` (`src/app/MainWindowRibbon.cpp:481-483`).

### 1.3 Menu Rows in Order, Disabled Rules, and Target Functions

| Order | Row text (verbatim) | Disabled when | Function called | Citation |
|---:|---|---|---|---|
| 1 | `VWorld 계정 설정` | **Never disabled** (`setEnabled` is never called on this `QAction`) | `MainWindow::configureCadastralAccount()` (`src/app/MainWindowCadastral.cpp:34-54`) | `src/app/MainWindowRibbon.cpp:477` |
| 2 | `선 색·지번 표시` | **Never disabled** (`setEnabled` is never called on this `QAction`; absence of a downloaded cadastral layer is handled after click inside `configureCadastralStyle()` with an info box) | `MainWindow::configureCadastralStyle()` (`src/app/MainWindowCadastral.cpp:56-80`) | `src/app/MainWindowRibbon.cpp:478` |

- **Separators / submenus / checkmarks / icons in `cadastralMenu`**: None (`src/app/MainWindowRibbon.cpp:476-479`).
- **Other UI entry points to `configureCadastralAccount()`**:
  1. Ribbon `더보기` menu (`moreMenu`) -> row `「VWorld 지적도 아이디·비밀번호」` (`src/app/MainWindowRibbon.cpp:678`).
  2. Home screen connection status card (`MainWindow::openAccountSetup(AccountStatus::Source::CadastralAccount)`, `src/app/MainWindowChrome.cpp:218-224`).
  3. Automatic prompt inside `MainWindow::downloadCadastral()` when credentials are empty (`src/app/MainWindowCadastral.cpp:117-120`) or when VWorld rejects login and the user chooses `「아이디·비밀번호 다시 입력」` (`src/app/MainWindowCadastral.cpp:142-154`).
- **Other UI entry point to `configureCadastralStyle()`**:
  1. Legend layer right-click menu `「면·외곽선 색」` (`layer.style`) -> `MainWindow::editCurrentLayerStyle(layer)` when `layer->customProperty("ka_hgis/cadastral").toBool()` is `true` (`src/app/MainWindowContextMenus.cpp:284-285`, `src/app/MainWindowEditing.cpp:444-447`).

---

## Part 2 — The Account Dialog (`MainWindow::configureCadastralAccount`)

### 2.1 Window Title and Layout Frame
- **Function**: `bool MainWindow::configureCadastralAccount()` (`src/app/MainWindowCadastral.cpp:34-54`).
- **Window type & modality**: Modal `QDialog dialog(this)` run synchronously with `dialog.exec()` (`src/app/MainWindowCadastral.cpp:36, 47`).
- **Window title (verbatim)**: `「VWorld 지적도 계정」` (`src/app/MainWindowCadastral.cpp:36`).
- **Layout & plate styling**: `QFormLayout(&dialog)` dressed by `KaFormPlate::dress(&dialog, form)` (`src/app/MainWindowCadastral.cpp:37, 44`; `src/app/KaFormPlate.cpp:10-29`, active when `qApp->property("kaPaperLook").toBool()` is true at `:11`):
  - `dialog->setMinimumWidth(440)` (`src/app/KaFormPlate.cpp:13`)
  - Content margins `(24, 24, 24, 16)`, horizontal spacing `0`, vertical spacing `12` (`src/app/KaFormPlate.cpp:14-16`)
  - Form labels given `setFixedWidth(96)` (`:18-22`), spanning `QLabel` children given `setWordWrap(true)` (`:23-27`).

### 2.2 Every Control in Order, Verbatim Texts, Placeholders, and Defaults
On open, `const auto previous = CadastralPortal::credentials();` reads the stored credentials (`src/app/MainWindowCadastral.cpp:35`):

| Order | Form slot | Label / text (verbatim) | Widget & echo mode | Placeholder | Initial / default value | Citation |
|---:|---|---|---|---|---|---|
| 1 | Row 0 (`label`, `field`) | `아이디` | `QLineEdit` (`Normal` echo mode) | `not in the source` (empty) | `previous.username` (`""` if never saved) | `src/app/MainWindowCadastral.cpp:38, 40` |
| 2 | Row 1 (`label`, `field`) | `비밀번호` | `QLineEdit` with `QLineEdit::Password` (`setEchoMode(QLineEdit::Password)`) | `not in the source` (empty) | `previous.password` (`""` if never saved or undecryptable) | `src/app/MainWindowCadastral.cpp:39-40` |
| 3 | Row 2 (spanning `QLabel`) | `이 PC의 개인 설정에 저장하여 다음 다운로드에 사용합니다.` | `QLabel` (`setWordWrap(true)` via `KaFormPlate::dress`) | — | Static hint text | `src/app/MainWindowCadastral.cpp:41`, `src/app/KaFormPlate.cpp:25` |
| 4 | Row 3 (spanning `QDialogButtonBox`) | Qt standard buttons `QDialogButtonBox::Save \| QDialogButtonBox::Cancel` (localized by Qt translation if loaded, else `"Save"` / `"Cancel"`; exact Korean string `not in the source`) | `QDialogButtonBox` (`accepted` -> `dialog.accept()`, `rejected` -> `dialog.reject()`) | — | `Save` is default accept button | `src/app/MainWindowCadastral.cpp:42-46` |

- **Show-password checkbox**: None (`src/app/MainWindowCadastral.cpp:36-46`).

### 2.3 Button Actions, Input Validation, and Error Messages Verbatim
- **Cancel / window close (`dialog.exec() != QDialog::Accepted`)**: Writes nothing to disk and returns `false` (`src/app/MainWindowCadastral.cpp:47`).
- **Save (`dialog.Accepted`)**: Calls `CadastralPortal::saveCredentials({id->text(), password->text()}, &error)` (`src/app/MainWindowCadastral.cpp:48-49`).
  - **Input validation**: **None** in `configureCadastralAccount()` or `saveCredentials()`.
    - `username` is `.trimmed()` when written (`src/core/CadastralPortal.cpp:142`); `password` is **not** trimmed (`:143`).
    - Empty `username` and/or empty `password` (including half-filled: ID only or PW only) is **not** rejected at save time. If `password.isEmpty()`, `KaSecretStore::writePassword` calls `clearSecretKeys(settings, "vworld_account")` (removing `password`, `password_dpapi`, and `password_portable`) and returns `true` (`src/core/KaSecretStore.cpp:124-128, 141-144`). However, `CadastralPortal::hasCredentials()` requires both `!username.trimmed().isEmpty()` and `hasReadablePassword` (`src/core/CadastralPortal.cpp:129-133`), so a half-filled or cleared account counts as unconfigured on the next `downloadCadastral()` (`src/app/MainWindowCadastral.cpp:117`).
  - **On save success**: Dialog closes, no toast/status message is shown, and `configureCadastralAccount()` returns `true` (`src/app/MainWindowCadastral.cpp:49, 53`).
  - **On save failure**: Dialog has already closed (`exec()` returned `Accepted`); `KaUserError::warn` opens a modal `QMessageBox` (`src/app/MainWindowCadastral.cpp:50`, `src/app/KaUserError.cpp:65-116`) and `configureCadastralAccount()` returns `false` (`:51`):
    - **Window title**: `「VWorld 지적도 계정」` (`dialog.windowTitle()`, `src/app/MainWindowCadastral.cpp:50`)
    - **Heading (`what`)**: `「계정을 저장하지 못했습니다」` (`src/app/MainWindowCadastral.cpp:50`)
    - **Body (`why` = `error`, verbatim by failure path)**:
      1. Installed build DPAPI failure (`!protectBytes`): `「비밀번호를 암호화하지 못했습니다. 평문으로는 저장하지 않습니다.」` (`src/core/KaSecretStore.cpp:156-158`)
      2. Portable build encoding failure (`!writePortable`): `「비밀번호를 포터블 형식으로 저장하지 못했습니다.」` (`src/core/KaSecretStore.cpp:146-148`)
      3. `QSettings::sync()` error: `「계정 설정을 기록하지 못했습니다.」` (`src/core/CadastralPortal.cpp:145`)
      4. Temp dir or `QSaveFile::commit()` failure: `「VWorld 계정을 저장하지 못했습니다. 설정 폴더 권한을 확인하세요.」` (`src/core/CadastralPortal.cpp:153`)
    - **Buttons in `KaUserError::warn`**: Action `「로그 폴더 열기」` (if `KaUserError::logFolder()` is non-empty, `src/app/KaUserError.cpp:88-90`) and default/escape `「닫기」` (`src/app/KaUserError.cpp:97-100`).

### 2.4 Where the Account Is Stored and How the Password Is Protected
- **File path (`accountPath()`)**: `<KaPortableRuntime::userConfigDir()>/vworld-account.ini` (`src/core/CadastralPortal.cpp:28-31`), written in `QSettings::IniFormat` with `setFallbacksEnabled(false)` via a staged temp directory `<configDir>/.vworld-account-XXXXXX/account.ini` + `QSaveFile` atomic commit (`src/core/CadastralPortal.cpp:136-151`).
  - `KaPortableRuntime::userConfigDir()` resolves to (`src/core/KaPortableRuntime.cpp:359-374`):
    1. Bundled portable build (`paths.looksBundled()`): `<exeDir>/config` (`:361-364`).
    2. Installed build: `QStandardPaths::writableLocation(QStandardPaths::AppConfigLocation)` (`:366-369`).
    3. Fallback if empty: `<tempDir>/ka-hgis-config` (`:371-373`).
- **INI section (group) & key names**:
  - Group: `[vworld_account]` (`src/core/CadastralPortal.cpp:125-126, 142-143`).
  - Keys (`src/core/CadastralPortal.cpp:125, 142`; `src/core/KaSecretStore.cpp:95-105, 160-162`):
    - `vworld_account/username` — trimmed username string.
    - `vworld_account/password_dpapi` — Base64 DPAPI blob written in installed mode (`!KaSecretStore::usePortableSecrets()`, `src/core/KaSecretStore.cpp:100, 145, 160`).
    - `vworld_account/password_portable` — Base64 XOR-wrapped string written only in bundled portable mode (`KaSecretStore::usePortableSecrets()`, `src/core/KaSecretStore.cpp:104, 133-136, 145-150`).
    - `vworld_account/password` — legacy plaintext key; **never written**, deleted on save (`src/core/KaSecretStore.cpp:96, 125, 161`), and if encountered on `credentials()` (`readPassword(..., migrate = true)`), migrated in-place into `password_dpapi` (or `password_portable`) (`src/core/CadastralPortal.cpp:126`, `src/core/KaSecretStore.cpp:183-190`).
- **Password protection functions (`src/core/KaSecretStore.cpp`)**:
  - **Installed mode (default)**: `KaSecretStore::writePassword` (`:138-164`) -> `protectBytes` (`:47-68`) calls Win32 `CryptProtectData(&in, L"ka-hgis", &entropy, nullptr, nullptr, CRYPTPROTECT_UI_FORBIDDEN, &out)` with ASCII entropy `"ka-hgis-account-v1"` (`kEntropy`, `:25`), Base64-encodes the `DATA_BLOB`, frees `out.pbData` with `LocalFree`, and wipes UTF-8 plaintext bytes via `secureWipe` (`SecureZeroMemory`, `:32-39, 155`). Decryption uses `KaSecretStore::readPassword` (`:166-190`) -> `unprotectBytes` (`:70-92`) -> Win32 `CryptUnprotectData` with the same entropy + `CRYPTPROTECT_UI_FORBIDDEN` + `secureWipe`.
  - **Portable mode (`KaPortableRuntime::discover(...).looksBundled()`)**: `writePortable` (`:107-115`) -> `xorEntropy` (`:41-45`, repeating bytewise XOR against `"ka-hgis-account-v1"`) + Base64; `unwrapPortable` (`:117-122`) Base64-decodes and runs `xorEntropy`.

---

## Part 3 — The Colour and Lot-Number Dialog (`MainWindow::configureCadastralStyle`)

### 3.1 Guard When No Downloaded Cadastral Layer Exists Yet
- **Function**: `void MainWindow::configureCadastralStyle()` (`src/app/MainWindowCadastral.cpp:56-80`).
- **Layer lookup**: Scans `QgsProject::instance()->mapLayers()` for `QgsVectorLayer*` where `vector->customProperty("ka_hgis/cadastral").toBool()` is `true` (`src/app/MainWindowCadastral.cpp:57-59`). (Note: VWorld's raster WMS layer `"연속지적도"` is a `QgsRasterLayer` without `ka_hgis/cadastral`, so it does **not** satisfy this check.)
- **When `layers.isEmpty()`**: Opens a modal information box and returns immediately (`src/app/MainWindowCadastral.cpp:60`):
  - Call: `QMessageBox::information(this, QStringLiteral("지적도"), QStringLiteral("지적도를 먼저 받아 주세요."))`
  - Window title (verbatim): `「지적도」`
  - Body message (verbatim): `「지적도를 먼저 받아 주세요.」`
  - Button: Qt standard `QMessageBox::Ok`.

### 3.2 Window Title and Controls in Order (When Layer Is Present)
- **Window type & modality**: Modal `QDialog dialog(this)` with `QFormLayout(&dialog)` (note: unlike `configureCadastralAccount`, `KaFormPlate::dress` is **not** called here; it uses plain `QDialog` + `QFormLayout` sizing, `src/app/MainWindowCadastral.cpp:61-62`).
- **Window title (verbatim)**: `「지적도 선 색·지번」` (`src/app/MainWindowCadastral.cpp:61`).

| Order | Form slot | Label / text (verbatim) | Widget & colour choices | Initial / default value | Citation |
|---:|---|---|---|---|---|
| 1 | Row 0 (`label`, `field`) | Label: `경계선 0.2mm`<br>Button text: `color.name()` (`#rrggbb` lowercase hex, e.g. `"#000000"`) | `QPushButton` (`choose`). Clicking opens Qt's full colour picker `QColorDialog::getColor(color, &dialog, QStringLiteral("경계선 색"))` — **no fixed preset swatch list**; accepts **any valid 24-bit RGB `(0..255, 0..255, 0..255)`** offered by `QColorDialog` (Qt's standard basic/custom colour grid + HSV/RGB inputs). | `QColor` parsed from `layers.first()->customProperty("ka_hgis/cadastral_color", "#000000").toString()`. Freshly imported default is `Qt::black` = **RGB `(0, 0, 0)`** (`"#000000"` on the button; stored as HexArgb `"#ff000000"`). | `src/app/MainWindowCadastral.cpp:63-69`, `src/core/CadastralImport.h:25`, `src/core/CadastralImport.cpp:265` |
| 2 | Row 1 (spanning `QCheckBox`) | `지번 표시` | `QCheckBox` (`labels`) — lot-number visibility switch | `layers.first()->labelsEnabled()`. Freshly imported default is **`true` (checked)**. | `src/app/MainWindowCadastral.cpp:70-71`, `src/core/CadastralImport.h:25`, `src/core/CadastralImport.cpp:263` |
| 3 | Row 2 (spanning `QLabel`) | `지번은 1:10,000보다 확대했을 때 표시됩니다.` | `QLabel` (`hint`) | Static hint text | `src/app/MainWindowCadastral.cpp:72` |
| 4 | Row 3 (spanning `QDialogButtonBox`) | Qt standard buttons `QDialogButtonBox::Ok \| QDialogButtonBox::Cancel` (localized by Qt if loaded, else `"OK"` / `"Cancel"`; exact Korean string `not in the source`) | `QDialogButtonBox` (`accepted` -> `dialog.accept()`, `rejected` -> `dialog.reject()`) | `Ok` is default accept button | `src/app/MainWindowCadastral.cpp:73-75` |

### 3.3 What Happens at Once vs. What Happens on a Button
- **At once (inside the open dialog, before `Ok`)**:
  - Clicking `choose` opens modal `QColorDialog::getColor(color, &dialog, "경계선 색")` (`src/app/MainWindowCadastral.cpp:65-66`).
  - If the user cancels `QColorDialog` (`!selected.isValid()`), nothing changes (`:67`).
  - If the user confirms a colour in `QColorDialog` (`selected.isValid()`), the local variable `color` and the button's caption `choose->setText(color.name())` (`#rrggbb`) update immediately (`:67`), **without** modifying the map layer or canvas yet.
  - Toggling `지번 표시` only toggles `QCheckBox` state inside `dialog` (`:70`); the map does **not** live-preview.
- **On `Cancel` / dialog close (`dialog.exec() != QDialog::Accepted`)**:
  - Returns immediately (`src/app/MainWindowCadastral.cpp:76`); the layer's renderer, label colour, `labelsEnabled()`, `ka_hgis/cadastral_color`, and project dirty state remain untouched.
- **On `Ok` (`dialog.exec() == QDialog::Accepted`)**:
  1. Calls `CadastralImport::applyStyle(layer, color, labels->isChecked())` on **every** `QgsVectorLayer` with `ka_hgis/cadastral == true` (`src/app/MainWindowCadastral.cpp:77`):
     - Replaces renderer with a no-fill `QgsFillSymbol` (`style = "no"`, `outline_color = color.name(QColor::HexArgb)`, `outline_width = "0.2"`, `outline_width_unit = "MM"`, `src/core/CadastralImport.cpp:246-249`).
     - Replaces labeling with `QgsVectorLayerSimpleLabeling` on field `"JIBUN"` (`Malgun Gothic`, `8` pt, **label text colour = the same `color`**, `OverPoint`, `scaleVisibility = true`, `minimumScale = 10000.`, `maximumScale = 0.`) and sets `layer->setLabelsEnabled(labels)` (`src/core/CadastralImport.cpp:250-263`).
     - Writes `layer->setCustomProperty("ka_hgis/cadastral", true)` and `layer->setCustomProperty("ka_hgis/cadastral_color", color.name(QColor::HexArgb))` (`#ffrrggbb`, `src/core/CadastralImport.cpp:264-265`), then calls `layer->triggerRepaint()` (`:266`).
  2. Marks the workspace dirty: `QgsProject::instance()->setDirty(true)` (`src/app/MainWindowCadastral.cpp:78`).
  3. Refreshes the map canvas: `LayerOps::refreshCanvasIfIdle(m_canvas)` (`src/app/MainWindowCadastral.cpp:79`).

---

## Part 4 — A Second Press on `지적` With the Layer Already on the Map

Pressing `지적` **never** hides or toggles off `지적도 · 조사 주변 5km`; it always invokes `MainWindow::downloadCadastral()` (`src/app/MainWindowRibbon.cpp:474`, `src/app/MainWindowCadastral.cpp:82`).

### Case A — Pressed Again While a Reference Download Is Still Running
- **Check**: `if (m_referenceDownload)` (`src/app/MainWindowCadastral.cpp:84`).
- **What runs**: Nothing; returns immediately (`:84`).
- **What the user sees**: 5-second status-bar message `「지도 자료를 준비 중입니다. 완료를 기다리거나 취소하세요.」` (`src/app/MainWindowCadastral.cpp:84`), while the existing `KaDownloadProgressDialog` stays open.

### Case B — Pressed Again After `지적도 · 조사 주변 5km` Is Already on the Map

#### Step-by-step: What Runs Again vs. What Is Reused on Disk
1. **Runs again — Survey geometry union (UI thread)**: Re-reads all non-empty features from every `survey_area` vector layer, transforms them to `project->crs()`, unions them (`QgsGeometry::unaryUnion(parts)`), loads the VWorld API key (`VworldSettings::loadApiKey()`) and stored VWorld credentials (`CadastralPortal::credentials()`), and sets `request.directory = <surveyDir>/지적도` (`src/app/MainWindowCadastral.cpp:89-121`).
2. **Runs again — Progress dialog & background task**: Creates and `show()`s a new `KaDownloadProgressDialog("지적도 다운로드", "VWorld · 조사구역 경계에서 주변 5km의 지적도를 준비합니다.", this)` (`0..100`, initial `0%`) and starts `KaReferenceDownloadJob("조사 주변 지적도", ...)` (`src/app/MainWindowCadastral.cpp:122-125, 170-173`).
3. **Runs again — 5 km buffer & VWorld county query (network, `2%`)**: Buffers `request.survey` by `request.bufferMeters` (`5000.` m, `24` segments), transforms bbox to `EPSG:4326`, and queries `https://api.vworld.kr/req/data` (`service=data&data=LT_C_ADSIGG_INFO`) page by page for intersecting 5-digit `sig_cd` districts (`src/core/CadastralPortal.cpp:222-263`).
4. **Runs again — VWorld portal login & archive listing (network, `8%` -> `12%`)**: Fetches the dataset page, posts Base64-encoded credentials to `https://www.vworld.kr/v4po_usrlogin_a004.do`, and pages through `dtmk_ntads_s002.do?dsId=30563` (`datPageSize=100`, up to 10 pages) to obtain the current `fileNo`, `sizeKb`, `revision` (`YYYY-MM-DD`), and `LSMD_CONT_LDREG_*.zip` name for each `sig_cd` (`src/core/CadastralPortal.cpp:264-305`).
5. **Reused on disk (unless VWorld published a newer revision) — Raw county ZIP (`<surveyDir>/지적도/원본/`)**:
   - Checked file path: `<surveyDir>/지적도/원본/<fileNo>-<sizeKb>-<revision>-<name>` (`src/core/CadastralPortal.cpp:315-318`).
   - If that file exists **and** `TopographicArchive::prepare(path, originals.filePath("추출"), {}, cancel)` returns non-empty `unpacked.files` (`src/core/CadastralPortal.cpp:317-319`), the HTTP download to `/dtmk/downloadResourceFile.do?ds_id=30563&fileNo=...` is **skipped** (`:320-329`). If VWorld published a newer monthly archive (`fileNo`/`sizeKb`/`revision` changed), the new filename does not exist yet, so only the updated county ZIP is downloaded.
6. **Reused on disk — Extracted shapefile payload (`<surveyDir>/지적도/원본/추출/<sourceSha256>/`)**:
   - `TopographicArchive::prepare` hashes the ZIP bytes (`SHA-256`) to form `<surveyDir>/지적도/원본/추출/<sourceSha256>` (`src/core/TopographicArchive.cpp:215-232`).
   - Checked files (`cached()`, `src/core/TopographicArchive.cpp:169-212`):
     - `<extractDir>/manifest.json` (`version == 1`, `sourceSha256 == <sourceHash>`, `files` array of relative `path` + `size` + `sha256`)
     - `<extractDir>/payload/<relPath>` (verifies every listed file exists, is not a symlink/junction, matches `size` and `sha256`, and that `payload/` contains no unlisted files); `CadastralPortal::prepare` then verifies at least one `.shp` is present (`src/core/CadastralPortal.cpp:332-335`).
   - When valid, ZIP extraction is **skipped** (`src/core/TopographicArchive.cpp:210-211`).
7. **Reused on disk (when survey geometry, work CRS, transform context, and source `.shp`/`.dbf`/`.shx`/`.prj`/`.cpg` SHA-256s are unchanged) — Clipped GeoPackage (`<surveyDir>/지적도/표시/<identityHex>.gpkg`)**:
   - `CadastralImport::prepare` (`src/core/CadastralImport.cpp:87-116`) SHA-256 hashes `"cadastral-scope-v2"`, `scope.asWkb()`, `crs.toWkt().toUtf8()`, `QgsCoordinateTransformContext` XML, and each sorted `.shp` plus its sibling `.dbf`/`.shx`/`.prj`/`.cpg` paths and SHA-256 digests.
   - Checked files (`src/core/CadastralImport.cpp:116-131`):
     - Lock file `<surveyDir>/지적도/표시/<identityHex>.gpkg.lock` (`QLockFile`, `tryLock(0)`)
     - `<surveyDir>/지적도/표시/<identityHex>.gpkg`
     - `<surveyDir>/지적도/표시/<identityHex>.gpkg.sha256` (matching `hashFile(path, cancel)`), plus opening `<path>|layername=cadastral` via OGR and verifying `cached.isValid() && cached.crs() == crs && cached.fields().indexOf("JIBUN") >= 0 && cached.featureCount() > 0`.
   - When valid, polygon filtering/reprojection/writing is **skipped** and progress emits `100%`, `「저장된 조사 주변 지적도를 사용합니다.」` (`src/core/CadastralImport.cpp:126-128`).
8. **Layer replacement with style preservation (`CadastralImport::addPrepared`, `src/core/CadastralImport.cpp:269-294`)**:
   - Opens `<identityHex>.gpkg|layername=cadastral` as a new `QgsVectorLayer` named `「지적도 · 조사 주변 5km」` (`:272-273`).
   - Scans `project->mapLayers()` for any existing layer whose `name() == "지적도 · 조사 주변 5km"` (`:279-285`), records its ID in `previous`, and copies `retainedColor` (`customProperty("ka_hgis/cadastral_color", "#000000")`) and `retainedLabels` (`labelsEnabled()`) onto the new layer via `applyStyle(layer.get(), retainedColor, retainedLabels)` (`:283-286`). *(Edge case: if the user renamed the existing layer away from `"지적도 · 조사 주변 5km"`, `:281` does not match it, so the renamed layer is not removed and the new layer starts at `#000000` + `labels = true`.)*
   - Marks `ka_hgis/layer_role = "cadastral"` (`markCadastralLayer`, `:287`), clears `ka_hgis/user_removed_cadastral` (`:288`), inserts the new layer at root index `0` (`placeCadastralLayer`, `:290`), calls `prepared.retainFiles()`, removes `previous` (`project->removeMapLayers(previous)`, `:291`), and refreshes the canvas (`:292`).

#### What the User Sees on a Second Press
1. Non-modal progress window (`setModal(false)`) titled `「지적도 다운로드」` opens at `0%` with subtitle `「VWorld · 조사구역 경계에서 주변 5km의 지적도를 준비합니다.」`, card heading `「자료 준비 및 다운로드」`, hint `「다운로드 중에도 지도 작업을 계속할 수 있습니다.」`, and `「취소」` button (`src/app/MainWindowCadastral.cpp:122-125`, `src/app/KaDownloadUi.cpp:38-57`).
2. Progress advances through:
   - `2%`: `「조사 주변 5km의 시·군·구를 확인하고 있습니다…」` (`src/core/CadastralPortal.cpp:229`)
   - `8%`: `「VWorld 계정을 확인하고 있습니다…」` (`src/core/CadastralPortal.cpp:264`)
   - `12%`: `「해당 시·군·구의 지적도 파일을 찾고 있습니다…」` (`src/core/CadastralPortal.cpp:293`)
   - `15%..65%`: skipped immediately when all county ZIPs & extracted payloads are cached on disk; otherwise streams download progress (`src/core/CadastralPortal.cpp:316-328`)
   - `65%`: `「5km 범위의 경계선과 지번을 정리하고 있습니다…」` (`src/core/CadastralPortal.cpp:338`)
   - `100%` (`65 + 100 * 35 / 100`): `「저장된 조사 주변 지적도를 사용합니다.」` when `<identityHex>.gpkg` is reused (`src/core/CadastralImport.cpp:127`, `src/core/CadastralPortal.cpp:341`) — or `65%..96%` `「조사 주변 필지와 지번을 정리하고 있습니다…」` -> `100%` `「주변 필지 %1개를 준비했습니다.」` when rebuilt (`src/core/CadastralImport.cpp:155, 240`).
3. Progress window hides and deletes itself (`src/app/MainWindowCadastral.cpp:135`), `지적도 · 조사 주변 5km` stays at the top of the legend with its user-chosen line colour and lot-number switch preserved (`src/core/CadastralImport.cpp:283-291`), becomes the selected tree row (`src/app/MainWindowCadastral.cpp:166`), marks the project dirty (`*`, `:167`), and shows the 10-second status-bar message:
   - `「조사 주변 5km 지적도를 추가했습니다. 지적도 옆 메뉴에서 선 색·지번 표시를 바꿀 수 있습니다.」` (`src/app/MainWindowCadastral.cpp:168`).

---

## Part 5 — The Legend's Right-Click Menu for `지적도 · 조사 주변 5km`

### 5.1 How `MainWindow::showLayerTreeContextMenu` Classifies Both Layers
- **Cadastral layer (`지적도 · 조사 주변 5km`)**: `QgsVectorLayer` (`Polygon`), OGR GeoPackage `<surveyDir>/지적도/표시/<identityHex>.gpkg|layername=cadastral`, `ka_hgis/cadastral = true`, `ka_hgis/layer_role = "cadastral"` (`src/core/CadastralImport.cpp:264, 272-273, 287`; `src/core/LayerOps.cpp:1058-1060`).
  - `LayerOps::isCadastralLayer(layer)` = `true` (`src/core/LayerOps.cpp:1062-1067`); `LayerOps::isReferenceLayer(layer)` = `false` (`:1050-1056`, checks `layer_role == "reference"`).
  - `menuKind(layer)` returns **`LayerMenuKind::ExternalVector`** (`src/app/MainWindowContextMenus.cpp:61-62`), `domain = false` (`:221`), and `editableKind = false` because `:412-413` explicitly requires `!LayerOps::isCadastralLayer(vector)`.
  - Tree parent (`node->parent()`): **`layerTreeRoot()`** (top-level row at index `0`, outside `"참조 지도"`, `src/core/LayerOps.cpp:1091-1101`).
- **Ordinary reference vector layer** (`ka_hgis/layer_role = "reference"`, e.g. `수계망`, `토양도`, `지질도`, `수치지형도`, `주변유적`):
  - `LayerOps::isReferenceLayer(layer)` = `true`; `menuKind(layer)` returns **`LayerMenuKind::Reference`** (`src/app/MainWindowContextMenus.cpp:61`), `domain = false`, `editableKind = false` (`:412`).
  - Tree parent: `"참조 지도"` group (`QgsLayerTreeGroup`), or a subgroup under `"참조 지도"` (`src/core/LayerOps.cpp:1070-1075`). (Raster/tile reference layers such as `대동여지도`, `1919 조선지형도`, `DEM` also have `layer_role = "reference"` with `vector == nullptr`.)

### 5.2 Every Menu Row in Order: `지적도 · 조사 주변 5km` vs. Ordinary Reference Layer

All rows come from `MainWindow::showLayerTreeContextMenu` (`src/app/MainWindowContextMenus.cpp:176-448`) and `LayerLabelControls::describe` (`src/core/LayerLabelControls.cpp:85-153`):

| # | Action ID & Row Text (verbatim) | `지적도 · 조사 주변 5km` (`ka_hgis/cadastral = true`) | Ordinary Reference Layer (`ka_hgis/layer_role = "reference"`) | Citation |
|---:|---|---|---|---|
| 1 | `layer.import` — `레이어 불러오기 (SHP·DXF·GPKG)` | **Offered** (enabled unless `m_isOpeningSurvey`) -> `addUserLayer()` | **Same** — offered | `src/app/MainWindowContextMenus.cpp:368` |
| — | *(separator)* | Shown | Shown | `:369` |
| 2 | `layer.rename` — `이름 바꾸기 (두 번 클릭)` | **Offered** (enabled) -> `renameSelectedLayer(layer)` (also on double-click, `src/app/MainWindow.cpp:1676-1683`). *(Note: renaming breaks name-matching `"지적도 · 조사 주변 5km"` on a subsequent `addPrepared`, `src/core/CadastralImport.cpp:279-282`.)* | **Same** — offered | `:283`, `src/app/MainWindow.cpp:1634-1683` |
| 3 | `layer.style` — `면·외곽선 색` | **Offered** (enabled) -> `editCurrentLayerStyle` checks `ka_hgis/cadastral == true` and opens **`configureCadastralStyle()` (`「지적도 선 색·지번」`)** (`0.2` mm fixed width + colour + `지번 표시`) | **Different target**: offered (enabled for vector; disabled on raster with `"영상 지도는 투명도로 표시를 조절하세요."`), calls **`KaLineColorDialog::edit`** (`「선 색」`, colour + `0.1..5.0` mm width + `원래 색으로`) for line-map layers (`LineMapStyle::isLineMap`), or **refuses** with `KaUserError::warn` `「「%1」은 색을 바꾸지 않습니다.」` on filled-polygon (`토양도`, `지질도`) / point reference layers | `:284-285`, `src/app/MainWindowEditing.cpp:443-449`, `src/app/KaLineColorDialog.cpp:48-75` |
| 4 | `layer.attributes` — `도형 속성` | **Offered** (enabled on OGR `.gpkg` with `ChangeAttributeValues` capability and `featureCount > 0`), starts feature picker (`editCurrentLayerAttributes`), then `editFeatureAttributes` **blocks editing** with `KaUserError::warn`: `「「%1」의 속성은 고칠 수 없습니다.」` / `「참조 지도와 지적도는 읽기 전용입니다. 조사 원본을 지키기 위해 막아 둡니다.」` | **Same** on OGR `.gpkg`/`.shp` reference vectors (blocked by the exact same `!KaAttributeMapTool::isEditableLayer` guard in `editFeatureAttributes`); **disabled** on read-only providers (`GeoJSON` without `ChangeAttributeValues`) or rasters (`"영상 지도에는 편집할 도형 속성이 없습니다."`) | `:270-275, 286`, `src/app/MainWindowEditing.cpp:369-406, 621-629` |
| 5 | `layer.labels` — `라벨·명칭 속성` ▸ | **Offered & enabled** (`special = true`, `caption = "지번"`, `supported = true`; tooltip at `> 1:10,000` when scale passed: `"확대하면 지번 표시 (1:10000부터)"`):<br>• `layer.labelsVisible`: **`지번 숨기기`** (when `labelsEnabled()`) / **`지번 보이기`** (when off) — **enabled**<br>• `layer.labelField`: **`표시할 항목`** ▸ (`PNU`, `JIBUN` checked) — **disabled** (`fieldEditable = false` because `special = true`)<br>• `layer.labelArea`: **Withheld** (`areaEditable = false` because `special = true`)<br>• `layer.labelSize`: **`글자 크기`** ▸ (`3 pt`..`14 pt`, **`8 pt` checked**) — **enabled** | **Different**:<br>• Raster reference: submenu **disabled** (`"영상에 포함된 글자는 별도로 크기를 바꿀 수 없습니다."`)<br>• Vector reference: depends on schema/labeling — e.g. `주변유적` (`heritage = true`, `special = false`): `유적명 숨기기/보이기` enabled, **`표시할 항목` enabled**, `면적(㎡) 함께 표시` withheld (`!heritage`), `글자 크기` enabled on map / disabled on layout; `토양도`/`수계망`/`지질도(기호)`/`수치지형도`: `special = true` (`지형명`/`하천명`/`기호`/`높이`), `표시할 항목` disabled, `면적(㎡) 함께 표시` withheld; unlabelled vector without `knownField`: submenu **disabled** (`"표시할 이름이나 번호 항목이 없습니다."`) | `:287-348`, `src/core/LayerLabelControls.cpp:85-153` |
| 6 | `layer.encoding` — `깨진 한글 바로잡기` ▸ | **Withheld** (source is `.gpkg`, not `.shp`) | **Offered** (`CP949`, `EUC-KR`, `UTF-8`, `이 컴퓨터 설정`) **only when** reference vector source ends with `.shp`; withheld for `.gpkg` / GeoJSON / raster | `:349-354` |
| 7 | `layer.opacity` — `투명도` ▸ | **Offered & enabled** (`0%`, `20%`, `40%`, `50%`, `60%`, `80% 투명하게`; `0% 투명하게` checked at opacity `1.0`) | **Same** — offered & enabled for all valid vector and raster reference layers (`LayerStyleOpacity.cpp:6-20`) | `:371-393`, `src/core/LayerStyleOpacity.cpp:6-29` |
| — | *(separator)* | Shown | Shown | `:394` |
| 8 | `layer.zoom` — `이 레이어로 이동` | **Offered** (enabled when `featureCount > 0`) -> zooms canvas to the 5 km cadastral layer extent (`LayerOps::zoomToLayerMax`) | **Offered** (zooms to vector extent, or `1:50,000` if global XYZ/WMS raster) | `:248-260, 395` |
| 9 | `layer.fullExtent` — `전체 보기` | **Offered** (always enabled) -> `zoomMapToFullMax()` | **Same** — offered | `:396` |
| 10 | `layer.bottom` — `같은 묶음의 맨 아래로` | **Offered** — operates on `node->parent()` = **`layerTreeRoot()`**: enabled when any tree node (e.g. `"조사 데이터"`, `"참조 지도"`, `"배경지도"`) sits below it, moving `지적도 · 조사 주변 5km` to the **very bottom of the entire layer tree** | **Different scope**: operates on `"참조 지도"` (or its subgroup), moving the layer only to the bottom of `"참조 지도"` (stays above `"배경지도"`) | `:397-408` |
| — | *(separator)* | Shown (`polygon == true`) | Shown **only if** `polygon == true` (`토양도`, `지질도`, polygon `주변유적`); **withheld** for line/point/raster reference layers | `:411-414` |
| 11 | `layer.draw` — `그리기·편집 시작` | **Withheld** (`!LayerOps::isCadastralLayer(vector)` makes `editableKind = false`) | **Same** — withheld (`kind != LayerMenuKind::Reference && !isReferenceLayer`) | `:412-416` |
| 12 | `layer.vertices` — `꼭짓점 수정` | **Withheld** (`editableKind == false`) | **Same** — withheld | `:412-417` |
| 13 | `layer.merge` — `폴리곤 묶기` | **Withheld** (`editableKind == false`) | **Same** — withheld | `:412, 419-421` |
| 14 | `layer.area` — `면적 확인` | **Offered** (`polygon == true`; enabled when `featureCount > 0`) -> `showLayerAreaSummary(vector, false)` opens `QMessageBox::information` titled `「면적 확인」` with `<layerName>\n도형 N개 · 합계 면적 X ㎡` | **Offered only for Polygon** reference layers (`토양도`, `지질도`, polygon `주변유적`); **withheld** for Line (`수계망`, `등고선`), Point, and Raster reference layers | `:356-359, 411, 422, 450-481` |
| 15 | `layer.trenchCreate` — `시굴격자` ▸ | **Withheld** (`kind != LayerMenuKind::Area`) | **Same** — withheld | `:423-431` |
| 16 | `layer.offline` — `오프라인 저장 — 지금 화면 범위` | **Withheld** (`providerType == "ogr"`, not XYZ `wms`) | **Withheld** for vector/GDAL reference layers; **offered** on XYZ tile reference layers (`대동여지도`, `1919 조선지형도`) | `:432-433` |
| 17 | `layer.export` — `제출 꾸러미 만들기…` | **Withheld** (`domain == false`) | **Same** — withheld (`domain == false`) | `:360-366, 434` |
| — | *(separator)* | Shown | Shown | `:435` |
| 18 | `layer.clear` — `그린 도형 모두 지우기…` | **Withheld** (`domain == false`) | **Same** — withheld (`domain == false`) | `:436-440` |
| 19 | `layer.remove` — `레이어 삭제` | **Offered** (enabled unless `m_isOpeningSurvey`; tooltip `"목록에서 제거합니다. 원본 파일은 보존하며 Ctrl+Z로 복원할 수 있습니다."`) -> `removeLayersFromTree` removes the row onto the undo stack (`Ctrl+Z` restores it; `userRemovedCadastral` is **not** set because that flag applies only to VWorld's WMS `"연속지적도"`, `src/app/MainWindowUndo.cpp:200-204`, `tests/test_save_open.cpp:3098-3114`) | **Same** — offered & restorable with `Ctrl+Z` | `:441-446`, `src/app/MainWindowUndo.cpp:158-255` |

---

## Part 6 — Persistence Across Survey Save and Reopen

### 6.1 What Is Stored Where in C++

| Artefact / Location | Exact file / table / property | What is stored | When it is written | Citation |
|---|---|---|---|---|
| Personal config INI | `<userConfigDir>/vworld-account.ini` (`[vworld_account]`) | `vworld_account/username`, `vworld_account/password_dpapi` (or `password_portable` in bundled portable build) | Immediately on clicking `Save` in `VWorld 지적도 계정` | `src/core/CadastralPortal.cpp:29, 135-155`, `src/core/KaSecretStore.cpp:138-164` |
| Raw county archives & unpacked shapefiles | `<surveyDir>/지적도/원본/<fileNo>-<sizeKb>-<revision>-<name>` and `<surveyDir>/지적도/원본/추출/<sourceSha256>/{manifest.json, payload/*}` | Downloaded VWorld county ZIPs and verified extracted `.shp`/`.dbf`/`.shx`/`.prj`/`.cpg` | During `CadastralPortal::prepare` (`downloadCadastral`) | `src/core/CadastralPortal.cpp:306-336`, `src/core/TopographicArchive.cpp:169-232` |
| Clipped parcel GeoPackage (external to `<survey>.gpkg`) | `<surveyDir>/지적도/표시/<identityHex>.gpkg` (table `cadastral`: `fid`, MultiPolygon geometry in work CRS, `PNU`, `JIBUN`) + `<identityHex>.gpkg.sha256` | Clipped 5 km parcel polygons and attributes only (**no style/colour XML is written inside `<identityHex>.gpkg`**, and `absorbExternalVectors` explicitly **skips** copying `isCadastralLayer` into `<survey>.gpkg`) | During `CadastralImport::prepare`, kept via `prepared.retainFiles()` in `addPrepared` | `src/core/CadastralImport.cpp:116-144, 291`, `src/core/SurveyStorage.cpp:742-749` |
| In-memory `QgsVectorLayer` custom properties, renderer & labeling | `QgsProject::instance()` layer entry (`source = .../지적도/표시/<identityHex>.gpkg\|layername=cadastral`) | • `ka_hgis/cadastral = true`<br>• `ka_hgis/cadastral_color = "#ffrrggbb"` (`QColor::HexArgb`)<br>• `ka_hgis/layer_role = "cadastral"`<br>• `QgsSingleSymbolRenderer`: no fill, `outline_color` (`#ffrrggbb`), `0.2` mm<br>• `QgsVectorLayerSimpleLabeling`: field `"JIBUN"`, `Malgun Gothic` `8` pt in `color`, `minimumScale = 10000.`, `labelsEnabled()` (`true`/`false`)<br>• Layer name, visibility checkbox, opacity (`mapLayerOpacity`), and root tree position (index `0`) | In memory on `addPrepared`, `configureCadastralStyle` (`Ok`), or legend menu edits (each marks `QgsProject::setDirty(true)`) | `src/core/CadastralImport.cpp:246-266, 286-290`, `src/core/LayerOps.cpp:1058-1060, 1091-1101`, `src/app/MainWindowCadastral.cpp:77-78, 167` |
| Saved QGIS workspace (two copies written on save) | 1. Embedded SQLite table `qgis_projects` inside `<survey>.gpkg` (`writeEmbedded`)<br>2. Companion file `<survey>.qgz` beside `<survey>.gpkg` (`kaWriteQgisProjectAtomic`) | Full `QgsProject` XML, including the OGR layer datasource pointing to `지적도/표시/<identityHex>.gpkg\|layername=cadastral`, renderer XML, labeling XML (`labelsEnabled`, font size, colour), custom properties (`ka_hgis/cadastral`, `ka_hgis/cadastral_color`, `ka_hgis/layer_role`), layer visibility, opacity, custom layer name, and layer-tree order | **Only when the user saves the survey** (`저장` / `Ctrl+S` / save-on-close -> `MainWindow::persistSurveyWork` -> `SurveySession::persistWork` -> `SurveyStorage::persistWorkspace` + `writeEmbedded`) | `src/app/MainWindowSession.cpp:945-955`, `src/core/SurveySession.cpp:71-100`, `src/core/SurveyStorage.cpp:971-993, 1179-1210` |

### 6.2 What a Freshly Opened Survey Shows Without a New Download
1. **Survey saved (`Ctrl+S` / `저장`) after downloading and/or changing colour & lot-number switch**:
   - `MainWindow::openSurveyGpkg` (`src/app/MainWindowSession.cpp:284-345`) reads the embedded workspace from `<survey>.gpkg` first (`1순위: SurveyStorage::readEmbedded`, `:291-305`), falling back to companion `<survey>.qgz`/`.qgs` (`kaSafeReadQgisProject`, `:322-338`) only if embedded is absent or unreadable, then runs `finishOpenedProject` (`src/app/MainWindowSession.cpp:149-236`).
   - If the survey folder was moved/copied to another path or PC together with its `지적도/표시/<identityHex>.gpkg` subfolder, `LayerOps::repairPersistedFileSources` (`src/app/MainWindowSession.cpp:154`, `src/core/LayerOps.cpp:2127-2202`) rebases the broken path into the current `<surveyDir>/지적도/표시/<identityHex>.gpkg|layername=cadastral` (`SurveyBundle::rebaseIntoSurvey`, `:2179-2190`) while preserving renderer, labeling, and custom properties.
   - `ensureDefaultBasemaps` sees an existing cadastral layer (`ka_hgis/cadastral`), and `finishOpenedProject` syncs the canvas (`src/app/MainWindowSession.cpp:178-193`).
   - **Result**: Immediately on opening the survey — **with zero network requests and without pressing `지적`** — `「지적도 · 조사 주변 5km」` appears at its saved position in the legend (root top by default), checked/unchecked as saved, with its saved line colour, lot-number colour, label font size, opacity, and `지번 표시` on/off state intact.
2. **Survey closed *without* saving after downloading or changing style**:
   - `<surveyDir>/지적도/원본/` and `<surveyDir>/지적도/표시/<identityHex>.gpkg` remain on disk, but the unsaved `QgsProject` state was discarded; reopening `<survey>.gpkg` shows whatever workspace state was last saved (i.e. no cadastral layer if never saved, or the previously saved colour/label state).

---

## Part 7 — Rust Today (`8739aff`)

Classification key per `ORDER-CADMENU0.yaml:39`:
- `SAME`: Matches C++ behaviour and text.
- `DIFFERENT`: Works differently from C++ (exact difference stated).
- `MISSING`: Not implemented in Rust.
- `FAKE`: UI control or message claims or appears to perform an action, while the code at the cited line does not perform it on `지적도 · 조사 주변 5km`.

### 7.1 Part 1 Items (`지적` Button & Menu)

| # | Item (Part 1) | Status | Rust `file:line` & Explanation |
|---:|---|---|---|
| 1.1 | Ribbon button `"지적"` in `"자료 받기"` group, tooltip `"조사구역 주변 5km 지적도를 받아 경계선과 지번을 표시합니다. 우클릭: 계정·선 색 설정"`, left-click runs download | `SAME` | `crates/strata-app/src/gui_ribbon_top.rs:162-176`, `crates/strata-app/src/ribbon_icons_map.rs:54`, `crates/strata-app/src/gui_ribbon_click_a.rs:159` (`self.press_cadastral()`) |
| 1.2a | Open `지적` menu via **dropdown arrow** (`QToolButton::MenuButtonPopup`) | `MISSING` | `crates/strata-app/src/gui_ribbon_draw.rs:74, 212-217`, `crates/strata-app/src/gui_ribbon_top.rs:162-180` — `draw_ribbon_chip` draws no dropdown arrow and allocates a single click rect (`clicked` -> `handle_ribbon_click("지적")`) |
| 1.2b | Open `지적` menu via **right-click** | `SAME` | `crates/strata-app/src/gui_ribbon_draw.rs:216-217`, `crates/strata-app/src/gui_ribbon_top.rs:178-180` -> `open_ribbon_context_menu("지적", ...)` (`crates/strata-app/src/gui_ribbon_menu_actions.rs:27`) |
| 1.3a | Menu row 1 text `"VWorld 계정 설정"`, never disabled | `SAME` | `crates/strata-app/src/gui_ribbon_menus_a.rs:164-175` |
| 1.3b | Menu row 1 target (`configureCadastralAccount` single-purpose dialog) | `DIFFERENT` | `crates/strata-app/src/gui_ribbon_menus_a.rs:180-181` opens the combined 5-service `ActiveModalDialog::AccountSettings` instead of a dedicated `VWorld 지적도 계정` dialog |
| 1.3c | Menu row 2 text `"선 색·지번 표시"`, never disabled, calls cadastral style handler | `SAME` | `crates/strata-app/src/gui_ribbon_menus_a.rs:164-175, 182-184` calls `self.open_cadastral_style()` (`crates/strata-app/src/cadastral_layer.rs:94`). *(Note: at `845f290`, `:183` opened `ActiveModalDialog::CadastralDownload`; fixed in `8739aff`.)* |
| 1.4 | Secondary entry point in `더보기`: `"VWorld 지적도 아이디·비밀번호"` | `DIFFERENT` | `crates/strata-app/src/gui_ribbon_menus_b.rs:82, 114` offers a single combined row `"API 키 입력 · 계정 설정…"` -> `ActiveModalDialog::AccountSettings` |

### 7.2 Part 2 Items (Account Dialog & Credential Storage)

| # | Item (Part 2) | Status | Rust `file:line` & Explanation |
|---:|---|---|---|
| 2.1 | Dedicated modal window titled `"VWorld 지적도 계정"` (width `440`, 2 fields only) | `DIFFERENT` | `crates/strata-app/src/modal_paper_account.rs:148-154` renders `"API 키 · 사이트 계정 설정 - Strata"` (width `520.0`), combining 2 API keys and 3 site accounts in one grid (`:162-249`) |
| 2.2 | Labels `"아이디"` and `"비밀번호"` on two separate rows | `DIFFERENT` | `crates/strata-app/src/modal_paper_account.rs:198-214` renders one horizontal row labeled `"VWorld 지적도 ID / PW:"` with side-by-side username (`140.0` px) and password (`188.0` px) boxes; pre-filled from `read_from_dir` (`crates/strata-app/src/dialogs_account.rs:56-57`) |
| 2.3 | Password masking & show-password toggle | `DIFFERENT` | `crates/strata-app/src/modal_paper_account.rs:210, 265` masks with `.password(!show_pw)` by default (`false`) **and** adds a `"비밀번호 표시"` checkbox (`:265`), which C++ does not have |
| 2.4 | Hint `"이 PC의 개인 설정에 저장하여 다음 다운로드에 사용합니다."` | `DIFFERENT` | `crates/strata-app/src/modal_paper_account.rs:252-258` shows the NGII hint `"국토지리정보원 계정을 이 PC에 저장하면 수치지형도를 받을 때 자동 로그인에 사용합니다. 두 항목을 모두 비우고 저장하면 저장된 자동로그인 계정을 해제합니다."` instead of C++'s VWorld hint |
| 2.5 | Buttons `Save` / `Cancel` | `DIFFERENT` | `crates/strata-app/src/modal_paper_account.rs:267-276` uses `"저장"` and `"닫기"` (plus header `×`); `"닫기"` closes `modals.active` (`:268`) **without** reverting in-memory `app.accounts` edits made before closing |
| 2.6 | Validation on half-filled ID/PW | `DIFFERENT` | `crates/strata-app/src/dialogs_account.rs:87-101` (`validate_credentials`) blocks saving if only one of ID/PW is non-empty with inline red text `"아이디와 비밀번호를 모두 입력하거나 두 항목을 모두 비워 주세요."` (`crates/strata-app/src/modal_paper_account.rs:260-262, 276`); C++ does not block half-filled saves. Also `AccountSettings::save_to_file` (`crates/strata-secrets/src/accounts.rs:97`) does **not** `.trim()` `username` before writing |
| 2.7 | Save-success feedback & save-failure error UI | `DIFFERENT` | `crates/strata-app/src/modal_paper_account.rs:277-287`, `crates/strata-app/src/dialogs_account.rs:156-161` — on success, sets status text `"API 키와 사이트 계정(DPAPI 암호화)을 저장했습니다."` (C++ sets none); on failure, keeps the dialog open and shows inline red text (`"비밀번호를 암호화하지 못했습니다. 평문으로는 저장하지 않습니다."` or `"계정 설정을 기록하지 못했습니다. 저장 공간과 폴더 권한을 확인하세요."`) instead of closing and opening `KaUserError::warn` (`"계정을 저장하지 못했습니다"`) |
| 2.8 | INI file `vworld-account.ini`, group `[vworld_account]`, keys `vworld_account/username` & `vworld_account/password_dpapi`, DPAPI parameters (`CryptProtectData`, `"ka-hgis"`, entropy `"ka-hgis-account-v1"`, `CRYPTPROTECT_UI_FORBIDDEN`) | `SAME` | `crates/strata-secrets/src/accounts.rs:18, 34, 86-104`, `crates/strata-secrets/src/secret_store.rs:66-94`, `crates/strata-secrets/src/dpapi_win.rs:31-42` (invokes `CryptProtectData` via stdin-fed PowerShell P/Invoke with identical description, entropy, and flags) |
| 2.9 | Portable-build `vworld_account/password_portable` write & legacy plaintext migration on read | `DIFFERENT` | `crates/strata-app/src/dialogs_account.rs:156` always passes `portable = false` (`save_to_file` never auto-detects a bundled portable layout, though `read_password_from_map` at `crates/strata-secrets/src/secret_store.rs:101-105` can read `password_portable`), and `read_password_from_map` (`:112`) reads legacy `password` without migrating it to DPAPI on disk |

### 7.3 Part 3 Items (`지적도 선 색·지번` Dialog)

| # | Item (Part 3) | Status | Rust `file:line` & Explanation |
|---:|---|---|---|
| 3.1 | No-layer guard when `지적도 · 조사 주변 5km` is not on the map (`"지적도를 먼저 받아 주세요."`) | `DIFFERENT` | `crates/strata-app/src/cadastral_layer.rs:23, 94-97` checks `self.ref_vectors.spec_of("cadastral_near")` and shows `"지적도를 먼저 받아 주세요."` on the **status bar** (`show_status_message`), whereas C++ opens a modal `QMessageBox::information` titled `"지적도"` |
| 3.2 | Window title `"지적도 선 색·지번"`, label `"경계선 0.2mm"`, checkbox `"지번 표시"`, hint `"지번은 1:10,000보다 확대했을 때 표시됩니다."`, buttons `"확인"` / `"취소"`, defaults (`#000000` RGB `(0, 0, 0)`, `labels = true`) | `SAME` | `crates/strata-app/src/cadastral_layer.rs:36-41, 118-147` *(added in `8739aff`; at `845f290` this dialog was `MISSING`)* |
| 3.3 | Colour picker control (`QPushButton(color.name())` -> modal `QColorDialog` titled `"경계선 색"`) | `DIFFERENT` | `crates/strata-app/src/cadastral_layer.rs:129-133` renders an inline `egui` `color_edit_button_srgb` swatch (hover tooltip `"경계선 색"`) + separate `#rrggbb` label instead of a button opening a titled modal colour dialog |
| 3.4 | Live-preview isolation (edits apply only on `"확인"`, discarded on `"취소"`), applying both `0.2` mm outline colour and `8` pt `JIBUN` label colour (`1:10,000`) | `SAME` | `crates/strata-app/src/cadastral_layer.rs:45-53, 99, 104-107, 139-145`, `crates/strata-app/src/ref_vector.rs:159-174` |
| 3.5 | Marking survey workspace dirty (`*`) on `"확인"` | `DIFFERENT` | `crates/strata-app/src/cadastral_layer.rs:104-113` updates `ref_vectors` and `지적도/strata-kept.json` immediately on `"확인"` **without** setting `self.is_dirty = true` (C++ calls `QgsProject::instance()->setDirty(true)` at `src/app/MainWindowCadastral.cpp:78`) |

### 7.4 Part 4 Items (Second Press on `지적` With Layer on the Map)

| # | Item (Part 4) | Status | Rust `file:line` & Explanation |
|---:|---|---|---|
| 4.1 | In-flight guard (`"지도 자료를 준비 중입니다. 완료를 기다리거나 취소하세요."`) | `SAME` | `crates/strata-app/src/cadastral_run.rs:36, 142-144` |
| 4.2 | Re-runs `survey_area` union, 5 km buffer, `LT_C_ADSIGG_INFO` query, VWorld login, and archive list check; reuses `<surveyDir>/지적도/원본/<fileNo>-<sizeKb>-<revision>-<name>` and `<surveyDir>/지적도/원본/추출/<sourceSha256>/{manifest.json, payload/*}` | `SAME` | `crates/strata-app/src/cadastral_run.rs:148-179` (`kept_table` shortcut at `:154-159` runs **only** when the row is *not* on the map), `crates/strata-refmap/src/cadastral_portal.rs:176-265` |
| 4.3 | Clipped `.gpkg` cache identity in `<surveyDir>/지적도/표시/` | `DIFFERENT` | `crates/strata-refmap/src/cadastral.rs:64-81, 101-136, 237-266` checks `.gpkg.lock` (`Lock::take`, 30 s stale timeout) and `.gpkg.sha256` (`finished`), and hashes `.shp`/`.dbf`/`.shx`/`.prj`/`.cpg` digests just like C++, **except** its prefix/inputs (`"strata-cadastral-scope-v1"` + ring `f64` bytes + `"EPSG:{work_epsg}"`) differ from C++ (`"cadastral-scope-v2"` + `scope.asWkb()` + `crs.toWkt()` + `context.writeXml`), so C++ and Rust write distinct `<identity>.gpkg` filenames in `지적도/표시/` |
| 4.4 | Progress UI while running (`KaDownloadProgressDialog` window vs. bottom gauge) | `DIFFERENT` | `crates/strata-app/src/cadastral_run.rs:4, 176, 196-209` — owner decision (`docs/RUST_PLAN.md` §2.8): opens **no** progress dialog window; drives the bottom download gauge `"조사 주변 지적도"` with matching stage strings + status-bar `"VWorld · 조사구역 경계에서 주변 5km의 지적도를 준비합니다."` |
| 4.5 | Missing or refused account during `지적` press (`configureCadastralAccount` + auto-run / `"아이디·비밀번호 다시 입력"` retry) | `DIFFERENT` | `crates/strata-app/src/cadastral_run.rs:40, 167-171, 238-244` — when credentials are empty, opens `ActiveModalDialog::AccountSettings` + status `"지적도 메뉴에서 VWorld 계정을 설정하세요."` without continuing automatically after save; when VWorld rejects login (`Why::AccountRefused`), sets gauge `ACC-2` and opens `ActiveModalDialog::AccountSettings` directly without C++'s `KaUserError::warn` `"아이디·비밀번호 다시 입력"` prompt or automatic re-run |
| 4.6 | Preserving style & label switch on replacement + completion status text `"조사 주변 5km 지적도를 추가했습니다. 지적도 옆 메뉴에서 선 색·지번 표시를 바꿀 수 있습니다."` | `DIFFERENT` | `crates/strata-app/src/cadastral_run.rs:33, 229-233`, `crates/strata-app/src/cadastral_layer.rs:86-90` — preserves colour & label switch (keyed by `layer_key == "cadastral_near"` rather than display name, so it **also** preserves a user-renamed row) and shows the exact status text, **except** it does not select the row in `right_inspector` or set `self.is_dirty = true` (`src/app/MainWindowCadastral.cpp:166-167`) |

### 7.5 Part 5 Items (Legend Layer Right-Click Menu on `지적도 · 조사 주변 5km`)

| # | Item (Part 5) | Status | Rust `file:line` & Explanation |
|---:|---|---|---|
| 5.1 | Withheld rows (`깨진 한글 바로잡기`, `그리기·편집 시작`, `꼭짓점 수정`, `폴리곤 묶기`, `시굴격자`, `오프라인 저장 — 지금 화면 범위`, `제출 꾸러미 만들기…`, `그린 도형 모두 지우기…`) | `SAME` | `crates/strata-app/src/layer_menu_input.rs:43-44, 73-75`, `crates/strata-app/src/layer_menu_model.rs:174, 189, 213, 230, 236, 239, 246` |
| 5.2 | `레이어 불러오기 (SHP·DXF·GPKG)` (`layer.import`) | `MISSING` | `crates/strata-app/src/layer_menu_model.rs:107-112, 182` — shown disabled with deferred tooltip `"아직 Rust 판에서 연결하지 않았습니다 (지시 W-A12)"`; no handler in `gui_layer_panel_context_menu.rs:111` |
| 5.3 | `이름 바꾸기 (두 번 클릭)` (`layer.rename`) | `SAME` | `crates/strata-app/src/layer_menu_model.rs:184`, `crates/strata-app/src/gui_layer_panel_context_menu.rs:85`, `crates/strata-app/src/app_state_layer_menu.rs:97-152` |
| 5.4 | `면·외곽선 색` (`layer.style`) on `지적도 · 조사 주변 5km` | `FAKE` | `crates/strata-app/src/gui_layer_panel_context_menu.rs:64` opens `ActiveModalDialog::LineColor` (`"선 색"`, `crates/strata-app/src/modal_style.rs:11-181`) instead of `open_cadastral_style()`, and clicking `"적용"` (`crates/strata-app/src/modal_style.rs:166-177`) writes `app.modals.applied_line_color_rgb` (read only for `"조사구역"` stroke at `crates/strata-app/src/gui_tab_map_heritage.rs:77-80`) and displays `"「지적도 · 조사 주변 5km」 선 색 #... · 굵기 ... mm 적용됨"` **without** updating `app.ref_vectors` (`cadastral_near` stays unchanged on the map) |
| 5.5 | `도형 속성` (`layer.attributes`) | `MISSING` | `crates/strata-app/src/ref_vector_show.rs:34` sets `entry.read_only = true`, so `crates/strata-app/src/layer_menu_model.rs:179, 186-187` disables `"도형 속성"` with `"읽기 전용 자료입니다. 편집 가능한 조사 자료를 선택하세요."` (or `"이 레이어에는 도형이 없습니다."` before first paint sets `feature_count`), and `gui_layer_panel_context_menu.rs:111` has no `"layer.attributes"` handler |
| 5.6 | `라벨·명칭 속성` ▸ (`layer.labels`, `지번 숨기기/보이기`, `표시할 항목`, `글자 크기`) | `DIFFERENT` | `crates/strata-app/src/layer_menu_input.rs:47, 54-61, 78-86`, `crates/strata-app/src/layer_menu_label.rs:49-83`, `crates/strata-app/src/layer_menu_model.rs:259-276` — because `cadastral_near` is in `ref_vectors` rather than `survey_tables`, `setting = LabelSetting::Unconfigured` and `fields = []`, making `supported = false`: the entire **`"라벨·명칭 속성"` submenu is disabled** (`"표시할 이름이나 번호 항목이 없습니다."`, child `"글자 보이기"` disabled, `"글자 크기"` disabled), whereas C++ enables `"라벨·명칭 속성"`, `"지번 숨기기/보이기"`, and `"글자 크기"` (`8 pt`). Also, toggling `real_survey.toggle_label_visible` (`crates/strata-app/src/gui_layer_panel_rows.rs:185`, `crates/strata-app/src/gui_layer_panel_context_menu.rs:82`) is **not** consulted when painting `ref_vectors` labels (`crates/strata-app/src/ref_vector_show.rs:64, 88` passes `|_| true`) |
| 5.7 | `투명도` ▸ (`0/20/40/50/60/80% 투명하게`), `이 레이어로 이동`, `전체 보기`, `레이어 삭제` (`Ctrl+Z` restore) | `DIFFERENT` | `crates/strata-app/src/layer_menu_model.rs:201-209, 250`, `crates/strata-app/src/gui_layer_panel_context_menu.rs:77-79, 95-97, 102-110`, `crates/strata-app/src/ref_vector_show.rs:89-91` — all four work once `cadastral_near` has painted once, **except** `entry.feature_count` starts at `0` (`ref_vector_show.rs:34`) until `paint_reference_vector_layers_in` receives `painted.rows` (`:89-91`), so before first paint finishes `"이 레이어로 이동"` is temporarily disabled (`empty = "이 레이어에는 도형이 없습니다."`, `layer_menu_model.rs:176, 208`) |
| 5.8 | `같은 묶음의 맨 아래로` (`layer.bottom`) | `DIFFERENT` | `crates/strata-app/src/ref_vector_show.rs:27`, `crates/strata-app/src/layer_menu_input.rs:62, 76`, `crates/strata-app/src/app_state_layer_menu.rs:82-94` — `cadastral_near` is the sole entry in `LegendGroup::Cadastral`, so `at_group_bottom` is **always `true`** and `"같은 묶음의 맨 아래로"` is permanently disabled (in C++, its parent is `layerTreeRoot()`, so it moves below all groups) |
| 5.9 | `면적 확인` (`layer.area`) on `지적도 · 조사 주변 5km` | `FAKE` | `crates/strata-app/src/layer_menu_model.rs:212, 227-229` offers `"면적 확인"` enabled (once `feature_count > 0`), and clicking it calls `self.layer_area_summary(&label, false)` (`crates/strata-app/src/gui_layer_panel_context_menu.rs:70-76`), which looks up `self.table_for_label(label)` in `self.survey_tables` (`crates/strata-app/src/survey_measure.rs:51-52`) — always `None` for `cadastral_near`, so clicking enabled `"면적 확인"` silently does nothing (and for survey layers shows status-bar text rather than a modal `QMessageBox`) |

### 7.6 Part 6 Items (Persistence & Freshly Opened Survey)

| # | Item (Part 6) | Status | Rust `file:line` & Explanation |
|---:|---|---|---|
| 6.1 | VWorld account persisted across sessions in `<userConfigDir>/vworld-account.ini` | `SAME` | `crates/strata-secrets/src/accounts.rs:34, 75-104`, `crates/strata-app/src/cadastral_run.rs:84-88` |
| 6.2 | Cadastral parcels kept in external `<surveyDir>/지적도/표시/<identity>.gpkg` (not copied into `<survey>.gpkg`) | `SAME` | `crates/strata-refmap/src/cadastral.rs:237-266`, `crates/strata-storage/src/survey_gpkg_save.rs:40-45` (`save_survey` writes only `survey_tables`) |
| 6.3 | Where layer registration, line colour & lot-number switch are stored (`qgis_projects` + `.qgz` on `Ctrl+S` vs. `지적도/strata-kept.json` immediately) | `DIFFERENT` | `crates/strata-storage/src/survey_gpkg_save.rs:7` does **not** write `qgis_projects` or `.qgz`; instead, Rust writes `<surveyDir>/지적도/strata-kept.json` (`{"radius_m", "work_epsg", "areas", "file", "colour", "labels"}`) immediately on download completion (`crates/strata-app/src/cadastral_run.rs:203`, `crates/strata-app/src/cadastral_layer.rs:64-67`) and immediately on clicking `"확인"` in `지적도 선 색·지번` (`crates/strata-app/src/cadastral_layer.rs:108-112`), even if the survey is never saved with `Ctrl+S`. Opacity, visibility off-state, custom font size, and custom layer rename are **not** persisted |
| 6.4 | Freshly opened survey shows `지적도 · 조사 주변 5km` immediately on the map without pressing `지적` | `MISSING` | `crates/strata-app/src/gui_tab_home_open.rs:56-67`, `crates/strata-app/src/gui_tab_home_survey.rs:106-107` — `open_survey_gpkg` calls `reset_survey_workspace` -> `remove_reference_vector_rows()` and never calls `kept_table` / `show_cadastral_layer`; reopening a saved survey shows **no** cadastral layer until the user presses `지적` once (`crates/strata-app/src/cadastral_run.rs:154-159`, which then restores the table, `colour`, and `labels` from `지적도/strata-kept.json` without network access) |

---

## All Differences Table

| where | C++ file:line | Rust file:line or `none` | kind |
|---|---|---|---|
| `지적` ribbon split-button dropdown arrow (`MenuButtonPopup`) | `src/app/MainWindowRibbon.cpp:479-480` | `crates/strata-app/src/gui_ribbon_draw.rs:74` | `MISSING` |
| `지적` menu row `VWorld 계정 설정` opens single-purpose 2-field dialog vs. combined 5-service dialog | `src/app/MainWindowRibbon.cpp:477` | `crates/strata-app/src/gui_ribbon_menus_a.rs:180-181` | `DIFFERENT` |
| `더보기` menu row `VWorld 지적도 아이디·비밀번호` vs. combined `API 키 입력 · 계정 설정…` | `src/app/MainWindowRibbon.cpp:678` | `crates/strata-app/src/gui_ribbon_menus_b.rs:82` | `DIFFERENT` |
| Account dialog window title, width, and layout (`VWorld 지적도 계정`, 440 px, 2 rows vs. `API 키 · 사이트 계정 설정 - Strata`, 520 px, 5-service grid) | `src/app/MainWindowCadastral.cpp:36-44` | `crates/strata-app/src/modal_paper_account.rs:148-214` | `DIFFERENT` |
| Account dialog field labels (`아이디` / `비밀번호` rows vs. `VWorld 지적도 ID / PW:`) and extra `비밀번호 표시` checkbox | `src/app/MainWindowCadastral.cpp:38-40` | `crates/strata-app/src/modal_paper_account.rs:198-213, 265` | `DIFFERENT` |
| Account dialog hint text (VWorld personal-settings hint vs. NGII auto-login hint) | `src/app/MainWindowCadastral.cpp:41` | `crates/strata-app/src/modal_paper_account.rs:252-258` | `DIFFERENT` |
| Account dialog buttons (`Save`/`Cancel` discarding edits vs. `저장`/`닫기` leaving `app.accounts` mutated on close) | `src/app/MainWindowCadastral.cpp:42-47` | `crates/strata-app/src/modal_paper_account.rs:267-276` | `DIFFERENT` |
| Account dialog validation & username trimming (no half-filled block, `username.trimmed()` vs. half-filled error & untrimmed save) | `src/core/CadastralPortal.cpp:142-143` | `crates/strata-app/src/dialogs_account.rs:87-101` | `DIFFERENT` |
| Account save feedback & failure UI (silent on ok, modal `KaUserError::warn` on error vs. status message on ok, inline red label on error) | `src/app/MainWindowCadastral.cpp:49-53` | `crates/strata-app/src/modal_paper_account.rs:277-287` | `DIFFERENT` |
| Portable-mode `password_portable` write & legacy `password` -> DPAPI migration on read | `src/core/KaSecretStore.cpp:145-151, 183-190` | `crates/strata-app/src/dialogs_account.rs:156` | `DIFFERENT` |
| `선 색·지번 표시` no-layer feedback (`QMessageBox::information` `"지적도"` vs. status-bar message only) | `src/app/MainWindowCadastral.cpp:60` | `crates/strata-app/src/cadastral_layer.rs:95-97` | `DIFFERENT` |
| `지적도 선 색·지번` colour control (`QPushButton("#rrggbb")` -> modal `QColorDialog` `"경계선 색"` vs. inline `color_edit_button_srgb` swatch + hex label) | `src/app/MainWindowCadastral.cpp:64-69` | `crates/strata-app/src/cadastral_layer.rs:129-133` | `DIFFERENT` |
| `지적도 선 색·지번` `확인` marks project dirty (`setDirty(true)`) | `src/app/MainWindowCadastral.cpp:78` | `crates/strata-app/src/cadastral_layer.rs:104-113` | `DIFFERENT` |
| Clipped `.gpkg` cache identity hash inputs (`cadastral-scope-v2` + WKB + WKT + transform XML vs. `strata-cadastral-scope-v1` + ring coords + `EPSG:{work_epsg}`) | `src/core/CadastralImport.cpp:87-116` | `crates/strata-refmap/src/cadastral.rs:101-124` | `DIFFERENT` |
| Download progress UI (`KaDownloadProgressDialog` window vs. owner-decided bottom gauge `조사 주변 지적도`) | `src/app/MainWindowCadastral.cpp:122-125, 170-173` | `crates/strata-app/src/cadastral_run.rs:196-209` | `DIFFERENT` |
| Missing/refused VWorld account during `지적` download (inline `configureCadastralAccount` + auto-continue and `"아이디·비밀번호 다시 입력"` retry vs. opening `AccountSettings` without auto-retry) | `src/app/MainWindowCadastral.cpp:117-120, 142-155` | `crates/strata-app/src/cadastral_run.rs:167-171, 238-244` | `DIFFERENT` |
| Post-download selection & dirty flag + behaviour when existing cadastral row was renamed | `src/app/MainWindowCadastral.cpp:166-167`, `src/core/CadastralImport.cpp:279-291` | `crates/strata-app/src/cadastral_run.rs:229-233` | `DIFFERENT` |
| Legend menu `레이어 불러오기 (SHP·DXF·GPKG)` (`layer.import`) | `src/app/MainWindowContextMenus.cpp:368` | `crates/strata-app/src/layer_menu_model.rs:182` | `MISSING` |
| Legend menu `면·외곽선 색` (`layer.style`) on `지적도 · 조사 주변 5km` opens generic `LineColor` and claims `"적용됨"` without changing `cadastral_near` | `src/app/MainWindowEditing.cpp:444-447` | `crates/strata-app/src/gui_layer_panel_context_menu.rs:64`, `crates/strata-app/src/modal_style.rs:166-177` | `FAKE` |
| Legend menu `도형 속성` (`layer.attributes` picker + read-only `KaUserError::warn` guard) | `src/app/MainWindowContextMenus.cpp:270-275, 286`, `src/app/MainWindowEditing.cpp:621-629` | `crates/strata-app/src/layer_menu_model.rs:186-187` | `MISSING` |
| Legend menu `라벨·명칭 속성` (`지번 숨기기/보이기`, `글자 크기` enabled in C++ vs. entire submenu disabled for `cadastral_near` and `toggle_label_visible` ignored by `ref_vectors` in Rust) | `src/app/MainWindowContextMenus.cpp:287-348`, `src/core/LayerLabelControls.cpp:116-145` | `crates/strata-app/src/layer_menu_input.rs:54-61`, `crates/strata-app/src/layer_menu_model.rs:259-276`, `crates/strata-app/src/ref_vector_show.rs:64` | `DIFFERENT` |
| Legend menu `이 레이어로 이동` temporarily disabled before first paint populates `entry.feature_count` | `src/app/MainWindowContextMenus.cpp:225, 250` | `crates/strata-app/src/ref_vector_show.rs:34, 89-91` | `DIFFERENT` |
| Legend menu `같은 묶음의 맨 아래로` (`layerTreeRoot()` scope vs. single-row `LegendGroup::Cadastral` always disabled) | `src/app/MainWindowContextMenus.cpp:397-408` | `crates/strata-app/src/layer_menu_input.rs:62, 76` | `DIFFERENT` |
| Legend menu `면적 확인` (`layer.area`) enabled on `지적도 · 조사 주변 5km` but silently no-ops because `layer_area_summary` checks `survey_tables` only | `src/app/MainWindowContextMenus.cpp:356-359, 422, 450-481` | `crates/strata-app/src/gui_layer_panel_context_menu.rs:70-76`, `crates/strata-app/src/survey_measure.rs:51-52` | `FAKE` |
| Workspace persistence (`qgis_projects` + `.qgz` written on `Ctrl+S` vs. `지적도/strata-kept.json` written immediately without opacity/visibility/rename/font-size) | `src/core/SurveySession.cpp:71-100`, `src/core/SurveyStorage.cpp:971-993` | `crates/strata-app/src/cadastral_layer.rs:64-67, 108-112`, `crates/strata-storage/src/survey_gpkg_save.rs:7` | `DIFFERENT` |
| Reopening a saved survey automatically displays `지적도 · 조사 주변 5km` on the map without pressing `지적` | `src/app/MainWindowSession.cpp:149-236, 284-345` | `crates/strata-app/src/gui_tab_home_open.rs:56-67` | `MISSING` |

---

## Could Not Determine

1. **Exact runtime Korean captions of Qt standard dialog buttons (`QDialogButtonBox::Save | Cancel` in `configureCadastralAccount` at `src/app/MainWindowCadastral.cpp:42`, `QDialogButtonBox::Ok | Cancel` in `configureCadastralStyle` at `:73`, and `QMessageBox::Ok` in `QMessageBox::information`)**: `not in the source` — depends on whether Qt's `qtbase_ko.qm` translation catalog is loaded in the shipped C++ runtime environment (`"저장"`/`"취소"`/`"확인"` vs. `"Save"`/`"Cancel"`/`"OK"`), which cannot be verified under the read-only, no-run rule.

---

## Counts

- **Part 7 items evaluated (`parts_1_to_6_items`)**: `36`
  - `SAME`: `10`
  - `DIFFERENT`: `20`
  - `MISSING`: `4`
  - `FAKE`: `2`
- **Unified difference rows (`all_differences_rows`)**: `26` (`20 DIFFERENT`, `4 MISSING`, `2 FAKE`)
- **20-citation random check (`citations_checked` / `citations_exact_before_fix` / `citations_corrected`)**: `20` checked, `10` exact on initial check, `10` corrected in place (`20/20` exact after correction)
- **Technical decisions recorded**: `3`
  1. `main` advanced from `845f290` to `8739aff` (`6b350b6` `CAD1` + `8739aff` `CAD1b`) while read-only inspection was running; per `ORDER-CADMENU0.yaml:7, 74`, all Rust `file:line` citations target the live files on disk at `8739aff` and note where `8739aff` changed `845f290`.
  2. Treated `src/app/MainWindowCadastral.cpp:134-175` in `ORDER-CADMENU0.yaml:24` as `:134-174` (file has `174` lines) and documented both the menu row labels (`VWorld 계정 설정`, `선 색·지번 표시`) and actual C++ window titles (`VWorld 지적도 계정`, `지적도 선 색·지번`).
  3. Classified enabled legend actions on `지적도 · 조사 주변 5km` whose click handlers either claim success without touching `ref_vectors` (`layer.style` -> `LineColor` `"적용됨"`) or silently drop the click (`layer.area` -> `layer_area_summary` returning `None`) as `FAKE`, and explicitly deferred-disabled rows (`layer.import`, `layer.attributes`) plus unimplemented reopen restoration and split-button arrow as `MISSING`.
- **Could not determine (`could_not_determine`)**: `1`

---

## 20-Citation Random Re-Open Check (`verification-before-completion`)

Picked 20 citations across C++ (`target/cpp-ref/`) and Rust (`crates/`) and re-opened each file directly (`10/20` exact on initial check; `10/20` corrected in place above):

1. `src/app/MainWindowRibbon.cpp:678` — **Exact** (`moreMenu->addAction("VWorld 지적도 아이디·비밀번호", this, &MainWindow::configureCadastralAccount)`).
2. `src/app/KaFormPlate.cpp:10-29` — **Corrected** (`setMinimumWidth(440)` is `:13`, `setHorizontalSpacing(0)` is `:15`, `setFixedWidth(96)` is `:20`, `setWordWrap(true)` is `:25`, gated by `kaPaperLook` at `:11`).
3. `src/app/KaUserError.cpp:88-100` — **Corrected** (draft cited `:74-77`; `"로그 폴더 열기"` is `:88-90` and `"닫기"` is `:97-100`).
4. `src/core/KaPortableRuntime.cpp:359-374` — **Corrected** (`userConfigDir()` resolves `<exeDir>/config` at `:361-364`, `QStandardPaths::AppConfigLocation` at `:366-369`, and `<tempDir>/ka-hgis-config` at `:371-373`).
5. `src/core/CadastralImport.h:25` — **Exact** (`bool applyStyle(QgsVectorLayer* layer, const QColor& color = Qt::black, bool labels = true);`).
6. `src/core/TopographicArchive.cpp:169-212` — **Corrected** (draft said `:169-214` and `zipSha256`; line `:176` checks `"sourceSha256"` and `cached()` ends at `:212`).
7. `src/core/CadastralImport.cpp:87-131` — **Corrected** (draft cited `:56-103`; `identity` hash `"cadastral-scope-v2"` is `:87-116` and `.lock`/`.sha256` reuse is `:116-131`).
8. `src/app/MainWindowEditing.cpp:443-449` — **Exact** (`ka_hgis/cadastral` routes `editCurrentLayerStyle` to `configureCadastralStyle()`, else `KaLineColorDialog::edit`).
9. `src/app/MainWindowEditing.cpp:621-629` — **Exact** (`!KaAttributeMapTool::isEditableLayer` warning `"「%1」의 속성은 고칠 수 없습니다."`).
10. `src/core/SurveySession.cpp:71-100` — **Exact** (`SurveySession::persistWork` writing embedded workspace + companion `.qgz`).
11. `src/app/MainWindowSession.cpp:284-345` — **Corrected** (`openSurveyGpkg` reads `SurveyStorage::readEmbedded` first as `1순위` at `:291-305`, falling back to companion `.qgz`/`.qgs` at `:322-338`, rather than comparing timestamps).
12. `tests/test_save_open.cpp:3098-3114` — **Corrected** (exact for Part 5.2 row 19 `!LayerOps::userRemovedCadastral(project)` on `Delete`; removed duplicate citation from Part 6.2).
13. `crates/strata-app/src/gui_ribbon_top.rs:162-180` — **Corrected** (draft cited `:157-161`; `draw_ribbon_chip` + `chip_clicked`/`chip_secondary` is `:162-180`).
14. `crates/strata-app/src/gui_ribbon_click_a.rs:159` — **Exact** (`"지적" => self.press_cadastral()`).
15. `crates/strata-app/src/cadastral_layer.rs:94-113` — **Exact** (`open_cadastral_style` `:94-101` and `apply_cadastral_style` `:104-113`).
16. `crates/strata-refmap/src/cadastral_portal.rs:176-265` — **Exact** (`login` outcome `:176-183`, `archives` `:186-205`, `shapefiles` `:209-248`, `receive` `:254-268`).
17. `crates/strata-refmap/src/cadastral.rs:64-81, 101-136, 237-266` — **Corrected** (draft cited `:150-189` and said no `.lock` or `.shp` hash; `Lock::take` is `:67-74`, `identity` `"strata-cadastral-scope-v1"` with `.shp/.dbf/.shx/.prj/.cpg` digests is `:101-124`, `finished` `.sha256` check is `:128-136`).
18. `crates/strata-app/src/gui_layer_panel_rows.rs:185` — **Exact** (`self.home_map.real_survey.toggle_label_visible(name)`).
19. `crates/strata-app/src/gui_tab_map_heritage.rs:77-80` — **Corrected** (`self.modals.applied_line_color_rgb` is at `:78`, inside `if rs.is_layer_visible("조사구역")`, not heritage shapes).
20. `crates/strata-app/src/modal_style.rs:166-177` — **Exact** (`LineColor` `"적용"` sets `applied_line_color_rgb` and `draw_guide_message` without touching `ref_vectors`).
