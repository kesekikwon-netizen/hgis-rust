# C++ QtTest Parity Audit — Set C

Read-only comparison of the 37 C++ QtTest files (`target/cpp-ref/tests/test_*.cpp`) assigned to Set C against the Rust workspace tests in `crates/`.

- **Statuses**: `PORTED` (Rust test covers the C++ assertions), `PARTIAL` (Rust test covers part of the slot; uncovered parts noted), `MISSING` (Rust implementation exists, no test covers the slot), `BLOCKED` (underlying Rust feature is `NOT BUILT`).
- Qt lifecycle fixtures (`init`, `cleanup`, `initTestCase`, `cleanupTestCase`) and `*_data()` table-builder helpers are omitted as standalone rows and noted inside their corresponding test slot.

## test_above_labels.cpp

| # | C++ test function | C++ line | what it checks | Rust test | Rust file:line | Rust code under test | status | note |
|---|---|---|---|---|---|---|---|---|
| 1 | overlayMatchesCanvas | 52 | 4 `_data()` ops (initial, pan, zoom, resize): floating label overlay pixel-matches base canvas | — | — | NOT BUILT | BLOCKED | Overlay pixel cache & above-labels renderer not built |
| 2 | cacheUsesCanvasPixelsAndRedrawsOnlyOnChange | 138 | Base pixel cache avoids redundant redraws when canvas state is unchanged | — | — | NOT BUILT | BLOCKED | Overlay pixel cache not built |
| 3 | heavyLayerDoesNotFreezeTheWindow | 188 | Off-UI-thread rendering keeps event loop responsive with dense label features | — | — | NOT BUILT | BLOCKED | Async overlay label renderer not built |

## test_above_labels_names.cpp

| # | C++ test function | C++ line | what it checks | Rust test | Rust file:line | Rust code under test | status | note |
|---|---|---|---|---|---|---|---|---|
| 1 | sameNameLabelledOnceWhenCanvasAlsoDrawsLayer | 83 | Same feature name is not drawn twice across overlay and canvas layers | — | — | NOT BUILT | BLOCKED | Overlay label compositor not built |
| 2 | layerTreeBridgeKeepsOverlayLayersOffTheMap | 144 | Layer tree bridge isolates internal overlay layers from user map layers | — | — | NOT BUILT | BLOCKED | Overlay layer tree bridge not built |

## test_buffer.cpp

| # | C++ test function | C++ line | what it checks | Rust test | Rust file:line | Rust code under test | status | note |
|---|---|---|---|---|---|---|---|---|
| 1 | ringHasGapAndLabel | 34 | 500 m buffer ring leaves gap between inner and outer rings and formats 4 cardinal labels | ring_has_one_line_and_four_labels | crates/strata-engine/src/rings_tests.rs:11 | strata_engine::rings | PORTED | Exact ring geometry and 4-label check |
| 2 | offsetIs500mFromSource | 77 | Buffer boundary vertices lie 500 m from source survey polygon | ring_is_500m_from_the_source | crates/strata-engine/src/rings_tests.rs:26 | strata_engine::rings | PORTED | 500 m radial tolerance check |
| 3 | multiPartSurveyArea_ringsEveryPart | 127 | Multi-part survey area produces merged 500 m ring covering all parts | multi_part_area_rings_every_part | crates/strata-engine/src/rings_tests.rs:47 | strata_engine::rings | PORTED | Multi-polygon union ring check |
| 4 | ringLayers_bakeIntoSurveyOnlyOnSave | 166 | Ring layers bake into survey GPKG only upon explicit save | — | — | NOT BUILT | BLOCKED | Ring layer bake-on-save workflow not built |
| 5 | moveFeatureVertex_updatesPolygonCorner | 199 | Moving a polygon vertex updates geometry corner and undo stack | — | — | NOT BUILT | BLOCKED | Interactive vertex-drag geometry tool not built |
| 6 | setLabelsVisible_togglesPolygonLabels | 225 | Toggling layer label visibility enables/disables polygon labels | actions_apply_to_the_clicked_layer | crates/strata-app/src/gui_layer_panel_context_menu_tests.rs:55 | strata_app::gui_layer_panel_context_menu | PARTIAL | Tests menu action toggle; canvas label visibility suppression not verified |
| 7 | clipLayerByBoundary_clipsIntersectingFeatures | 241 | Clips layer features against survey boundary polygon | — | — | NOT BUILT | BLOCKED | Layer boundary clip command not built |
| 8 | splitPolygonWithLine_splitsGeometry | 280 | Splitting a polygon with a line produces two valid child polygons | polygon_split_and_helmert_georef_produce_two_pieces_and_submillimeter_rms | crates/strata-geo/src/polygon_ops_tests.rs:15 | strata_geo::polygon_ops | PARTIAL | Also `test_polygon_ops_split_10x10_square_yields_two_50m2_pieces` (`crates/strata-geo/src/polygon_ops_tests.rs:75`); interactive layer split/FID/undo not built |
| 9 | splitTwoOverlappingFeatures_splitsTargetPolygon | 316 | Splitting across overlapping features cuts only the target polygon | — | — | NOT BUILT | BLOCKED | Interactive multi-feature canvas split tool not built |

## test_checklist.cpp

| # | C++ test function | C++ line | what it checks | Rust test | Rust file:line | Rust code under test | status | note |
|---|---|---|---|---|---|---|---|---|
| 1 | loadRules | 82 | Loads 27 JSON checklist rules in exact order and validates rule metadata | test_drawing_checklist_v1_json_loads_all_27_cpp_rules_in_exact_order | crates/strata-check-export/src/checklist_engine_tests.rs:191 | strata_check_export::checklist_engine | PARTIAL | Tests bundled JSON rules; omits file-load failure paths |
| 2 | evaluateFailsWithoutSurvey | 110 | Checklist fails required survey rules when project has no survey area | cpp_new_survey_matches_cpp | crates/strata-app/src/checklist_state_tests.rs:95 | strata_check_export::checklist_engine | PARTIAL | Tests new GPKG missing `survey_area`; direct `ChecklistEngine::evaluate({})` on empty map not asserted |
| 3 | evaluatePassesHappyFixture | 120 | Clean populated state passes all 27 checklist rules with zero errors/warnings | demo_matches_cpp | crates/strata-app/src/checklist_state_tests.rs:71 | strata_check_export::checklist_engine | PARTIAL | `demo.gpkg` still fails 3 sheet rules; no all-27-passing happy-state fixture test |
| 4 | exportBlocksOnError | 147 | Submission export aborts when error-severity checklist rules fail | test_submit_package_exporter_blocks_on_missing_feat_no_and_succeeds_after_fix | crates/strata-check-export/src/submit_package_tests.rs:30 | strata_check_export::submit_package | PORTED | Blocks on missing `feat_no` error then succeeds |
| 5 | exportAllowsWhenClean | 158 | Clean checklist allows submission package export | submission_package_writes_5179_manifest_and_non_blocking_readme | crates/strata-check-export/src/submit_package_tests.rs:10 | strata_check_export::submit_package | PORTED | Writes EPSG:5179 SHP, manifest, and README |
| 6 | pdfViaLayout | 179 | Print layout PDF exporter generates non-empty submission PDF | — | — | NOT BUILT | BLOCKED | Print layout PDF exporter not built |
| 7 | surveyCreatesGpkg | 192 | Creating a new survey creates a valid GPKG with schema tables | created_gpkg_passes_sqlite_pragma_integrity_check_and_has_gpkg_contents | crates/strata-storage/src/gpkg_core_tests.rs:16 | strata_storage::gpkg::GpkgRepository | PORTED | Checks `PRAGMA integrity_check` and `gpkg_contents` |
| 8 | liveStateFromEmptyProject | 202 | Live `ProjectStateBuilder` on an empty unsaved project yields zero counts | — | — | strata_app::checklist_state | MISSING | `StrataAppState::checklist_state` on blank startup state has no test |
| 9 | fromProject_emptySeededSiteLocation_layoutExistsFalse | 209 | Empty seeded `site_location` layout yields `layoutExists = false` | — | — | NOT BUILT | BLOCKED | Print layout `isComposedStudioSheet` validator not built |
| 10 | fromProject_composedUserSheet_layoutExistsTrue | 225 | User-composed print sheet layout yields `layoutExists = true` | — | — | NOT BUILT | BLOCKED | Print layout `isComposedStudioSheet` validator not built |
| 11 | test_uncomposed_user_sheet_rejected | 311 | Uncomposed user sheet layout is rejected | — | — | NOT BUILT | BLOCKED | Print layout `isComposedStudioSheet` validator not built |
| 12 | test_dummy_blank_layer_rejected | 368 | Dummy blank layer in layout map item does not satisfy composition | — | — | NOT BUILT | BLOCKED | Print layout `isComposedStudioSheet` validator not built |
| 13 | test_autogenerated_template_not_passing_feature_plan | 414 | Auto-generated template does not pass `feature_plan` requirement | — | — | NOT BUILT | BLOCKED | Print layout `isComposedStudioSheet` validator not built |
| 14 | challenge1_isComposed_emptyVectorAndRaster | 465 | Layout composition rejects empty vector/raster layers | — | — | NOT BUILT | BLOCKED | Print layout `isComposedStudioSheet` validator not built |
| 15 | challenge2_isComposed_basemapCombinations | 502 | Layout composition checks valid basemap combinations | — | — | NOT BUILT | BLOCKED | Print layout `isComposedStudioSheet` validator not built |
| 16 | challenge3_isComposed_zeroAreaPolygon | 563 | Zero-area polygon inside layout map extent does not satisfy composition | — | — | NOT BUILT | BLOCKED | Print layout `isComposedStudioSheet` validator not built |
| 17 | challenge4_isComposed_nanInfExtentsNegativeScales | 648 | Rejects NaN/Inf extents and negative scales in layout map item | — | — | NOT BUILT | BLOCKED | Print layout `isComposedStudioSheet` validator not built |
| 18 | challenge5_isComposed_customNamedLayoutsAndMultipleMaps | 718 | Validates custom-named layouts and multi-map layout sheets | — | — | NOT BUILT | BLOCKED | Print layout `isComposedStudioSheet` validator not built |
| 19 | challenge6_projectState_featurePlanWithoutActualFeatures | 785 | Rejects `feature_plan` sheet state when project lacks actual features | — | — | NOT BUILT | BLOCKED | Print layout `isComposedStudioSheet` validator not built |
| 20 | challenge7_projectState_siteLocationWithoutSurveyArea | 846 | Rejects `site_location` sheet state when project lacks `survey_area` | — | — | NOT BUILT | BLOCKED | Print layout `isComposedStudioSheet` validator not built |
| 21 | challenge8_isComposed_autoGeneratedTemplateBypass | 911 | Rejects bypass attempt using auto-generated template flag | — | — | NOT BUILT | BLOCKED | Print layout `isComposedStudioSheet` validator not built |

## test_checklist_rules.cpp

| # | C++ test function | C++ line | what it checks | Rust test | Rust file:line | Rust code under test | status | note |
|---|---|---|---|---|---|---|---|---|
| 1 | cleanStatePassesEveryRule | 106 | Synthetic completely clean JSON state passes every rule | — | — | strata_check_export::checklist_engine | MISSING | `ChecklistEngine::evaluate` clean-all pass case untested |
| 2 | everyRuleHasAViolationRow | 113 | Ensures violation test table covers every rule ID in `drawing_checklist_v1.json` | — | — | strata_check_export::checklist_engine | MISSING | Exhaustive per-rule coverage matrix not ported |
| 3 | singleViolationFailsOnlyThatRule | 132 | Parameterized over 27 violations: injecting one bad key fails only its matching rule | — | — | strata_check_export::checklist_engine | MISSING | Per-rule isolation matrix over all 27 rules untested |
| 4 | countMinReadsItsParameter | 155 | Dynamic `count_min:<key>:<n>` rule parses `<n>` threshold parameter | — | — | strata_check_export::checklist_engine | MISSING | `count_min` threshold parametrization untested directly |
| 5 | unknownCheckOrSeverityFailsClosed | 168 | Unknown `check_type` or `severity` in JSON fails closed (rejected/error) | — | — | strata_check_export::checklist_engine | MISSING | Fail-closed unknown-rule validation untested |
| 6 | failedLoadKeepsPreviousRules | 182 | Loading malformed rules file preserves previously loaded ruleset | — | — | strata_check_export::checklist_engine | MISSING | Ruleset reload rollback not tested |
| 7 | offendersAndNotesReachTheResult | 193 | Offending feature labels and Korean notes propagate into `CheckResult` | feature_drawn_outside_the_survey_area_is_flagged_with_a_target | crates/strata-app/src/checklist_state_tests.rs:150 | strata_app::checklist_state | PARTIAL | Tests offender target on outside polygon; misses arbitrary `notes` passthrough check |

## test_checklist_state.cpp

| # | C++ test function | C++ line | what it checks | Rust test | Rust file:line | Rust code under test | status | note |
|---|---|---|---|---|---|---|---|---|
| 1 | everyControlPointNeedsMeta | 76 | Missing any of datum/ellipsoid/projection/origin flags corresponding GCP meta error | bad_matches_cpp | crates/strata-app/src/checklist_state_tests.rs:79 | strata_app::checklist_state | PARTIAL | Tests all 4 GCP meta failures together on `bad.gpkg`; individual single-field drops untested |
| 2 | symbolsAreNotSurveyAreas | 111 | 5 `_data()` rows (rectangle, irregular, triangle, point, circle): markers fail `SURVEY_NO_ABSTRACT_MARKER` | unsaved_survey_area_counts_and_a_triangle_is_a_symbol | crates/strata-app/src/checklist_state_tests.rs:119 | strata_app::checklist_state_geometry::is_abstract_marker | PARTIAL | Tests triangle vs quad; point and 8-vertex buffered circle rows omitted |
| 3 | featuresMustLieInsideTheSurveyPolygon | 117 | Polygons and points outside `survey_area` fail `EXTENT_MATCH` and list offender labels | feature_drawn_outside_the_survey_area_is_flagged_with_a_target | crates/strata-app/src/checklist_state_tests.rs:150 | strata_app::checklist_state_geometry::scan_geometry | PARTIAL | Tests protruding polygon offender; `artifact_point` outside offender omitted |
| 4 | featureLayersHoldRealShapesAndCrs | 138 | Checks wrong geometry type or non-metric CRS on domain feature layers | — | — | strata_app::checklist_state_geometry::scan_geometry | MISSING | Geometry-type mismatch and CRS check on feature layers untested |
| 5 | emptyRequiredFieldsAreReported | 162 | Reports missing required attributes (`feature_no`, `kind`, `period`, `artifact_no`, `section_no`) | — | — | strata_app::checklist_state | MISSING | Required attribute reporting per layer untested |
| 6 | staleCountStillCountsTheSurveyArea | 177 | Stale SQLite provider feature count still detects newly added `survey_area` | — | — | NOT BUILT | BLOCKED | QGIS `forceSinglePart` / stale provider cache condition not applicable/not built |
| 7 | staleCountStillChecksTheAttributes | 224 | 3 `_data()` rows (`feature_poly`, `control_points`, `section_line`): stale provider count still checks attributes | — | — | strata_app::checklist_state | MISSING | Attribute check across unsaved `feature_poly`/`control_points`/`section_line` untested |
| 8 | sheetOfAnotherPlaceDoesNotPass | 249 | Drawing sheet whose map extent covers a different place does not pass checklist | — | — | NOT BUILT | BLOCKED | Print layout sheet geographic extent check not built |

## test_contour_safety.cpp

