# Beginner Workflow and Layer UX Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make a survey project a guided seven-step field-archaeology workflow with intuitive VWorld controls and safe, reorderable layers.

**Architecture:** Keep project-state interpretation in a new testable `WorkflowGuide` core module; `MainWindow` renders that state and routes UI events. Extend `LayerOps` with testable VWorld configuration and layer-tree operations while keeping QGIS UI widgets out of core services. Store the VWorld key in per-user `QSettings`; it is never a C++ default value, test fixture, or distribution setting.

**Tech Stack:** C++20, Qt6 Widgets/Core/Test, QGIS 4.x (`QgsLayerTreeModel`, `QgsLayerTreeView`, `QgsProject`, `QgsMapCanvas`), CMake/CTest.

## Global Constraints

- Preserve all unrelated dirty working-tree changes; do not reset, checkout, or mass-format files.
- Keep Korean labels short, concrete, and paired with accessible tooltips.
- The initial user is a GIS-new field archaeology worker; expert menus remain available but are not the primary workflow.
- Never hardcode, log, commit, or distribute a VWorld key. Read it only from user-scoped `QSettings` at the point of use.
- The seven steps are: 새 조사, 배경·지적도, 조사구역, 유구·단면선, GPS 기준점, 도면 검수, 제출 패키지.
- Submission package creation remains blocked while checklist error count is nonzero.
- Every production behavior starts with a focused failing QtTest assertion, then minimal implementation, then a green re-run.
- QGIS layer drag/reorder must use `QgsLayerTreeModel::AllowNodeReorder`; the test must exercise `mimeData()` and `dropMimeData()` and assert the resulting root-node order.

---

## File Structure

| File | Responsibility |
| --- | --- |
| `src/core/WorkflowGuide.h/.cpp` | Pure seven-step definitions and completion evaluation from a snapshot. |
| `src/core/VworldSettings.h/.cpp` | Read/write a single per-user VWorld API key without a fallback key. |
| `src/core/LayerOps.h/.cpp` | Build VWorld raster layer URIs from an injected key, report background visibility, and remove confirmed layers. |
| `src/app/KaIcons.cpp` | Draw non-textual, recognizable map-source and action icons. |
| `src/app/MainWindow.h/.cpp` | Render the workflow rail, contextual action toolbar, map-source cards, and safe layer interaction. |
| `tests/test_workflow.cpp` | Test `WorkflowGuide`, `VworldSettings`, VWorld layers, QGIS model drag/drop, and confirmed removal behavior. |
| `CMakeLists.txt` | Compile new core modules into `ka_core`. |
| `docs/user/gui-scenario-checklist.md` | Add the manual acceptance path for the seven-step UI, VWorld overlays, reordering, and deletion. |

### Task 1: Add a testable seven-step workflow guide

**Files:**
- Create: `src/core/WorkflowGuide.h`
- Create: `src/core/WorkflowGuide.cpp`
- Modify: `CMakeLists.txt: KA_INCLUDES and ka_core source list`
- Modify: `tests/test_workflow.cpp: TestWorkflow declarations and implementations`

**Interfaces:**
- Consumes: `QJsonObject` produced by `ProjectStateBuilder`, `bool hasVisibleReferenceLayer`, `int checklistErrorCount`, `bool packageCreated`.
- Produces:

```cpp
enum class WorkflowStep : int {
  Survey = 0, Background, SurveyArea, Features, ControlPoints, Review, Submission
};

struct WorkflowStepState {
  WorkflowStep step;
  QString title;
  QString completionHint;
  QString actionId;
  bool complete;
};

class WorkflowGuide {
public:
  static QList<WorkflowStepState> evaluate(
      const QJsonObject& projectState,
      bool hasVisibleReferenceLayer,
      int checklistErrorCount,
      bool packageCreated);
  static bool canCreateSubmission(int checklistErrorCount);
};
```

