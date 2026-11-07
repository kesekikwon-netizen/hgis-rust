# Strata 2차 안 — BINDING design specification

Synthesis of `dir-A.md` (Quiet editorial), `dir-B.md` (Survey instrument), `dir-C.md` (발굴 도판집). Judge means: C 83.0, A 82.0, B 73.3. Base = C's plate idea and its fidelity to today's layout, governed by A's token discipline and clay rule (two judges' base), with B's state contrast and precision pieces. Where this file and a direction file disagree, this file wins. Sample values are samples, not statistics. **new (proposal)** = the app cannot show it today.

## 0. Name and idea

**「조용한 도판」 (Quiet plate).** The window is one plate from a published 발굴조사 보고서: the map is the figure, the left panel its printed 범례, the right panel a 유구 대장 entry, the status bar the sheet's bottom strip. Chrome is one ivory sheet with ink text and hairlines; every colour belongs to survey data; clay is spent only where the archaeologist's hand is needed. What people remember: the **유구 표지** (square legend patch of 시대 색 × 종류 무늬 + serif 「1호 주거지」) repeated identically in the layer key, the inspector head and the sheet legend, and exactly one clay mark per view.

## 1. Principles

1. **One sheet.** Every docked region (ribbon, bars, panels, Home, dialog body) is Ivory `#FAF9F5`. White only for inputs, depicted paper and floating things. Never a card in a card, never border plus shadow on a docked thing. The map runs edge to edge between panels.
2. **Rules, not boxes.** Regions are divided by one 1 px hairline or 24 px of air, never both. Tables are 삼선표 (ink rule on top, hairline under heads, ink rule at the end).
3. **Ink is state, clay is the hand.** Ink/slate says "current, in hand, on". Clay says "do this now": one filled `#B5573A` button per view, the unsaved 저장 tile, a 6 px dot that always sits beside words (「저장 안 됨」「설정 필요」「다음」). On the map clay is the 조사구역. Clay never on tabs, focus rings, links, progress, selected rows or active tools.
4. **Three looks for three meanings of "on".** Tool in hand = slate fill `#30302E` with ivory glyph. Shown / switched on / selected = Oat `#E3DACC` (with a `#B0AEA5` edge on toggles, a 1 px ink outline on chosen segments and swatches). Current view = 2 px ink underline. Never by going bold, never by changing width.
5. **Paper is square, controls are round.** Anything that depicts print (sheet, sheet thumbnail, print proof, 표제란, legend patch) is r0 with a 1 px ink frame. Anything pressed is r8. Small marks r4.
6. **Serif is a proper name, mono is a measure.** Serif (500) only at 20 px and above: survey name, selected object name, verdict or question of a window. Mono for digits, codes and paths only (Hangul never in mono): tabular, right-aligned, fixed cells, fixed decimals, grey unit. Everything else sans 400.
7. **Quiet by default, mark + words by exception.** Saved, set, passed = plain grey words, no tint, no pill anywhere. Needs-you = clay dot + ink words. Caution = triangle + ink words. Red only for what blocks submission.
8. **Legible in the field.** Nothing that must be read is lighter than `#73726C`; control edges `#C2C0B6`; targets 32 (24 in bars and the layer key, 36 on the map); state is always glyph plus words.
9. **Qt-true.** Flat fills, integer-px lines, QSS radii, no blur, gradient or translucency. Shadows only on floating things and painted inside the widget's own margin (like `KaSplashArt::paintShadow`; no `QGraphicsDropShadowEffect`). Custom painting limited to: 시대 띠, legend patch, 경도 눈금자, 도곽 band, ticked meter, splash.

## 2. Tokens (same names in `tokens.json`)

### 2.1 Chrome colour

| Token | Hex | Role |
|---|---|---|
| `ground` | `#FAF9F5` | all docked chrome, Home, dialog body, splash card |
| `surface` | `#FFFFFF` | inputs, paper, overlays, menus, popups |
| `wash` | `#F5F4ED` | row hover, disabled input ground, group row in a table, 도곽 band |
| `desk` | `#F0EEE6` | behind paper (도면 desk, print proof, sheet strip), meter track |
| `press` | `#E8E6DC` | pressed face, table row line, progress track |
| `oat` | `#E3DACC` | on / shown / selected fill |
| `hand` | `#30302E` | tool-in-hand fill (ribbon tile, chosen draw-bar tool, engaged map mode); icon stroke at rest |
| `ink` | `#141413` | text, plate frames, table rules, tab underline, focus ring, check/radio marks, progress fill, tooltip fill |
| `ink2` | `#3D3D3A` | strong secondary text (고대비 maps `muted` here) |
| `muted` | `#5E5D59` | secondary text, labels, captions, hints, 「자석 끔」 |
| `faint` | `#73726C` | units, placeholders, disabled text (lightest text allowed) |
| `line` | `#DEDCD1` | hairline between regions, ribbon separators |
| `edge` | `#C2C0B6` | 1 px border of inputs, secondary buttons, overlays, paper on the desk; dotted leaders |
| `ring` | `#B0AEA5` | edge of an oat toggle that is on; control hover edge; dashed "missing sheet" outline |
| `action` | `#B5573A` | the one filled button, unsaved 저장 tile (white text); hover `#A44D32`, pressed `#8F4229` |
| `clay` | `#D97757` | 6 px attention dot, 조사구역 line, the one filled stratum of the splash |
| `clayText` | `#9C4A2F` | the word 「다음」 and 「추천」 only |
| `block` / `blockWash` | `#A33B3B` / `#F3DEDA` | blocking error glyph and words; wash only as the blocking band in 검수·제출 and under an invalid field |
| `caution` | `#7A5A00` | caution triangle (words beside it stay ink) |
| `done` | `#3F6B31` | check glyph only where a result is announced (all passed, success notice) |
| Ribbon slots | tile `#F0EEE6` · hover `#E8E6DC` · pressed chip `#E3E0D4` + 1 px `#DEDCD1` · disabled tile `#F5F4ED`, glyph 45 %, label `#696862` · group caption `#5E5D59` (dimmed `#C2C0B6`) · glyph `#30302E` · label `#141413` | as built; only the "on" slots below change |
| Ribbon on: tool | tile `#30302E`, 3.125 px border `#30302E`, glyph `#FAF9F5`, label `#141413` 700 | 선택 · 측거 · 그리기 · 버퍼 |
| Ribbon on: shown | tile `#E3DACC`, border `#B0AEA5`, glyph and label `#141413` 700 | 지형 · DEM · 토양 · 지질 · 수계 |
| Ribbon unsaved | tile `#B5573A`, glyph `#FFFFFF`, label `#141413` 700, dot same size and place, white ring, core `#141413` | 저장 only. Chip focus: 2 px `#141413` |

Needs one id-to-kind flag beside `m_strong` in `KaIconsMockupEngine::lookFor`; fallback if not built: every on-tile takes the tool look.

### 2.2 Data colour (map, legend patches, sheets only; a colour the user saved always wins)