| # | C++ test function | C++ line | what it checks | Rust test | Rust file:line | Rust code under test | status | note |
|---|---|---|---|---|---|---|---|---|
| 1 | fitCellSizeKeepsTheGridUnderTheLimit | 72 | Computes TIN/grid cell size keeping raster dimensions under pixel limit | test_suspect_point_filtering_and_survey_contour_tin_builder | crates/strata-terrain3d/src/trench_profile3d_tests.rs:49 | strata_terrain3d::survey_contour_tin | PARTIAL | Exercises TIN contour builder on small test cloud; explicit grid pixel-limit clamp formula untested |
| 2 | wideDensePointsWidenTheCellInsteadOfFailing | 81 | Widens cell size for wide dense point bounds rather than aborting | — | — | NOT BUILT | BLOCKED | Adaptive raster contour grid sizer not built |
| 3 | longTrianglesAcrossAGapAreEmptied | 98 | Drops TIN triangles exceeding maximum edge length across gaps | — | — | NOT BUILT | BLOCKED | Long-edge TIN triangle culling not built |
| 4 | breaklineNodesShapeTheSurfaceOnly | 120 | Breakline vertices constrain TIN surface elevation without turning into stray contour artifacts | test_suspect_point_filtering_and_survey_contour_tin_builder | crates/strata-terrain3d/src/trench_profile3d_tests.rs:49 | strata_terrain3d::survey_contour_tin | PARTIAL | Exercises basic TIN surface; constrained breakline surface shaping untested |
| 5 | clipReportsWhetherItReallyApplied | 161 | Survey-area contour clip reports boolean whether clipping actually ran | — | — | NOT BUILT | BLOCKED | Contour polygon clip status reporter not built |
| 6 | installKeepsThePreviousResultWhenASwapFails | 187 | Contour layer swap retains prior result if new GPKG/layer replacement fails | — | — | NOT BUILT | BLOCKED | Atomic contour layer swap rollback not built |
| 7 | aPreviousResultCanBeShownAgain | 219 | Previously generated contour layer can be reloaded onto canvas | — | — | NOT BUILT | BLOCKED | Contour layer reload/install service not built |

## test_control_point_csv.cpp

| # | C++ test function | C++ line | what it checks | Rust test | Rust file:line | Rust code under test | status | note |
|---|---|---|---|---|---|---|---|---|
| 1 | koreanHeaders_mapToColumns | 49 | Resolves Korean header aliases (`점명`, `기준점번호`, `동경`, `북거`, `표고`) to columns | — | — | strata_storage::cad_csv_io | MISSING | Alias table exists; header-to-column permutation test missing |
| 2 | namedPointColumn_winsOverRowNumber | 69 | Explicit point-name column overrides sequential row number column | — | — | strata_storage::cad_csv_io | MISSING | Column-precedence rule for point names untested |
| 3 | quotedCellsAndDelimiters | 76 | Parses RFC-style quoted CSV cells, commas inside quotes, and tab/semicolon delimiters | — | — | strata_storage::cad_csv_io | MISSING | Quoted fields and alternate delimiters untested |
| 4 | headerless_numericFourthColumnIsZ | 89 | Headerless 4-column CSV treats numeric 4th column as Z elevation | test_csv_benchmark_importer_bom_and_skip_malformed_row_extracts_2_valid_points_and_1_warning | crates/strata-storage/src/cad_csv_io_tests.rs:26 | strata_storage::cad_csv_io | PARTIAL | Tests BOM + malformed row warning; explicit headerless 4-column numeric-Z branch untested |
| 5 | shortRowsAndMissingXY | 100 | Skips short rows and missing X/Y coordinates while counting warnings | test_csv_benchmark_importer_bom_and_skip_malformed_row_extracts_2_valid_points_and_1_warning | crates/strata-storage/src/cad_csv_io_tests.rs:26 | strata_storage::cad_csv_io | PARTIAL | Tests 1 malformed row warning; short (<2 col) and blank X/Y row counts not systematically asserted |
| 6 | import_readsZAndSkipsReimportedPoints | 111 | Imports control points with Z into layer and deduplicates already-imported points | — | — | NOT BUILT | BLOCKED | Control-point GPKG deduplicating importer not built |
| 7 | swapSuggestion_findsSurveyAreaTheLayerCountMissed | 158 | Suggests X/Y axis swap when swapped points fall inside `survey_area` | test_cp949_detection_and_lossless_conversion_and_korean_alias_resolution | crates/strata-storage/src/cad_csv_io_tests.rs:79 | strata_storage::cad_csv_io | PARTIAL | Tests CP949 & Korean alias resolution; spatial axis-swap suggestion against `survey_area` not built |

## test_dem_trench.cpp

| # | C++ test function | C++ line | what it checks | Rust test | Rust file:line | Rust code under test | status | note |
|---|---|---|---|---|---|---|---|---|
| 1 | trench1x1Is40m2 | 143 | Single 1x1 trench defaults to 2 m x 20 m = 40 m^2 rectangle | trench_1x1_is_40m2 | crates/strata-geo/src/trench_plan_a_tests.rs:24 | strata_geo::trench_plan | PORTED | Exact 40 m^2 1x1 spec check |
| 2 | trenchTwoColsSpacedByBalk | 166 | 2-column trench grid spaces columns by exact balk width | trench_two_cols_spaced_by_balk | crates/strata-geo/src/trench_plan_a_tests.rs:47 | strata_geo::trench_plan | PORTED | Column balk spacing verified |
| 3 | buildInAreaKeepsCellsInsideAndRatio | 184 | `buildInArea` clips trenches inside survey polygon and computes area ratio | build_in_area_keeps_cells_inside_and_ratio | crates/strata-geo/src/trench_plan_a_tests.rs:60 | strata_geo::trench_plan | PORTED | Inside containment and coverage ratio verified |
| 4 | buildInArea_shortAreaTrimsInside | 214 | Short survey area trims edge trenches instead of overflowing | build_in_area_short_area_trims_inside | crates/strata-geo/src/trench_plan_a_tests.rs:92 | strata_geo::trench_plan | PORTED | Trimmed containment verified |
| 5 | pickAutoFillArea_usesNewestNotAllUnion | 232 | Auto-fill picks newest survey polygon instead of unioning all polygons | pick_auto_fill_area_uses_newest_not_all_union | crates/strata-geo/src/trench_plan_a_tests.rs:111 | strata_geo::trench_plan | PORTED | Also `pick_auto_fill_area_uses_newest_not_all_union` (`crates/strata-app/src/survey_measure_tests.rs:86`) |
| 6 | pickAutoFillArea_usesSelectedOnly | 256 | Auto-fill uses selected survey feature only when one is selected | pick_auto_fill_area_uses_selected_only | crates/strata-geo/src/trench_plan_a_tests.rs:136 | strata_geo::trench_plan | PORTED | Also `pick_auto_fill_area_uses_selected_only` (`crates/strata-app/src/survey_measure_tests.rs:94`) |
| 7 | startTrenchGrid_doesNotUnionAllSurveyAreas | 278 | `startTrenchGrid` picks selected/newest survey area rather than all-area union | trench_survey_area_uses_the_selected_or_the_largest_fid | crates/strata-app/src/survey_measure_tests.rs:59 | strata_app::app_state | PARTIAL | Verifies app `trench_survey_area_m2` selection; interactive `startTrenchGrid` placement not wired |
| 8 | buildForTargetRatio_hitsTenPercent | 295 | Auto ratio planner achieves 10% target trench coverage | build_for_target_ratio_hits_ten_percent | crates/strata-geo/src/trench_plan_b_tests.rs:37 | strata_geo::trench_plan | PORTED | Also `demo_survey_ten_percent_matches_observed_cpp_summary` at `:270` |
| 9 | buildForTargetRatio_hitsTwoPercent | 311 | Auto ratio planner achieves 2% target trench coverage | build_for_target_ratio_hits_two_percent | crates/strata-geo/src/trench_plan_b_tests.rs:55 | strata_geo::trench_plan | PORTED | 2% sample survey ratio verified |
| 10 | upslopeAspect_pointsUphillNotAlongContour | 324 | Slope aspect vector points uphill rather than along contour | upslope_aspect_points_uphill_not_along_contour | crates/strata-geo/src/trench_plan_a_tests.rs:163 | strata_geo::trench_plan | PORTED | Uphill azimuth verified |
| 11 | upslopeAspect_flatGroundHasNoDirection | 354 | Flat terrain yields `None` / no upslope azimuth | upslope_aspect_flat_ground_has_no_direction | crates/strata-geo/src/trench_plan_a_tests.rs:192 | strata_geo::trench_plan | PORTED | Flat ground returns `None` |
| 12 | buildForTargetRatio_honoursTerrainAzimuth | 366 | Target-ratio fill rotates grid along upslope terrain azimuth | build_for_target_ratio_honours_terrain_azimuth | crates/strata-geo/src/trench_plan_b_tests.rs:70 | strata_geo::trench_plan | PORTED | Azimuth orientation verified |
| 13 | buildInArea_trimsEdgeTrenchesInsteadOfDropping | 386 | Partial boundary overlap trims trenches rather than discarding them | build_in_area_trims_edge_trenches_instead_of_dropping | crates/strata-geo/src/trench_plan_a_tests.rs:218 | strata_geo::trench_plan | PORTED | Boundary edge trim verified |
| 14 | tilePack_xmlUsesXyzTopOrigin | 426 | Offline tile-pack XML specifies XYZ top-left tile origin | — | — | NOT BUILT | BLOCKED | Offline `TilePackService` MBTiles/XML packer not built |
| 15 | tilePack_tileCountMatchesWebMercator | 442 | Tile-pack counter matches WebMercator XYZ z-range total | — | — | NOT BUILT | BLOCKED | Offline `TilePackService` not built |
| 16 | tilePack_earlyCancellationPreservesExistingFile | 468 | Cancelling tile-pack download leaves pre-existing output file intact | — | — | NOT BUILT | BLOCKED | Offline `TilePackService` not built |
| 17 | ratioFill_respectsGeometryContract | 502 | 7 `_data()` geometries: all cells lie inside area, non-overlapping, valid polygons | ratio_fill_respects_geometry_contract | crates/strata-geo/src/trench_plan_b_tests.rs:89 | strata_geo::trench_plan | PORTED | Covers all 7 `_data()` geometries |
| 18 | ratioFill_rejectsNarrowArea | 540 | Narrow strip unable to fit minimum valid cell returns empty plan + reason | ratio_fill_rejects_narrow_area | crates/strata-geo/src/trench_plan_b_tests.rs:153 | strata_geo::trench_plan | PORTED | Empty cells with Korean reason |
| 19 | trenchRejectsInvalidSpec | 564 | 9 `_data()` cases: rejects >2m width, >20m length, negative, NaN/Inf dimensions | trench_rejects_invalid_spec | crates/strata-geo/src/trench_plan_a_tests.rs:253 | strata_geo::trench_plan | PORTED | Covers all 9 invalid spec rows |
| 20 | writeGpkg_rejectsInvalidCellsWithoutChangingExisting | 576 | Refuses writing invalid trench cells without altering existing GPKG | cells_valid_matches_writer_rules | crates/strata-geo/src/trench_plan_b_tests.rs:252 | strata_geo::trench_plan | PARTIAL | Tests `cells_valid` validation predicate; GPKG writer atomic preservation not built |
| 21 | writeGpkg_rollsBackFailedReplacement | 596 | Rolls back GPKG replacement on commit failure | — | — | NOT BUILT | BLOCKED | Trench GPKG writer rollback transaction not built |
| 22 | demDownload_rejectsInvalidRangeWithoutChangingFiles | 631 | 4 `_data()` extents (empty, >180 lon, >90 lat, >24 tiles): rejects without touching disk | test_dem_prepare_rejects_bad_extent_and_missing_folder_without_network | crates/strata-net/src/dem_copernicus_tests.rs:168 | strata_net::dem_copernicus | PARTIAL | Also `test_dem_extent_validation_matches_cpp_limits` at `:133` and `test_dem_maximum_tiles_is_24_and_over_limit_reports_cpp_message` at `:97`; skips checking existing disk files stay untouched |
| 23 | demDownload_earlyCancellationPreservesExistingFile | 650 | Cancelled DEM download preserves existing target file bytes | test_dem_prepare_returns_cancelled_when_flag_is_already_set | crates/strata-net/src/dem_copernicus_tests.rs:195 | strata_net::dem_copernicus | PARTIAL | Tests cancellation flag return; does not verify pre-existing file preservation on disk |
| 24 | demDownload_livePortable | 670 | Live Copernicus DEM fetch (QSKIP offline) writes Float32 GeoTIFF | — | — | NOT BUILT | BLOCKED | Live Copernicus GeoTIFF writer & raster merge not built (`network_tests.rs:123` checks curl CLI offline only) |
| 25 | layerTreeMenu_hasLabelToggleAndTrenchRatio | 700 | Layer context menu exposes label toggle and trench ratio items | each_survey_branch_shows_the_cpp_rows | crates/strata-app/src/layer_menu_model_tests.rs:64 | strata_app::layer_menu_model | PARTIAL | Shows label row and deferred `~"시굴격자"` tooltip (`지시 W-T1`); live trench-ratio action not built |
| 26 | applySnapConfig_vertexAndSegmentNotWmsPromise | 719 | Snap config enables vertex/segment snapping and skips WMS/reference layers | — | — | NOT BUILT | BLOCKED | QGIS `QgsSnappingConfig` WMS-promise exclusion not built |
| 27 | startTrenchGrid_placesOnMapWithoutApplyClick | 741 | Map-click trench placement places grid immediately without Apply button | — | — | NOT BUILT | BLOCKED | Interactive map-click trench placement tool not built |
| 28 | trenchWholeMove_isUndoableOnMouseRelease | 759 | Whole-grid drag produces single undo step on mouse release | — | — | NOT BUILT | BLOCKED | `KaTrenchMoveTool` interactive drag + undo not built |
| 29 | clearLayerReplacesPreviousGrid | 775 | Re-running grid clears previous features rather than appending duplicates | — | — | NOT BUILT | BLOCKED | `TrenchGridGenerator::writeGpkg` layer replacement not built |
| 30 | hillshadeWritesByteTif | 812 | Computes 8-bit Horn hillshade raster and writes byte GeoTIFF | test_terrain_rgb_decoding_and_flat_terrain_hillshade | crates/strata-terrain3d/src/trench_batch62_tests.rs:37 | strata_terrain3d::trench_hillshade | PARTIAL | Tests in-memory `compute_horn_hillshade_byte_grid`; GeoTIFF file writer (`DemAnalyzer::runHillshadeFile`) not built |
| 31 | niceMeterStepAround120px | 835 | Selects clean 1-2-5-10 decade meter grid step around 120 px target | test_d14_region_locator_and_canvas_grid_band | crates/strata-app/src/gui_shell_tests_b.rs:55 | strata_app::gui_tab_map_geom::nice_grid_step_meters | PORTED | Tests 120/240/600/0 inputs (`0 -> 10`; skips `8 -> 10`) |

## test_editing_tools.cpp

| # | C++ test function | C++ line | what it checks | Rust test | Rust file:line | Rust code under test | status | note |
|---|---|---|---|---|---|---|---|---|
| 1 | captureStartsNewShapeOnSavedCornerWithoutWriting | 93 | Clicking saved vertex starts a new sketch without modifying existing saved feature | clicking_saved_line_vertex_starts_new_sketch_without_touching_existing_line | crates/strata-app/src/app_state_draw_commit_line_tests.rs:197 | strata_app::app_state_draw_commit | PARTIAL | Verifies saved-line vertex capture; saved-polygon corner case not separately tested |
| 2 | captureReportsBowTieAfterFinishingAndCancelsCleanly | 126 | Bow-tie polygon sketch warns after finish and cleans up cleanly on cancel | bow_tie_keeps_the_largest_part_and_tells_the_dropped_one | crates/strata-app/src/edit_geometry_repair_tests.rs:50 | strata_app::edit_geometry_repair | PARTIAL | Tests bow-tie repair split + warning; `KaCaptureMapTool` interactive abort/cancel path untested |
| 3 | easyDrawTakesCornersAtOnceAndEdgesAfterRest | 159 | Easy-draw snaps corners on click and inserts edge vertices after mouse rest | — | — | NOT BUILT | BLOCKED | `easy_draw` mouse-rest timer auto-vertex placement not built |
| 4 | selectPrefersSmallShapeAndCyclesOverlaps | 199 | Select tool prefers smallest enclosed feature first and cycles overlapping picks | select_on_demo_cycles_overlaps_like_cpp | crates/strata-app/src/feature_pick_tests.rs:69 | strata_app::feature_pick | PORTED | Also `small_unsaved_shape_on_top_is_picked` at `:21` |
| 5 | clickedHandleIsPickedAndDeleteRemovesOnlyThatVertex | 228 | Clicks vertex handle and Delete key deletes only that vertex (>=3 ring guarded) | test_delete_vertex_triangle_blocked_and_ortho_45_snap | crates/strata-geo/src/polygon_ops_tests.rs:106 | strata_geo::polygon_ops | PARTIAL | Tests `delete_vertex` 3-vertex floor guard; interactive canvas handle pick & Delete key edit not built |
| 6 | attributeEditingSkipsReferenceAndCadastral | 269 | Reference and cadastral layers cannot enter attribute edit mode | reference_branches | crates/strata-app/src/layer_menu_model_tests.rs:154 | strata_app::layer_menu_model | PARTIAL | Tests reference-branch context menu restrictions; map-click edit guard on reference/cadastral untested |