- Uses later: `MainWindow::refreshWorkflowGuide()` renders every returned `WorkflowStepState` without duplicating completion rules.

- [ ] **Step 1: Write the failing workflow test**

Add to `TestWorkflow`:

```cpp
void workflowGuideTracksSevenRealMilestones();

void TestWorkflow::workflowGuideTracksSevenRealMilestones() {
  QJsonObject state = ProjectStateBuilder::empty();
  const auto empty = WorkflowGuide::evaluate(state, false, 3, false);
  QCOMPARE(empty.size(), 7);
  QVERIFY(!empty.at(int(WorkflowStep::Survey)).complete);
  QVERIFY(!WorkflowGuide::canCreateSubmission(3));

  state.insert(QStringLiteral("survey_area_count"), 1);
  state.insert(QStringLiteral("feature_poly_count"), 1);
  state.insert(QStringLiteral("has_kind_period"), true);
  state.insert(QStringLiteral("control_points_count"), 2);
  state.insert(QStringLiteral("has_datum"), true);
  state.insert(QStringLiteral("has_ellipsoid"), true);
  state.insert(QStringLiteral("has_projection"), true);
  const auto ready = WorkflowGuide::evaluate(state, true, 0, true);
  QVERIFY(ready.at(int(WorkflowStep::Background)).complete);
  QVERIFY(ready.at(int(WorkflowStep::ControlPoints)).complete);
  QVERIFY(ready.at(int(WorkflowStep::Review)).complete);
  QVERIFY(ready.at(int(WorkflowStep::Submission)).complete);
  QVERIFY(WorkflowGuide::canCreateSubmission(0));
}
```

- [ ] **Step 2: Run the focused test and observe RED**

Run:

```powershell
cmake --build build --config Release --target ka_workflow_tests
& .\build\Release\ka_workflow_tests.exe workflowGuideTracksSevenRealMilestones
```

Expected: compilation fails because `core/WorkflowGuide.h` and `WorkflowGuide` do not exist.

- [ ] **Step 3: Implement the smallest workflow guide**

Create the declared interface and return exactly seven ordered values. Use existing state keys as follows:

```cpp
const bool hasSurvey = projectState.value("project_crs_set").toBool();
const bool hasArea = projectState.value("survey_area_count").toInt() > 0;
const bool hasFeatures = projectState.value("feature_poly_count").toInt() > 0
    && projectState.value("has_kind_period").toBool();
const bool hasGcp = projectState.value("control_points_count").toInt() >= 2
    && projectState.value("has_datum").toBool()
    && projectState.value("has_ellipsoid").toBool()
    && projectState.value("has_projection").toBool();
```

Use `checklistErrorCount == 0` for review and `packageCreated && checklistErrorCount == 0` for submission. Give each state its Korean title, concise completion hint, and existing action ID (`newSurvey`, `background`, `surveyArea`, `featurePoly`, `controlPoint`, `checklist`, `submission`). Add source files to `ka_core` in CMake.

- [ ] **Step 4: Run the focused test and observe GREEN**

Run the same command from Step 2.

Expected: `workflowGuideTracksSevenRealMilestones` passes.

- [ ] **Step 5: Run the entire workflow test executable**

Run:

```powershell
& .\build\Release\ka_workflow_tests.exe
```

Expected: all pre-existing workflow tests and the new guide test pass.

- [ ] **Step 6: Commit the isolated core behavior**

```powershell
git add CMakeLists.txt src/core/WorkflowGuide.h src/core/WorkflowGuide.cpp tests/test_workflow.cpp
git commit -m "feat: model seven-step survey workflow"
```

### Task 2: Move VWorld configuration out of source and test source layers

**Files:**
- Create: `src/core/VworldSettings.h`
- Create: `src/core/VworldSettings.cpp`
- Modify: `src/core/LayerOps.h`
- Modify: `src/core/LayerOps.cpp`
- Modify: `src/app/MainWindow.h`
- Modify: `src/app/MainWindow.cpp`
- Modify: `CMakeLists.txt`
- Modify: `tests/test_workflow.cpp`