| Data | Symbol |
|---|---|
| 조사구역 | Clay `#D97757` 2 px on a white casing (1 px each side), fill clay 8 % |
| 시굴격자 | `#B5573A` 1 px dashed, fill clay 6 %, mono 10 `Tr-1` |
| 유구 (면) | fill 시대 색 at 70 %, hatch 종류 무늬 in ink, outline ink 1.25 px on white casing; unclassified: Olive `#788C5D` outline, 20 % fill |
| 유구 (선) | ink 1.5 px in the kind's dash on white casing; unclassified Olive dashed |
| 단면선 | Fig `#C46686` 2 px on white casing, 8 px end ticks, letters `A` `A′` sans 12 with halo |
| 기준점 | 10 px triangle, ink stroke, fill `#C9A82D`, mono 10 `GCP1` |
| 유물 | violet `#6B4D9E` disc r4 (dataviz anchor, tertiary), white halo 1.5 px, mono 10 number |
| 참조 도면 (CAD·SHP) | `#73726C` 0.75 px, labels sans 10 `#73726C` |
| 지적 / 등고선 / 수계 | `#87867F` 0.75 px, lot numbers mono 10 `#9C9A92` / `#C2C0B6` 0.75 px, index `#9C9A92` / Sky `#6A9BCC` 1.5 px |
| 주변유적경계 | `#5E5D59` 1 px dashed rings, mono `500 m` `1000 m` |
| 주변유적 (fixed, unchanged) | 지정유산 `#8E44AD` · 현상변경허용기준 `#E67E22` (P) · 매장유산유존지역 `#2E86C1` (B) · 문화유적분포지도 `#27AE60` (U) · 지표조사구역 `#D4AA00` (R) · 발굴조사구역 `#C2187D` (E) |
| 시대 띠 (app ramp, unchanged) | 구석기 `#53291B` · 신석기 `#674028` · 청동기 `#7A5737` · 초기철기 `#8B7048` · 원삼국 `#9C8A5B` · 삼국 `#ABA571` · 통일신라 `#BAC08A` · 고려 `#C7DCA6` · 조선 `#D3F8C4` · 미정 = empty box |
| 종류 무늬 (unchanged) | 주거지 none · 수혈 `/` · 구 horizontal · 도랑 vertical · 분묘 `\` · 가마 cross · 유물 diagonal cross · 기타 dots |
| 상태 marks (replace chips) | 저장됨 · 설정됨 · 검수 전 · 통과 = `muted` words · 저장 안 됨 · 설정 필요 = 6 px `clay` dot + ink words · 주의 = `caution` triangle + ink · 오류 · 원본 없음 = `block` glyph + `block` words |

Selection: outline ink 2 px with casing; 7 × 7 white squares, 1 px ink edge (2 px on boards for clarity) on real vertices; grabbed vertex filled `#B5573A`. No bounding box. Map labels stay sans with a 2 px white halo.

### 2.3 Type

Serif `"Noto Serif KR", Batang, serif` (500 only, never under 20 px; implementation must bundle a Noto Serif KR subset in `data/fonts`, else Batang shows). Sans `"Malgun Gothic", "Noto Sans KR", sans-serif`. Mono `Consolas, "Cascadia Mono", "IBM Plex Mono", monospace`. Letter-spacing 0; `word-break: keep-all`.