## test_export_survey_areas.cpp

| # | C++ test function | C++ line | what it checks | Rust test | Rust file:line | Rust code under test | status | note |
|---|---|---|---|---|---|---|---|---|
| 1 | everySurveyAreaLayerReachesTheSubmissionShp | 130 | Multiple `survey_area` layers merge into one submission SHP preserving every feature | merge_keeps_fields_of_every_layer | crates/strata-engine/src/submit_shp_types_tests.rs:113 | strata_engine::submit_shp | PORTED | Also `readme_records_the_survey_and_merged_layers` (`crates/strata-check-export/src/submit_builder_tests.rs:239`) |
| 2 | emptyFirstLayerDoesNotDropTheSubmissionShp | 158 | Empty first `survey_area` layer does not drop populated second `survey_area` layer | key_without_features_writes_nothing | crates/strata-engine/src/submit_shp_tests.rs:146 | strata_engine::submit_shp | PARTIAL | Tests empty layer alongside valid key; paired empty+nonempty `survey_area` key case untested |
| 3 | referenceLayerNamedLikeDomainStaysOut | 183 | Reference layer named `"조사구역"` without survey domain key is excluded from SHP | — | — | strata_check_export::submit_builder | MISSING | Domain-key filtering of identically named reference layer untested |
| 4 | legacyLayerWithoutKeyStillExports | 219 | Legacy survey layer missing `layer_key` custom property falls back by name and exports | — | — | strata_check_export::submit_builder | MISSING | Legacy fallback without `layer_key` untested |
| 5 | invalidGeometryInSecondLayerBlocksSubmission | 245 | Self-intersecting polygon in second `survey_area` layer blocks submission export | test_submit_package_exporter_blocks_on_missing_feat_no_and_succeeds_after_fix | crates/strata-check-export/src/submit_package_tests.rs:30 | strata_check_export::submit_package | PARTIAL | Verifies checklist error blocks export; specific invalid-geometry second-layer trigger untested |
| 6 | emptyGeometryBlocksSubmission | 283 | Feature with empty geometry blocks submission export | nothing_to_submit_makes_no_folder | crates/strata-check-export/src/submit_builder_tests.rs:201 | strata_check_export::submit_builder | PARTIAL | Tests zero total features; empty WKB geometry on a record blocking via checklist untested |
| 7 | zeroAreaPolygonBlocksSubmission | 318 | Zero-area collapsed polygon blocks submission export | — | — | strata_check_export::checklist_engine | MISSING | `GEOMETRY_NONZERO_AREA` blocking submission export untested |
| 8 | longFieldNamesFitTheShapefile | 351 | Multi-byte Korean field names truncate cleanly within 10-byte DBF field limit | korean_field_names_cut_on_code_points | crates/strata-engine/src/submit_shp_tests.rs:195 | strata_engine::submit_shp | PORTED | Also `schema_fields_are_shapefile_safe` at `:208` and `long_euc_kr_text_is_cut_at_the_byte_width` (`crates/strata-engine/src/submit_shp_types_tests.rs:59`) |
| 9 | exportShp_matchesProjDirectWithinOneMillimetre | 454 | 2 `_data()` CRS rows (5186, 5187): exported EPSG:5179 SHP matches PROJ within 1 mm | demo_shapefiles_equal_the_cpp_package | crates/strata-engine/src/submit_shp_tests.rs:136 | strata_engine::submit_shp | PORTED | Byte-matches C++ 5187 & 5186 (`bad_survey_euc_kr_5186_equals_the_cpp_package` `submit_shp_types_tests.rs:68`) fixtures |
| 10 | exportSubmissionPackage_withoutUserSheetWritesShpOnly | 507 | Package export without user-composed sheet writes SHP bundle without PDF | demo_package_equals_the_cpp_package | crates/strata-check-export/src/submit_builder_tests.rs:74 | strata_check_export::submit_builder | PARTIAL | Verifies SHP+README+manifest output; auto-generated-template suppression logic not built |

## test_feature_card.cpp

| # | C++ test function | C++ line | what it checks | Rust test | Rust file:line | Rust code under test | status | note |
|---|---|---|---|---|---|---|---|---|
| 1 | form_suggestsNextNumberPerKindAndOnlyWarns | 86 | Record form suggests next sequential number per kind and warns (never blocks) on duplicate | form_suggests_next_number_per_kind_and_only_warns | crates/strata-app/src/feature_form_state_tests.rs:24 | strata_app::feature_form_state | PORTED | Exact match |
| 2 | form_applyToWritesEditBufferOnly | 121 | Accepting form updates in-memory edit buffer without saving GPKG | apply_writes_the_edit_buffer_only | crates/strata-app/src/app_state_feature_form_tests.rs:72 | strata_app::app_state_feature_form | PORTED | Also `skip_keeps_the_shape_with_empty_values` at `:102` |
| 3 | card_showsOneSelectedFeature | 147 | Attribute card binds to single selected feature and hides on multi/none | card_shows_one_feature | crates/strata-app/src/feature_card_tests.rs:47 | strata_app::feature_card | PORTED | Single feature card model check |
| 4 | card_editsStayInEditBuffer | 166 | Card field edits stay buffered in edit buffer and support undo without saving GPKG | card_edits_stay_in_the_edit_buffer | crates/strata-app/src/feature_card_tests.rs:58 | strata_app::feature_card | PORTED | Also `a_card_edit_is_one_undo_step_without_status_text` (`crates/strata-app/src/app_state_feature_card_tests.rs:81`) |
| 5 | card_pointsOutRepeatedNumber | 196 | Card warns on duplicate number within the same feature kind | card_points_out_a_repeated_number | crates/strata-app/src/feature_card_tests.rs:77 | strata_app::feature_card | PORTED | Duplicate-within-kind warning check |
| 6 | card_headerLineIsKindAndNumber | 212 | Card header combines feature number and kind word | card_header_is_number_and_kind | crates/strata-app/src/feature_card_tests.rs:88 | strata_app::feature_card | PORTED | Header string check |
| 7 | card_measuresAreReadOnlyBoxes | 223 | Area/perimeter/length measures show in read-only card boxes | card_measures_area_and_perimeter | crates/strata-app/src/feature_card_tests.rs:99 | strata_app::feature_card | PORTED | Data values checked; rendered `egui::Label` is read-only |
| 8 | card_stateIsChipWithTone | 238 | Card status chip shows edited vs saved wording and tone | card_state_words | crates/strata-app/src/feature_card_tests.rs:108 | strata_app::feature_card | PORTED | Status chip wording check |
| 9 | card_noteIsTwoLineAndCommitsOnFocusOut | 253 | Multiline note box preserves newlines and commits when focus leaves | memo_commits_when_the_box_loses_focus | crates/strata-app/src/ri_attr_card_tests.rs:36 | strata_app::ri_attr_card | PORTED | Focus-loss commit tested via egui harness |
| 10 | card_openFormEmitsLayerAndFid | 272 | "전체 기록 보기" opens full record dialog for the active layer and FID | record_window_sees_the_typed_memo | crates/strata-app/src/app_state_feature_card_tests.rs:58 | strata_app::app_state_feature_card | PARTIAL | Verifies record dialog targets card layer+fid with live draft; UI button signal itself implicit in egui |

## test_feature_card_fields.cpp

| # | C++ test function | C++ line | what it checks | Rust test | Rust file:line | Rust code under test | status | note |
|---|---|---|---|---|---|---|---|---|
| 1 | surveyStatusSurveyorAndDateAreOnTheCard | 52 | `survey_status`, `surveyor`, `survey_date` combos appear on card and edit layer attributes | card_words_per_layer | crates/strata-app/src/feature_card_tests.rs:118 | strata_app::feature_card | PARTIAL | Verifies per-layer field presence in model; interactive combo edits in `ri_attr_card` UI untested |
| 2 | enterFinishesOnlyThatBox | 63 | Pressing Return inside a card single-line field finishes that box only without activating dialog buttons | enter_elsewhere_saves | crates/strata-app/src/dlg_feature_record_tests.rs:70 | strata_app::dlg_feature_record | PARTIAL | Tests Return inside modal record dialog; inline card Return focus-clear without dialog submit untested |
| 3 | measureFigureStandsAlone | 86 | Measure figure and unit/label render in separate standalone labels | card_measures_area_and_perimeter | crates/strata-app/src/feature_card_tests.rs:99 | strata_app::feature_card | PARTIAL | Checks value/label model; rendered UI label split in `ri_attr_card` untested |
| 4 | typedTextStaysWithItsRecord | 105 | Card keeps draft text bound to original feature when selection switches | typed_memo_stays_on_its_own_shape | crates/strata-app/src/app_state_feature_card_tests.rs:44 | strata_app::app_state_feature_card | PORTED | Selection switch isolates typed memo |

## test_feature_card_plate.cpp

| # | C++ test function | C++ line | what it checks | Rust test | Rust file:line | Rust code under test | status | note |
|---|---|---|---|---|---|---|---|---|
| 1 | cardIsALedgerOnThePageInPaper | 56 | Paper-look card uses warm cream ledger palette/border colors | — | — | strata_app::ri_attr_card | MISSING | Rendered pixel/color style assertions for attribute card missing |
| 2 | parcelReadsAsALockedRecordInPaper | 97 | Cadastral parcel opens as read-only locked record in paper card | — | — | NOT BUILT | BLOCKED | Locked read-only cadastral parcel card view not built |
| 3 | cardFollowsALiveSwitch | 131 | Live tool/selection switch updates rendered card fields and colors cleanly | leaving_the_draw_tool_reads_the_selection_again | crates/strata-app/src/ri_attr_card_tests.rs:60 | strata_app::ri_attr_card | PARTIAL | Verifies tool-switch rebinding in UI harness; misses palette contrast checks |
| 4 | periodBandMarksTheRecordedPeriodInPaper | 153 | Period badge renders distinct non-background color band in paper look | — | — | strata_app::ri_attr_card | MISSING | Period band swatch paint color untested |
| 5 | controlPointShowsItsCoordinatesInPaper | 177 | Control-point card displays formatted Easting/Northing/Elevation coordinates | control_point_rows | crates/strata-app/src/feature_card_tests.rs:133 | strata_app::feature_card | PARTIAL | Tests formatted X/Y/Z rows in card model; UI paper-palette render untested |
| 6 | nameNumberWindowReadsLikeTheBoardInPaper | 234 | Name/number prompt dialog applies paper palette and compact geometry | form_opens_after_drawing_and_saves_into_the_shape | crates/strata-app/src/app_state_feature_form_tests.rs:35 | strata_app::app_state_feature_form | PARTIAL | Tests modal form behavior; paper palette and pixel geometry checks missing |
| 7 | noteIsSeveralLinesInPaper | 255 | Multiline note box allocates >= 56 px multi-line editor height | enter_in_the_note_adds_a_line | crates/strata-app/src/dlg_feature_record_tests.rs:58 | strata_app::dlg_feature_record | PARTIAL | Tests multiline Enter behavior; >= 56 px widget height untested |

## test_feature_records.cpp

| # | C++ test function | C++ line | what it checks | Rust test | Rust file:line | Rust code under test | status | note |
|---|---|---|---|---|---|---|---|---|
| 1 | normalizedAndParse | 67 | Normalizes feature strings and parses prefix/number/suffix components | parse_number_parts | crates/strata-app/src/feature_form_tests.rs:52 | strata_app::feature_form | PORTED | Also `spelling_keys` at `:45` |
| 2 | nextFrom_keepsTheWrittenForm | 82 | `nextFrom` preserves user-written prefix/padding/suffix formatting | next_from_keeps_the_written_form | crates/strata-app/src/feature_form_tests.rs:61 | strata_app::feature_form | PORTED | Preserves e.g. `001호`, `No. 3` |
| 3 | suggestNext_runsPerKind | 92 | Suggests next feature number independently per feature kind | form_suggests_next_number_per_kind_and_only_warns | crates/strata-app/src/feature_form_state_tests.rs:24 | strata_app::feature_form_state | PORTED | Kind-scoped numbering verified |
| 4 | artifactNumbers_runOverTheLayer | 102 | Artifact numbers increment across the whole `artifact_point` layer regardless of kind | artifact_layer_offers_its_own_kinds_only | crates/strata-app/src/feature_form_tests.rs:88 | strata_app::feature_form | PARTIAL | Also exercised in `placing_artifact_points_offers_form_without_polygon_presets_updates_card_and_saves_to_gpkg` (`crates/strata-app/src/app_state_draw_commit_point_tests.rs:56`); direct `suggest_next` unit case without `kind` filter omitted |
| 5 | duplicates_warnOnlyWithinKind | 118 | Duplicate number warning fires only within matching `kind` | form_suggests_next_number_per_kind_and_only_warns | crates/strata-app/src/feature_form_state_tests.rs:24 | strata_app::feature_form_state | PORTED | Cross-kind reuse produces no duplicate warning |
| 6 | choices_presetOrderThenSurveySpelling | 136 | Dropdown choices order built-in presets first then survey custom spellings | choices_preset_order_then_survey_spelling | crates/strata-app/src/feature_form_tests.rs:74 | strata_app::feature_form | PORTED | Exact ordering check |
| 7 | canonicalValue_reusesSurveySpelling | 151 | Canonicalizes typed choice to existing survey spelling when equivalent | form_values_use_the_survey_spelling | crates/strata-app/src/feature_form_state_tests.rs:52 | strata_app::feature_form_state | PORTED | Also `values_clear_empty_boxes_and_keep_the_survey_spelling` (`crates/strata-app/src/feature_record_tests.rs:155`) |
| 8 | stampNewAndTouch_fillAuditFields | 162 | `stampNew` and `touch` populate `uid`, `created_at`, `updated_at` on schema columns | stamps_fill_only_existing_columns | crates/strata-app/src/app_state_draw_commit_tests.rs:176 | strata_app::app_state_draw_commit | PORTED | Also `new_survey_drawing_is_saved_with_stamps` at `:218` and `a_bad_number_changes_nothing_and_a_good_save_touches_updated_at` (`app_state_feature_record_tests.rs:66`) |
| 9 | measure_areaPerimeterLength | 190 | Computes polygon area/perimeter and line length identically for record display | card_measures_area_and_perimeter | crates/strata-app/src/feature_card_tests.rs:99 | strata_app::feature_card | PORTED | Also `test_r9_format_length_m_matches_measureops` (`crates/strata-app/src/gui_tab_map_tests.rs:12`) |

## test_grid_math.cpp