**Interfaces:**
- Consumes: a caller-provided `QSettings&` or a caller-provided nonempty VWorld key.
- Produces:

```cpp
class VworldSettings {
public:
  static QString apiKey(QSettings& settings);
  static void setApiKey(QSettings& settings, const QString& key);
  static bool hasApiKey(QSettings& settings);
};

static bool LayerOps::addVworldBaseMap(
    QgsProject* project, QgsMapCanvas* canvas, const QString& apiKey, QString* errorOut = nullptr);
static bool LayerOps::hasVisibleReferenceLayer(const QgsProject* project);
```

- Uses later: Map-source cards call `VworldSettings` before calling a `LayerOps::addVworld*` method; an empty key returns a Korean actionable error and makes no network request.

- [ ] **Step 1: Write failing settings and no-key layer tests**

Add the declarations and these tests:

```cpp
void vworldKeyUsesOnlyUserSettings();
void vworldLayerRejectsMissingKey();

void TestWorkflow::vworldKeyUsesOnlyUserSettings() {
  QTemporaryDir dir;
  QVERIFY(dir.isValid());
  QSettings settings(QDir(dir.path()).filePath("vworld.ini"), QSettings::IniFormat);
  QVERIFY(!VworldSettings::hasApiKey(settings));
  VworldSettings::setApiKey(settings, QStringLiteral("test-key"));
  QCOMPARE(VworldSettings::apiKey(settings), QStringLiteral("test-key"));
}

void TestWorkflow::vworldLayerRejectsMissingKey() {
  QgsProject project;
  QString error;
  QVERIFY(!LayerOps::addVworldBaseMap(&project, nullptr, {}, &error));
  QVERIFY(error.contains(QStringLiteral("VWorld")));
  QVERIFY(project.mapLayersByName(QStringLiteral("VWorld 배경지도")).isEmpty());
}
```

Change the existing VWorld layer test to pass `QStringLiteral("test-key")` explicitly, and remove every use of `DEFAULT_VWORLD_KEY`.

- [ ] **Step 2: Run the focused tests and observe RED**

Run:

```powershell
cmake --build build --config Release --target ka_workflow_tests
& .\build\Release\ka_workflow_tests.exe vworldKeyUsesOnlyUserSettings vworldLayerRejectsMissingKey
```

Expected: compilation fails because `VworldSettings` does not exist; after adding only declarations, `vworldLayerRejectsMissingKey` fails because the current fallback key allows the layer.

- [ ] **Step 3: Implement the smallest safe configuration path**

Implement `VworldSettings` with setting group `ka-hgis/vworld` and key `apiKey`. Trim input in `setApiKey`; remove the setting for an empty value. Remove `DEFAULT_VWORLD_KEY`, `m_vworldKey` initialization, and every default `apiKey` function parameter. At the top of each `LayerOps::addVworld*` method return false with `VWorld API 키를 설정하세요.` when `apiKey.trimmed().isEmpty()`.

In `MainWindow::configureVworldKey()`, read and write `QSettings`, never display the existing key in a dialog, and call `refreshWorkflowGuide()` after a successful map add. Each map action obtains the key only immediately before the `LayerOps` call.

- [ ] **Step 4: Run the focused tests and observe GREEN**

Run the command from Step 2.

Expected: both tests pass and no VWorld layer is inserted for an empty key.

- [ ] **Step 5: Search for forbidden key defaults and run all workflow tests**

Run:

```powershell
rg -n "DEFAULT_VWORLD_KEY|899763|m_vworldKey" src tests dist
& .\build\Release\ka_workflow_tests.exe
```

Expected: `rg` has no match; all workflow tests pass.

- [ ] **Step 6: Commit the secure VWorld configuration**