| Role | Family | px / line | Weight | Colour | Used for |
|---|---|---|---|---|---|
| Display | Serif | 40 / 48 | 500 | ink | survey name in Home 표제란 (once in the app) |
| Title | Serif | 28 / 36 | 500 | ink | Home wordmark; verdict title of L windows (splash wordmark keeps 60) |
| Heading | Serif | 20 / 28 | 500 | ink | selected object name; message-box question; 도면명 |
| Lead | Sans | 15 / 24 | 400 | ink | Home section headings, Home row names, one lead sentence |
| Body | Sans | 13 / 20 | 400 (700 only on the filled button and the ribbon's own bold labels) | ink | controls, rows, menus, panel titles |
| Label | Sans | 12 / 18 | 400 | muted | field labels, hints, secondary lines, status sentence |
| Caption | Sans | 11 / 16 | 400 | muted | section captions, column heads, units row; ribbon group caption stays 11 / 700 |
| Key | Sans | 10 / 14 | 400 | ink | layer key rows only (user decision) |
| Figure | Mono | 20 / 28 | 400 | ink | 면적 · 둘레, 검수 counts, 시굴 result, 측거 total |
| Mono | Mono | 12 / 18 | 400 | ink | coordinates, scale, codes, paths, table numbers, spin values (13 in form fields) |
| Mono small | Mono | 10 / 14 | 400 | muted | on-map names, 도곽 numbers, layer counts, file extensions (11 on paper) |

### 2.4 Space, size, shape

- Spacing 4 · 8 · 12 · 16 · 24 · 32 · 48. Panel padding 16. Dialog padding 24 all sides. Form row gap 8 in panels, 12 in dialogs. Label column 96 in dialogs, 64 in the inspector. Lines of prose at most 40 characters.
- Heights 24 (bars, layer key rows, quiet row-end buttons) · 32 (default control, list row, menu row, dialog button, status bar) · 40 (Home action row, 새 조사 choice buttons) · 36 map overlay buttons · 48 Home recent row.
- Radii 0 paper · 4 check box, swatch · 8 every control, overlay, menu · 12 splash card only (built 16 → 12). Ribbon tile keeps 12.5, chip 8, 「지역」 and search 8.
- Borders 1 px only (2 px only for focus and chosen swatch). `line` for structure, `edge` for controls, `ink` for plate frames and table rules. No left accent bars.
- Shadows: `overlay` 0 2px 12px rgba(20,20,19,.12); `menu` 0 8px 24px rgba(20,20,19,.14); dialogs = native window shadow. Nothing docked, not the paper on the desk (1 px `edge`).
- Focus ring: the 1 px border becomes 2 px `ink` with padding −1 (no size change); on `hand`-filled and `action`-filled controls the ring is 2 px `ink` outside a 1 px ivory gap.
- Icon stroke: Lucide/ka SVG as shipped, 2 on a 24 grid; 16 px in tabs, menus, fields; 20 px tile icons in the draw bar as built; 20 px on map overlay buttons; colour `hand` at rest, ivory on `hand` fill.
- Motion: 120 ms ease-out for hover and on/off fills, 160 ms disclosure; off when Windows animations are off.
- Options: 고대비 → `line` `#87867F`, `edge` `#73726C`, `muted` `#3D3D3A`, `faint` `#5E5D59`, oat on-tile border `ink`. 큰 글씨 → Body 15, Label 13, Caption 12 (ribbon and layer key unchanged). 새 모양 off → old blue look untouched.

## 3. Components

- **Title bar.** Native caption tinted `ground`, slate text, real app icon. `Strata` / `광령리1 - Strata` / `광령리1 * - Strata`. 32 high on boards.
- **Ribbon.** Paste `ctx/ribbon*.html` unchanged: 1920 × 118, 27 chips, pitch 58, tile 50, glyph 28.125, label 13, caption 11 / 700, separators, 「지역」 77 × 40 and search 158 × 32 (white, 1 px `edge`, r8, focus 2 px ink). Swap only the colour slots of §2.1. Placeholder shown as 「주소·지번 찾기  Ctrl+F」 (copy proposal; today's text is cut off).
- **View tabs.** Row 36, `ground`, bottom `line`. Tab = 16 px glyph + Body, padding 0 16, `muted`; hover ink; current ink + 2 px ink underline on the row's line. Work tabs show a 16 px close glyph. Right corner (KaSurveyBadge, same click and tooltip), plain text 13, no pill: `muted` 「조사 열림 · 광령리1」, or ink 「광령리1」 + clay dot + 「저장 안 됨」.
- **Sub toolbar.** Row 44, `ground`, bottom `line`, padding 0 16. Caption 「그리기 ›」 Label. Tool = 32 high, `surface`, 1 px `edge`, r8, padding 0 10, built 20 px tile icon + 6 + Body; text-only tools stay text-only. Chosen tool: fill `hand`, text and glyph `ground`, same weight. Check toggles on (연속 그리기) = oat + `ring`. Clusters in the built order split by a 1 × 20 `line` rule with 12 px each side: 도형선택 | 자석 · 공유 경계 · 허용치 `12 px` · 모든 조사 레이어 | 쉽게그리기 · 조사구역 그리기 · 유구 그리기 · 유구 선 · 유물위치표시 | 완료 · 되돌리기 · 취소 (only while sketching) · 연속 그리기 | 폴리곤 묶기 · 폴리곤 나누기 · 구간 분리 · 겹친 곳 지우기 | 닫기 (far right, quiet).
- **Panel.** `ground`, padding 16, no frame. Splitter 7 px of `ground` with a centred 1 px `line`. Widths 348 (left), 272 (inspector), 320 (도면 right).
- **Panel header.** Row 32: title Body 13 ink left; quiet 24 px buttons right (checkable on = oat + `ring`). Section caption inside a panel: Caption `muted`, then a `line` leader from 8 px after the text to the padding edge; 16 px above, 8 below.
- **Tree / list row.** Hover `wash`; selected `oat` full width, same weight; disabled `faint`. Layer key row 24: check 14 (r4, 1 px `#87867F`; on = ink fill, ivory tick) · legend patch 20 × 14 r0 drawn exactly like the map symbol · name Key 10 · mono 10 count (new) · pencil 12 when edited (kept) · 「글자」 column 56: check 14 + caption 10 `muted` or `—`. Sub-rows 20, indent 20: patch + 「청동기 · 주거지」. Bands 「조사 데이터」「주변·참조」「배경 지도」 = section caption with leader, 28 high. List rows 32; 파일함 rows 24 with 14 px glyph, name 12, extension mono 10 `muted` right-aligned.
- **Table (삼선표).** 1 px ink top rule; heads 28, Caption `muted`, left over text, right over numbers, 1 px `line` under; rows 32 (48 two-line) with 1 px `press` between; numbers mono right-aligned; group row 28 on `wash`; row-end action = quiet 24; 1 px ink rule after the last row; nothing below it. No zebra, no vertical lines.
- **Form row and inputs.** Label (Label 12 `muted`) left in the label column, control right. Input 32, `surface`, 1 px `edge`, r8, padding 0 10, Body; placeholder `faint`; hover edge `ring`; focus ring; disabled `wash` + `faint`; invalid 1 px `block` + 12 px `block` line 4 px below. Numeric: mono 13 right-aligned, unit `faint` inside right, stepper 20 px column behind a `line`. Combo chevron 16 `muted`; combos for 종류 / 시대 lead with their 20 × 14 patch. Check 16 r4 and radio 16: 1 px `#87867F`, on = ink. Slider: 2 px `edge` track, ink filled part, 14 px white thumb with 1 px ink edge. Segmented choice: joined 32 px `surface` buttons with `edge`; chosen = oat fill + 1 px ink outline. Swatch 44 × 24 r4; chosen = 2 px ink outline with 2 px ivory gap + name beside the row. Read-only spec row (dimension line): Label left, `1px dotted #C2C0B6` leader, mono value right, unit `faint`; row 24.
- **Ledger figure.** Between a 1 px ink rule and a `line`: full-width rows 36: Caption left, Figure mono 20 right-aligned + unit Label `faint` after 4 px. Used for 면적 / 둘레 (stacked, so 6-digit areas fit), 검수 counts and 시굴 result (cells side by side when width allows).
- **시대 띠.** 168 × 8 (panel) or 240 × 10 (key), ten r0 cells, 1 px `line` between; chosen cell framed 1 px ink with a 4 px tick below; 미정 empty box. Display only. new (proposal).
- **Buttons.** Primary: `action`, white 13 / 700, r8, padding 0 16, min 88; disabled `press` + `faint`; one per view. Secondary: `surface`, 1 px `edge`, ink 400; hover `wash`, pressed `press`. Quiet: no border or fill, ink; hover `desk`; underlined when it is a link. Danger: secondary with `block` text, hover `blockWash`; never filled red. Sizes 24 (12 px, padding 0 8) · 32 · 40 (15 px, padding 0 20). Footer order: secondary actions far left · stretch · **주 버튼 → 취소 / 닫기**, gap 8.
- **Chips / badges.** None. Mark + words inline at 12 px (§2.2 상태 marks). Counts (「1개 선택」「3개」「광령리1 기준」) are plain `muted` text.
- **Tooltip.** `ink` fill, ivory 12 / 18, r4, padding 6 8, max 280; shortcut as plain mono text after two spaces (no key cap).
- **Notice (QgsMessageBar, docked) and toast.** Band at the top of the map, 36 high, `surface`, bottom `line`: 14 px glyph (`done` / `caution` / `block`) + one Body sentence + quiet buttons + close glyph. Error: `blockWash` ground. No floating toast replaces it; the non-modal progress boxes stay small windows.
- **Menu / popup.** `surface`, 1 px `edge`, r8 where the OS allows, `menu` shadow, padding 4. Item 32, Body, padding 0 12, hover `desk`, checked = 14 px ink check, disabled `faint`, shortcut mono 12 `muted` right; separator 1 px `line`; icons on all or none. Section captions (Caption `muted`, 24 high) via `QWidgetAction`.
- **Progress.** 4 px bar, ink on `press`, r2, mono percent at its right; staged downloads add stage ticks or the six-stop rail. Splash keeps its 200 × 2 line, in ink.
- **Dialog.** Native caption with the plain title. Body `ground`, padding 24, inputs white. Forms start with their fields (no invented serif title). Message boxes start with a Heading serif 20 question or outcome, then one Label sentence. L result windows start with a Title serif 28 verdict. No icon disc, no footer band; footer row of 32 px buttons 24 px under the content; L adds a `line` above the footer. Widths S 440 · M 560 · L 860. Windows with a built fixed size keep it: download 680 × 330, 검수·제출 820 × 580, 자료 준비 상태 860 × 420, 시·군 선택 520 × 300, 상세 보기 980 × 720, 수치지형도 좌표계 확인 1050 × 720.
- **Map overlay.** One recipe: `surface`, 1 px `edge`, r8, `overlay` shadow, 12 px from the map edge (or from the 도곽 band), 8 px between, padding 8 12, text 12, values mono. Buttons 36 × 36 with 20 px glyphs; joined groups share one overlay with `line` dividers; an engaged mode button = `hand` fill. Scale bar is bare: 4-part ink / white bar 6 high cut to a round length, 1 px ink outline, mono 11 `0` `10` `20 m` with a 2 px white halo.
- **도곽 band (restyle of the 좌표격자 edge labels).** Only when 좌표격자 is on: opaque `wash` band 18 px along the top and left map edges, `line` on the map side, 4 px ticks `#87867F`, mono 10 `muted` labels: real TM numbers when unturned (`147,980`), 「원점」「+20 m」 when turned, D°M′S″ for 경위도 격자.
- **Status bar.** 32 high (built 34; 2 px change), `ground`, top `line`. Left at x 16: next-step sentence Label `muted`. Right: cells on one baseline split by 1 × 16 `line` rules, 12 px each side, built order: tool name 12 ink | 14 px magnet glyph + 「자석 켬」 ink or 「자석 끔」 `muted` | clay dot + 「저장 안 됨 3건」 ink (only while unsaved) | Caption `X` + mono 12 in a fixed 96 px right-aligned cell, `Y` likewise | Caption 「축척」 + editable combo 88 × 24 mono `1:500` | two-part control: secondary 24 「작업 5186 · 중부원점」 with chevron + `→ 제출 5179` mono `muted` (원점 name = copy proposal) | 「지도갱신 켜짐」 `muted`, or `block` glyph + 「지도갱신 멈춤」.

## 4. Board blueprints

Common stack at 1920 × 1040: caption 0–32 · ribbon 32–150 · [sub toolbar 150–194 when a mode is on] · tabs 36 · page · status bar 1008–1040. Every board uses §5 data.

### 4.1 Main.dc.html (1920 × 1040) — 지도, 그리기 줄 열림, 「1호 주거지」 선택, 저장 안 됨

- Caption 「광령리1 * - Strata」. Ribbon = `ribbon-drawing.html`: 저장 clay with dot (the only clay fill on screen), 그리기 `hand` tile, 지형 oat tile. 그리기 줄 150–194, 도형선택 chosen (`hand`), 자석 checked, 허용치 `12 px`, 「모든 조사 레이어」. Tabs 194–230: 홈 · 지도 (current); corner 「광령리1 ● 저장 안 됨」. Page 230–1008 (778 high), no margin: left 0–348 | splitter 7 | map 355–1641 (1286 × 778) | splitter 7 | inspector 1648–1920.
- **Left panel.** Header 「레이어」; quiet 「파일함」 (on) 「전체 끄기」. Filter 32 「레이어 찾기」. Heads 24 「도면 · 레이어」 | 「글자」 over a `line`. Band 조사 데이터 → 조사구역 `1` · 유구 (면) `5` (selected oat, pencil; sub-rows 「청동기 · 주거지」「원삼국 · 수혈」「미분류 · 수혈」) · 유구 (선) `1` · 단면선 `1` · 시굴격자 `6` · 유물 `4` · 기준점 `4`. Band 주변·참조 → 지적도 · 조사 주변 5km · 문화유적분포지도 (off, `faint` name). Band 배경 지도 → 참조 지도 (group) · 위성 (off) · 지형. Disclosure row 32 「유구 (면) · 표시 설정」. Lower 2/5 after a splitter: 「파일함」; quiet 24 buttons in two rows (내 PC · 바탕화면 · 문서 / 내려받기 · SD 카드 · 폴더 지정 · 상위, all alike); rows: 광령리1, 드론 정사영상, 광령리_측량_0923 `.dxf`, 광령리_지적 `.shp`, 정사영상_0921 `.tif`, 기준점_GPS `.csv`.
- **Map** as a publishable 유구배치도 on the 지형 + 지적 ground `#F7F5EE`: 조사구역 fills about 60 %; Tr-1…Tr-6; 1호 주거지 (selected, five vertex handles, one grabbed), 2호 주거지, 1호 수혈, 2호 수혈, 3호 수혈 (Olive, unclassified), 1호 구; 단면선 A–A′ across 1호 주거지; GCP1–GCP4 on the boundary; 유물 1–4. 좌표격자 on: thin ink dashed grid at 20 m and the 도곽 band with `147,960` `147,980` `148,000` / `98,120` `98,140` `98,160`.
- **Overlays.** Top-left guide band 36: glyph + 「도형선택」 13 ink + 1 × 16 rule + 「도형을 클릭하면 수정점이 나옵니다. 점을 끌어 옮깁니다」 12 `muted`; joined 되돌리기 | 다시 실행 73 × 36. Under it 투명도·밝기 260: 「유구 (면)」, 「투명도」 slider `20%`, 「밝기」 no track + 「그림만」 `faint`. Top-right zoom column 36 × 110 (＋ − 조사구역 맞춤). Bottom-left scale bar `0 10 20 m` + 12 × 20 ink arrow with Caption 「도북」 (new). Bottom-right 좌표격자 card open: [미터 격자 ▾] [간격 `20.0 m`] [회전 `0.0°`] [원점] / [굵기 `1.2`] [실선 | 점선 | 점] [3 swatches + …] / check 「좌표격자」.
- **Inset frame 560 × 320 (bottom-centre of the board margin or a second artboard)**: same site on 위성, 「그리기 · 유구 면 · 점 3」 in progress, guide band ending with mono `변 4.20 m · 면적 32.9 ㎡` (new). Proves casing and halo on imagery.
- **Inspector (padding 16, inner 240).** Header: 「선택한 유구」 13 ink · 「1개 선택」 `muted` · collapse 24. Tabs 32: 속성 (ink underline) | 스타일. 유구 표지: patch 28 × 20 (청동기 fill, no hatch, ink frame) + Heading 「1호 주거지」; sub-line Label 「유구 면 · 주거지 · 청동기」. Ledger: 「면적」 `32.91` ㎡ / 「둘레」 `24.60` m; Caption 「자동 계산」 `faint` right of the top rule. Form (label 64): 유구번호 [`1호`] [quiet 「다음 번호」] · 종류 [patch 주거지] · 시대 [patch 청동기] + 시대 띠 (청동기 framed; new) · 조사 상태 [조사 중] · 조사자 [권영인] · 조사일 `2026-10-03` · 메모 (52 high) 「북서쪽 모서리 바닥 다짐 확인 필요」. State line: clay dot + 「변경됨 · 아직 저장 안 됨」 ink 12. Secondary full width 「조사카드 열기」. Pinned bottom after a `line`: caption 「배경 지도」; three joined toggles 위성 · 지형 (oat) · 옛 지도.
- **Status bar.** 「다 그렸으면 리본의 「도면」으로 종이에 옮기세요.」 … 도형선택 | 자석 켬 | ● 저장 안 됨 3건 | X `147,999.305` Y `98,157.620` | 축척 `1:500` | 작업 5186 · 중부원점 → 제출 5179 | 지도갱신 켜짐.

### 4.2 Home.dc.html (1920 × 1040) — 광령리1 was open last, three recent rows

Structure and positions as built (hero 236 with 이어서 작업 at its right, 최근 조사 left, 작업 순서 right 400, 연결 상태 strip); the contour art and row thumbnails are removed (decoration, no feature). Size change to state: 이어서 작업 widens beyond 470 at 1920 (collapses to the built 300–470 stacked block under 1500 px).

- Caption `Strata`; ribbon = `ribbon-home.html` (기록 tools disabled); tabs 150–186 (홈 current); page 186–1008, padding 24 top, 32 sides; status 「먼저 「새 조사」로 오늘 현장을 만드세요.」
- **Hero band** y 210–446 between two 1 px ink rules, no fill. Left x 32–728: app icon 48 + `Strata` Title serif 28 + 1 × 28 `edge` rule + 「필드고고학 GIS」 Lead `muted` (y 242); lead 「오늘 현장을 새로 만들거나, 지난 조사를 이어서 여세요.」 (y 312); two secondary 40 buttons 「새 조사」「조사 열기」 with 16 px glyphs (y 362). Right x 776–1888: **이어서 작업 표제란**, `surface`, 1 px ink frame, r0, y 234–422 (188 high), cells split by `line`: row 32 [Caption 「이어서 작업」 … `muted` 「저장됨」]; row 92 [Display 40 「광령리1」, padding-left 24 | cell 320 wide: primary 40 「이어서 열기 →」 inset 24]; row 64, five cells, Caption over value: 「폴더」 `바탕 화면 › 광령리1` (sans 12, middle-elided) | 「마지막 열림」 오늘 07:04 | 「조사구역」 mono `1` | 「유구」 mono `6` | 「검수」 `오류 1 · 주의 3` (number of 오류 in `block`). Recovery copy state adds one 24 row: clay dot + built sentence + quiet links 사본 열기 · 폴더 열기 · 숨기기. No openable survey: the block is absent, the left block spans the band and 「새 조사」 becomes the primary.
- **최근 조사** x 32–1456, y 478: header 40: Lead 「최근 조사」 + `muted` 「3개」; right Caption 「오른쪽 클릭: 목록에서 제거」 + filter 260 × 32 「이름·폴더로 찾기」. 삼선표: 조사 | 상태 120 | 구역 72 | 유구 72 | 검수 160 | 마지막 열림 140 right. Rows 48: name Lead 15 ink over path Label; 상태 words; mono numbers; date sans 12 `muted`. Rows: 광령리1 · 저장됨 · 1 · 6 · 오류 1 · 주의 3 · 오늘 07:04 / 병산동 시굴 · ● 저장 안 됨 · 1 · 0 · 검수 전 · 어제 17:42 / 하회리 표본조사 (`faint`) · 원본 없음 (`block`) · — · — · — · 찾을 수 없음. Table ends after row 3.
- **작업 순서** x 1488–1888, y 478: header 40: Lead + `muted` 「광령리1 기준」. 1 px `edge` rail at x 13 with three stops 26 px: done = `surface` disc, 1 px ink ring, ink check; next = 8 px `clay` dot on the rail + 「다음」 `clayText` after the title; todo = `ring` circle with `muted` mono numeral. Titles Body ink, bodies Label `muted` (built sentences): ① 새 조사 (done) ② 조사구역 그리기 (done) ③ 도면 · 제출 (next), ending 「제출 준비」 mono `오류 1 · 주의 3`.
- **연결 상태** x 32–1888, y 900–984, top `line`: header 32: Lead 「연결 상태」, lock 14 + Caption 「비밀값은 표시되지 않습니다」; right 「인터넷 · 확인 안 함」 `muted` + secondary 24 「인터넷 확인」. Five cells 48 high split by 1 × 32 `line`: name Body over state Label (`muted` 「설정됨」 ×4; 수치지형도 계정: clay dot + 「설정 필요」), quiet 24 「설정」.
- Second small frame (640 × 360): first run, no survey: band shows only the left block with primary 「새 조사」, table area the sentence 「아직 최근 조사가 없습니다. 새 조사 또는 조사 열기로 시작하세요.」

### 4.3 Panels.dc.html (1920 × 1320, `desk` ground; each specimen at real width with a Caption line under it; problem states not in §5 carry 「견본 상태」)

- **Row 1 — 오른쪽 판 (272 × 560 each):** ① nothing chosen: sentence only 「지도에서 도형 하나를 고르면 기록이 여기에 나옵니다.」 ② drawing: 「그리는 중에도 여기서 바로 고칠 수 있습니다. 지도를 누르면 이어서 그립니다.」 over the live form ③ 유구 saved: as 4.1 with `muted` 「저장된 값입니다」 ④ duplicate number: triangle + 「같은 종류에 같은 번호가 1개 더 있습니다. 제출 전 검수에서도 알려 드립니다.」 ⑤ 기준점: Heading `GCP1`; meta line 「세계측지계 · GRS80 · TM · 중부원점」 `muted` (견본: clay dot + 「원점 비어 있음」); dotted-leader rows X `147,962.418` / Y `98,131.207` / Z `EL. 152.36` (new); 점 이름 + 「다음 번호」, 메모 ⑥ 스타일 tab: 「유구 (면)」, 「색」 8 swatches 4 × 2 (주황 chosen), checks 「채우기 없음 (선만)」「선 없음」, 「외곽선 굵기」 `0.5 mm`, the built note.
- **Row 1 right — smaller states:** 조사구역 record (조사명 광령리1, 유적명, 이름; ledger `2,352.60` ㎡ / `198.40` m) · reference record read-only (fields flat, lock glyph) · 「바꾸지 못했습니다: …」 in `block`.
- **Row 2 — 레이어 목록 (348 × 440 each):** working (as 4.1) · empty (three built lines centred, `muted`) · no match 「「옹관」에 맞는 레이어가 없습니다.」 · 표시 설정 open (「유구 (면) · 표시 설정」, 「이름 표시」, 내용 combo, 「면적(㎡) 함께 표시」, 크기 `9 pt`, 글자 색 combo with patch, 「흰 테두리 (위성·지적 위에서 읽기)」) · layer right-click menu (built order, 32 px rows, disabled `faint`).
- **Row 3 — 그리기 줄과 자석 (1600 × 44 each):** idle (no tool chosen) · 「유구 그리기」 chosen with 완료 · 되돌리기 · 취소 visible · 자석 off. Beside: 버퍼 줄 「주변유적경계 › 500m 1000m 닫기」; 맞추기 줄 「맞추기 › 이동 · 점 지우기 · 되돌리기 · 맞추기 저장 · 닫기」. Status-bar strip in two states (자석 켬 / 자석 끔 + 지도갱신 멈춤).
- **Row 4 — 검수 목록 (820 × 580, blocked state) + two 400 × 200 crops (warnings only; all passed).** Title serif 28 「오류 1건을 고쳐야 제출할 수 있습니다」; Label 「발굴 위치도면 유의사항 기반 검수 v1」; right: three ledger cells 「오류」 `1` (`block`) 「주의」 `3` 「통과」 `24`. **필수 도면 5종 strip** on a `desk` band 108 high (new): five paper thumbnails 88 × 62 r0 ink frame, name 12, mono 11 scale: 조사구역도 `1:5,000` ✓ · 유적위치도 `1:25,000` ✓ · 유구배치도 `1:1,000` ✓ · 개별유구실측도 `1:100` dashed `ring` outline + triangle 「없음」 · 층위도 `1:200` dashed + 「없음」. 삼선표 tree 항목 | 고치는 법 | (actions): 「막는 항목(오류) 1건」 open: `block` glyph 「종류·시대가 빈 유구가 있습니다.」, fix + 「근거: …」 Caption, child 「유구 (면) · 3호 수혈」, quiet 「위치 보기」 + secondary 24 「속성 고치기」, row on `blockWash` (the only red). 「확인할 항목(주의) 3건」 open: 「개별유구실측도가 없습니다.」 [도면 열기] · 「층위도가 준비되지 않았습니다.」 [단면도 열기] · 「국가유산 인트라넷 자료(주변유적)가 지도에 있습니다. …」. 「통과 24건」 closed. Mono 11 `muted` line `EPSG:5179 SHP · 조사도면.pdf · MANIFEST.sha256`. Footer: 「다시 검수」「레이어 5179 변환…」 … 「제출 꾸러미 만들기…」 (primary, disabled) 「닫기」. Crops: 「오류가 없습니다. 주의 항목은 확인한 뒤 진행할 수 있습니다.」 with clay primary; 「오류와 주의 항목이 없습니다.」 with `done` check.
- **Row 4 right — 알림과 빈 상태:** notice band ×3 (success 「제출 꾸러미를 만들었습니다. 인트라넷에는 SHP를 하나씩 올리세요.」; CAD notice with 「다른 위치로 바꾸기」「직접 맞추기」; error on `blockWash`) · tooltip 「조사구역이 화면에 꽉 차게 봅니다」 · 측거 HUD (300: 거리 | 면적 segments, Figure `36.80 m`, rows `1구간 ····· 12.40 m` `2구간 ····· 24.40 m`, quiet 마침 · 복사 · 한 점 취소 · 다시, built hint) · 투명도 card idle 「투명도·밝기: 조절할 레이어를 고르세요」.

### 4.4 Layout.dc.html (1920 × 1040) — 도면 탭, 유적위치도 A4 가로

- Ribbon as built on a studio tab (기록 · 자료 받기 · 배경 지도 · 정합 captions `#C2C0B6`, GeoTIFF disabled). Tabs 홈 · 지도 · 도면 (current, close glyph). Page: left 0–348 | 7 | centre 355–1593 | 7 | right 1600–1920.
- **Left.** Header 「레이어」 + quiet 「파일함」「전체 끄기」「세트」; same tree with 주변유적 layers on; 「표시 설정」; 파일함.
- **Centre.** Desk `desk`. Paper 1000 × 707 centred, `surface`, 1 px `edge`, r0, no shadow. Map frame 37 px in from top, left, right, 100 from the bottom: 1 px ink neatline, inner cross ticks, mono 9 integer TM numbers outside (existing 「칸 밖에 좌표 숫자」; no zebra). Inside at 1:25,000: grey 지형 ground; 조사구역 clay; dashed 500 m and 1000 m rings with mono labels; 주변유적 outlines in the six fixed colours; numbered badges (14 px circles, dataset colour edge, white fill, mono numeral, 0.5 px `#1F2937` leader only where moved), never overlapping, off paper or under the legend. Legend top-right inside the frame: `surface`, 1 px ink, r0, 220 wide: 「범례」 centred over a `line`; rows patch 20 × 14 + 11 px name (조사구역 · 주변유적경계 500 m · 1000 m · dataset outlines with code letters P B U R E); numbered list mono numeral + name (samples: 1 광령리 유물산포지 1 · 2 광령리 지석묘 1호 · 3 광령리 지석묘 2호 · 4 광령리 유물산포지 2 · 5 고성리 유물산포지). Bottom strip: 쌍칸 scale bar + 「축척 1 : 25000」 left; mono `EPSG:5186 · 중부원점(GRS80)`; 방위표 「북 화살」 28 mm far right. 표제란 bottom-right, 1 px ink r0, five rows: 도면명 유적위치도 (Heading serif 20) / 조사명 광령리1 / 유적명 제주 광령리 유적 / 축  척 1 : 25000 / 작성일 2026-10-04.
- **Floating.** 투명도·밝기 card top-left of the desk. Bottom-centre tool strip, one overlay, seven cells 64 × 52 (22 px glyph over 11 px label, `line` dividers): 좌표점 · 레이어를 가운데 · 지도 조정 · 화면 맞춤 · 항목 이동 (engaged = `hand`) · 용지/방향 · 도면 목록.
- **Right (padding 16, scrolls), three sections by leader captions, no cards.** 도면 항목: tiles 72 × 56 범례 · 표제란 (on = oat + `ring`); primary 「PDF 내보내기」 (the only clay) + secondary 「인쇄」; 「범례 제목」 [범례]; 「글자」 `9 pt` + B / I toggles. 방위 (도북): four 64 × 64 preview tiles 북 글자 · 북 화살 (chosen: oat + ink outline) · 나침반 · 바람장미; 「크기」 `28 mm`; built 도북 note Label. 도면 정보: 「1 :」 `25000` + 「적용」; fourteen scale buttons 24 high, three per row, mono (`25000` chosen); bar tiles 쌍칸 · 외칸 · 눈금 drawn as real bars; 「선택 항목 크기」 `100 %`; toggles 축척 글자 · 좌표계 · 좌표 격자 (all on).
- Status: 「도면 만드는 중 — 항목을 끌어 옮기고, 끝나면 「PDF 내보내기」. 작업 좌표계 → 제출 5179.」 (built text) … 축척 `1:25000` | 작업 5186 · 중부원점 → 제출 5179.

### 4.5 Dialogs.dc.html (2160 × 1760, `press` ground; every window with caption bar and native-like shadow, grouped S / M / L with a Caption heading)

- **S 440:** 「조사 닫기」 — Heading 「광령리1을 저장할까요?」 (copy proposal), Label 「저장하지 않은 작업이 3건 있습니다.」, mono path; 「저장」 primary · 「저장 안 함」 danger text · 「취소」. 「이름·번호」 — tip, 종류 [수혈], 시대, 번호 [`3호`]; 「저장」 · 「건너뛰기」. 「용지 설정」 — segments A4 | A3 | 직접 입력, two paper tiles 88 × 56 r0 세로 · 가로, 가로/세로 mm; 「용지 만들기」 · 「취소」. 「조사구역 그리기 설정」 — 이름 [조사구역], 외곽선 색 8 swatches (주황 chosen), 선 굵기 segments 보통 1.0 | 기본 1.5 | 굵게 2.0 | 강조 3.0 + `1.5 mm`; 「그리기 시작 (Enter)」 · 「취소」. 「API 키 입력」. 「격자 원점」. KaUserError — Heading = what failed, Label = why, primary = the fix. 「GPS 기준점」 — 점ID, X (동쪽), Y (북쪽), check 「X·Y 교환 (측량 X=북, Y=동)」, Z; meta row 측지기준계 · 타원체 · 투영 · 원점.
- **M 560:** 「새 조사」 — 조사명 [광령리1]; 「작업 좌표계」 **경도 눈금자** 512 × 72 (new): 124°–132°E degree ticks, 128° 1 px ink line, band captions 중부원점 | 동부원점, place ticks 제주 · 광주 · 서울 · 대전 · 춘천 · 원주 | 대구 · 강릉 · 부산 · 삼척 · 울산, `action` triangle 「지금 지도 중심 126.4°」; under each band its 40-high choice 「5186  중부원점」 (chosen: oat + ink outline, 「추천」 `clayText`) / 「5187  동부원점」; built suggestion sentence Label ink; built tip Caption; 「다음」 primary · 「취소」. 「시굴격자 속성」 (non-modal) — check 「지금 그린(또는 선택한) 조사구역에 자동 배치」; three radios 시굴조사 — 전체 면적의 10% (on) / 표본조사 — 전체 면적의 2% / 직접 지정 — 규격·둑을 내가 정함; check 「지형 사면 방향으로 넣기(등고선 직교)」 + Label 「지형 방향을 쓰려면 DEM을 올리세요. …」; two-column form 규격 `2 × 20 m` · 둑(간격) · 행 · 열 · 회전(방위) + 40 px dial · 이름 접두 `Tr-`; ledger 「트렌치」 `6` 개 · 「총 면적」 `240` ㎡ · 「시굴 비율」 `10.2` %; meter 512 × 4 with a tick + mono 「기준 10%」; 「구역에 깔기」 primary · 「맵에 찍기」 · 「닫기」, 「개별 편집」「전체 이동」 far left. 「도형 속성 — 유구 (면)」 (조사카드) — leader captions 식별 / 층위 / 표고 (mono, right-aligned, `m`) / 조사 / 사진 · 비고; 「저장」 · 「취소」. 「주변유적 받을 시·군 선택」 520 × 300.
- **Download 680 × 330 ×2:** 「지적도 다운로드」 — Body 13 subtitle (built), stage line 13 ink, detail Label, 4 px ink progress + mono `42%`; Caption 「다운로드 중에도 지도 작업을 계속할 수 있습니다.」 · 「취소」. 「수치지형도 내려받기」 — adds the six-stop rail 도엽 확인 → 로그인 → 신청 → 파일 준비 → 다운로드 → 지도에 올리기 (done ink check, current ink dot, todo ring), `[3/6] 신청`; 「상세 보기」 quiet · 「받은 자료 불러오기…」 · 「취소」.
- **L 860:** 「도면 인쇄」 860 × 640 — left 340: 프린터 + 「프린터 속성…」, 방법 radios, 붙였을 때 크기, 용지 A4 | A3 + 겹침 `10 mm`, 해상도, two checks; dotted-leader spec block 용지 `A3 · 6장` / 배열 `3 × 2` / 붙인 크기 `84.1 × 59.4 cm` / 축척 `1:100`; caution line. Right 448: proof on `desk`: white r0 sheets 3 × 2 with ink frames, overlap bands hatched `edge`, mono numbers, sheet 5 excluded (outline only); hint Caption; quiet 「모두 다시 넣기」. Footer 「나눈 장을 PDF로 저장…」 … 「인쇄 · 5장 + 안내 1장」 primary · 「닫기」. 「자료 준비 상태」 860 × 420 as a 삼선표.
- **Popups:** 더보기 menu (20 items, built order, `QWidgetAction` captions 「불러오기·배경」「화면」「계정과 키」「상태와 정보」) · 「지역」 tile map (17 chips 46 × 28) + address popup · 옛 지도 menu · map right-click menu · 화면 보기 submenu (새 모양 · 고대비 화면 · 큰 글씨).
- **작업 화면 견본 760 × 460 ×2 (required by review):** 단면도 studio centre: A4 paper r0 on desk, 표고 axis mono `EL. 152.00 … 153.20` at 0.10 m, 거리 axis 1-2-5, red dashed 기준선, 주기 「A–A′ 단면 · 북벽」, 축척자; right cards as leader sections 도면 / 표고·거리 눈금 / 기준선 / 축척자; 「단면도 만들기」 primary · 「PDF 저장」. 정합 split view: 맞추기 줄 on top, left pane 「왼쪽 · 맞출 도면」 with point list, right the map, numbered pairs joined by 1 px ink links.

### 4.6 Splash.dc.html (1920 × 1120) — five scenes of the 700 × 400 card + timeline

Card `ground`, r12, native shadow. Constant: icon 44 at (32, 32); `Strata` serif 60 at (32, 96); 「필드고고학 GIS · v<버전>」 14 `muted`; status 13 `muted` over a 200 × 2 ink progress line at (32, 208); bedrock band `#262624` 56 high at the bottom with the two credit lines verbatim, ivory 12 px, 16 px padding. Section drawing area: lower 160 px above the bedrock. Boards: five cards at 560 × 320 in a row with mono time captions, then a 1760 × 120 timeline (0–5 s axis with ticks every 0.5 s and one bar per element), then one card at full 700 × 400.

1. `0.0 s` — paper, wordmark, bedrock with credits; 「앱을 준비하고 있습니다」.
2. `0.4–1.8 s` — four strata boundaries inked left to right as 1 px ink lines, bottom up, 0.35 s apart, 0.8 s each, ease-out; a levelling staff (1 px ink, ticks) grows at the right with mono 10 `60.0 61.0 62.0`.
3. `1.9–2.4 s` — the pit is cut (1 px ink outline, two `muted` lens lines); ground line drawn left to right, dashed level string, two pegs, mono `EL 62.0 m`; numerals Ⅰ–Ⅴ appear at the right edge.
4. `2.6–3.2 s` — stratum Ⅲ wipes to flat `clay` in 0.6 s, the only colour on the card; 「잠시 후 홈 화면이 열립니다」.
5. `3.2–4.8 s` — two hairlines slide in from the top and left and cross where the 기준점 triangle lands, then fade to the triangle alone; progress full; 「준비 완료」. Windows animations off: scene 5 from the start.

### 4.7 System.dc.html (1920 × 2400, `ground`; eight ruled regions with leader captions, Korean headings)

1. **색** — chrome swatches 64 × 40 r4 with token name, hex (mono), role, contrast on ground (calculated); clay budget drawn as three permitted uses (primary, unsaved tile, dot + words) vs three forbidden (tab underline, focus ring, active tool).
2. **켜짐의 세 모습** — ribbon tile in every state from `ribbon-states.html` with §2.1 slots (note 「크기 · 순서 · 그림은 앱 그대로」), then draw-bar tool, oat toggle, ink tab underline.
3. **글꼴** — the eleven roles at real size with real strings (광령리1 / 1호 주거지 / body / caption / `X 147,999.305`); number-format row (`147,999.305` · `EL. 152.36` · `32.91 ㎡` · `1:25,000` · 「축척 1 : 25000」 · `5186 → 5179`).
4. **간격 · 반지름 · 높이** — spacing ruler 4…48; radii 0 / 4 / 8; heights 24 / 32 / 40 (+36 map, 48 Home row).
5. **부품** — buttons 4 kinds × 3 sizes × states, inputs, numeric, combo with patch, check / radio / slider, segments, swatches, 삼선표, ledger, dotted-leader row, tooltip, notice, menu, overlay, dialog S / M / L outlines with padding guides.
6. **상태 표시** — mark + words set (저장됨 / ● 저장 안 됨 / △ 주의 / 오류), status-bar strip with cell guides.
7. **데이터 색** — layer-kind table of §2.2 as 면 · 선 · 점 patches on paper and on 위성; 시대 띠 with labels; eight 종류 무늬; six 주변유적 colours with code letters; 유구 표지 in its three places.
8. **이렇게 / 이렇게 말고** — six before/after pairs against 1차 안: card nest → rules · pills → words · bounding box → vertices · boxed 면적 → ledger · clay on three things → one · three clay "on" tiles → three looks. Footnote: 고대비 / 큰 글씨 mapping.

## 5. Sample survey (use everywhere, unchanged)

- Survey 「광령리1」, 유적명 「제주 광령리 유적」, 제주 애월읍 광령리. File `바탕 화면 › 광령리1 › 광령리1.gpkg`. Work CRS EPSG:5186 중부원점, map centre 동경 126.4°. 마지막 열림 오늘 07:04. Cursor X `147,999.305` Y `98,157.620`. Map scale `1:500`.
- Counts: 조사구역 1 (`2,352.60` ㎡, 둘레 `198.40` m) · 유구 (면) 5 · 유구 (선) 1 (Home 「유구 6」) · 단면선 1 (A–A′) · 시굴격자 6 (Tr-1…Tr-6, 2 × 20 m, 총 240 ㎡, 10.2 %) · 유물 4 · 기준점 4.

| 유구 | 종류 | 시대 | 면적 / 길이 | 둘레 |
|---|---|---|---|---|
| 1호 주거지 (selected) | 주거지 | 청동기 | `32.91` ㎡ | `24.60` m |
| 2호 주거지 | 주거지 | 청동기 | `27.48` ㎡ | `21.90` m |
| 1호 수혈 | 수혈 | 원삼국 | `4.12` ㎡ | `7.40` m |
| 2호 수혈 | 수혈 | 원삼국 | `3.56` ㎡ | `6.90` m |
| 3호 수혈 | 수혈 | (비어 있음 → 오류) | `2.87` ㎡ | `6.20` m |
| 1호 구 (선) | 구 | 조선 | `18.40` m | |

| 기준점 | X (동) | Y (북) | Z |
|---|---|---|---|
| GCP1 | `147,962.418` | `98,131.207` | `152.36` |
| GCP2 | `148,021.774` | `98,128.560` | `151.82` |
| GCP3 | `148,026.093` | `98,181.945` | `153.10` |
| GCP4 | `147,958.650` | `98,186.312` | `153.74` |

- Checklist (28 rules): 오류 1 (「종류·시대가 빈 유구가 있습니다.」 → 유구 (면) · 3호 수혈) · 주의 3 (개별유구실측도 없음 · 층위도 없음 · 주변유적 자료가 지도에 있음) · 통과 24. Sheets: 조사구역도 · 유적위치도 · 유구배치도 준비됨. Unsaved on Main: 3건. People and dates: 조사자 권영인, 조사일 `2026-10-03`.

## 6. Copy rules

- Field Korean in 합니다체, one idea per sentence, at most 40 characters a line; 「」 around button names; no em dash in new copy (built sentences stay verbatim), 가운뎃점 only between equals; say 조사 · 도형 · 도면 · 면, never 프로젝트 · 피처 · 레이아웃. No emoji, no invented statistics.
- Numbers: 좌표 3 decimals with thousands separators; 표고 `EL. 152.36`; 면적 2 decimals + ㎡ (one unit in chrome); 길이 · 둘레 2 decimals + m; scale `1:25,000` in prose, `1:500` in the status combo as built, 「축척 1 : 25000」 on paper; 유구 「1호 주거지」 (number first), 트렌치 `Tr-1`, 단면 `A–A′`, 기준점 `GCP1`; dates `2026-10-03` in fields, relative in lists (오늘 07:04).
- EPSG code always with its 원점 name the first time on a screen. North is always 「도북」.
- **new (proposal), seven, display only:** live 변 · 면적 in the guide band · 도북 mark by the scale bar · feature counts in the layer key · 시대 띠 · 경도 눈금자 in 새 조사 · 필수 도면 5종 strip · 기준점 X / Y / Z + meta in the panel. **Restyle:** 좌표격자 labels as the 도곽 band. **Copy proposals:** 「주소·지번 찾기  Ctrl+F」 · 「광령리1을 저장할까요?」 · 「작업 5186 · 중부원점」. **Size changes to state:** status bar 34 → 32, splash radius 16 → 12, Home 이어서 작업 width, 도면 인쇄 fixed at 860 × 640.

## 7. Do-not list

- No change to ribbon geometry, order, labels or drawings; no new icons; no removed feature; draw-bar order, status-bar contents, 10 px layer key, two inspector tabs, edit pencil and 「글자」 check box stay.
- No cards, nested boxes, pills, bordered tags, icon discs, zebra rows, footer bands, left accent bars, rounded or inset map, shadow on docked things.
- No clay on tabs, focus rings, links, progress, selected rows, active tools or basemap toggles; no second filled button in a view; no green marks on steady states; no filled red button.
- No serif under 20 px, on section labels, buttons, tabs or map labels; no bold for hierarchy or selection; no text lighter than `#73726C`; no Hangul in mono.
- No permanent map or paper rulers, registration crosses, contour decoration, survey-outline thumbnails, peninsula drawing, zebra sheet frame, floating toast, white tooltip, key-cap in tooltips.
- No gradient, blur, glass, translucency, bounce or overshoot; no `QGraphicsDropShadowEffect`; no fractional borders; nothing that draws on or refreshes the QGIS canvas.
- No dark theme, photo tab, mini-map, stratigraphy matrix, finds table, autosave; no Anthropic or Claude name, logo, mark or proprietary font.