| # | C++ test function | C++ line | what it checks | Rust test | Rust file:line | Rust code under test | status | note |
|---|---|---|---|---|---|---|---|---|
| 1 | rotationIsClockwiseFromEast | 25 | `CanvasGridMath::toGrid`/`toMap` treats rotation angle clockwise from East | — | — | NOT BUILT | BLOCKED | Rotated `CanvasGridMath::toGrid`/`toMap` transform not built |
| 2 | gridFrameMatchesTrenchAzimuth | 37 | Grid coordinate frame aligns with trench azimuth angle | — | — | NOT BUILT | BLOCKED | Rotated `CanvasGridMath::toGrid`/`toMap` transform not built |
| 3 | roundTripKeepsPoints | 54 | `toGrid` -> `toMap` round-trip preserves arbitrary points within floating tolerance | — | — | NOT BUILT | BLOCKED | Rotated `CanvasGridMath::toGrid`/`toMap` transform not built |
| 4 | snapHonoursOriginAndRotation | 67 | `snapToNode` snaps to rotated/shifted grid intersections | lshape_trench_all_four_bends_keep_exact_15m_arms_and_snap_to_coordinate_grid | crates/strata-geo/src/trench_grid_tests.rs:32 | strata_geo::trench_lshape | PARTIAL | Only axis-aligned 5 m snap checked; general rotated+shifted `snapToNode` not built |
| 5 | denseGridsAreThinnedNotCut | 84 | `lineStride` thins dense grids (>80 lines) smoothly rather than truncating viewport | test_d14_region_locator_and_canvas_grid_band | crates/strata-app/src/gui_shell_tests_b.rs:55 | strata_app::gui_tab_map_geom::grid_line_stride | PORTED | Checks `grid_line_stride` at `:142-143` |
| 6 | labelsKeepSubMetreSteps | 93 | `labelDecimals` keeps sub-metre precision for `0.5`/`0.25` steps and non-integer origins | test_d14_region_locator_and_canvas_grid_band | crates/strata-app/src/gui_shell_tests_b.rs:55 | strata_app::gui_tab_map_geom::grid_label_decimals | PARTIAL | `grid_label_decimals` exists and runs under `grid_label_text`, but sub-metre `0.5`/`0.25` inputs are not directly asserted |
| 7 | niceStepsAreUnchanged | 102 | `niceStepMeters` acts as identity on exact `1-2-5` steps (`0.5`..`1000`) | test_d14_region_locator_and_canvas_grid_band | crates/strata-app/src/gui_shell_tests_b.rs:55 | strata_app::gui_tab_map_geom::nice_grid_step_meters | PARTIAL | Tests 120/240/600/0 -> 100/200/500/10 at `:129-132`; misses full `0.5`..`1000` fixed-point sweep |

## test_inner_piece.cpp

| # | C++ test function | C++ line | what it checks | Rust test | Rust file:line | Rust code under test | status | note |
|---|---|---|---|---|---|---|---|---|
| 1 | unsavedShapeIsPickedAndEdited | 108 | Select picks unsaved polygon in edit buffer and Delete deletes it | select_picks_an_unsaved_drawing | crates/strata-app/src/feature_pick_tests.rs:92 | strata_app::feature_pick | PARTIAL | Verifies picking unsaved negative FID; Delete-key removal of picked unsaved shape untested |
| 2 | shapeLiftedAboveLabelsIsStillPicked | 131 | Shape temporarily lifted above labels remains pickable on click | — | — | NOT BUILT | BLOCKED | `LayerAboveLabels` overlay pick stack not built |
| 3 | clickInsideHolePicksItAndDeleteFillsIt | 159 | Clicking inside a polygon hole selects hole; Delete fills the hole | — | — | NOT BUILT | BLOCKED | Interactive hole-piece pick & Delete-to-fill not built |
| 4 | clickOnOuterAreaKeepsWholeShapeBehaviour | 182 | Clicking outside interior holes selects whole polygon normally | — | — | NOT BUILT | BLOCKED | Inner-piece vs whole-shape hit disambiguation not built |
| 5 | innerPartCyclesToWholeShapeAndCutsOut | 194 | Inner enclosed polygon cycles on click and Delete cuts hole in outer polygon | — | — | NOT BUILT | BLOCKED | Interactive inner-part cycle & cut-out not built |
| 6 | anySurveyLayerGetsTheMenuAndARightClickStartsNoCycle | 221 | Any survey layer gets context menu; right-click never triggers overlap pick cycle | each_survey_branch_shows_the_cpp_rows | crates/strata-app/src/layer_menu_model_tests.rs:64 | strata_app::layer_menu_model | PARTIAL | Tests layer-panel survey menu rows; map right-click non-cycling interaction not built |
| 7 | keyASwitchesToSelectWithoutTogglingOrDroppingASketch | 240 | Key `A` switches active tool to Select without toggling off or dropping sketch | — | — | strata_app::gui_tab_map | MISSING | Map `Key::A` select shortcut preservation untested |

## test_layer_edits.cpp

| # | C++ test function | C++ line | what it checks | Rust test | Rust file:line | Rust code under test | status | note |
|---|---|---|---|---|---|---|---|---|
| 1 | mergeNeverTakesTheWholeLayer | 42 | `LayerMerge` refuses merging when < 2 features are selected | — | — | NOT BUILT | BLOCKED | `LayerMerge` tool not built |
| 2 | mergeWarnsAboutLostAttributesAndCanBeUndone | 54 | Merging selected polygons warns about dropped differing attributes and undoes cleanly | — | — | NOT BUILT | BLOCKED | `LayerMerge` tool not built |
| 3 | mergeRefusesSeparatePolygonsInASinglePartLayer | 97 | Refuses merging disjoint polygons in single-part layer | — | — | NOT BUILT | BLOCKED | `LayerMerge` tool not built |
| 4 | sharedVertexMoveTouchesOnlyNeighbours | 108 | Topology vertex move edits only shared vertices of adjacent polygons | — | — | NOT BUILT | BLOCKED | `SharedVertexMove` topology tool not built |
| 5 | labelStackQueryDoesNotWrite | 131 | Querying stacked labels leaves GPKG file mtime/contents untouched | — | — | NOT BUILT | BLOCKED | Overlay label-stack query not built |

## test_measure.cpp

| # | C++ test function | C++ line | what it checks | Rust test | Rust file:line | Rust code under test | status | note |
|---|---|---|---|---|---|---|---|---|
| 1 | length10mOn5187 | 32 | 10 m segment on EPSG:5187 measures exact `10.0` m | test_r9_format_length_m_matches_measureops | crates/strata-app/src/gui_tab_map_tests.rs:12 | strata_app::gui_tab_map | PARTIAL | Also `test_measure_tool_azimuth_0_90_270_and_unit_formatting_1500m_25000m2` (`cad_interaction_tests.rs:142`); misses explicit `(200000,450000)->(200010,450000)` EPSG:5187 planar check |
| 2 | rectangleArea200m2 | 41 | 20 m x 10 m rectangle on EPSG:5187 measures `200.0` m^2 | test_r9_format_area_m2_matches_measureops | crates/strata-app/src/gui_tab_map_tests.rs:27 | strata_app::gui_tab_map | PARTIAL | Also `cad_interaction_tests.rs:142`; misses explicit 20x10 EPSG:5187 `200.0` m^2 case |
| 3 | formatters | 53 | Distance (`m`/`km`) and area (`m^2`/`평`/`ha`) formatter strings match C++ | test_r9_format_length_m_matches_measureops | crates/strata-app/src/gui_tab_map_tests.rs:12 | strata_app::gui_tab_map | PORTED | Covered across `gui_tab_map_tests.rs:12,27` and `cad_interaction_tests.rs:142` |
| 4 | planarAreaMatchesLabelSelectAndTape | 60 | Drawing tape, feature card, and measure tool agree on planar area on EPSG:5187 | test_d17_draw_guide_measure_and_reference_status | crates/strata-app/src/gui_shell_tests_b.rs:17 | strata_app::gui_shell | PARTIAL | Each component tested separately (`cad_interaction_tests.rs:8`, `feature_card_tests.rs:99`); cross-surface consistency not asserted on single polygon |
| 5 | gridBearingIsClockwiseFromGridNorth | 113 | Grid azimuth runs clockwise from North (N=0, E=90, S=180, W=270) | test_r9_grid_bearing_degrees_matches_measureops | crates/strata-app/src/gui_tab_map_tests.rs:43 | strata_app::gui_tab_map | PORTED | Verified N/E/S/W + DMS formatter (`cad_interaction_tests.rs:142`) |
| 6 | pinPlanarEllipsoidKeepsSavedState | 126 | QGIS ellipsoid setting pins planar/GRS80 without overwriting saved state | — | — | NOT BUILT | BLOCKED | QGIS `pinPlanarEllipsoid` API not applicable/not built |

## test_parcel_copy.cpp

| # | C++ test function | C++ line | what it checks | Rust test | Rust file:line | Rust code under test | status | note |
|---|---|---|---|---|---|---|---|---|
| 1 | pasteKeepsTheSamePlaceAndAttributes | 23 | Copies cadastral parcel into copy layer preserving coordinates and jibun attributes | — | — | NOT BUILT | BLOCKED | `ParcelClipboard` copy/paste tool not built |
| 2 | secondPasteAddsToTheSameCopyLayer | 44 | Second paste appends to existing copy layer rather than creating duplicate layer | — | — | NOT BUILT | BLOCKED | `ParcelClipboard` copy/paste tool not built |
| 3 | multipartParcelBecomesOneFeaturePerPart | 54 | Multi-part cadastral polygon splits into one copied feature per part | — | — | NOT BUILT | BLOCKED | `ParcelClipboard` copy/paste tool not built |
| 4 | copyStaysWhenItsSourceIsRemoved | 73 | Clipboard copy persists after source cadastral layer removal | — | — | NOT BUILT | BLOCKED | `ParcelClipboard` copy/paste tool not built |
| 5 | surveyShapesAreNotCopied | 85 | `ParcelClipboard` refuses copying domain survey shapes | — | — | NOT BUILT | BLOCKED | `ParcelClipboard` copy/paste tool not built |
| 6 | pasteWithNothingCopiedSaysWhy | 98 | Pasting with empty parcel clipboard emits Korean explanation | — | — | NOT BUILT | BLOCKED | `ParcelClipboard` copy/paste tool not built |
| 7 | pasteKeepsTheSamePlaceAcrossCoordinateSystems | 108 | Copy/paste reprojects parcel from source CRS (e.g. 3857/5186) to work CRS (5187) | — | — | NOT BUILT | BLOCKED | `ParcelClipboard` copy/paste tool not built |
| 8 | copySurvivesSaveAndReopen | 132 | Pasted parcel layer persists through save and GPKG reopen | — | — | NOT BUILT | BLOCKED | `ParcelClipboard` copy/paste tool not built |
| 9 | pastingASavedMapTwiceStillSaves | 173 | Second paste on already-saved map commits cleanly on save | — | — | NOT BUILT | BLOCKED | `ParcelClipboard` copy/paste tool not built |
| 10 | pasteAgainAfterReopenStillSaves | 207 | Pasting into reopened copy layer commits cleanly on save | — | — | NOT BUILT | BLOCKED | `ParcelClipboard` copy/paste tool not built |

## test_parcel_copy_keys.cpp

| # | C++ test function | C++ line | what it checks | Rust test | Rust file:line | Rust code under test | status | note |
|---|---|---|---|---|---|---|---|---|
| 1 | holdingCtrlVPastesOnce | 31 | Auto-repeat `Ctrl+V` key events paste only once | — | — | NOT BUILT | BLOCKED | `ParcelClipboard` keyboard copy/paste not built |
| 2 | ctrlCAndCtrlVInSelectToolPasteWhereTheParcelWas | 53 | `Ctrl+C` / `Ctrl+V` in Select tool paste parcel at original map coordinates | — | — | NOT BUILT | BLOCKED | `ParcelClipboard` keyboard copy/paste not built |
| 3 | ctrlCAndCtrlVDoNothingOutsideSelectTool | 79 | `Ctrl+C` / `Ctrl+V` outside Select tool ignore parcel copy | — | — | NOT BUILT | BLOCKED | `ParcelClipboard` keyboard copy/paste not built |
| 4 | snapTraceShowsNearbyVerticesOnlyWhenCloseAndFillsSkippedVertex | 122 | Hover snap trace reveals nearby parcel vertices only near cursor and fills intermediate corners | snap_trace_graph_fills_intermediate_parcel_vertices_and_snaps_rough_polygon | crates/strata-canvas2d/src/cad_interaction_tests.rs:70 | strata_canvas2d::cad_interaction | PARTIAL | Also `point_grid_nearby_vertices_deduplicates_and_sorts_by_distance` at `:95` and `just_drawn_area_snaps_to_the_line_and_undoes` (`app_state_snap_lines_tests.rs:32`); hover cursor-trace preview UI untested |
| 5 | snapTraceMouseMoveStaysFastOverDenseCadastral | 201 | Nearby-vertex spatial lookup stays bounded over dense cadastral rings | test_point_grid_budget_clamps_2000_dense_vertices_to_exact_900_sorted_by_distance | crates/strata-canvas2d/src/cad_interaction_tests.rs:109 | strata_canvas2d::cad_interaction | PORTED | Verifies 900-node clamp and distance sort on 2000-vertex input |

## test_polygon_erase.cpp

| # | C++ test function | C++ line | what it checks | Rust test | Rust file:line | Rust code under test | status | note |
|---|---|---|---|---|---|---|---|---|
| 1 | erased_cutsOnlyTheOverlappingPlace | 70 | Geometric erase subtracts only intersecting region from target polygon | test_multipolygon_decompose_3_islands_preserves_exact_count_and_total_area | crates/strata-storage/src/polygon_pieces.rs:199 | strata_storage::polygon_pieces::erase_polygon_area_difference | PARTIAL | Also `test_interior_hole_punch_10x10_minus_2x2_yields_96m2_and_sliver_cleaning_under_0_05m2` (`crates/strata-geo/src/polygon_ops_tests.rs:162`); multi-part/disjoint erase edge cases untested |
| 2 | oneSelectedShapeCutsTheShapeUnderItAndOneUndoRestoresBoth | 87 | Selected cutter subtracts shape underneath in single undoable step | — | — | NOT BUILT | BLOCKED | Interactive `PolygonErase::plan/apply` tool not built |
| 3 | selectingTheBigShapeOrALonelyShapeOnlyExplains | 125 | Selecting enclosing larger or non-overlapping shape explains why no cut ran | — | — | NOT BUILT | BLOCKED | Interactive `PolygonErase` planner not built |
| 4 | anotherLayerIsCutOnlyWhenExactlyOneShapeLiesUnder | 147 | Cross-layer erase cuts target only when ambiguity-free single target lies under cutter | — | — | NOT BUILT | BLOCKED | Interactive `PolygonErase` cross-layer tool not built |
| 5 | severalSelectedShapesCutTheLargestAcrossLayers | 170 | Multiple selected cutters slice largest underlying shape across survey layers | — | — | NOT BUILT | BLOCKED | Interactive `PolygonErase` multi-cutter tool not built |
| 6 | referenceAndCadastralShapesAreNeverCutOrUsed | 196 | Reference and cadastral layers are skipped as targets/cutters | — | — | NOT BUILT | BLOCKED | Interactive `PolygonErase` tool not built |
| 7 | shapeCutInTwoStaysValidInASinglePolygonFile | 227 | Cutting single-polygon layer feature into two splits into two valid records | — | — | NOT BUILT | BLOCKED | Interactive `PolygonErase` split-save tool not built |
| 8 | shapeCutInTwoStaysOneShapeInAMultiPolygonLayer | 282 | Cutting multi-polygon layer feature into two keeps single multi-part record | — | — | NOT BUILT | BLOCKED | Interactive `PolygonErase` multi-part preservation not built |
| 9 | pick_takesTheSmallShapeDrawnOnTopEvenWhenUnsaved | 298 | Pick order prefers smaller unsaved top polygon | small_unsaved_shape_on_top_is_picked | crates/strata-app/src/feature_pick_tests.rs:21 | strata_app::feature_pick | PORTED | Exact port of pick priority |
| 10 | pick_findsAShapeInsideAShapeOfTheLayerAbove | 317 | Pick locates smaller polygon inside enclosing polygon of a higher layer | shape_inside_a_shape_of_the_layer_above_is_picked | crates/strata-app/src/feature_pick_tests.rs:30 | strata_app::feature_pick | PORTED | Exact cross-layer enclosure port |
| 11 | pick_prefersSurveyShapesAndNearbyLines | 333 | Pick ranks survey shapes ahead of reference shapes and nearby lines appropriately | survey_shapes_and_nearby_lines_come_first | crates/strata-app/src/feature_pick_tests.rs:41 | strata_app::feature_pick | PORTED | Exact ranking port |
| 12 | movingTheFirstVertexKeepsAHoleClosed | 361 | Moving ring vertex 0 updates closing vertex N to keep interior hole closed | — | — | NOT BUILT | BLOCKED | Interactive vertex-move `ringClosingVertex` helper not built |

## test_polygon_pieces.cpp