```powershell
git add CMakeLists.txt src/core/VworldSettings.h src/core/VworldSettings.cpp src/core/LayerOps.h src/core/LayerOps.cpp src/app/MainWindow.h src/app/MainWindow.cpp tests/test_workflow.cpp
git commit -m "fix: require configured VWorld access key"
```

### Task 3: Make layer reorder and removal behavior testable and safe

**Files:**
- Modify: `src/core/LayerOps.h`
- Modify: `src/core/LayerOps.cpp`
- Modify: `src/app/MainWindow.h`
- Modify: `src/app/MainWindow.cpp`
- Modify: `tests/test_workflow.cpp`

**Interfaces:**
- Consumes: a `QgsProject*`, IDs of confirmed target layers, and the existing QGIS layer-tree root.
- Produces:

```cpp
struct LayerRemovalPlan {
  QStringList layerIds;
  QStringList layerNames;
  bool hasEditableVectorLayer;
};

static LayerRemovalPlan LayerOps::planLayerRemoval(const QList<QgsMapLayer*>& layers);
static bool LayerOps::removeLayers(QgsProject* project, const QStringList& layerIds);
```

- Uses later: `MainWindow::removeSelectedLayers()` creates a plan, shows its names, resolves editing layers, and calls `removeLayers()` only after the user confirms.

- [ ] **Step 1: Write failing model-reorder and removal tests**

Add declarations and tests:

```cpp
void layerTreeDragDropReordersRootLayers();
void confirmedLayerRemovalRemovesOnlyPlannedLayers();

void TestWorkflow::layerTreeDragDropReordersRootLayers() {
  QgsProject project;
  auto* first = new QgsVectorLayer("Point?crs=EPSG:5179", QStringLiteral("first"), "memory");
  auto* second = new QgsVectorLayer("Point?crs=EPSG:5179", QStringLiteral("second"), "memory");
  QVERIFY(first->isValid());
  QVERIFY(second->isValid());
  project.addMapLayer(first);
  project.addMapLayer(second);
  QgsLayerTreeModel model(project.layerTreeRoot());
  model.setFlags(model.flags() | QgsLayerTreeModel::AllowNodeReorder);
  const QModelIndex firstIndex = model.index(0, 0);
  std::unique_ptr<QMimeData> mime(model.mimeData({firstIndex}));
  QVERIFY(model.dropMimeData(mime.get(), Qt::MoveAction, 2, 0, QModelIndex()));
  QCOMPARE(QgsLayerTree::toLayer(project.layerTreeRoot()->children().at(1))->layer()->name(), QStringLiteral("first"));
}

void TestWorkflow::confirmedLayerRemovalRemovesOnlyPlannedLayers() {
  QgsProject project;
  auto* keep = new QgsVectorLayer("Point?crs=EPSG:5179", "keep", "memory");
  auto* remove = new QgsVectorLayer("Point?crs=EPSG:5179", "remove", "memory");
  project.addMapLayer(keep);
  project.addMapLayer(remove);
  const LayerRemovalPlan plan = LayerOps::planLayerRemoval({remove});
  QVERIFY(LayerOps::removeLayers(&project, plan.layerIds));
  QVERIFY(project.mapLayersByName(QStringLiteral("remove")).isEmpty());
  QCOMPARE(project.mapLayersByName(QStringLiteral("keep")).size(), 1);
}
```

- [ ] **Step 2: Run the focused tests and observe RED**

Run:

```powershell
cmake --build build --config Release --target ka_workflow_tests
& .\build\Release\ka_workflow_tests.exe layerTreeDragDropReordersRootLayers confirmedLayerRemovalRemovesOnlyPlannedLayers
```

Expected: `confirmedLayerRemovalRemovesOnlyPlannedLayers` does not compile because `LayerRemovalPlan` and the two `LayerOps` methods do not exist. The reorder assertion may fail until the UI/model flags are applied correctly.

- [ ] **Step 3: Implement only confirmed removal and native reorder refresh**