| # | C++ test function | C++ line | what it checks | Rust test | Rust file:line | Rust code under test | status | note |
|---|---|---|---|---|---|---|---|---|
| 1 | holeIsFoundAndFilled | 43 | Identifies interior hole ring under click and fills it | — | — | NOT BUILT | BLOCKED | `PolygonPieces` ring/hole fill not built |
| 2 | innerPartIsRemovedOrCutOut | 59 | Removes inner polygon part or punches it as hole in enclosing polygon | — | — | NOT BUILT | BLOCKED | `PolygonPieces` cut-out not built |
| 3 | smallestPieceWinsWhenPartsAreStacked | 77 | Stacked interior holes/parts pick smallest enclosing piece first | — | — | NOT BUILT | BLOCKED | `PolygonPieces` piece-stacking not built |
| 4 | separatePartsAndStalePiecesAreLeftAlone | 86 | Non-overlapping/stale outer parts remain untouched when cutting inner piece | — | — | NOT BUILT | BLOCKED | `PolygonPieces` partial-cut guard not built |
| 5 | islandGoesWithItsHole | 102 | Filling a hole with an island inside also removes the enclosed island | — | — | NOT BUILT | BLOCKED | `PolygonPieces` island cleanup not built |
| 6 | pickListKeepsSurveyShapesAndFindsHolesCutBeforeSaving | 118 | Pick list detects holes freshly cut in edit buffer before GPKG save | — | — | NOT BUILT | BLOCKED | `PolygonPieces` unsaved-hole pick list not built |

## test_sketch_measure.cpp

| # | C++ test function | C++ line | what it checks | Rust test | Rust file:line | Rust code under test | status | note |
|---|---|---|---|---|---|---|---|---|
| 1 | polygonShowsLastEdgeAndArea | 13 | Polygon sketch >= 3 points shows last segment length and live area | sketch_measure_polygon_edge_and_area | crates/strata-app/src/draw_sketch_tests.rs:98 | strata_app::draw_sketch | PORTED | Exact segment + area readout check |
| 2 | lineShowsLastEdgeAndLength | 17 | Line sketch >= 2 points shows last segment length and cumulative length | sketch_measure_line_edge_and_length | crates/strata-app/src/draw_sketch_tests.rs:109 | strata_app::draw_sketch | PORTED | Exact segment + total length readout |
| 3 | twoPolygonPointsShowOnlyTheEdge | 21 | Two-point polygon sketch shows edge length without area | guide_band_shows_tool_chip_title_hint_and_cursor_measure_for_line_and_polygon | crates/strata-app/src/gui_tab_map_hud_tests.rs:121 | strata_app::draw_sketch | PARTIAL | Tests 2-point line and 3-point polygon HUD; 2-point polygon edge-only state not asserted |
| 4 | guideBandShowsTheFiguresOnlyInPaper | 25 | Guide band displays sketch measurements only in Paper chrome mode | guide_band_shows_measure_only_in_paper_look_after_hint | crates/strata-app/src/gui_tab_map_hud_tests.rs:156 | strata_app::gui_tab_map_hud | PORTED | Exact Paper-vs-Modern HUD measure check |
| 5 | drawCursorHasAnOpenCentre | 49 | Draw cursor leaves open transparent pixel at exact crosshair center | — | — | NOT BUILT | BLOCKED | Custom open-centre crosshair cursor pixmap not built |
| 6 | onePointSaysNothing | 57 | Single clicked sketch vertex produces empty measure string | sketch_measure_polygon_edge_and_area | crates/strata-app/src/draw_sketch_tests.rs:98 | strata_app::draw_sketch | PARTIAL | Tested for 3-point polygon; 1-point empty string case omitted |

## test_survey_contour.cpp

| # | C++ test function | C++ line | what it checks | Rust test | Rust file:line | Rust code under test | status | note |
|---|---|---|---|---|---|---|---|---|
| 1 | xlsxSuggestsAxisSwap | 39 | XLSX coordinate import detects swapped N/E columns and suggests axis swap | — | — | NOT BUILT | BLOCKED | XLSX contour point reader/swapper not built |
| 2 | dxfLayerFiltersPointZ | 63 | DXF contour import filters Z points per selected DXF layer | — | — | NOT BUILT | BLOCKED | DXF layer Z-point contour filter not built |
| 3 | planeContourSitsOnTheSlope | 84 | Interpolated contour lines on tilted plane match exact analytical elevation line | — | — | strata_terrain3d::survey_contour_tin | MISSING | Analytical plane-slope elevation accuracy test missing |
| 4 | northSlopeKeepsHighContoursNorth | 108 | Higher contour elevations lie north on north-sloping terrain | — | — | strata_terrain3d::survey_contour_tin | MISSING | Slope-direction contour elevation ordering untested |
| 5 | tenthMetreLabelsHaveNoBinaryResidue | 131 | `0.1` m contour level formatting avoids binary floating residue (`0.300000000004`) | — | — | NOT BUILT | BLOCKED | Contour level string formatter not built |
| 6 | contoursStayInsideThePointHull | 148 | Generated contours stay clipped within input sample convex/concave hull | — | — | strata_terrain3d::survey_contour_tin | MISSING | Hull containment of TIN contours untested |
| 7 | colorBandsMatchTheInterval | 175 | Hypsometric elevation color bands align with chosen contour interval | — | — | NOT BUILT | BLOCKED | Elevation color-band renderer not built |
| 8 | referenceGroupIsNotASubmitLayer | 200 | Contour layers placed in Reference group are excluded from submission export | — | — | NOT BUILT | BLOCKED | Contour layer group assignment not built |
| 9 | sourceFilesAreNotModified | 230 | Importing/contouring source CSV/XLSX/DXF leaves input files untouched | — | — | NOT BUILT | BLOCKED | File-backed survey contour importer pipeline not built |
| 10 | badRowsAreReportedAndTwoPointsAreRefused | 256 | Invalid point rows increment error notes and < 3 points refuse TIN generation | test_suspect_point_filtering_and_survey_contour_tin_builder | crates/strata-terrain3d/src/trench_profile3d_tests.rs:49 | strata_terrain3d::survey_contour_tin | PARTIAL | Tests suspect-point filtering & small TIN builder; < 3 points refusal and full row error counts omitted |
| 11 | twentyThousandPointsFinishWithinBudget | 293 | 20,000-point Delaunay TIN + contour extraction completes within latency budget | — | — | strata_terrain3d::survey_contour_tin | MISSING | 20k-point performance benchmark test missing |

## test_survey_contour_clip.cpp

| # | C++ test function | C++ line | what it checks | Rust test | Rust file:line | Rust code under test | status | note |
|---|---|---|---|---|---|---|---|---|
| 1 | contoursOutsideSurveyAreaAreRemoved | 17 | Survey-contour clipper removes contour segments outside `survey_area` | — | — | NOT BUILT | BLOCKED | Survey contour polygon clipper not built |
| 2 | namedNorthEastColumnsSwap | 37 | Explicit `North`/`East` columns swap properly when outside Korea bounds | — | — | NOT BUILT | BLOCKED | Contour column-swap service not built |
| 3 | dxfSkipsZeroElevationLayers | 64 | DXF contour loader ignores layers where all extracted elevations are `0` | — | — | NOT BUILT | BLOCKED | DXF contour elevation filter not built |

## test_survey_facts.cpp

| # | C++ test function | C++ line | what it checks | Rust test | Rust file:line | Rust code under test | status | note |
|---|---|---|---|---|---|---|---|---|
| 1 | rememberCounts_roundTrip | 68 | Stores and retrieves area/feature counts + work CRS matching C++ ini line format | remember_counts_writes_the_cpp_line | crates/strata-storage/src/survey_facts_tests.rs:37 | strata_storage::survey_facts | PORTED | Also `saving_records_counts_and_the_work_crs` (`crates/strata-app/src/app_state_home_facts_tests.rs:41`) |
| 2 | rememberCheck_roundTrip | 83 | Persists and reads back checklist error/warning counts and timestamp | — | — | NOT BUILT | BLOCKED | `remember_check` writer not built (read-only parsing only) |
| 3 | markPackaged_setsTime | 94 | Records `packaged_at_ms` timestamp for a survey path | — | — | NOT BUILT | BLOCKED | `mark_packaged` writer not built |
| 4 | fingerprintChange_dropsCounts | 105 | Changing file size/mtime invalidates cached counts on `lookup` | record_format_and_fingerprint | crates/strata-storage/src/survey_facts_tests.rs:19 | strata_storage::survey_facts | PORTED | Verifies `(size, mtime_ms)` invalidation |
| 5 | probe_countsFromLocalGpkg | 117 | `probeCountsIfUnknown` counts local unopened GPKG and writes facts ini | probe_counts_unknown_local_surveys_only | crates/strata-storage/src/survey_facts_probe_tests.rs:14 | strata_storage::survey_facts_probe | PORTED | Also `home_visit_counts_an_unopened_recent_survey` (`crates/strata-app/src/app_state_home_facts_tests.rs:17`) |
| 6 | probe_skipsRememberedRemoteRemovable | 140 | Skips already-known, remote/UNC, removable drives and enforces probe cap | non_gpkg_and_remote | crates/strata-storage/src/survey_facts_probe_tests.rs:35 | strata_storage::survey_facts_probe | PARTIAL | Tests UNC/Network/Removable/OpenInApp blocks; omits cap (`MAX_PROBES = 12`) enforcement |
| 7 | probe_disabledByEnv | 156 | `KA_HGIS_HOME_PROBE=0` disables background probing | — | — | strata_storage::survey_facts_probe::probe_enabled | MISSING | `probe_enabled()` exists at `survey_facts_probe.rs:27` with no env-var test |
| 8 | summaryLine_wordings | 167 | Formats `crs_text` and `check_text` summary labels cleanly | cell_texts | crates/strata-storage/src/survey_facts_tests.rs:62 | strata_storage::survey_facts | PORTED | Tests all `crs_text` and `check_text` branches |
| 9 | forget_removesEntry | 180 | Removes single survey entry from `factsOrder` and `facts\<key>` | — | — | NOT BUILT | BLOCKED | `forget` remover not built |
| 10 | capAt24Entries | 190 | Evicts oldest facts entries beyond `MAX_ENTRIES = 24` | — | — | strata_storage::survey_facts | MISSING | `MAX_ENTRIES = 24` cap logic in `survey_facts.rs` untested beyond 24 entries |

## test_survey_point_arrange.cpp

| # | C++ test function | C++ line | what it checks | Rust test | Rust file:line | Rust code under test | status | note |
|---|---|---|---|---|---|---|---|---|
| 1 | arrangeMatchesADirectRead | 43 | Point arranger preserves exact XYZ point count and coordinates of direct reader | — | — | NOT BUILT | BLOCKED | `SurveyPointArrange` service not built |
| 2 | fileCrsIsTransformedToTheWorkCrs | 70 | Reprojects imported survey points from source file CRS into active work CRS | — | — | NOT BUILT | BLOCKED | `SurveyPointArrange` CRS pipeline not built |
| 3 | notesNameSuspiciousDuplicateAndFarPoints | 106 | Diagnostics flag duplicate and far-outlier elevation points | test_suspect_point_filtering_and_survey_contour_tin_builder | crates/strata-terrain3d/src/trench_profile3d_tests.rs:49 | strata_terrain3d::survey_contour_tin | PARTIAL | Tests suspect elevation outlier filter; duplicate-XY diagnostics and C++ note wording not built |
| 4 | outsidePointsSuggestTheOtherAxisOrder | 132 | Points landing outside Korea work extent suggest X/Y axis swap | — | — | NOT BUILT | BLOCKED | `SurveyPointArrange` spatial swap detector not built |
| 5 | issuesAreCapped | 151 | Bounds diagnostic issue list size to fixed display limit | — | — | NOT BUILT | BLOCKED | `SurveyPointArrange` issue capper not built |
| 6 | dxfLinesBecomeBreaklines | 166 | DXF polylines extract into TIN breaklines alongside point cloud | — | — | NOT BUILT | BLOCKED | `SurveyPointArrange` DXF breakline extractor not built |

## test_survey_point_dialogs.cpp

| # | C++ test function | C++ line | what it checks | Rust test | Rust file:line | Rust code under test | status | note |
|---|---|---|---|---|---|---|---|---|
| 1 | contourDialogShowsDiagnosticsAndHandsOverPoints | 77 | Contour dialog previews file diagnostics and passes parsed points to generator | empty_path_accept_shows_choose_file_prompt_and_creates_nothing | crates/strata-app/src/dlg_survey_contour_tests.rs:62 | strata_app::dlg_survey_contour | PARTIAL | Tests empty-path prompt & no creation; live diagnostic preview & point hand-over not built |
| 2 | fileCrsChoiceMovesThePoints | 106 | Changing file CRS selector in contour dialog shifts point coordinates accordingly | test_d6_trench_dial_survey_kind_and_contour_advanced_toggle | crates/strata-app/src/dialogs_modal_tests_c.rs:187 | strata_app::dialogs_modal | PARTIAL | Verifies 5 CRS index state only; point transformation not built |
| 3 | surveyAreaSwatchesAreReadable | 129 | Survey area dialog color swatches maintain readable contrast | — | — | strata_app::dlg_survey_area_setup | MISSING | Color swatch contrast assertion missing |
| 4 | surveyAreaDialogIsCompact | 148 | Survey area setup dialog fits compactly without oversized padding | window_values_follow_cpp | crates/strata-app/src/survey_area_setup_tests.rs:14 | strata_app::dlg_survey_area_setup | PARTIAL | Tests strings and defaults; pixel window geometry not checked |
| 5 | surveyAreaStartIsTheThemedMainButton | 166 | Start drawing button uses primary theme accent style | — | — | strata_app::dlg_survey_area_setup | MISSING | Primary-button style color assertion missing |
| 6 | magnetIsOnUntilTheUserTurnsItOff | 185 | Snap magnet tool starts enabled and stays on until user explicitly toggles off | test_d13_snap_settings_tolerance_and_measure_copy_text | crates/strata-canvas2d/src/cad_interaction_tests.rs:35 | strata_canvas2d::cad_interaction | PARTIAL | Checks snap tolerance presets; default-on ribbon toggle persistence untested |
| 7 | surveyAreaToContinueIsTheSurveysOwn | 196 | Continuing to draw selects survey's own `survey_area` table rather than creating/reusing hidden ones | hidden_area_layer_gets_a_new_table_that_saves | crates/strata-app/src/survey_area_setup_tests.rs:71 | strata_app::survey_area_setup | PORTED | Also `new_survey_opens_the_window_and_reuses_the_empty_table` at `:52` |
| 8 | demDialogOffersTheViewFit | 226 | DEM dialog offers quick button to fill extent from current map view | — | — | NOT BUILT | BLOCKED | DEM download dialog view-fit action not built |
| 9 | gridOriginButtonMovesTheGrid | 264 | Canvas grid origin picker button shifts overlay origin on click | — | — | NOT BUILT | BLOCKED | Interactive map-click grid-origin picker not built |
| 10 | gridEdgeBandIsOpaqueInPaper | 282 | Grid border coordinate band renders opaque cream fill (`#F5F4ED`) in Paper mode | test_d14_region_locator_and_canvas_grid_band | crates/strata-app/src/gui_shell_tests_b.rs:55 | strata_app::gui_tab_map_format | PORTED | Verifies `GRID_BAND_FILL == #F5F4ED`, border color, and 18 px rects at `:114-126` |

## test_survey_scope_clip.cpp

| # | C++ test function | C++ line | what it checks | Rust test | Rust file:line | Rust code under test | status | note |
|---|---|---|---|---|---|---|---|---|
| 1 | noSurveyLeavesFeatures | 16 | Scope clipper preserves all features unchanged when no `survey_area` exists | — | — | NOT BUILT | BLOCKED | `SurveyScopeClip` layer clipper not built |
| 2 | surveyBufferDropsFarFeature | 33 | Scope clipper drops features outside buffered `survey_area` extent | test_rmse_grade_evaluation_and_subpixel_centering_on_crosshair | crates/strata-geo/src/georef_tests.rs:168 | strata_geo::georef::clip_line_segment_by_bbox | PARTIAL | Checks single line-segment bbox clip at `:214-220`; multi-feature buffered polygon layer clip not built |
| 3 | intersectingCopyKeepsNearLine | 68 | Clipping intersects nearby lines cleanly with buffered survey boundary | — | — | NOT BUILT | BLOCKED | `SurveyScopeClip` layer clipper not built |

## test_trench_edit.cpp

| # | C++ test function | C++ line | what it checks | Rust test | Rust file:line | Rust code under test | status | note |
|---|---|---|---|---|---|---|---|---|
| 1 | rightClickDeleteIsUndoableAndNotSaved | 92 | Right-click deleting a trench removes it in edit buffer with undo and leaves disk untouched | — | — | NOT BUILT | BLOCKED | `KaTrenchMoveTool` right-click delete not built |
| 2 | singleMoveUsesSnappedClickAndUndoes | 112 | Single-trench move snaps cursor delta and records one undo step | — | — | NOT BUILT | BLOCKED | `KaTrenchMoveTool` single move not built |
| 3 | wholeMoveIsOneUndoStep | 133 | Moving entire trench layer translates all trenches in one undo step | — | — | NOT BUILT | BLOCKED | `KaTrenchMoveTool` whole move not built |
| 4 | newUnsavedTrenchesCanBePickedAndDeleted | 158 | Unsaved newly generated trenches can be picked and deleted before save | — | — | NOT BUILT | BLOCKED | Interactive trench pick/delete tool not built |
| 5 | replaceIsOneUndoStepAndTracksHandEdits | 183 | Re-running trench placement is single undoable edit tracking hand modifications | — | — | NOT BUILT | BLOCKED | `TrenchLayerEdit` replacement transaction not built |
| 6 | replaceRefusesOtherCrsAndInvalidCells | 211 | Layer replace rejects foreign CRS and invalid trench polygons | — | — | NOT BUILT | BLOCKED | `TrenchLayerEdit` guard not built |
| 7 | dialogHidesPrefixInRatioMode | 228 | Trench dialog hides manual name prefix input while in ratio auto-fill mode | test_d19_trench_dialog_actions_and_drawing_studio_spinboxes | crates/strata-app/src/dialogs_modal_tests.rs:27 | strata_app::dlg_trench_generator_controls | PARTIAL | Checks button/label strings; UI visibility toggle in ratio mode untested |
| 8 | dialogPreviewRunsOnceAfterInputSettles | 249 | Debounced live preview recomputes once after spinbox/dial edits settle | — | — | NOT BUILT | BLOCKED | Trench dialog debounced preview timer not built |
| 9 | dialogShowsWhyInvalidAreaHasNoGrid | 273 | Trench dialog summary shows Korean reason when survey polygon cannot hold grid | invalid_area_explains_why_grid_is_empty | crates/strata-geo/src/trench_plan_b_tests.rs:196 | strata_geo::trench_plan | PARTIAL | Core planner returns empty reason; `dlg_trench_generator_summary` UI display untested |

## test_trench_grid_snap.cpp

| # | C++ test function | C++ line | what it checks | Rust test | Rust file:line | Rust code under test | status | note |
|---|---|---|---|---|---|---|---|---|
| 1 | cornersSitOnGridNodes | 97 | 3 `_data()` rows (5 m, 10 m, 1 m): L-trench outer corners snap onto grid nodes | lshape_trench_all_four_bends_keep_exact_15m_arms_and_snap_to_coordinate_grid | crates/strata-geo/src/trench_grid_tests.rs:32 | strata_geo::trench_lshape | PARTIAL | Tests 5 m coordinate snap only; 10 m/1 m & grid-snapped ratio fill (`TrenchGridSnap`) not built |
| 2 | turnedGridTurnsTheTrenches | 108 | Rotated canvas grid turns generated L-trenches and aligns corners to grid | — | — | NOT BUILT | BLOCKED | `TrenchGridSnap` rotated grid-node alignment not built |
| 3 | coarseGridSaysWhyTenPercentDoesNotFit | 121 | Coarse grid step reports Korean explanation when 10% target ratio cannot fit | — | — | NOT BUILT | BLOCKED | `TrenchGridSnap` grid-constrained ratio planner not built |
| 4 | planCacheKeepsGridPlansApart | 132 | `TrenchPlanCache` keys distinct grid snap/step specs separately | — | — | NOT BUILT | BLOCKED | `TrenchPlanCache` not built |

## test_trench_handle.cpp

| # | C++ test function | C++ line | what it checks | Rust test | Rust file:line | Rust code under test | status | note |
|---|---|---|---|---|---|---|---|---|
| 1 | gridDragPutsTheCornerOnANode | 83 | Dragging trench handle snaps corner directly onto active grid node | — | — | NOT BUILT | BLOCKED | `KaTrenchMoveTool` grid-node snap drag not built |
| 2 | arrowKeysMoveThePickedTrenchOneCell | 100 | Arrow keys translate picked trench by active grid step (or 0.1 m) with undo | test_rectangle_geometry_generation_and_translation_step_with_shift | crates/strata-geo/src/trench_grid_tests.rs:96 | strata_geo::trench_edit | PARTIAL | Tests `translate_trench_center_with_key` (0.1 m / 1.0 m); grid-cell step and canvas undo not built |
| 3 | arrowKeysMoveTheWholeGridWithoutAGridOneMetre | 125 | Shift+Arrow translates whole grid by 1 m when grid overlay is off | test_rectangle_geometry_generation_and_translation_step_with_shift | crates/strata-geo/src/trench_grid_tests.rs:96 | strata_geo::trench_edit | PARTIAL | Tests 1.0 m shift step on single center; whole-layer shift translation not built |

## test_trench_lshape.cpp

| # | C++ test function | C++ line | what it checks | Rust test | Rust file:line | Rust code under test | status | note |
|---|---|---|---|---|---|---|---|---|
| 1 | oneLHasTwoFifteenMetreArms | 87 | Single L-trench builds two orthogonal 15 m arms x 2 m width = 56 m^2 | test_trench_lshape_origin_1000_2000_area_56_and_invalid_width_guard | crates/strata-geo/src/trench_grid_tests.rs:51 | strata_geo::trench_lshape | PORTED | Exact 56 m^2 geometry check |
| 2 | bendTurnsTheArmsFourWays | 117 | 4 `_data()` bends (`ㄴ`, `ㄱ`, `ㄴ 좌우 반대`, `ㄱ 좌우 반대`) orient L-arms correctly | lshape_trench_all_four_bends_keep_exact_15m_arms_and_snap_to_coordinate_grid | crates/strata-geo/src/trench_grid_tests.rs:32 | strata_geo::trench_lshape | PORTED | Covers bends `0..=3` with 56 m^2 area & 5 m snap |
| 3 | targetCheckIsExactForStraightAndJustOverForL | 137 | Target cell area/width validation passes exact straight trenches and L-trench margin | cells_valid_matches_writer_rules | crates/strata-geo/src/trench_plan_b_tests.rs:252 | strata_geo::trench_plan | PARTIAL | Covers straight cells validation; 56 m^2 L-trench +2% margin rule not built |
| 4 | tenPercentFillStaysInsideWithoutOverlap | 161 | 3 `_data()` shapes (5187 square, 30 deg slope, concave mirror): 10% L-fill stays inside without overlap | — | — | NOT BUILT | BLOCKED | L-shape ratio auto-fill planner not built |
| 5 | tenPercentFillSpreadsOverTheArea | 202 | 10% L-fill distributes trenches across full survey polygon rather than clustering | — | — | NOT BUILT | BLOCKED | L-shape ratio auto-fill planner not built |
| 6 | brokenLIsRejected | 217 | Degenerate L-trench dimensions (width >= arm or non-positive) produce empty ring | test_trench_lshape_origin_1000_2000_area_56_and_invalid_width_guard | crates/strata-geo/src/trench_grid_tests.rs:51 | strata_geo::trench_lshape | PARTIAL | Tests width == arm -> empty; negative/NaN arm cases not explicitly tested |
| 7 | lTrenchSavesAsOnePolygon | 236 | L-trench plan commits into GPKG as single continuous 7-vertex polygon feature | — | — | NOT BUILT | BLOCKED | L-trench GPKG writer integration not built |
| 8 | planCacheKeepsStraightAndLApart | 270 | `TrenchPlanCache` caches straight vs L-trench plans under distinct keys | — | — | NOT BUILT | BLOCKED | `TrenchPlanCache` not built |

## test_trench_plan.cpp

| # | C++ test function | C++ line | what it checks | Rust test | Rust file:line | Rust code under test | status | note |
|---|---|---|---|---|---|---|---|---|
| 1 | numbersRunFromTheHighSideDown | 65 | Trench sequential IDs number downward from high elevation side | numbers_run_from_the_high_side_down | crates/strata-geo/src/trench_plan_b_tests.rs:167 | strata_geo::trench_plan | PORTED | Exact high-to-low numbering check |
| 2 | invalidAreaExplainsWhyGridIsEmpty | 89 | Self-intersecting or empty area explains why grid is empty in Korean | invalid_area_explains_why_grid_is_empty | crates/strata-geo/src/trench_plan_b_tests.rs:196 | strata_geo::trench_plan | PORTED | Exact empty-reason check |
| 3 | everyEmptyGridHasAReason | 102 | Ensures no valid/invalid failed plan returns an empty string reason | every_empty_grid_has_a_reason | crates/strata-geo/src/trench_plan_b_tests.rs:210 | strata_geo::trench_plan | PORTED | Non-empty Korean reason invariant verified |
| 4 | cellsValidMatchesWriterRules | 119 | `cellsValid` accepts legal cells and rejects oversize/empty/non-finite cells | cells_valid_matches_writer_rules | crates/strata-geo/src/trench_plan_b_tests.rs:252 | strata_geo::trench_plan | PORTED | Exact writer rule check |
| 5 | clearLayerReportsUnreadableFile | 133 | `clearLayer` returns readable Korean error when target GPKG cannot be opened | — | — | NOT BUILT | BLOCKED | `TrenchGridGenerator` GPKG writer/clearLayer not built |
| 6 | planCacheReturnsGeneratorResult | 150 | `TrenchPlanCache` stores and returns identical `GeneratorResult` on repeat key | — | — | NOT BUILT | BLOCKED | `TrenchPlanCache` not built |
| 7 | placeGridKeepsHandEditsAndUndoesReplacement | 175 | Re-placing grid preserves hand-edited trenches or groups replacement in one undo step | — | — | NOT BUILT | BLOCKED | `TrenchLayerEdit` replacement not built |
| 8 | unknownGridIsTreatedAsAdjusted | 255 | Legacy/unknown grid layer metadata defaults safely to adjusted mode | — | — | NOT BUILT | BLOCKED | `TrenchLayerEdit` metadata handler not built |

## test_trench_plate.cpp

| # | C++ test function | C++ line | what it checks | Rust test | Rust file:line | Rust code under test | status | note |
|---|---|---|---|---|---|---|---|---|
| 1 | meterFillsToTheRatioAndMarksTheTarget | 29 | Coverage gauge meter fills proportionally to computed ratio and draws target tick | — | — | strata_app::dlg_trench_generator_summary | MISSING | Gauge meter rendering logic in summary panel untested |
| 2 | dialNeedleFollowsTheTurn | 41 | Azimuth dial updates angle and summary note when turned (`0 deg` vs rotated) | test_d6_trench_dial_survey_kind_and_contour_advanced_toggle | crates/strata-app/src/dialogs_modal_tests_c.rs:187 | strata_app::dlg_trench_generator_controls | PARTIAL | Also `azimuth_dial_note_ko` (`cad_interaction_tests.rs:165`); interactive dial mouse-drag angle rotation untested |
| 3 | terrainNoteSaysWhyTheGridRunsNorthSouth | 57 | Terrain note explains why flat/default grid runs North-South vs slope direction | — | — | strata_app::dlg_trench_generator_summary | MISSING | Terrain orientation note text untested |
| 4 | lShapeChoiceLaysTheTenPercentPlan | 75 | Selecting L-shape choice in dialog runs 10% L-trench fill plan | — | — | NOT BUILT | BLOCKED | UI L-shape mode & L-shape ratio planner not built |
| 5 | gridLockLaysTheLsOnTheCoordinateGrid | 94 | Enabling grid-lock checkbox aligns L-trenches to active coordinate grid nodes | — | — | NOT BUILT | BLOCKED | UI grid-lock snap option not built |
| 6 | paperWindowCarriesTheMeterAndTheDial | 114 | Paper-chrome trench dialog renders readable paper palette hosting meter + dial | test_d19_trench_dialog_actions_and_drawing_studio_spinboxes | crates/strata-app/src/dialogs_modal_tests.rs:27 | strata_app::dlg_trench_generator | PARTIAL | Checks button text; paper-look palette and widget layout untested |

## test_trench_terrain.cpp

| # | C++ test function | C++ line | what it checks | Rust test | Rust file:line | Rust code under test | status | note |
|---|---|---|---|---|---|---|---|---|
| 1 | contoursRisingNorthPointNorth | 45 | Vector contour features rising northward yield `0 deg` North upslope aspect | — | — | NOT BUILT | BLOCKED | `TrenchTerrain::contourSamples` vector contour extraction not built (`upslope_aspect` takes raw samples) |
| 2 | contoursRisingEastPointEast | 55 | Vector contour features rising eastward yield `90 deg` East upslope aspect | — | — | NOT BUILT | BLOCKED | `TrenchTerrain::contourSamples` not built |
| 3 | linesWithoutElevationAreNotContours | 65 | Vector lines missing valid elevation field are ignored by contour aspect sampler | — | — | NOT BUILT | BLOCKED | `TrenchTerrain::contourSamples` not built |

## Port now

Ordered by failure severity (data loss/corruption -> wrong coordinates -> wrong behaviour -> looks):

### Data loss / corruption
1. `test_export_survey_areas.cpp` — `emptyFirstLayerDoesNotDropTheSubmissionShp` (158) — `PARTIAL`: add pair test where 1st `survey_area` layer is empty and 2nd holds features so submission SHP is never silently dropped.
2. `test_export_survey_areas.cpp` — `invalidGeometryInSecondLayerBlocksSubmission` (245) — `PARTIAL`: add explicit test that self-intersecting geometry in a second `survey_area` layer blocks submission export.
3. `test_export_survey_areas.cpp` — `emptyGeometryBlocksSubmission` (283) — `PARTIAL`: test empty WKB geometry on a feature blocking export via `GEOMETRY_NOT_EMPTY`.
4. `test_export_survey_areas.cpp` — `zeroAreaPolygonBlocksSubmission` (318) — `MISSING`: test zero-area collapsed polygon blocking submission export.
5. `test_dem_trench.cpp` — `writeGpkg_rejectsInvalidCellsWithoutChangingExisting` (576) — `PARTIAL`: `cells_valid_matches_writer_rules` only tests validation; still verify disk bytes remain unchanged when invalid cells are rejected.
6. `test_dem_trench.cpp` — `demDownload_rejectsInvalidRangeWithoutChangingFiles` (631) — `PARTIAL`: verify existing DEM cache files on disk keep exact bytes when invalid extent is rejected.
7. `test_dem_trench.cpp` — `demDownload_earlyCancellationPreservesExistingFile` (650) — `PARTIAL`: verify early cancel leaves pre-existing tile file bytes and directory count untouched.
8. `test_survey_facts.cpp` — `capAt24Entries` (190) — `MISSING`: test > 24 recent survey records so oldest cached fact evicts cleanly without growing unbounded.
9. `test_buffer.cpp` — `splitPolygonWithLine_splitsGeometry` (280) — `PARTIAL`: only `strata-geo` split math tested; commit/undo path onto edited layer missing.
10. `test_polygon_erase.cpp` — `erased_cutsOnlyTheOverlappingPlace` (70) — `PARTIAL`: test disjoint and multi-part geometry difference cases beyond single 10x10 minus 2x2 square.