Implement the declared plan and removal functions. `planLayerRemoval` must de-duplicate IDs and set `hasEditableVectorLayer` only for `QgsVectorLayer::isEditable()`. `removeLayers` must return false for a null project or empty ID list and otherwise call exactly `project->removeMapLayers(layerIds)`.

In `buildUi()`, set model flags:

```cpp
model->setFlags(model->flags()
    | QgsLayerTreeModel::AllowNodeReorder
    | QgsLayerTreeModel::AllowNodeChangeVisibility);
```

Keep `InternalMove` and connect the model's rows-moved and layout-changed notifications to one lambda that calls `m_canvas->refreshAllLayers()` and `refreshWorkflowGuide()`.

Replace direct removal with: no action on an empty plan; `QMessageBox::question` showing the planned names; for editable layers show Save, Discard, Cancel and commit or roll back only the selected editable layers; call `LayerOps::removeLayers` after confirmation. Do not handle Delete outside the layer tree event filter.

- [ ] **Step 4: Run the focused tests and observe GREEN**

Run the command from Step 2.

Expected: root children are reordered by actual `QgsLayerTreeModel::dropMimeData()` and only the confirmed layer is removed.

- [ ] **Step 5: Run full regression tests**

Run:

```powershell
& .\build\Release\ka_hgis_tests.exe
& .\build\Release\ka_workflow_tests.exe
ctest --test-dir build -C Release --output-on-failure
```

Expected: all commands exit zero.

- [ ] **Step 6: Commit safe layer operations**

```powershell
git add src/core/LayerOps.h src/core/LayerOps.cpp src/app/MainWindow.h src/app/MainWindow.cpp tests/test_workflow.cpp
git commit -m "feat: safely reorder and remove layers"
```

### Task 4: Render the guided UX and intuitive VWorld controls

**Files:**
- Modify: `src/app/KaIcons.cpp`
- Modify: `src/app/MainWindow.h`
- Modify: `src/app/MainWindow.cpp`
- Modify: `docs/user/gui-scenario-checklist.md`

**Interfaces:**
- Consumes: `WorkflowGuide::evaluate()`, `VworldSettings`, the existing map tools, and the existing `KaIcons::icon(id)` factory.
- Produces: `MainWindow::refreshWorkflowGuide()`, `MainWindow::selectWorkflowStep(int)`, and clickable map-source controls with visible name, icon, purpose, and opacity.

- [ ] **Step 1: Write the failing icon and workflow-rail test**

Extend `tests/test_workflow.cpp` with the app icon source and a focused Qt Widget test target in CMake:

```cmake
target_sources(ka_workflow_tests PRIVATE src/app/KaIcons.cpp)
target_link_libraries(ka_workflow_tests PRIVATE Qt6::Svg)
```

Add:

```cpp
void vworldSourceIconsAreAvailable();

void TestWorkflow::vworldSourceIconsAreAvailable() {
  for (const QString& id : {"vworld_base", "vworld_sat", "vworld_cadastral", "vworld_hybrid", "vworld_contour"})
    QVERIFY2(!KaIcons::icon(id).isNull(), qPrintable(id));
}
```

- [ ] **Step 2: Run the focused test and observe RED**

Run:

```powershell
cmake --build build --config Release --target ka_workflow_tests
& .\build\Release\ka_workflow_tests.exe vworldSourceIconsAreAvailable
```

Expected: compilation fails because `KaIcons.cpp` is not linked into the workflow test target and its header is not included.

- [ ] **Step 3: Implement the smallest guided screen**

Restore the left workflow rail above the layer dock. Each row shows numbered icon, Korean title, and state colour; selecting it calls `selectWorkflowStep(int)` and only replaces the contextual action toolbar/help panel. `refreshWorkflowGuide()` obtains live `ProjectStateBuilder::fromProject()`, calls `runChecklist` evaluation without modifying the visible report, detects reference-layer visibility, and updates all row states.

Build five source cards with `QToolButton` using `ToolButtonTextUnderIcon`; set text to `기본지도`, `위성사진`, `지적도`, `하이브리드`, `등고선`; add short tooltips from the design. Keep opacity sliders as secondary controls under each active card. Remove words drawn inside icon pixels: use distinct geometric drawings for the five icon IDs while retaining text labels outside the icons.

Invoke `refreshWorkflowGuide()` after new/open survey, map-source visibility/opacity changes, geometry capture, control-point add/import, checklist execution, layer reorder, layer removal, and submission result. Track package creation in `MainWindow` only after `ExportService::exportSubmissionPackage` returns a nonempty output.

- [ ] **Step 4: Run the focused test and observe GREEN**

Run the command from Step 2.

Expected: all five map-source icons are non-null.

- [ ] **Step 5: Add the manual field scenario and build the application**

Add this exact section to `docs/user/gui-scenario-checklist.md`:

```markdown
### 초보자 7단계와 레이어 조작
1. 새 조사를 만든다. 왼쪽의 1단계가 완료로 바뀌는지 확인한다.
2. 위성사진과 지적도를 차례로 켜고, 지적도 투명도를 낮춘다.
3. 지적도를 위성사진 위·아래로 드래그하고 지도 표현 순서가 바뀌는지 확인한다.
4. 지적도를 선택해 Delete를 누른다. 취소하면 유지되고, 확인하면 지적도만 삭제되는지 확인한다.
5. 6단계 검수에 error가 있으면 7단계에서 패키지 생성이 차단되는지 확인한다.
```

Run:

```powershell
cmake --build build --config Release --target ka-hgis ka_workflow_tests
& .\build\Release\ka_workflow_tests.exe vworldSourceIconsAreAvailable
```

Expected: application and test executable build, and the icon test passes.

- [ ] **Step 6: Commit the guided interface**

```powershell
git add CMakeLists.txt src/app/KaIcons.cpp src/app/MainWindow.h src/app/MainWindow.cpp tests/test_workflow.cpp docs/user/gui-scenario-checklist.md
git commit -m "feat: guide novice survey workflow"
```

### Task 5: Full verification and delivery audit

**Files:**
- Modify when required by generated artifacts only: `docs/COMMIT_STATUS.md`

**Interfaces:**
- Consumes: completed Tasks 1–4 and existing build scripts.
- Produces: verified build, tests, portable smoke result, and a manual GUI evidence checklist.

- [ ] **Step 1: Build the complete Release configuration**

Run:

```powershell
cmake --build build --config Release
```

Expected: exit code zero and `build/Release/ka-hgis.exe`, `ka_hgis_tests.exe`, and `ka_workflow_tests.exe` exist.

- [ ] **Step 2: Run every CTest test**

Run:

```powershell
ctest --test-dir build -C Release --output-on-failure
```

Expected: all configured tests pass.

- [ ] **Step 3: Run the portable smoke and end-to-end scripts**

Run:

```powershell
.\scripts\run-ka-hgis.ps1 --smoke-quit
.\scripts\e2e-smoke.ps1
```

Expected: both scripts exit zero; record output if they cannot be run because QGIS runtime or display access is unavailable.

- [ ] **Step 4: Perform the manual acceptance scenario**

Run the five steps written in `docs/user/gui-scenario-checklist.md` and record pass/fail for: stage statuses, VWorld satellite plus cadastral overlay, drag/drop paint order, Delete cancellation and confirmation, and failed-checklist submission block.

- [ ] **Step 5: Audit source and working tree before delivery**

Run:

```powershell
rg -n "DEFAULT_VWORLD_KEY|899763|m_vworldKey" src tests dist
git diff --check
git status --short
```

Expected: forbidden key search has no matches; whitespace check exits zero; status lists only intended, disclosed changes.

- [ ] **Step 6: Commit verification metadata if the hook leaves it modified**

```powershell
git add docs/COMMIT_STATUS.md
git commit -m "docs: refresh commit status"
```