### Wrong coordinates
11. `test_export_survey_areas.cpp` — `referenceLayerNamedLikeDomainStaysOut` (183) — `MISSING`: verify reference layer named `"조사구역"` without `layer_key` never enters submission SHP.
12. `test_export_survey_areas.cpp` — `legacyLayerWithoutKeyStillExports` (219) — `MISSING`: verify legacy GPKG layer missing `layer_key` metadata still exports cleanly in EPSG:5179.
13. `test_control_point_csv.cpp` — `koreanHeaders_mapToColumns` (49) — `MISSING`: verify Korean header column permutation maps X/Y/Z without swapping axes.
14. `test_control_point_csv.cpp` — `namedPointColumn_winsOverRowNumber` (69) — `MISSING`: verify point-name column wins over row-number index column.
15. `test_control_point_csv.cpp` — `headerless_numericFourthColumnIsZ` (89) — `PARTIAL`: test headerless 4-column numeric CSV maps col 4 to Z elevation rather than text.
16. `test_survey_contour.cpp` — `planeContourSitsOnTheSlope` (84) — `MISSING`: verify TIN contour elevations match tilted analytical plane within metre tolerance.
17. `test_survey_contour.cpp` — `northSlopeKeepsHighContoursNorth` (108) — `MISSING`: verify north-sloping point cloud orders higher contours north.
18. `test_survey_contour.cpp` — `contoursStayInsideThePointHull` (148) — `MISSING`: verify TIN contours remain inside sample convex/concave hull.
19. `test_survey_scope_clip.cpp` — `surveyBufferDropsFarFeature` (33) — `PARTIAL`: `clip_line_segment_by_bbox` only covers single segment; test buffered polygon scope check.
20. `test_measure.cpp` — `length10mOn5187` (32) — `PARTIAL`: add exact `(200000,450000)->(200010,450000)` planar distance assertion on EPSG:5187.
21. `test_measure.cpp` — `rectangleArea200m2` (41) — `PARTIAL`: add exact `20x10 = 200.0` m^2 assertion on EPSG:5187.
22. `test_grid_math.cpp` — `snapHonoursOriginAndRotation` (67) — `PARTIAL`: extends 5 m axis-aligned snap to non-zero origin check.
23. `test_trench_grid_snap.cpp` — `cornersSitOnGridNodes` (97) — `PARTIAL`: covers 5 m coordinate snap only; test 10 m / 1 m step cases when `TrenchGridSnap` is built.
24. `test_parcel_copy_keys.cpp` — `snapTraceShowsNearbyVerticesOnlyWhenCloseAndFillsSkippedVertex` (122) — `PARTIAL`: test hover snap-trace cursor proximity threshold and intermediate path fill.
25. `test_survey_point_dialogs.cpp` — `fileCrsChoiceMovesThePoints` (106) — `PARTIAL`: wire & test changing contour dialog file-CRS selector shifting coordinates.
26. `test_editing_tools.cpp` — `captureStartsNewShapeOnSavedCornerWithoutWriting` (93) — `PARTIAL`: cover saved-polygon corner start in addition to saved-line vertex.

### Wrong behaviour
27. `test_checklist.cpp` — `liveStateFromEmptyProject` (202) — `MISSING`: assert `checklist_state()` on blank `StrataAppState` yields zero counts and false flags.
28. `test_checklist.cpp` — `loadRules` (82) — `PARTIAL`: test invalid/missing rules file load handling in `ChecklistEngine`.
29. `test_checklist.cpp` — `evaluateFailsWithoutSurvey` (110) — `PARTIAL`: test `ChecklistEngine::evaluate` directly on empty JSON map.
30. `test_checklist.cpp` — `evaluatePassesHappyFixture` (120) — `PARTIAL`: add synthetic all-27-passing fixture test (`demo.gpkg` only passes 24/27).
31. `test_checklist_rules.cpp` — `cleanStatePassesEveryRule` (106) — `MISSING`: verify clean state passes all 27 rules.
32. `test_checklist_rules.cpp` — `everyRuleHasAViolationRow` (113) — `MISSING`: ensure violation test table covers every rule ID in `drawing_checklist_v1.json`.
33. `test_checklist_rules.cpp` — `singleViolationFailsOnlyThatRule` (132) — `MISSING`: verify injecting each of the 27 violations fails only its matching rule.
34. `test_checklist_rules.cpp` — `countMinReadsItsParameter` (155) — `MISSING`: verify dynamic `count_min` threshold reads JSON suffix.
35. `test_checklist_rules.cpp` — `unknownCheckOrSeverityFailsClosed` (168) — `MISSING`: verify unknown `check_type` or `severity` fails closed.
36. `test_checklist_rules.cpp` — `failedLoadKeepsPreviousRules` (182) — `MISSING`: verify failed JSON reload preserves active ruleset.
37. `test_checklist_rules.cpp` — `offendersAndNotesReachTheResult` (193) — `PARTIAL`: assert arbitrary notes field passthrough into `CheckResult`.
38. `test_checklist_state.cpp` — `everyControlPointNeedsMeta` (76) — `PARTIAL`: test dropping each of datum/ellipsoid/projection/origin individually.
39. `test_checklist_state.cpp` — `symbolsAreNotSurveyAreas` (111) — `PARTIAL`: add `POINT` and 8-vertex buffered circle cases to `is_abstract_marker` test.
40. `test_checklist_state.cpp` — `featuresMustLieInsideTheSurveyPolygon` (117) — `PARTIAL`: add `artifact_point` outside-survey offender assertion.
41. `test_checklist_state.cpp` — `featureLayersHoldRealShapesAndCrs` (138) — `MISSING`: test wrong geometry type and wrong CRS on domain feature layers.
42. `test_checklist_state.cpp` — `emptyRequiredFieldsAreReported` (162) — `MISSING`: test required-field offender labels across `feature_poly`, `artifact_point`, `section_line`.
43. `test_checklist_state.cpp` — `staleCountStillChecksTheAttributes` (224) — `MISSING`: test unsaved attribute gaps on `feature_poly`, `control_points`, `section_line`.
44. `test_export_survey_areas.cpp` — `exportSubmissionPackage_withoutUserSheetWritesShpOnly` (507) — `PARTIAL`: test explicit absence of PDF when `drawing_sheets` is false.
45. `test_control_point_csv.cpp` — `quotedCellsAndDelimiters` (76) — `MISSING`: test quoted commas and tab/semicolon CSV delimiters.
46. `test_control_point_csv.cpp` — `shortRowsAndMissingXY` (100) — `PARTIAL`: test short (<2 col) and blank X/Y row skipping + warning count.
47. `test_control_point_csv.cpp` — `swapSuggestion_findsSurveyAreaTheLayerCountMissed` (158) — `PARTIAL`: add spatial axis-swap suggestion check when implemented.
48. `test_contour_safety.cpp` — `fitCellSizeKeepsTheGridUnderTheLimit` (72) — `PARTIAL`: assert grid dimension cap calculation directly.
49. `test_contour_safety.cpp` — `breaklineNodesShapeTheSurfaceOnly` (120) — `PARTIAL`: assert breakline elevation constraints shape contours cleanly.
50. `test_survey_contour.cpp` — `badRowsAreReportedAndTwoPointsAreRefused` (256) — `PARTIAL`: assert < 3 points refusal and per-row error reporting.
51. `test_survey_contour.cpp` — `twentyThousandPointsFinishWithinBudget` (293) — `MISSING`: add 20,000-point TIN timing test.
52. `test_survey_point_arrange.cpp` — `notesNameSuspiciousDuplicateAndFarPoints` (106) — `PARTIAL`: assert duplicate-point & outlier Korean warning text.
53. `test_dem_trench.cpp` — `startTrenchGrid_doesNotUnionAllSurveyAreas` (278) — `PARTIAL`: cover `startTrenchGrid` entry once interactive placement is wired.
54. `test_dem_trench.cpp` — `layerTreeMenu_hasLabelToggleAndTrenchRatio` (700) — `PARTIAL`: test live trench-ratio context action once un-deferred (`W-T1`).
55. `test_dem_trench.cpp` — `hillshadeWritesByteTif` (812) — `PARTIAL`: test GeoTIFF byte output once file writer exists.
56. `test_trench_lshape.cpp` — `targetCheckIsExactForStraightAndJustOverForL` (137) — `PARTIAL`: test L-trench +2% target margin rule.
57. `test_trench_lshape.cpp` — `brokenLIsRejected` (217) — `PARTIAL`: add negative/NaN arm-length rejection checks.
58. `test_trench_handle.cpp` — `arrowKeysMoveThePickedTrenchOneCell` (100) — `PARTIAL`: cover grid-step translation and undo once tool exists.
59. `test_trench_handle.cpp` — `arrowKeysMoveTheWholeGridWithoutAGridOneMetre` (125) — `PARTIAL`: cover whole-grid Shift+Arrow shift.
60. `test_trench_edit.cpp` — `dialogHidesPrefixInRatioMode` (228) — `PARTIAL`: assert prefix box hides in ratio mode inside egui harness.
61. `test_trench_edit.cpp` — `dialogShowsWhyInvalidAreaHasNoGrid` (273) — `PARTIAL`: assert invalid-area explanation renders in `dlg_trench_generator_summary`.
62. `test_trench_plate.cpp` — `dialNeedleFollowsTheTurn` (41) — `PARTIAL`: test dial drag updates azimuth angle and Korean note in UI harness.
63. `test_trench_plate.cpp` — `terrainNoteSaysWhyTheGridRunsNorthSouth` (57) — `MISSING`: test `dlg_trench_generator_summary` N-S / slope explanation note.
64. `test_editing_tools.cpp` — `captureReportsBowTieAfterFinishingAndCancelsCleanly` (126) — `PARTIAL`: test interactive sketch cancel cleanly after bow-tie prompt.
65. `test_editing_tools.cpp` — `clickedHandleIsPickedAndDeleteRemovesOnlyThatVertex` (228) — `PARTIAL`: test Delete key removing single vertex once canvas handle editing exists.
66. `test_editing_tools.cpp` — `attributeEditingSkipsReferenceAndCadastral` (269) — `PARTIAL`: test clicking reference/cadastral polygon does not open editable attribute card.
67. `test_inner_piece.cpp` — `unsavedShapeIsPickedAndEdited` (108) — `PARTIAL`: add Delete-key removal check on picked unsaved drawing.
68. `test_inner_piece.cpp` — `anySurveyLayerGetsTheMenuAndARightClickStartsNoCycle` (221) — `PARTIAL`: test map right-click never triggers overlap pick cycling.
69. `test_inner_piece.cpp` — `keyASwitchesToSelectWithoutTogglingOrDroppingASketch` (240) — `MISSING`: test `Key::A` switches to Select without toggling off or discarding active sketch.
70. `test_feature_records.cpp` — `artifactNumbers_runOverTheLayer` (102) — `PARTIAL`: add direct unit test for whole-layer artifact numbering across mixed kinds.
71. `test_feature_card.cpp` — `card_openFormEmitsLayerAndFid` (272) — `PARTIAL`: verify clicking card "전체 기록 보기" opens record modal for exact layer + FID in egui harness.
72. `test_feature_card_fields.cpp` — `surveyStatusSurveyorAndDateAreOnTheCard` (52) — `PARTIAL`: test live UI combo selection updating `survey_status`/`surveyor`/`survey_date`.
73. `test_feature_card_fields.cpp` — `enterFinishesOnlyThatBox` (63) — `PARTIAL`: test Return inside inline card input clears focus without triggering modal defaults.
74. `test_feature_card_fields.cpp` — `measureFigureStandsAlone` (86) — `PARTIAL`: inspect rendered `ri_attr_card` text widgets to confirm standalone figure label.
75. `test_measure.cpp` — `planarAreaMatchesLabelSelectAndTape` (60) — `PARTIAL`: assert sketch HUD, card, and measure tape show matching area on one test polygon.
76. `test_sketch_measure.cpp` — `twoPolygonPointsShowOnlyTheEdge` (21) — `PARTIAL`: assert 2-point polygon `draw_sketch` measure string omits area.
77. `test_sketch_measure.cpp` — `onePointSaysNothing` (57) — `PARTIAL`: assert 1-point `draw_sketch` returns `None`.
78. `test_survey_facts.cpp` — `probe_skipsRememberedRemoteRemovable` (140) — `PARTIAL`: assert `MAX_PROBES = 12` cap and skip of already-cached survey entries.
79. `test_survey_facts.cpp` — `probe_disabledByEnv` (156) — `MISSING`: test `KA_HGIS_HOME_PROBE=0` disabling `probe_enabled()`.
80. `test_survey_point_dialogs.cpp` — `contourDialogShowsDiagnosticsAndHandsOverPoints` (77) — `PARTIAL`: test point diagnostic counts & hand-over when wired.
81. `test_survey_point_dialogs.cpp` — `magnetIsOnUntilTheUserTurnsItOff` (185) — `PARTIAL`: test ribbon snap magnet starts enabled on launch and stays sticky.

### Looks
82. `test_buffer.cpp` — `setLabelsVisible_togglesPolygonLabels` (225) — `PARTIAL`: verify toggling label flag hides painted canvas labels.
83. `test_grid_math.cpp` — `labelsKeepSubMetreSteps` (93) — `PARTIAL`: directly assert `grid_label_decimals(0.5, 0.0)` and `grid_label_decimals(0.25, 100.25)`.
84. `test_grid_math.cpp` — `niceStepsAreUnchanged` (102) — `PARTIAL`: assert `nice_grid_step_meters` is identity across `0.5`..`1000`.
85. `test_feature_card_plate.cpp` — `cardIsALedgerOnThePageInPaper` (56) — `MISSING`: test paper-mode cream ledger palette colors in `ri_attr_card`.
86. `test_feature_card_plate.cpp` — `cardFollowsALiveSwitch` (131) — `PARTIAL`: assert paper-mode readability across tool switches.
87. `test_feature_card_plate.cpp` — `periodBandMarksTheRecordedPeriodInPaper` (153) — `MISSING`: test period badge swatch color difference in paper mode.
88. `test_feature_card_plate.cpp` — `controlPointShowsItsCoordinatesInPaper` (177) — `PARTIAL`: render-test paper-mode X/Y/Z coordinate card rows.
89. `test_feature_card_plate.cpp` — `nameNumberWindowReadsLikeTheBoardInPaper` (234) — `PARTIAL`: test paper-look styling and compact size of `dlg_feature_form`.
90. `test_feature_card_plate.cpp` — `noteIsSeveralLinesInPaper` (255) — `PARTIAL`: assert multiline note input height >= 56 px in record window.
91. `test_trench_plate.cpp` — `meterFillsToTheRatioAndMarksTheTarget` (29) — `MISSING`: test gauge fill width and 10%/2% target mark in `dlg_trench_generator_summary`.
92. `test_trench_plate.cpp` — `paperWindowCarriesTheMeterAndTheDial` (114) — `PARTIAL`: test paper-look colors and embedded meter+dial layout.
93. `test_survey_point_dialogs.cpp` — `surveyAreaSwatchesAreReadable` (129) — `MISSING`: test `dlg_survey_area_setup` swatch contrast.
94. `test_survey_point_dialogs.cpp` — `surveyAreaDialogIsCompact` (148) — `PARTIAL`: assert compact width/height bounds on `dlg_survey_area_setup`.
95. `test_survey_point_dialogs.cpp` — `surveyAreaStartIsTheThemedMainButton` (166) — `MISSING`: test accent fill on "그리기 시작" primary button.

## Blocked

Grouped by missing Rust feature (`NOT BUILT`):

- **Above-labels overlay & pixel cache (`test_above_labels.cpp`, `test_above_labels_names.cpp`, `test_layer_edits.cpp`, `test_inner_piece.cpp`)** — 7 tests:
  - `test_above_labels.cpp`: `overlayMatchesCanvas` (52), `cacheUsesCanvasPixelsAndRedrawsOnlyOnChange` (138), `heavyLayerDoesNotFreezeTheWindow` (188)
  - `test_above_labels_names.cpp`: `sameNameLabelledOnceWhenCanvasAlsoDrawsLayer` (83), `layerTreeBridgeKeepsOverlayLayersOffTheMap` (144)
  - `test_layer_edits.cpp`: `labelStackQueryDoesNotWrite` (131)
  - `test_inner_piece.cpp`: `shapeLiftedAboveLabelsIsStillPicked` (131)
- **Print layout composer & PDF export (`isComposedStudioSheet` / `test_checklist.cpp`, `test_checklist_state.cpp`)** — 15 tests:
  - `test_checklist.cpp`: `pdfViaLayout` (179), `fromProject_emptySeededSiteLocation_layoutExistsFalse` (209), `fromProject_composedUserSheet_layoutExistsTrue` (225), `test_uncomposed_user_sheet_rejected` (311), `test_dummy_blank_layer_rejected` (368), `test_autogenerated_template_not_passing_feature_plan` (414), `challenge1_isComposed_emptyVectorAndRaster` (465), `challenge2_isComposed_basemapCombinations` (502), `challenge3_isComposed_zeroAreaPolygon` (563), `challenge4_isComposed_nanInfExtentsNegativeScales` (648), `challenge5_isComposed_customNamedLayoutsAndMultipleMaps` (718), `challenge6_projectState_featurePlanWithoutActualFeatures` (785), `challenge7_projectState_siteLocationWithoutSurveyArea` (846), `challenge8_isComposed_autoGeneratedTemplateBypass` (911)
  - `test_checklist_state.cpp`: `sheetOfAnotherPlaceDoesNotPass` (249)
- **`ParcelClipboard` cadastral copy/paste (`test_parcel_copy.cpp`, `test_parcel_copy_keys.cpp`)** — 13 tests:
  - `test_parcel_copy.cpp`: `pasteKeepsTheSamePlaceAndAttributes` (23), `secondPasteAddsToTheSameCopyLayer` (44), `multipartParcelBecomesOneFeaturePerPart` (54), `copyStaysWhenItsSourceIsRemoved` (73), `surveyShapesAreNotCopied` (85), `pasteWithNothingCopiedSaysWhy` (98), `pasteKeepsTheSamePlaceAcrossCoordinateSystems` (108), `copySurvivesSaveAndReopen` (132), `pastingASavedMapTwiceStillSaves` (173), `pasteAgainAfterReopenStillSaves` (207)
  - `test_parcel_copy_keys.cpp`: `holdingCtrlVPastesOnce` (31), `ctrlCAndCtrlVInSelectToolPasteWhereTheParcelWas` (53), `ctrlCAndCtrlVDoNothingOutsideSelectTool` (79)
- **Interactive polygon erase, piece cut/fill & merge/vertex editing (`test_polygon_erase.cpp`, `test_polygon_pieces.cpp`, `test_inner_piece.cpp`, `test_layer_edits.cpp`, `test_buffer.cpp`, `test_editing_tools.cpp`)** — 26 tests:
  - `test_polygon_erase.cpp`: `oneSelectedShapeCutsTheShapeUnderItAndOneUndoRestoresBoth` (87), `selectingTheBigShapeOrALonelyShapeOnlyExplains` (125), `anotherLayerIsCutOnlyWhenExactlyOneShapeLiesUnder` (147), `severalSelectedShapesCutTheLargestAcrossLayers` (170), `referenceAndCadastralShapesAreNeverCutOrUsed` (196), `shapeCutInTwoStaysValidInASinglePolygonFile` (227), `shapeCutInTwoStaysOneShapeInAMultiPolygonLayer` (282), `movingTheFirstVertexKeepsAHoleClosed` (361)
  - `test_polygon_pieces.cpp`: `holeIsFoundAndFilled` (43), `innerPartIsRemovedOrCutOut` (59), `smallestPieceWinsWhenPartsAreStacked` (77), `separatePartsAndStalePiecesAreLeftAlone` (86), `islandGoesWithItsHole` (102), `pickListKeepsSurveyShapesAndFindsHolesCutBeforeSaving` (118)
  - `test_inner_piece.cpp`: `clickInsideHolePicksItAndDeleteFillsIt` (159), `clickOnOuterAreaKeepsWholeShapeBehaviour` (182), `innerPartCyclesToWholeShapeAndCutsOut` (194)
  - `test_layer_edits.cpp`: `mergeNeverTakesTheWholeLayer` (42), `mergeWarnsAboutLostAttributesAndCanBeUndone` (54), `mergeRefusesSeparatePolygonsInASinglePartLayer` (97), `sharedVertexMoveTouchesOnlyNeighbours` (108)
  - `test_buffer.cpp`: `moveFeatureVertex_updatesPolygonCorner` (199), `clipLayerByBoundary_clipsIntersectingFeatures` (241), `splitTwoOverlappingFeatures_splitsTargetPolygon` (316)
  - `test_editing_tools.cpp`: `easyDrawTakesCornersAtOnceAndEdgesAfterRest` (159)
- **`ringLayers_bakeIntoSurveyOnlyOnSave` (`test_buffer.cpp`)** — 1 test:
  - `test_buffer.cpp`: `ringLayers_bakeIntoSurveyOnlyOnSave` (166)
- **Trench interactive move/edit, GPKG writer, L-fill, grid-snap, `TrenchPlanCache` & `TrenchTerrain` (`test_dem_trench.cpp`, `test_trench_edit.cpp`, `test_trench_grid_snap.cpp`, `test_trench_handle.cpp`, `test_trench_lshape.cpp`, `test_trench_plan.cpp`, `test_trench_plate.cpp`, `test_trench_terrain.cpp`)** — 30 tests:
  - `test_dem_trench.cpp`: `writeGpkg_rollsBackFailedReplacement` (596), `startTrenchGrid_placesOnMapWithoutApplyClick` (741), `trenchWholeMove_isUndoableOnMouseRelease` (759), `clearLayerReplacesPreviousGrid` (775)
  - `test_trench_edit.cpp`: `rightClickDeleteIsUndoableAndNotSaved` (92), `singleMoveUsesSnappedClickAndUndoes` (112), `wholeMoveIsOneUndoStep` (133), `newUnsavedTrenchesCanBePickedAndDeleted` (158), `replaceIsOneUndoStepAndTracksHandEdits` (183), `replaceRefusesOtherCrsAndInvalidCells` (211), `dialogPreviewRunsOnceAfterInputSettles` (249)
  - `test_trench_grid_snap.cpp`: `turnedGridTurnsTheTrenches` (108), `coarseGridSaysWhyTenPercentDoesNotFit` (121), `planCacheKeepsGridPlansApart` (132)
  - `test_trench_handle.cpp`: `gridDragPutsTheCornerOnANode` (83)
  - `test_trench_lshape.cpp`: `tenPercentFillStaysInsideWithoutOverlap` (161), `tenPercentFillSpreadsOverTheArea` (202), `lTrenchSavesAsOnePolygon` (236), `planCacheKeepsStraightAndLApart` (270)
  - `test_trench_plan.cpp`: `clearLayerReportsUnreadableFile` (133), `planCacheReturnsGeneratorResult` (150), `placeGridKeepsHandEditsAndUndoesReplacement` (175), `unknownGridIsTreatedAsAdjusted` (255)
  - `test_trench_plate.cpp`: `lShapeChoiceLaysTheTenPercentPlan` (75), `gridLockLaysTheLsOnTheCoordinateGrid` (94)
  - `test_trench_terrain.cpp`: `contoursRisingNorthPointNorth` (45), `contoursRisingEastPointEast` (55), `linesWithoutElevationAreNotContours` (65)
- **Offline `TilePackService`, live Copernicus GeoTIFF & DEM dialog view-fit (`test_dem_trench.cpp`, `test_survey_point_dialogs.cpp`)** — 5 tests:
  - `test_dem_trench.cpp`: `tilePack_xmlUsesXyzTopOrigin` (426), `tilePack_tileCountMatchesWebMercator` (442), `tilePack_earlyCancellationPreservesExistingFile` (468), `demDownload_livePortable` (670)
  - `test_survey_point_dialogs.cpp`: `demDialogOffersTheViewFit` (226)
- **Rotated `CanvasGridMath` (`toGrid`/`toMap`/`snapToNode`) & grid-origin click (`test_grid_math.cpp`, `test_survey_point_dialogs.cpp`)** — 4 tests:
  - `test_grid_math.cpp`: `rotationIsClockwiseFromEast` (25), `gridFrameMatchesTrenchAzimuth` (37), `roundTripKeepsPoints` (54)
  - `test_survey_point_dialogs.cpp`: `gridOriginButtonMovesTheGrid` (264)
- **Survey contour safety/DXF/XLSX/color-bands, `SurveyPointArrange`, `ControlPointImporter`, `SurveyScopeClip` (`test_contour_safety.cpp`, `test_control_point_csv.cpp`, `test_survey_contour.cpp`, `test_survey_contour_clip.cpp`, `test_survey_point_arrange.cpp`, `test_survey_scope_clip.cpp`)** — 22 tests:
  - `test_contour_safety.cpp`: `wideDensePointsWidenTheCellInsteadOfFailing` (81), `longTrianglesAcrossAGapAreEmptied` (98), `clipReportsWhetherItReallyApplied` (161), `installKeepsThePreviousResultWhenASwapFails` (187), `aPreviousResultCanBeShownAgain` (219)
  - `test_control_point_csv.cpp`: `import_readsZAndSkipsReimportedPoints` (111)
  - `test_survey_contour.cpp`: `xlsxSuggestsAxisSwap` (39), `dxfLayerFiltersPointZ` (63), `tenthMetreLabelsHaveNoBinaryResidue` (131), `colorBandsMatchTheInterval` (175), `referenceGroupIsNotASubmitLayer` (200), `sourceFilesAreNotModified` (230)
  - `test_survey_contour_clip.cpp`: `contoursOutsideSurveyAreaAreRemoved` (17), `namedNorthEastColumnsSwap` (37), `dxfSkipsZeroElevationLayers` (64)
  - `test_survey_point_arrange.cpp`: `arrangeMatchesADirectRead` (43), `fileCrsIsTransformedToTheWorkCrs` (70), `outsidePointsSuggestTheOtherAxisOrder` (132), `issuesAreCapped` (151), `dxfLinesBecomeBreaklines` (166)
  - `test_survey_scope_clip.cpp`: `noSurveyLeavesFeatures` (16), `intersectingCopyKeepsNearLine` (68)
- **`SurveyFacts` mutators (`remember_check`, `mark_packaged`, `forget`) (`test_survey_facts.cpp`)** — 3 tests:
  - `test_survey_facts.cpp`: `rememberCheck_roundTrip` (83), `markPackaged_setsTime` (94), `forget_removesEntry` (180)
- **QGIS/UI-specific controls (`staleCount`, `applySnapConfig` WMS, `pinPlanarEllipsoid`, locked cadastral card, open-centre draw cursor) (`test_checklist_state.cpp`, `test_dem_trench.cpp`, `test_measure.cpp`, `test_feature_card_plate.cpp`, `test_sketch_measure.cpp`)** — 5 tests:
  - `test_checklist_state.cpp`: `staleCountStillCountsTheSurveyArea` (177)
  - `test_dem_trench.cpp`: `applySnapConfig_vertexAndSegmentNotWmsPromise` (719)
  - `test_measure.cpp`: `pinPlanarEllipsoidKeepsSavedState` (126)
  - `test_feature_card_plate.cpp`: `parcelReadsAsALockedRecordInPaper` (97)
  - `test_sketch_measure.cpp`: `drawCursorHasAnOpenCentre` (49)

## Could not determine

1. **`docs/senior/tools/check-tests.py` `slots()` inline-struct/brace truncation (6 files)**: `check-tests.py`'s `slots()` stops scanning `private slots:` on the first line starting with `}` or ending with `};` inside an inline test class or inline helper struct, undercounting C++ test slots on:
   - `test_feature_card_plate.cpp` (table: 7 rows; `check-tests.py` reports 2 slots)
   - `test_layer_edits.cpp` (table: 5 rows; `check-tests.py` reports 4 slots)
   - `test_parcel_copy_keys.cpp` (table: 5 rows; `check-tests.py` reports 4 slots)
   - `test_polygon_erase.cpp` (table: 12 rows; `check-tests.py` reports 7 slots)
   - `test_trench_plan.cpp` (table: 8 rows; `check-tests.py` reports 1 slot)
   - `test_trench_plate.cpp` (table: 6 rows; `check-tests.py` reports 3 slots)
   - Every real C++ slot in those 6 files is included in the per-file tables above.
2. **`test_checklist.cpp` (`exportAllowsWhenClean`, `surveyCreatesGpkg`, `fromProject_*` vs `demo_matches_cpp`)**: `demo_matches_cpp` (`crates/strata-app/src/checklist_state_tests.rs:71`) and `submission_package_writes_5179_manifest_and_non_blocking_readme` (`crates/strata-check-export/src/submit_package_tests.rs:10`) use different fixtures (`demo.gpkg` vs synthetic clean package), so `evaluatePassesHappyFixture` is classified `PARTIAL` rather than `PORTED`.
3. **`test_control_point_csv.cpp` (`headerless_numericFourthColumnIsZ`, `shortRowsAndMissingXY`)**: mapped `PARTIAL` to `test_csv_benchmark_importer_bom_and_skip_malformed_row_extracts_2_valid_points_and_1_warning` (`crates/strata-storage/src/cad_csv_io_tests.rs:26`), which exercises the unified CSV importer helper without parameterizing all C++ edge cases separately.
4. **`test_dem_trench.cpp` (`writeGpkg_rejectsInvalidCellsWithoutChangingExisting`)**: `cells_valid` validation rule itself is `PORTED` via `cells_valid_matches_writer_rules` (`crates/strata-geo/src/trench_plan_b_tests.rs:252`), while the GPKG writer rollback layer is `NOT BUILT`; classified `PARTIAL` (the companion `writeGpkg_rollsBackFailedReplacement` is `BLOCKED`).

## Counts

| File | Total | PORTED | PARTIAL | MISSING | BLOCKED |
|---|---:|---:|---:|---:|---:|
| test_above_labels.cpp | 3 | 0 | 0 | 0 | 3 |
| test_above_labels_names.cpp | 2 | 0 | 0 | 0 | 2 |
| test_buffer.cpp | 9 | 3 | 2 | 0 | 4 |
| test_checklist.cpp | 21 | 3 | 3 | 1 | 14 |
| test_checklist_rules.cpp | 7 | 0 | 1 | 6 | 0 |
| test_checklist_state.cpp | 8 | 0 | 3 | 3 | 2 |
| test_contour_safety.cpp | 7 | 0 | 2 | 0 | 5 |
| test_control_point_csv.cpp | 7 | 0 | 3 | 3 | 1 |
| test_dem_trench.cpp | 31 | 16 | 6 | 0 | 9 |
| test_editing_tools.cpp | 6 | 1 | 4 | 0 | 1 |
| test_export_survey_areas.cpp | 10 | 3 | 4 | 3 | 0 |
| test_feature_card.cpp | 10 | 9 | 1 | 0 | 0 |
| test_feature_card_fields.cpp | 4 | 1 | 3 | 0 | 0 |
| test_feature_card_plate.cpp | 7 | 0 | 4 | 2 | 1 |
| test_feature_records.cpp | 9 | 8 | 1 | 0 | 0 |
| test_grid_math.cpp | 7 | 1 | 3 | 0 | 3 |
| test_inner_piece.cpp | 7 | 0 | 2 | 1 | 4 |
| test_layer_edits.cpp | 5 | 0 | 0 | 0 | 5 |
| test_measure.cpp | 6 | 2 | 3 | 0 | 1 |
| test_parcel_copy.cpp | 10 | 0 | 0 | 0 | 10 |
| test_parcel_copy_keys.cpp | 5 | 1 | 1 | 0 | 3 |
| test_polygon_erase.cpp | 12 | 3 | 1 | 0 | 8 |
| test_polygon_pieces.cpp | 6 | 0 | 0 | 0 | 6 |
| test_sketch_measure.cpp | 6 | 3 | 2 | 0 | 1 |
| test_survey_contour.cpp | 11 | 0 | 1 | 4 | 6 |
| test_survey_contour_clip.cpp | 3 | 0 | 0 | 0 | 3 |
| test_survey_facts.cpp | 10 | 4 | 1 | 2 | 3 |
| test_survey_point_arrange.cpp | 6 | 0 | 1 | 0 | 5 |
| test_survey_point_dialogs.cpp | 10 | 2 | 4 | 2 | 2 |
| test_survey_scope_clip.cpp | 3 | 0 | 1 | 0 | 2 |
| test_trench_edit.cpp | 9 | 0 | 2 | 0 | 7 |
| test_trench_grid_snap.cpp | 4 | 0 | 1 | 0 | 3 |
| test_trench_handle.cpp | 3 | 0 | 2 | 0 | 1 |
| test_trench_lshape.cpp | 8 | 2 | 2 | 0 | 4 |
| test_trench_plan.cpp | 8 | 4 | 0 | 0 | 4 |
| test_trench_plate.cpp | 6 | 0 | 2 | 2 | 2 |
| test_trench_terrain.cpp | 3 | 0 | 0 | 0 | 3 |
| **Total** | **289** | **66** | **66** | **29** | **128** |
