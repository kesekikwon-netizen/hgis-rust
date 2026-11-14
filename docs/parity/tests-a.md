# C++ -> Rust Test Parity Audit (Part A: Saving, Storage, and Core Lifecycle)

Read-only audit of the 19 C++ QtTest suites under `target/cpp-ref/tests` against the Rust workspace tests on branch `ag/t1-a`.
Each row maps one C++ `private slots:` test function (with `_data()` row counts noted in `note`).
Status definitions:
- `PORTED`: Rust has a test checking the same behaviour.
- `PARTIAL`: Rust tests part of the behaviour; uncovered aspects are stated in `note`.
- `MISSING`: Underlying Rust code exists, but no Rust test covers this scenario.
- `BLOCKED`: Underlying feature is not built yet in Rust.

---

## test_blocking_task.cpp

| # | C++ test function | C++ line | what it checks | Rust test | Rust file:line | Rust code under test | status | note |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 1 | cancel_keepsTheDialogUntilTheWorkStops | 62 | Clicking Cancel on blocking task dialog keeps dialog open until background worker actually stops | — | — | NOT BUILT | BLOCKED | BlockingTaskDialog modal worker runner not built |
| 2 | escape_cancelsAndKeepsTheDialog | 77 | Pressing Escape cancels blocking task without prematurely closing dialog before work stops | — | — | NOT BUILT | BLOCKED | BlockingTaskDialog modal Escape handling not built |

## test_catch_log.cpp

| # | C++ test function | C++ line | what it checks | Rust test | Rust file:line | Rust code under test | status | note |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 1 | defaultMaxBytes_is10MiB | 236 | Session log default maximum size is 10 MiB | — | — | NOT BUILT | BLOCKED | SessionLog rotating file logger not built |
| 2 | sessionLog_writesLine | 246 | Session log writes formatted line to log file on disk | — | — | NOT BUILT | BLOCKED | SessionLog file writer not built |
| 3 | sessionLog_rotatesWhenOverMax | 261 | Session log rotates file when size exceeds max bytes | — | — | NOT BUILT | BLOCKED | SessionLog size rotation not built |
| 4 | srcCatchHandlers_allLog | 279 | Static check that every C++ catch block logs through session log or crash log | — | — | NOT BUILT | BLOCKED | C++-specific source catch-handler linter not applicable / SessionLog not built |
| 5 | srcAppNestedEventLoops_onlyAllowlisted | 286 | Static check that nested QEventLoop calls stay within explicit allowlist | — | — | NOT BUILT | BLOCKED | Qt QEventLoop static audit not applicable in egui |

## test_edit_buffer_summary.cpp

| # | C++ test function | C++ line | what it checks | Rust test | Rust file:line | Rust code under test | status | note |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 1 | countsUniqueFeatureIds | 47 | Multiple edits on same feature ID count once in dirty feature count | unsaved_chip_counts_edited_features | crates/strata-app/src/app_state_draw_commit_tests.rs:192 | crates/strata-app/src/app_state_status_bar.rs | PARTIAL | Counts modified/deleted baseline diffs in status chip; does not test EditBufferSummary aggregating add+geom+attr edits to one FID |
| 2 | addedChangedDeleted | 66 | Edit buffer summary separates added, changed, and deleted FID counts | — | — | NOT BUILT | BLOCKED | EditBufferSummary added/changed/deleted breakdown struct not built |
| 3 | ignoresLayersWithoutEditBuffer | 87 | Non-editing and raster layers contribute zero edits to summary | — | — | NOT BUILT | BLOCKED | EditBufferSummary multi-layer filter not built |
| 4 | projectDirtyFlagPassesThrough | 108 | Project-level dirty flag surfaces even when buffer edit count is zero | ribbon_save_writes_the_added_feature_and_clears_the_star | crates/strata-app/src/app_state_save_tests.rs:45 | crates/strata-app/src/app_state_status_bar.rs:103 | PARTIAL | Checks dirty indicator sets on edit and clears on save; lacks projectDirty=true with zero buffer edits passthrough test |
| 5 | tenThousandEditsUnderTenMs | 119 | Summarizing 10,000 buffered edits completes in under 10 ms | — | — | crates/strata-storage/src/edit_buffer.rs:55 | MISSING | EditBuffer exists without a 10k-edit summary benchmark test |

## test_editing_history.cpp

| # | C++ test function | C++ line | what it checks | Rust test | Rust file:line | Rust code under test | status | note |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 1 | stampsOrderEditsAcrossLayers | 42 | Editing history assigns monotonic sequence stamps across multiple layers | — | — | NOT BUILT | BLOCKED | EditingHistory global sequence stamp ledger not built |
| 2 | commandStateFollowsUndoCommitAndDiscard | 66 | Command states transition between Current, Undone, Committed, and Discarded | undo_and_redo_after_drawing_follow_the_layer_buffer | crates/strata-app/src/app_state_draw_commit_tests.rs:155 | crates/strata-storage/src/edit_buffer.rs:83 | PARTIAL | Tests undo/redo step restoration on layer buffer; lacks Current/Undone/Committed/Discarded state machine |
| 3 | saveThatDropsTheBufferKeepsWrittenCommandsCommitted | 97 | Save dropping edit buffers marks previously active commands as Committed | — | — | NOT BUILT | BLOCKED | EditingHistory markSavedCommandsCommitted state retention not built |
| 4 | redoPrefersTheLastUndone | 130 | Redo picks the most recently undone command across multiple layers | — | — | NOT BUILT | BLOCKED | Multi-layer last-undone redo arbiter not built (edit_buffer.rs:93 is per-table) |
| 5 | bowTieKeepsLargestPartAndSaysSo | 150 | Self-intersecting bow-tie polygon keeps largest valid part and emits notice | bow_tie_keeps_the_largest_part_and_tells_the_dropped_one | crates/strata-app/src/edit_geometry_repair_tests.rs:50 | crates/strata-app/src/edit_geometry_repair.rs:36 | PORTED | Exact C++ bow-tie repair and dropped-part notice match |
| 6 | nearlyStraightRingIsFlagged | 169 | Collinear / zero-area sketch is flagged as degenerate and rejected | nearly_collinear_sketch_is_degenerate | crates/strata-app/src/edit_geometry_repair_tests.rs:68 | crates/strata-app/src/edit_geometry_repair.rs:36 | PORTED | Matches nearly straight ring rejection |
| 7 | plainSquareHasNoNotice | 177 | Valid simple polygon closes cleanly with empty notice | plain_sketch_is_closed_without_notice | crates/strata-app/src/edit_geometry_repair_tests.rs:13 | crates/strata-app/src/edit_geometry_repair.rs:36 | PORTED | Matches clean square close without warning |

## test_file_cleanup.cpp

| # | C++ test function | C++ line | what it checks | Rust test | Rust file:line | Rust code under test | status | note |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 1 | removeWhenFree_removesAtOnceWhenNobodyHoldsIt | 38 | Deferred file cleanup deletes unheld temporary file immediately | — | — | NOT BUILT | BLOCKED | FileCleanup::removeWhenFree deferred deletion queue not built |
| 2 | removeWhenFree_removesLaterOnceTheReaderCloses | 46 | Locked file is queued and deleted later after reader handle releases | — | — | NOT BUILT | BLOCKED | FileCleanup retry/sweep after handle close not built |

## test_gdal_error_log.cpp

| # | C++ test function | C++ line | what it checks | Rust test | Rust file:line | Rust code under test | status | note |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 1 | dumpHint_namesCrashDump | 53 | Fatal GDAL dump hint names crash dump folder and log file | — | — | NOT BUILT | BLOCKED | GDAL CPL error handler & crash dump hint not built |
| 2 | onlyFailuresAreRecorded | 60 | Debug/None CE levels are ignored while Warnings/Failures are recorded | — | — | NOT BUILT | BLOCKED | gdal_error_log CPL callback filter not built |
| 3 | failuresReachTheSessionLogAndRepeatsFold | 71 | GDAL failures write to session log and fold consecutive repeats | — | — | NOT BUILT | BLOCKED | gdal_error_log repeat folding to SessionLog not built |
| 4 | failuresAreNotRepeatedOnTheTerminal | 91 | GDAL errors installed with session logger suppress duplicate stderr noise | — | — | NOT BUILT | BLOCKED | gdal_error_log terminal suppression not built |
| 5 | uninstallRestoresPreviousAndInstallWorksAgain | 109 | Uninstall restores previous CPL error handler and reinstall works cleanly | — | — | NOT BUILT | BLOCKED | gdal_error_log CPL handler push/pop lifecycle not built |
| 6 | nestedCplErrorFromPreviousDoesNotRecurse | 130 | Nested CPLError triggered inside forwarded handler does not recurse | — | — | NOT BUILT | BLOCKED | gdal_error_log re-entrancy guard not built |

## test_georef_backup.cpp

| # | C++ test function | C++ line | what it checks | Rust test | Rust file:line | Rust code under test | status | note |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 1 | plainScan_moveThenRestore_leavesNoTrace | 69 | Backing up and restoring non-georeferenced scan leaves original byte-identical | — | — | NOT BUILT | BLOCKED | GeorefBackup raster move/restore workflow not built |
| 2 | geotiff_restoreBringsBackEmbeddedCoordinates | 121 | Restoring GeoTIFF backup recovers embedded transform and SRS | — | — | NOT BUILT | BLOCKED | GeorefBackup GeoTIFF tag backup/restore not built |
| 3 | pamWithGeoref_isBackedUpAndRestored | 195 | Sidecar .aux.xml PAM georef file is backed up and restored cleanly | — | — | NOT BUILT | BLOCKED | GeorefBackup PAM sidecar handling not built |
| 4 | uniqueOutputPath_neverReusesAnExistingFile | 203 | Unique georef output path increments suffix rather than clobbering existing file | — | — | NOT BUILT | BLOCKED | GeorefBackup unique output path allocator not built |
| 5 | backupRoot_isInsideTheSurveyFolder | 216 | Backup directory resolves inside the active survey folder | — | — | NOT BUILT | BLOCKED | GeorefBackup survey-folder backup root not built |
| 6 | readPixelWindow_readsOriginalPixels | 225 | Pixel window reader extracts unmodified raster samples for control point picking | — | — | NOT BUILT | BLOCKED | Raster pixel-window reader for georeferencing not built |

## test_layer_tree_undo.cpp

| # | C++ test function | C++ line | what it checks | Rust test | Rust file:line | Rust code under test | status | note |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 1 | restorePutsOrderAndChecksBack | 30 | Layer tree undo restores exact node ordering and check states | — | — | NOT BUILT | BLOCKED | LayerTreeUndo tree snapshot/restore stack not built |
| 2 | onlyTheUsersChangesBecomeUndoSteps | 57 | Programmatic layer tree edits outside user interaction do not push undo steps | — | — | NOT BUILT | BLOCKED | LayerTreeUndo user-gesture filter not built |
| 3 | layerChangesMergeIntoOneRefresh | 92 | Rapid consecutive layer tree mutations coalesce into one refresh signal | — | — | NOT BUILT | BLOCKED | LayerTreeUndo coalesced refresh timer not built |

## test_log_session.cpp

| # | C++ test function | C++ line | what it checks | Rust test | Rust file:line | Rust code under test | status | note |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 1 | sessionLog_keepsSeveralRotations | 61 | Multiple rotated log files are retained up to rotation limit | — | — | NOT BUILT | BLOCKED | SessionLog multi-generation rotation retention not built |
| 2 | sessionLog_headerNamesProcess | 103 | New log file opens with header identifying app process and version | — | — | NOT BUILT | BLOCKED | SessionLog startup header not built |
| 3 | sessionLog_masksKeysAndAccounts | 114 | API keys and account passwords are masked before reaching logs | vworld_key_load_priority_and_secret_masking | crates/strata-secrets/src/vworld.rs:94 | crates/strata-secrets/src/mask.rs:5 | PARTIAL | Tests mask::redact helper on VWorld secrets; SessionLog file writing itself not built |
| 4 | secretMask_keepsOrdinaryText | 144 | Redaction filter preserves ordinary Korean text, paths, and URLs | — | — | crates/strata-secrets/src/mask.rs:5 | MISSING | mask::redact exists; no standalone test verifies ordinary Korean/path text passes unmasked |
| 5 | portableBundle_logsBesideItsConfig | 155 | Portable bundle places session logs beside portable config file | — | — | NOT BUILT | BLOCKED | Portable bundle log path resolution not built |
| 6 | logExcept_namesSourceAndMessage | 172 | Exception logging helper includes call site source and message text | — | — | NOT BUILT | BLOCKED | SessionLog exception logging helper not built |

## test_recent.cpp

| # | C++ test function | C++ line | what it checks | Rust test | Rust file:line | Rust code under test | status | note |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 1 | recentSurveyFollowsUsbDriveLetter | 27 | Opening from changed USB drive letter locates survey via drive/ID metadata | — | — | NOT BUILT | BLOCKED | Portable USB drive-letter tracking not built in recent_surveys.rs |
| 2 | rememberPutsNewestFirstAndDropsMissing | 51 | Remembering moves newest entry to top, preserves INI keys, caps at 10, drops missing | test_mru_max_limit_12_to_10_prune_3_missing_to_7_and_verify_gpkg_magic_bytes | crates/strata-storage/src/project_recent_tests.rs:51 | crates/strata-storage/src/recent_surveys.rs:144 | PORTED | Also covered by remember_rewrites_only_the_items_line (recent_surveys_tests.rs:28) |
| 3 | remember_keepsOfflineSurveysForLater | 74 | Opening another survey keeps offline paths stored until manually removed or pruned | load_all_keeps_missing_and_last_path_skips_them | crates/strata-storage/src/recent_surveys_tests.rs:110 | crates/strata-storage/src/recent_surveys.rs:100 | PARTIAL | Checks load_all retains missing paths and last_path skips them; split across remember_ignores_missing_files without USB offline re-appear flow |
| 4 | forget_keepsOtherOfflineSurveys | 100 | Forgetting one entry leaves other offline surveys intact | forgetting_a_row_removes_only_that_survey | crates/strata-app/src/app_state_recent_tests.rs:115 | crates/strata-app/src/app_state_recent.rs:117 | PORTED | Matches removing one row while preserving remaining entries |
| 5 | remember_sameSurveyDifferentCaseIsOneEntry | 118 | Same Windows path with different case or slashes deduplicates cleanly | case_and_separator_variants_are_one_survey | crates/strata-storage/src/recent_surveys_tests.rs:78 | crates/strata-storage/src/recent_surveys.rs:144 | PORTED | Exact Windows path-normalization deduplication test |
| 6 | remember_tabInNameDoesNotShiftFields | 135 | Tabs in filenames do not corrupt portable metadata fields | — | — | NOT BUILT | BLOCKED | Tab-delimited portable-drive-index format not used (Rust stores plain paths) |
| 7 | forget_removesSurveyShownOnNewDriveLetter | 151 | Forgetting a survey removes entry even when shown under new USB drive letter | — | — | NOT BUILT | BLOCKED | USB drive-letter matching on forget not built |
| 8 | takeSkipAutoRestore_clearsOneShot | 177 | One-shot skip-auto-restore flag reads true once and resets | — | — | NOT BUILT | BLOCKED | Auto-restore removed per AGENTS.md startup-on-home design |
| 9 | forgetRemovesPath | 187 | Forgetting last entry writes @Invalid() matching QSettings empty array | forget_removes_one_and_empty_lists_store_invalid | crates/strata-storage/src/recent_surveys_tests.rs:127 | crates/strata-storage/src/recent_surveys.rs:210 | PORTED | Matches removing one path and @Invalid() empty storage |
| 10 | lastPath_isNewestRemembered | 199 | lastPath returns the most recently remembered existing survey | remember_rewrites_only_the_items_line | crates/strata-storage/src/recent_surveys_tests.rs:28 | crates/strata-storage/src/recent_surveys.rs:182 | PORTED | Checked together with load_all_keeps_missing_and_last_path_skips_them (:110) |
| 11 | bootStaysOnHome_doesNotAutoOpenLastSurvey | 212 | App boot always starts on Home screen regardless of recent survey list | startup_has_no_survey_data_and_no_user_settings | crates/strata-app/src/app_state_recent_tests.rs:42 | crates/strata-app/src/app_state_recent.rs:25 | PORTED | Verifies cold boot starts on Home with zero loaded survey data |
| 12 | closeEvent_asksBeforeDiscardingUnsavedWork | 230 | Window close prompts Save/Discard/Cancel when edits are unsaved and honors choice | test_d7_close_prompt_korean_particle_and_control_point_layout | crates/strata-app/src/dialogs_modal_tests.rs:243 | crates/strata-app/src/modal_controller_impl.rs:84 | PARTIAL | Verifies close prompt text/particles; does not simulate OS close event Cancel/Discard/Save-then-close state transitions |
| 13 | captureTool_alwaysStartsNewShape | 283 | Clicking near existing feature vertex with capture tool starts new shape | clicking_saved_line_vertex_starts_new_sketch_without_touching_existing_line | crates/strata-app/src/app_state_draw_commit_line_tests.rs:197 | crates/strata-app/src/draw_sketch.rs:67 | PORTED | Verifies starting draw on existing vertex leaves saved shape untouched |

## test_save_handles.cpp

| # | C++ test function | C++ line | what it checks | Rust test | Rust file:line | Rust code under test | status | note |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 1 | removedLayerHeldForUndo_doesNotBlockTheSave | 57 | Saving succeeds when removed vector layer wrapper is still held on undo stack | a_removed_layer_is_not_saved_until_restored | crates/strata-app/src/app_state_layer_remove_tests.rs:54 | crates/strata-app/src/app_state_layer_remove.rs:27 | PARTIAL | Tests layer removal and save exclusion/restore; does not hold open Windows SQLite reader handle in undo stack |
| 2 | removedLayerWithUnsavedEdits_doesNotBlockTheSave | 90 | Saving succeeds when removed layer held for undo also carried unsaved edits | remove_and_undo_keep_the_unsaved_shape | crates/strata-app/src/app_state_layer_remove_tests.rs:35 | crates/strata-app/src/app_state_layer_remove.rs:27 | PARTIAL | Tests undo restores unsaved shape cleanly; lacks Windows GPKG file-lock contention test |
| 3 | singleLayerReadThenSaveWithoutEdits_replacesInPlace | 118 | Reading single layer cleanly then saving without edits replaces target without sharing lock error | delete_mode_leaves_folder_and_bytes_unchanged_for_all_readers | crates/strata-storage/src/sqlite_read_tests.rs:145 | crates/strata-storage/src/sqlite_read.rs:107 | PARTIAL | Verifies read-only SQLite access leaves original unblocked; lacks end-to-end reopen then no-op save in-place replacement test |
| 4 | readerAwaitingDeletion_doesNotBlockTheSave | 136 | Old reader queued in FileCleanup does not block saving a new generation | — | — | NOT BUILT | BLOCKED | FileCleanup::removeWhenFree reader release queue not built |

## test_save_open.cpp

| # | C++ test function | C++ line | what it checks | Rust test | Rust file:line | Rust code under test | status | note |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 1 | compactLayerListKeepsHierarchyAndMapLabelSize | 573 | Layer list keeps parent/child hierarchy intact at compact window width and map label size | legend_layout_follows_the_cpp_measurements_and_bands_sit_on_the_first_row | crates/strata-app/src/gui_tab_home_open_view_tests.rs:70 | crates/strata-app/src/left_layer_panel.rs | PARTIAL | Checks C++ legend measurements and row order; omits compact-width splitter/label-size assertion |
| 2 | layerInformationControlsIntegrateWithMapAndDrawingStudio | 648 | Layer color/opacity/label toggles apply live across layer tree, map, and Drawing Studio | size_and_transparency_are_per_layer | crates/strata-app/src/app_state_layer_menu_tests.rs:52 | crates/strata-app/src/app_state_layer_menu.rs:64 | PARTIAL | 3 data rows in C++; Rust tests per-layer size/opacity state, not live sync across Map and Drawing Studio |
| 3 | geoTiffButtonOnlyEnabledOnMapTab | 838 | Ribbon GeoTIFF button enabled only when Map tab is active | — | — | NOT BUILT | BLOCKED | GeoTIFF export (지시 R7) intentionally deferred/disabled across all tabs (verified in geotiff_chip_is_deferred_r7_and_click_does_nothing at gui_ribbon_click_b_tests.rs:8) |
| 4 | windowGeometryFitsLogicalMonitors | 864 | Window frame geometry clamps inside logical monitor bounds across configs | — | — | NOT BUILT | BLOCKED | 6 data rows in C++; Qt multi-monitor window clamping not built |
| 5 | mainWindowFitsAvailableScreenOnShow | 873 | First show fits main window cleanly inside available screen rect | — | — | NOT BUILT | BLOCKED | Show-event QScreen clamp not built in egui runner |
| 6 | everyTabFitsA1920ScreenInPaper | 888 | Switching across Home/Map/Drawing/etc fits within 1920px paper viewport | test_u1_ux_layout_spec_1600x900_skeleton_ribbon_and_3_column_split | crates/strata-app/src/gui_shell_tests_a.rs:162 | crates/strata-app/src/gui_ribbon_consts.rs:51 | PARTIAL | Checks 1600x900 skeleton layout and 3-column split; does not step every tab on 1920px screen |
| 7 | monitorFitHomeMapDrawing | 927 | Home/Map/Drawing tabs fit multi-monitor DPR screen sizes without clipping | — | — | NOT BUILT | BLOCKED | 4 data rows in C++; headless DPR monitor-fit harness not built |
| 8 | monitorFitDialogs | 1012 | Modals stay inside available screen geometry across DPR/monitor configs | test_d24_stage_rail_and_responsive_clamp | crates/strata-app/src/dialogs_download_print_tests.rs:10 | crates/strata-app/src/dialogs_download_print.rs | PARTIAL | 4 data rows in C++; covers responsive clamp for download/print modals only |
| 9 | homePageShowsStrataHeroWithoutOpeningASurvey | 1043 | Home tab renders Strata hero banner without requiring open survey | startup_opens_home_tab_and_drawing_ribbon_tool_switches_to_map_tab | crates/strata-app/src/app_state_tests.rs:50 | crates/strata-app/src/app_state_lifecycle.rs | PARTIAL | Checks boot stays on Home without survey; does not inspect rendered Home hero widgets |
| 10 | drawingToolsGetTheirOwnRowBelowTheRibbon | 1085 | Clicking Drawing chip toggles separate draw sub-toolbar row directly below ribbon | draw_button_opens_the_row_without_starting_a_tool_and_closes_it_again | crates/strata-app/src/app_state_draw_row.rs:48 | crates/strata-app/src/gui_ribbon_draw_row.rs:126 | PARTIAL | Tests toggle state open/close; does not check rendered Y-coordinates below ribbon bar |
| 11 | firstStartIsMaximizedAndAChosenSizeIsKept | 1140 | Cold start opens maximized and preserves user-chosen window geometry thereafter | — | — | NOT BUILT | BLOCKED | Window maximized/size settings persistence not built in user_settings.rs |
| 12 | filesBoxStartsOpenAndKeepsTheChosenState | 1153 | Home Files box starts expanded and remembers user collapse/expand state | — | — | NOT BUILT | BLOCKED | Home Files group collapse state INI persistence not built |
| 13 | narrowWindowKeepsSearchOnTheRibbonRow | 1199 | Shrinking window width keeps ribbon search bar inline on ribbon row | test_ribbon_compact_mode_1024px_and_exclusive_tool_switch_clears_rubberband | crates/strata-app/src/ribbon_tests.rs:14 | crates/strata-app/src/gui_ribbon_consts.rs:51 | PARTIAL | Tests 1024px compact ribbon layout; omits inline search widget rect assertion |
| 14 | ribbonGroupsMatchTheirPurpose | 1236 | Ribbon groups contain expected actions in exact C++ order | ribbon_groups_hold_cpp_buttons_in_cpp_order | crates/strata-app/src/gui_shell_tests_a.rs:243 | crates/strata-app/src/gui_ribbon_consts.rs:51 | PORTED | Verifies every ribbon group label and chip order |
| 15 | narrowWindowKeepsDrawingAndPrintVisible | 1266 | Compact ribbon keeps Drawing and Print chips visible without overflow | ribbon_group_layout_uses_unreduced_chip_base_widths | crates/strata-app/src/gui_shell_tests_a.rs:261 | crates/strata-app/src/gui_ribbon_consts.rs:51 | PARTIAL | Checks group layout widths; does not render narrow window to assert visible Drawing/Print rects |
| 16 | subToolbarFitsAt1904WhileSketching | 1332 | Drawing sub-toolbar fits inside 1904px width while actively sketching | draw_row_buttons_work_and_line_sketch_paints_open_polyline_without_fill | crates/strata-app/src/gui_tab_map_capture_line_tests.rs:194 | crates/strata-app/src/gui_ribbon_draw_row.rs:126 | PARTIAL | Exercises draw row buttons during line sketch without checking 1904px width clipping |
| 17 | unsavedSaveChipLabelTurnsBoldAndBack | 1422 | Save ribbon chip bolds when dirty and returns to normal weight on save | ribbon_save_writes_the_added_feature_and_clears_the_star | crates/strata-app/src/app_state_save_tests.rs:45 | crates/strata-app/src/app_state_status_bar.rs:103 | PARTIAL | Tests save star/dirty flag toggle; does not assert Save chip bold font state |
| 18 | recordToolsWaitForASurvey | 1456 | Record-editing tools remain disabled until a survey is open | without_a_survey_nothing_is_targeted | crates/strata-app/src/app_state_draw_entry_tests.rs:115 | crates/strata-app/src/app_state_draw_entry.rs:60 | PARTIAL | Checks draw target stays None with no survey; does not assert all ribbon tool disabled states |
| 19 | ribbonButtonsAllHaveDifferentIcons | 1484 | Every ribbon button uses a distinct icon path | — | — | crates/strata-app/src/svg_path_paint.rs:82 | MISSING | SVG icon paths and ribbon constants exist without icon-uniqueness test |
| 20 | mapControlsZoomTheCanvasAndShowAScaleBar | 1517 | Map overlay +/- buttons change zoom scale bar reading | overlay_plus_and_minus_buttons_zoom_by_cpp_wheel_factor | crates/strata-app/src/map_wheel_zoom_tests.rs:239 | crates/strata-app/src/gui_tab_map_geom.rs:20 | PARTIAL | Checks zoom factor (scale bar tested separately at gui_shell_tests_b.rs:172); not end-to-end scale bar update after zoom click |
| 21 | topographicActualWindowRemainsStableAfterCompletion | 1561 | Topographic downloader window stays visually stable when background job finishes | test_topo_browser_compact_toggle_stage_progress_and_import_fallback | crates/strata-app/src/dialogs_modal_tests_b.rs:51 | crates/strata-app/src/dialogs_download_print.rs | PARTIAL | Checks compact toggle and fallback status; omits async job completion window geometry check |
| 22 | topographicDownloadKeepsMapVisibleAndUsesSurveyFolder | 1699 | Non-modal topographic downloader keeps map interactive and writes inside survey folder | test_r14_topographic_import_reads_the_real_folder_instead_of_fixed_rows | crates/strata-app/src/dialogs_modal_tests_c.rs:68 | crates/strata-app/src/dialogs_download_print.rs | PARTIAL | Verifies reading real topographic folder beside survey; does not run non-modal download keeping map interactive |
| 23 | provinceChipMovesMapWithoutLoadingLayers | 1778 | Clicking region locator chips pans map without adding extra layers | test_d14_region_locator_and_canvas_grid_band | crates/strata-app/src/gui_shell_tests_b.rs:55 | crates/strata-app/src/gui_shell_tests_b.rs:55 | PARTIAL | 2 data rows in C++; checks region locator chips exist and pan, without asserting layer count stays constant |
| 24 | drawingStudio_legendStaysWhereTheUserPutsItOutsideThePaper | 1812 | Manual off-page legend position is preserved across sheet layout updates | paper_size_and_orientation_change_keeps_decorations_in_bounds_and_preserves_off_page_legend | crates/strata-layout/src/sheet_composer_tests.rs:11 | crates/strata-layout/src/sheet_drawing.rs:125 | PORTED | Preserves user off-page legend placement on paper changes |
| 25 | drawingStudio_decorationsMoveAndShrink | 1878 | Legend/scalebar/north-arrow decorations clamp when paper shrinks or reorients | paper_change_puts_chrome_back_in_bottom_strip_and_recovers_orphaned_legend | crates/strata-layout/src/sheet_composer_tests.rs:26 | crates/strata-layout/src/sheet_drawing.rs:125 | PARTIAL | Checks bottom strip recovery and bounds clamp on paper resize; no interactive drag move/resize test |
| 26 | drawingStudio_northArrowRendersOnOpen | 2019 | Opening Drawing Studio renders non-empty north arrow decoration | test_north_arrow_grid_convergence_and_legend_sort_and_title_block_elide | crates/strata-layout/src/sheet_composer_tests.rs:72 | crates/strata-layout/src/sheet_layout.rs:266 | PARTIAL | Tests grid convergence rotation math; does not open Drawing Studio tab and verify rendered north arrow painter output |
| 27 | drawingStudio_fieldPageGrowsA4LandscapeByOneCentimetre | 2060 | Field sheet mode expands A4 landscape map frame by 10 mm and restores cleanly | legacy_a4_landscape_detection_and_field_page_grow_and_reopen_bounds | crates/strata-layout/src/sheet_drawing_tests.rs:40 | crates/strata-layout/src/sheet_drawing.rs:119 | PORTED | Matches A4 landscape field page +1cm expansion and reopen clamp |
| 28 | drawingStudio_heritageRefreshRequestsCoalesceAndReuseOnRevisit | 2125 | Concurrent heritage-map refresh requests coalesce and reuse cached image on revisit | — | — | NOT BUILT | BLOCKED | Drawing Studio offscreen heritage raster cache & request coalescing not built |
| 29 | drawingStudioAboveLabelsMapDrawsGeometryWithoutDuplicateNumberLabels | 2434 | Above-labels drawing layer omits duplicate number text over map geometry | — | — | NOT BUILT | BLOCKED | Drawing Studio split above-labels overlay rendering not built |
| 30 | drawingStudio_centerLayerMovesAboveLabelsMapToo | 2540 | Center-on-layer shifts both main map item and synced above-labels item | sheet_map_centre_is_the_map_view_centre | crates/strata-app/src/gui_tab_drawing_frame_tests.rs:24 | crates/strata-app/src/gui_tab_drawing_frame.rs:92 | PARTIAL | Tests map frame center sync; lacks separate above-labels overlay item offset check |
| 31 | drawingStudio_largeScaleChipsApplyToMapAndInput | 2606 | Clicking scale preset chip updates both drawing map frame scale and scale edit field | test_drawing_studio_dpi_validation_72_200_1200_and_preserve_scale_sync | crates/strata-app/src/tab_drawing.rs:125 | crates/strata-layout/src/sheet_drawing.rs:119 | PARTIAL | Tests DPI validation and scale sync logic; no click test driving large-scale preset chips into both frame and input box |
| 32 | drawingInspectorUsesHeightWithoutOverlapping | 2667 | Drawing Studio right inspector arranges cards without vertical overlap across heights | test_d10_sheet_bottom_stops_above_seven_button_toolbar | crates/strata-app/src/gui_tab_drawing_tests.rs:46 | crates/strata-app/src/gui_tab_drawing_frame.rs:73 | PARTIAL | 3 data rows in C++; checks sheet bottom vs toolbar only, not inspector panel widget overlap across viewport heights |
| 33 | layerContextMenu_matchesLayerKind | 2768 | Layer right-click context menu offers exact actions for each layer kind | test_d21_map_and_layer_context_menus | crates/strata-app/src/gui_shell_tests_a.rs:76 | crates/strata-app/src/layer_menu_model.rs:28 | PORTED | 10 data rows in C++; also covered by layer_menu_model_tests.rs:154 |
| 34 | changingLabelFontKeepsEveryLayerAndItsVisibility | 2868 | Changing label font preserves layer list order and visibility checkboxes | rename_keeps_the_layer_settings | crates/strata-app/src/app_state_layer_menu_tests.rs:79 | crates/strata-app/src/app_state_layer_menu.rs:64 | PARTIAL | Checks settings survive layer rename (label area at :38); global label font change UI not built/tested |
| 35 | trenchPresetRespectsAreaAndDimensions | 2935 | Trench preset populates area, dimensions, and trench count accurately | trench_1x1_is_40m2 | crates/strata-geo/src/trench_plan_a_tests.rs:24 | crates/strata-geo/src/trench_generate.rs:25 | PARTIAL | 2 data rows in C++; geo grid/area math tested cleanly, but dialog preset radio/spinbox UI synchronization untested |
| 36 | layerDeleteKeyPreservesSourceAndUndoRestoresPendingEdits | 3024 | Delete key removes layer without deleting disk table and Ctrl+Z restores unsaved edits | remove_and_undo_keep_the_unsaved_shape | crates/strata-app/src/app_state_layer_remove_tests.rs:35 | crates/strata-app/src/app_state_layer_remove.rs:27 | PORTED | Verifies layer remove undo preserves pending edits |
| 37 | layerDeleteKeyRemovesReferenceFromMap | 3059 | Delete key removes non-domain reference layer from layer tree and map | — | — | crates/strata-app/src/app_state_layer_remove.rs:27 | MISSING | remove_layer exists; no test deletes a reference layer row via key/action |
| 38 | layerDeleteKeyRemovesCadastralAndKeepsItGone | 3089 | Deleting WMS cadastral layer keeps it removed across save and reopen | — | — | crates/strata-app/src/app_state_layer_remove.rs:27 | MISSING | remove_layer and cadastral WMS exist without save-and-reopen removal persistence test |
| 39 | cadastralGroupDeleteKeyRemovesChildren | 3116 | Delete key on Cadastral group removes group and all child layers | group_removal_and_checklist | crates/strata-app/src/app_state_layer_remove_tests.rs:70 | crates/strata-app/src/app_state_layer_remove.rs:85 | PORTED | Tests group removal removing all children under the group |
| 40 | drawnShapeShowsItsRecordInTheInspector | 3153 | Drawing/selecting a shape populates attribute card and record inspector | card_follows_the_selection_but_keeps_a_drawn_shape | crates/strata-app/src/app_state_feature_card_tests.rs:27 | crates/strata-app/src/app_state_feature_card.rs:26 | PARTIAL | Tests feature card tracking drawn/selected shape (record_window_sees_the_typed_memo at :58); lacks full inspector form assertion |
| 41 | cardKeysLeaveTheSketchAlone | 3174 | Typing or pressing shortcuts in inspector card does not alter active map sketch | main_shortcuts_wait_while_the_window_is_open | crates/strata-app/src/dlg_feature_record_tests.rs:92 | crates/strata-app/src/app_state_feature_card.rs:26 | PARTIAL | Tests modal window keyboard isolation; does not type keys inside feature card while a polygon sketch is active |
| 42 | surveyAreaDrawingContinuesTheExistingAreaWithoutAWindow | 3203 | Starting survey-area draw when area exists continues adding to it without setup modal | survey_area_drawing_goes_into_survey_area | crates/strata-app/src/app_state_draw_commit_tests.rs:254 | crates/strata-app/src/app_state_draw_entry.rs:19 | PORTED | Matches continuing directly on existing survey_area layer |
| 43 | cadastralGroupContextMenuCanRemove | 3227 | Right-click menu Remove on Cadastral group removes group cleanly | group_removal_and_checklist | crates/strata-app/src/app_state_layer_remove_tests.rs:70 | crates/strata-app/src/app_state_layer_remove.rs:85 | PARTIAL | Exercises group removal directly without testing group context-menu click path |
| 44 | referenceGroupMenuRemovesEveryRowAndCtrlZRestores | 3258 | Reference group context-menu Remove deletes all children and Ctrl+Z restores order | remove_and_undo_keep_the_unsaved_shape | crates/strata-app/src/app_state_layer_remove_tests.rs:35 | crates/strata-app/src/app_state_layer_remove.rs:85 | PARTIAL | Single-layer undo and group removal tested separately; lacks whole-group context-menu delete + Ctrl+Z restore test |
| 45 | emptyGroupDeleteKeyRemovesRowAndCtrlZRestores | 3337 | Delete key removes empty legend group and Ctrl+Z restores empty group row | — | — | crates/strata-app/src/app_state_layer_remove.rs:85 | MISSING | Empty group removal via Delete key and Ctrl+Z restore untested |
| 46 | ctrlZRestoresVertexEditsAndGroupedFeatureDeletion | 3364 | Ctrl+Z restores map vertex edits and multi-feature deletions as cohesive undo steps | — | — | NOT BUILT | BLOCKED | Interactive vertex-edit tool + grouped delete undo not built in GUI |
| 47 | clearDrawnFeaturesLeavesOtherLayersSelectionAlone | 3435 | Clear-all-drawn action wipes target layer sketches/features without clearing other layers' selection | — | — | NOT BUILT | BLOCKED | Clear-all-drawn action on layer context menu not built |
| 48 | eraseOverlap_cutsTheShapeUnderTheSelectedOneAndCtrlZRestores | 3494 | Overlap-erase clips underlying polygon against selected cutter and Ctrl+Z restores both | — | — | crates/strata-storage/src/polygon_pieces.rs:109 | BLOCKED | GUI overlap-erase command wiring not built (only geo helper erase_polygon_area_difference exists) |
| 49 | eraseOverlap_acrossLayersUndoesAndRedoesTogether | 3592 | Multi-layer overlap erase groups edits into single Undo/Redo step | — | — | crates/strata-storage/src/polygon_pieces.rs:159 | BLOCKED | MainWindow cross-layer erase command & KaUndoGroup integration not wired in GUI |
| 50 | eraseOverlap_severalShapesAskBeforeRemovingThem | 3647 | Overlap erase consuming multiple features prompts user confirmation first | — | — | NOT BUILT | BLOCKED | Multi-feature erase confirmation dialog not built |
| 51 | eraseOverlap_secondPressUsesTheShapeJustDrawn | 3700 | Second shortcut press reuses shape just drawn as overlap cutter | — | — | NOT BUILT | BLOCKED | GUI overlap-erase shortcut flow not built |
| 52 | drawnShapeOnTopOffersEraseWhenFinished | 3745 | Finishing a polygon overlapping existing shapes offers overlap-erase bar/action | — | — | NOT BUILT | BLOCKED | Post-draw overlap prompt not built |
| 53 | selectTool_picksTheShapeDrawnOnTopAndKeyAStartsIt | 3807 | Selection picks topmost drawn polygon; pressing key A activates selection/attribute flow | small_unsaved_shape_on_top_is_picked | crates/strata-app/src/feature_pick_tests.rs:21 | crates/strata-app/src/feature_pick.rs:80 | PARTIAL | Picks smallest/topmost unsaved shape (also select_picks_an_unsaved_drawing at :92); lacks keyboard shortcut A test |
| 54 | selectTool_rightClickErasesAndKeyAEndsSplitMode | 3854 | Select tool right-click overlap erase and key A exits split mode | — | — | NOT BUILT | BLOCKED | Select-tool right-click erase & split mode key A not built |
| 55 | polygonCommands_leaveReferenceAndCadastralShapesAlone | 3922 | Erase/split commands ignore reference and cadastral layers | — | — | NOT BUILT | BLOCKED | Map polygon erase/split commands not wired in GUI |
| 56 | layerContextMenu_usesClickedRowAndLeavesSourceIntact | 3984 | Context menu mutates the right-clicked layer rather than previously active layer | actions_apply_to_the_clicked_layer | crates/strata-app/src/gui_layer_panel_context_menu_tests.rs:55 | crates/strata-app/src/app_state_layer_menu.rs:64 | PORTED | Verifies context menu acts strictly on clicked row |
| 57 | layerContextMenu_moveToBottomPreservesRegisteredLayer | 4046 | Move-to-bottom reorders legend node without losing registered layer/features | move_to_group_bottom | crates/strata-app/src/app_state_layer_menu_tests.rs:66 | crates/strata-app/src/app_state_layer_menu.rs:82 | PORTED | Matches C++ move-to-group-bottom test |
| 58 | mapContextMenu_onlyOffersRecordEditingForSurveyFeatures | 4110 | Map right-click offers record editing for domain survey polygons only, not reference polygons | attributes_item_comes_first_over_a_survey_shape | crates/strata-app/src/map_context_menu_model_tests.rs:111 | crates/strata-app/src/gui_tab_map_context.rs:47 | PARTIAL | Checks attributes entry on survey shape and disabled state; does not explicitly test clicking a reference shape omits it |
| 59 | newSurvey_selectedCrsSurvivesSaveAndOpen | 4254 | New survey CRS choice (5186/5187) survives save and reopen intact | test_project_lifecycle_creates_and_reopens_4_default_layers_and_epsg_5186 | crates/strata-app/src/app_state_tests.rs:85 | crates/strata-app/src/app_state_projects.rs:23 | PORTED | 2 data rows in C++; 5186 lifecycle tested here and 5186/5187 GPKG verified at survey_gpkg_create_tests.rs:97 |
| 60 | embeddedOpenRestoresDemInActualTree | 4356 | Opening survey restores embedded DEM raster layer into actual layer tree | — | — | NOT BUILT | BLOCKED | 4 data rows in C++; companion .qgz DEM tree restoration on open not built |
| 61 | surveyReopensAfterUsbDriveOrFolderChange | 4444 | Moving survey folder to another path or USB drive reopens cleanly | open_demo_copy_matches_the_cpp_app | crates/strata-app/src/gui_tab_home_open_tests.rs:47 | crates/strata-app/src/gui_tab_home_open.rs:30 | PARTIAL | Opens copied .gpkg in new tempdir cleanly; does not test repairing external companion .qgz paths after folder/drive move |
| 62 | savedWorkspaceNeverPointsIntoTheSaveGenerationFolder | 4505 | Saved workspace file never stores .ka-survey-gen staging paths in datasource XML | — | — | NOT BUILT | BLOCKED | Companion .qgz XML datasource rewriting not built (Rust reads .gpkg directly) |
| 63 | newPortableFolderInheritsRecentSurveys | 4541 | Copying portable bundle inherits portable recent-surveys config cleanly | — | — | NOT BUILT | BLOCKED | Portable bundle folder-migration recent-list inheritance not built |
| 64 | saveAgainAfterAddingSurveyAreaKeepsEveryFeature | 4574 | Re-saving after adding survey area preserves all prior features across tables | drawn_polygon_is_written_by_save_and_reopens | crates/strata-app/src/app_state_draw_commit_tests.rs:79 | crates/strata-app/src/app_state_projects.rs:136 | PORTED | Verifies drawn polygon writes through save and persists across reopen |
| 65 | oldVersionSurveysStillOpen | 4850 | Older schema/legacy surveys open without failing and migrate cleanly on save | save_migrates_the_copy_and_keeps_unknown_tables_byte_for_byte | crates/strata-storage/src/survey_gpkg_save_more_tests.rs:18 | crates/strata-storage/src/survey_gpkg_migrate.rs:24 | PARTIAL | Tests migrating old GPKG copy while keeping unknown tables; lacks GUI open_project test over fixture old .gpkg+.qgz bundles |
| 66 | samplesOpenAsSurveys | 4950 | Bundled sample projects open without warnings or missing layers | demo_matches_cpp | crates/strata-app/src/checklist_state_tests.rs:71 | crates/strata-app/src/gui_tab_home_open.rs:30 | PARTIAL | Covers demo.gpkg and bad_matches_cpp (:79); does not iterate all sample directories under samples/ |
| 67 | saveAndReopen_keepsTreeGeometryAttributesAndStyle | 5013 | Save and reopen preserves layer tree order, geometry, attributes, and styling | test_r3_save_project_writes_real_gpkg_and_reopens | crates/strata-app/src/app_state_tests.rs:9 | crates/strata-app/src/app_state_projects.rs:136 | PARTIAL | Verifies GPKG geometry & attributes survive roundtrip; custom layer tree order/colors do not yet persist in .qgz |
| 68 | saveReopenSubmit_preservesWorkAndPackage | 5052 | Full save, reopen, and submit SHP export matches C++ output cleanly | schema_fields_are_shapefile_safe | crates/strata-engine/src/submit_shp_tests.rs:208 | crates/strata-engine/src/submit_shp.rs:120 | PARTIAL | 2 data rows in C++; checks SHP schema/truncation rules without end-to-end app save->reopen->submit ZIP assertion |
| 69 | open_repairsRegistryOnlyLayers | 5241 | Opening fixes registry-only/stale QgsMapLayer entries in loaded project | — | — | NOT BUILT | BLOCKED | .qgz registry-only stale XML layer repair not built |
| 70 | startupHome_ignoresRememberedSurveyUntilUserOpensIt | 5254 | Startup ignores last remembered survey until explicitly opened from Home | startup_has_no_survey_data_and_no_user_settings | crates/strata-app/src/app_state_recent_tests.rs:42 | crates/strata-app/src/app_state_recent.rs:25 | PORTED | Matches Home-first boot isolation |
| 71 | drawingStudioPrintSplitsTheDrawingIntoSheets | 5286 | Print action tiles oversized sheet layout into printable pages cleanly | test_tile_print_grid_calculation_a1_landscape_on_a3_printable_400x277_yields_3x3_9_tiles_and_tolerance | crates/strata-layout/src/tile_print_tests.rs:23 | crates/strata-layout/src/sheet_scalebar.rs:37 | PARTIAL | Tests tile-grid math only; multi-sheet PDF export deferred (print_save_tiles_pdf_is_deferred_w50 at dlg_drawing_print_actions_tests.rs:75) |
| 72 | ribbonPrintOpensTheDrawingThenThePrintWindow | 5351 | Clicking Print on ribbon opens Drawing Studio and presents Print dialog | toolbar_paper_orientation_button_opens_paper_setup_dialog_and_preserves_sheet_context | crates/strata-app/src/gui_tab_drawing_tests.rs:68 | crates/strata-app/src/dlg_drawing_print.rs:24 | PARTIAL | Tests Drawing Studio toolbar dialog flow; no test clicks ribbon Print button to transition Map -> Drawing -> Print modal |
| 73 | scaleControlFollowsActiveDrawingWithoutChangingMapCanvas | 5394 | Scale control tracks Drawing Studio active sheet scale without changing Map zoom | test_drawing_studio_dpi_validation_72_200_1200_and_preserve_scale_sync | crates/strata-app/src/tab_drawing.rs:125 | crates/strata-app/src/app_state_status_bar.rs | PARTIAL | Checks sheet scale sync; does not test status-bar scale widget isolation between Map and Drawing tabs |
| 74 | scaleControl_isSingleWidgetAcceptingBothForms | 5478 | Status-bar scale input accepts both 1:1000 and 1000 formats | test_status_bar_scale_1000_from_0264583333m_and_dirty_label_and_studio_tab_close | crates/strata-app/src/app_state_tests.rs:104 | crates/strata-app/src/status_bar_text.rs:258 | PARTIAL | Tests scale calculation string "1:1000"; lacks interactive input parser test accepting both "1:1000" and "1000" |
| 75 | open_blocksNestedOpenAndAutosave | 5505 | Opening re-entrancy guard blocks overlapping open/autosave calls | test_modal_exclusive_guard_blocks_duplicate_popup_and_dirty_reject_confirmation | crates/strata-app/src/dialogs_modal_tests_b.rs:86 | crates/strata-app/src/gui_tab_home_open.rs:30 | PARTIAL | Tests modal exclusivity guard; no re-entrancy guard test on nested open_survey_gpkg invocations |
| 76 | partialCommit_preservesRemainingBufferAndCreatesIndependentRecovery | 5542 | Partial layer commit failure preserves remaining edit buffer and writes recovery copy | — | — | NOT BUILT | BLOCKED | Multi-layer partial commit & independent recovery snapshot not built |
| 77 | failedCommit_doesNotOverwriteSavedWorkspace | 5645 | Save commit failure leaves previously saved workspace and files intact | a_failed_save_keeps_the_file_the_edits_and_the_star | crates/strata-app/src/app_state_save_tests.rs:85 | crates/strata-app/src/app_state_projects.rs:136 | PARTIAL | Verifies failed GPKG save preserves disk file, edits, and dirty star; companion .qgz not yet written |
| 78 | emptyGpkgWithoutWorkspace_opensWithoutDomainLayers | 5686 | Opening bare empty GeoPackage creates no domain layers in tree | test_new_survey_starts_empty_and_keeps_no_previous_survey | crates/strata-app/src/gui_tab_home_tests.rs:16 | crates/strata-app/src/gui_tab_home_open.rs:30 | PARTIAL | Tested via new survey creation (:16, :115); no direct open_survey_gpkg test on empty bare .gpkg fixture |
| 79 | invalidSurvey_doesNotReplaceCurrentWork | 5699 | Opening corrupt or invalid survey leaves open survey and layers undisturbed | unreadable_or_damaged_file_keeps_the_current_survey | crates/strata-app/src/gui_tab_home_open_tests.rs:111 | crates/strata-app/src/gui_tab_home_open.rs:30 | PORTED | 2 data rows in C++; checks damaged/unreadable file preserves current survey |
| 80 | openWhileDrawing_resetsCaptureBeforeReplacingLayers | 5737 | Opening another survey resets active sketch tool/vertices before swapping layers | opening_another_survey_clears_the_target | crates/strata-app/src/app_state_draw_entry_tests.rs:124 | crates/strata-app/src/gui_tab_home_open.rs:30 | PARTIAL | Checks edit_layer_key cleared; does not assert mid-sketch rubberband vertices are cleared first |
| 81 | open_unsavedWorkRespectsAnswer | 5780 | Opening over dirty survey honors Save, Discard, or Cancel prompt choice | test_d25_drop_close_prompt_cad_notice_and_resolutions | crates/strata-app/src/gui_shell_tests_a.rs:20 | crates/strata-app/src/shell_dialogs.rs:67 | PARTIAL | 4 data rows in C++; tests close prompt text/options, not Save/Discard/Cancel execution around open_project |
| 82 | saveAs_preservesActualTablesAndReopensCopy | 5822 | Save As copies non-domain/actual tables cleanly and reopens copied survey | workspace_file_actions_save_reopen_and_import_dxf_csv_roundtrip | crates/strata-storage/src/project_recent_tests.rs:15 | crates/strata-app/src/app_state_projects.rs:152 | PARTIAL | Checks workspace save/reopen; no end-to-end Save As test verifying extra tables survive copy and reopen |
| 83 | save_keepsDrawnSurveyArea | 5854 | Saving preserves newly drawn survey_area features in GeoPackage | drawn_polygon_is_written_by_save_and_reopens | crates/strata-app/src/app_state_draw_commit_tests.rs:79 | crates/strata-app/src/app_state_projects.rs:136 | PORTED | Verified for polygons and survey_area table at survey_area_drawing_goes_into_survey_area (:254) |
| 84 | save_fieldPackageKeepsSurveyAreaReadable | 5883 | Exported field package keeps survey_area geometry readable offline | — | — | NOT BUILT | BLOCKED | Field-package bundle exporter not built |
| 85 | newSurvey_existingNamePreservesUnsavedCurrentSurvey | 5916 | Refusing New Survey over existing name keeps open dirty edits undisturbed | test_new_survey_dialog_shows_cpp_refusal_text_as_is | crates/strata-app/src/gui_tab_home_tests.rs:257 | crates/strata-app/src/app_state_projects.rs:23 | PARTIAL | Checks refusal text and existing disk bytes; does not hold unsaved edits in active survey across refusal |
| 86 | closeSave_preservesMemoryReferenceVectorOnReopen | 5983 | Closing after Save preserves memory-only reference vectors on reopen | — | — | NOT BUILT | BLOCKED | Memory reference vector serialization on save/reopen not built |
| 87 | saveAs_embeddedFailureRestoresSourcesAndRemainsUnsaved | 6027 | Save As failure in embedded workspace restores original layer sources and dirty flag | — | — | NOT BUILT | BLOCKED | Companion .qgz Save As source-rewrite rollback not built |
| 88 | saveAs_existingDestinationIsNotOverwritten | 6087 | Save As refuses existing .gpkg or .qgz destination without overwriting | save_as_target_refuses_existing_survey_files_but_allows_the_open_one | crates/strata-app/src/app_state_tests.rs:135 | crates/strata-app/src/app_state_projects.rs:152 | PORTED | 2 data rows in C++; checks .gpkg and .qgz target collisions and self-overwrite allowance |
| 89 | save_qgisExceptionStaysInsideSlot | 6126 | Exception during save layer commit stays contained inside save slot with clean error | an_edit_that_cannot_be_written_fails_with_the_layer_and_leaves_the_original | crates/strata-storage/src/survey_gpkg_save_tests.rs:132 | crates/strata-storage/src/survey_gpkg_save.rs:42 | PARTIAL | 2 data rows in C++; verifies write failure returns clean Err without panic; lacks UI slot wrapper test |
| 90 | save_companionFailureKeepsEmbeddedWorkspaceAndWarns | 6173 | Companion .qgz failure leaves embedded workspace usable and warns user | — | — | NOT BUILT | BLOCKED | Companion .qgz workspace fallback & stale warning not built |
| 91 | fieldCopy_restoresAllValidLayersAndSavesTree | 6200 | Opening/restoring field copy rebuilds all valid domain layers and tree | — | — | NOT BUILT | BLOCKED | Field copy restoration pipeline not built |

## test_schema_version.cpp

| # | C++ test function | C++ line | what it checks | Rust test | Rust file:line | Rust code under test | status | note |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 1 | schemaTable_isShapefileSafeAndOptional | 114 | Submission SHP fields obey 10-char limit and schema table stays optional | schema_fields_are_shapefile_safe | crates/strata-engine/src/submit_shp_tests.rs:208 | crates/strata-types/src/schema.rs:131 | PARTIAL | Checks 10-char shapefile field limits; omits optional ka_schema_version metadata table absence test |
| 2 | newSurvey_isStampedWithRecordFields | 151 | New survey tables carry record fields and gpkg_metadata schema stamp | new_survey_file_matches_cpp_gdal_survey_for_5186_and_5187 | crates/strata-storage/src/survey_gpkg_create_tests.rs:97 | crates/strata-storage/src/survey_gpkg_create.rs:24 | PORTED | Also covered by layer_ddl_matches_the_schema_yaml_names_types_and_order (:197) and stamps_fill_only_existing_columns |
| 3 | oldSurvey_gainsFieldsOnlyThroughMigration | 171 | Opening old survey read-only never alters file; migration adds schema on copy | save_migrates_the_copy_and_keeps_unknown_tables_byte_for_byte | crates/strata-storage/src/survey_gpkg_save_more_tests.rs:18 | crates/strata-storage/src/survey_gpkg_migrate.rs:24 | PORTED | Verifies generation-copy migration adds schema stamp/fields while leaving read-only source untouched |
| 4 | migration_neverLowersANewerStamp | 208 | Migrating a survey with higher schema version never downgrades version stamp | the_stamp_merges_into_existing_gdal_metadata_and_never_lowers_a_newer_one | crates/strata-storage/src/survey_gpkg_save_more_tests.rs:47 | crates/strata-storage/src/survey_gpkg_migrate.rs:53 | PORTED | Direct port of forward-only gpkg_metadata stamp merge |

## test_storage_generation.cpp

| # | C++ test function | C++ line | what it checks | Rust test | Rust file:line | Rust code under test | status | note |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 1 | copySurvey_renameRefused_neverTruncatesOriginal | 64 | Locked target rename failure leaves original survey intact and writes side copy | a_read_only_or_locked_original_is_saved_beside_it | crates/strata-storage/src/survey_gpkg_save_more_tests.rs:95 | crates/strata-storage/src/survey_gpkg_save.rs:141 | PARTIAL | Writes -저장.gpkg preserving original bytes; does not verify OS sharing-violation path via copy_survey_snapshot + validate_survey_for_open SHA-256 explicitly |
| 2 | persistWorkspace_lateFailure_neverCommitsOriginal | 94 | Late serialization exception rolls back without overwriting original workspace | — | — | NOT BUILT | BLOCKED | Companion .qgz workspace late-exception commit rollback not built |
| 3 | persistWorkspace_blockedLayer_reportsPartialOriginalUpdate | 124 | Blocked layer commit failure reports partial original state clearly | — | — | NOT BUILT | BLOCKED | Per-layer partial commit originalStateNote not built |
| 4 | persistWorkspace_saveLeavesCommandsCommittedNotDiscarded | 154 | Save transitions edit commands cleanly to Committed rather than Discarded | — | — | NOT BUILT | BLOCKED | EditingHistory command-state retention across save not built |
| 5 | fingerprint_seesReplacementButNotInPlaceWrites | 183 | File fingerprint catches out-of-band file replacement while allowing own writes | test_atomic_save_success_and_fingerprint_and_stale_temp_hygiene | crates/strata-storage/src/session_save.rs:217 | crates/strata-storage/src/session_save.rs:77 | PARTIAL | Checks SHA-256 fingerprint diff; omits Windows FileIndex/volume replacedElsewhere in-place vs rename check |
| 6 | persistWorkspace_ownSaveIsNotSeenAsElsewhere | 220 | App own save updates fingerprint so subsequent save is not flagged external | rows_added_elsewhere_make_the_save_stop_with_the_original_intact | crates/strata-storage/src/survey_gpkg_save_tests.rs:113 | crates/strata-storage/src/survey_gpkg_save.rs:71 | PARTIAL | Checks baseline external-modification guard; lacks explicit sequential two-save own-fingerprint refresh assertion |
| 7 | persistWork_companionFailureMarksStaleWorkspace | 240 | Companion workspace failure marks companionMayBeStale flag for warning | — | — | NOT BUILT | BLOCKED | Companion .qgz workspace & stale marker not built |

## test_storage_hygiene.cpp

| # | C++ test function | C++ line | what it checks | Rust test | Rust file:line | Rust code under test | status | note |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 1 | staleStaging_listsOnlyAppLeftovers | 53 | Leftover scan matches only Strata staging temps, preserving real backups/generations | test_atomic_save_success_and_fingerprint_and_stale_temp_hygiene | crates/strata-storage/src/session_save.rs:217 | crates/strata-storage/src/session_save.rs:116 | PARTIAL | Tests sweep_stale_temp_files on .tmp/.journal; does not test .ka-survey-gen-* / .ka-new exclusion rules |
| 2 | editSignature_changesOnlyWithEdits | 94 | Edit signature stays identical across clean state/focus and changes only on real edits | — | — | crates/strata-storage/src/session_save.rs:109 | MISSING | recovery_snapshot_needed exists at session_save.rs:109 without deterministic no-op vs edited signature test |
| 3 | collect_sameSizeDifferentContentIsCopiedAgain | 123 | Bundle collector re-copies asset if same byte size has newer/different content | — | — | NOT BUILT | BLOCKED | SurveyBundle::collectIntoSurvey reference collector not built |

## test_storage_safety.cpp

| # | C++ test function | C++ line | what it checks | Rust test | Rust file:line | Rust code under test | status | note |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 1 | recoverySnapshot_preservesPendingEditsAndSource | 111 | Crash recovery snapshot saves unsaved edits and relative vector sources | — | — | NOT BUILT | BLOCKED | 2 data rows in C++ (5186, 5187); SurveyStorage::writeRecoverySnapshot not built |
| 2 | newSurvey_existingFilesRemainByteIdentical | 262 | Refusing New Survey over existing file preserves existing bytes identically | create_new_survey_refuses_existing_gpkg_and_keeps_its_bytes | crates/strata-storage/src/gpkg_core_tests.rs:164 | crates/strata-storage/src/survey_gpkg_create.rs:24 | PORTED | Matches exact byte-for-byte preservation on refusal |
| 3 | newSurvey_orphanWorkspaceIsNotOverwritten | 282 | New Survey refuses creation when companion .qgz exists even without .gpkg | create_new_survey_refuses_when_same_name_qgz_exists | crates/strata-storage/src/gpkg_core_tests.rs:179 | crates/strata-storage/src/survey_gpkg_create.rs:24 | PORTED | Direct port checking orphan .qgz collision |
| 4 | newSurvey_invalidCrsLeavesNoPartialFiles | 298 | Invalid CRS fails new survey creation cleanly without partial output files | — | — | NOT BUILT | BLOCKED | WorkCrs enum forbids invalid CRS at compile time; runtime unknown-CRS refusal not tested |
| 5 | newSurvey_workspaceCollisionLeavesNoPartialGpkg | 309 | Directory or read-only collision at .qgz path leaves no partial .gpkg | — | — | crates/strata-storage/src/survey_gpkg_create.rs:24 | MISSING | create_new_survey exists; no test creates directory named <survey>.qgz and asserts zero .gpkg artifact |
| 6 | newSurvey_schemaExistsButWorkspaceIsEmpty | 331 | New survey creates valid 7 domain tables across 5186/5187 | new_survey_file_matches_cpp_gdal_survey_for_5186_and_5187 | crates/strata-storage/src/survey_gpkg_create_tests.rs:97 | crates/strata-storage/src/survey_gpkg_create.rs:24 | PARTIAL | 2 data rows in C++ (5186, 5187); verifies schema DDL and SRS, without companion .qgz check |
| 7 | atomicWorkspace_preservesRelativeSourceAndPreviousGeneration | 352 | Atomic workspace write replaces file cleanly keeping relative links and .bak | write_workspace_file_replaces_own_workspace_and_leaves_no_temp_file | crates/strata-storage/src/gpkg_core_tests.rs:197 | crates/strata-storage/src/gpkg_repository_write.rs:23 | PARTIAL | Checks atomic .qgz write & temp cleanup; omits .qgz.bak generation & relative datasource check |
| 8 | atomicWorkspace_lockedTargetPreservesFileAndUnsavedState | 383 | Atomic workspace failure on locked .qgz keeps previous file untouched | — | — | NOT BUILT | BLOCKED | kaWriteQgisProjectAtomic locked target handling not built |
| 9 | atomicWorkspace_lockedBackupPreservesBothFiles | 409 | Atomic workspace failure on locked .bak leaves primary and backup intact | — | — | NOT BUILT | BLOCKED | kaWriteQgisProjectAtomic .bak lock rollback not built |
| 10 | atomicWorkspace_serializationExceptionKeepsFileAndState | 436 | QGZ serialization error leaves previous workspace bytes untouched | — | — | NOT BUILT | BLOCKED | kaWriteQgisProjectAtomic exception rollback not built |
| 11 | recoverySnapshot_layerFilterSkipsUnlistedVectors | 457 | Recovery snapshot skips reference vectors absent from recovery list | — | — | NOT BUILT | BLOCKED | SurveyStorage::writeRecoverySnapshot layer filter not built |
| 12 | recoveryPending_notesNewestAndPrunesOldCopies | 494 | Recovery tracker records newest snapshot and prunes stale copies | — | — | NOT BUILT | BLOCKED | SurveyStorage::noteRecoveryPending not built |
| 13 | absorbLeavesReferenceVectorsOutsideTheSurvey | 527 | Absorbing layers copies external vectors or keeps references intentionally | — | — | NOT BUILT | BLOCKED | SurveyStorage::absorbExternalVectors not built |
| 14 | persistWorkspace_secondLayerCommitFails_keepsEditsAndRecovery | 586 | Failure committing second layer retains pending edits and writes recovery | — | — | NOT BUILT | BLOCKED | Multi-layer partial commit recovery snapshot not built |
| 15 | persistWorkspace_lockedGpkg_keepsEditsAndRecovery | 631 | Locked target .gpkg falls back cleanly preserving edits and original file | a_read_only_or_locked_original_is_saved_beside_it | crates/strata-storage/src/survey_gpkg_save_more_tests.rs:95 | crates/strata-storage/src/survey_gpkg_save.rs:141 | PARTIAL | Saves beside original as -저장.gpkg leaving locked original intact; omits recovery folder & dirty state retention |
| 16 | persistWorkspace_missingExternal_keepsEditsAndRecovery | 674 | Missing external shapefile source aborts save keeping edits and recovery | — | — | NOT BUILT | BLOCKED | External vector missing-source save guard not built |
| 17 | persistWorkspace_writeExceptionKeepsPreviousGeneration | 738 | Exception writing generation keeps original generation intact | an_edit_that_cannot_be_written_fails_with_the_layer_and_leaves_the_original | crates/strata-storage/src/survey_gpkg_save_tests.rs:132 | crates/strata-storage/src/survey_gpkg_save.rs:71 | PARTIAL | Verifies failing table write preserves original bytes; omits recovery snapshot write |
| 18 | persistWork_writesCompanionQgzWithoutMainWindow | 794 | Persistence writes companion .qgz even in headless non-window calls | — | — | NOT BUILT | BLOCKED | Headless companion .qgz writer not built |
| 19 | persistWorkspace_missingSurveyFile_writesLocalCopy | 827 | Save handles missing original survey path without crash or edit loss | saving_a_vanished_survey_file_creates_nothing_and_keeps_edits | crates/strata-app/src/app_state_save_tests.rs:106 | crates/strata-app/src/app_state_projects.rs:136 | PARTIAL | Intentionally refuses instead of writing local fallback (지시 2E), keeping edits in memory |
| 20 | persistWorkspace_staleWalDoesNotBlockSave | 854 | Pre-existing -wal/-shm sidecars do not cause false lock errors on save | the_generation_copy_includes_rows_still_in_the_wal | crates/strata-storage/src/survey_gpkg_save_more_tests.rs:74 | crates/strata-storage/src/survey_gpkg_save.rs:30 | PARTIAL | Verifies live WAL rows merge into generation copy; does not pre-seed empty -wal/-shm handle case |
| 21 | persistWork_missingParentRelocatesToFallback | 880 | Unwritable survey parent relocates backup output to writable fallback dir | — | — | NOT BUILT | BLOCKED | writableSurveyPath directory relocation not built |
| 22 | persistWork_emptyPathDoesNotWrite | 905 | Empty survey path fails safely without creating files anywhere | — | — | crates/strata-storage/src/survey_gpkg_save.rs:42 | MISSING | save_survey exists; no unit test calls save_survey with empty Path verifying error and zero side effects |
| 23 | extractEmbeddedReferenceVectors_shrinksSurveyAndKeepsDomain | 913 | Extracting embedded reference vectors shrinks GPKG keeping domain tables | — | — | NOT BUILT | BLOCKED | extractEmbeddedReferenceVectors not built |
| 24 | repairPersisted_restoresDroppedWindowsHome | 995 | Repaired persisted path restores missing drive/home prefix on Windows | — | — | NOT BUILT | BLOCKED | repairPersistedFileSources path repair not built |
| 25 | persistWorkspace_keepsExternalHeritagePathOnReopen | 1036 | External heritage vector relative source survives save and reopen | — | — | NOT BUILT | BLOCKED | Companion .qgz external heritage vector persistence not built |

## test_survey_durability.cpp

| # | C++ test function | C++ line | what it checks | Rust test | Rust file:line | Rust code under test | status | note |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 1 | processesHolding_namesThisAppWhileItHoldsTheFile | 67 | Windows Restart Manager helper lists process holding open survey handle | — | — | NOT BUILT | BLOCKED | SurveyDurability Windows Restart Manager FFI not built |
| 2 | copySurvey_renameRefused_logsWhoHoldsTheFile | 88 | Failed replace logs process name locking the survey file | — | — | NOT BUILT | BLOCKED | SurveyDurability holder logging on lock failure not built |
| 3 | lockDiagnosis_saysWhenItCannotTell | 117 | Lock diagnosis states clearly when holder cannot be determined | — | — | NOT BUILT | BLOCKED | SurveyDurability::lockDiagnosis helper not built |
| 4 | lockDiagnosis_namesAReadOnlyTarget | 128 | Lock diagnosis distinguishes read-only attribute from process lock | — | — | NOT BUILT | BLOCKED | SurveyDurability read-only file diagnostic not built |

## test_survey_save_as.cpp

| # | C++ test function | C++ line | what it checks | Rust test | Rust file:line | Rust code under test | status | note |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 1 | moveToCopy_savesEditsIntoTheCopyOnly | 51 | Save As writes buffered edits into new copy leaving original file unmodified | save_as_copy_leaves_original_survey_fingerprint_untouched | crates/strata-storage/src/session_save.rs:177 | crates/strata-app/src/app_state_projects.rs:152 | PARTIAL | Storage fingerprint test checks original SHA-256 unchanged; app-level dirty edit Save As copy test missing |
| 2 | moveToCopy_leavesOtherFilesAlone | 77 | Save As never alters companion/reference files in original survey folder | — | — | crates/strata-app/src/app_state_projects.rs:152 | MISSING | save_survey_project_as exists; no test asserts unrelated files in source folder stay untouched |

## test_undo_group.cpp

| # | C++ test function | C++ line | what it checks | Rust test | Rust file:line | Rust code under test | status | note |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 1 | shapeAndName_undoAndRedoInOneStep | 42 | Drawing polygon and saving record form undo/redo together as one step | shape_and_values_undo_as_one_step | crates/strata-app/src/app_state_feature_form_tests.rs:125 | crates/strata-app/src/app_state_feature_form.rs:29 | PORTED | Direct port uniting drawn geometry and form attributes in one undo step |
| 2 | closingBeforeAnotherEdit_keepsTwoSteps | 63 | Closing an undo group before next edit preserves two separate undo steps | — | — | NOT BUILT | BLOCKED | Explicit KaUndoGroup boundary splitting across multi-layer ops not built |
| 3 | nothingInside_leavesNoStep | 77 | Empty edit transaction pushes no undo entry onto undo stack | undo_and_redo_after_drawing_follow_the_layer_buffer | crates/strata-app/src/app_state_draw_commit_tests.rs:155 | crates/strata-app/src/app_state_draw_commit_b.rs:31 | PARTIAL | Covers normal undo stack behavior; lacks explicit test asserting empty cancelled action adds zero steps |

---

## Port now

All `MISSING` and `PARTIAL` rows sorted from highest risk (`loss or corruption of survey data` -> `wrong coordinates` -> `wrong behaviour` -> `looks`):

### 1. Loss or corruption of survey data (24 items)

1. **`test_storage_safety.cpp:905` (`persistWork_emptyPathDoesNotWrite`)** — *MISSING*: `crates/strata-storage/src/survey_gpkg_save.rs:42` (`save_survey`). Assert calling `save_survey(Path::new(""), ...)` returns an `Err` and writes no file in CWD or temp.
2. **`test_storage_safety.cpp:309` (`newSurvey_workspaceCollisionLeavesNoPartialGpkg`)** — *MISSING*: `crates/strata-storage/src/survey_gpkg_create.rs:24` (`create_new_survey`). Assert when `<name>.qgz` exists as a directory, `create_new_survey` refuses cleanly and creates no `.gpkg` or `.gpkg-wal`.
3. **`test_survey_save_as.cpp:77` (`moveToCopy_leavesOtherFilesAlone`)** — *MISSING*: `crates/strata-app/src/app_state_projects.rs:152` (`save_survey_project_as`). Assert Save As writes only the copy and leaves unrelated sibling/reference files in the original folder byte-identical.
4. **`test_survey_save_as.cpp:51` (`moveToCopy_savesEditsIntoTheCopyOnly`)** — *PARTIAL*: `crates/strata-app/src/app_state_projects.rs:152` (`save_survey_project_as`). Assert Save As after adding dirty polygons in app writes them to the new `.gpkg` while original `.gpkg` byte SHA-256 and feature count remain unchanged.
5. **`test_storage_safety.cpp:631` (`persistWorkspace_lockedGpkg_keepsEditsAndRecovery`)** — *PARTIAL*: `crates/strata-storage/src/survey_gpkg_save.rs:141` (`save_survey`). Assert when target `.gpkg` is locked, fallback `-저장.gpkg` captures edits while original bytes stay unchanged and pending UI edits stay intact if fallback fails.
6. **`test_storage_safety.cpp:738` (`persistWorkspace_writeExceptionKeepsPreviousGeneration`)** — *PARTIAL*: `crates/strata-storage/src/survey_gpkg_save.rs:71`. Assert a mid-write commit error keeps the previous `.gpkg` generation byte-identical and retains dirty buffer state for retry.
7. **`test_storage_safety.cpp:827` (`persistWorkspace_missingSurveyFile_writesLocalCopy`)** — *PARTIAL*: `crates/strata-app/src/app_state_projects.rs:136` (`save_survey_project`). Assert when the open `.gpkg` vanishes before save, Rust refuses cleanly (`지시 2E`) without truncating/recreating empty DB and preserves all pending edits in memory.
8. **`test_storage_safety.cpp:854` (`persistWorkspace_staleWalDoesNotBlockSave`)** — *PARTIAL*: `crates/strata-storage/src/survey_gpkg_save.rs:30`. Assert pre-existing empty `-wal`/`-shm` sidecar files beside a survey `.gpkg` do not trigger false lock failure when saving.
9. **`test_storage_generation.cpp:64` (`copySurvey_renameRefused_neverTruncatesOriginal`)** — *PARTIAL*: `crates/strata-storage/src/survey_gpkg_save.rs:141`. Assert rename/replace refusal preserves 100% original file SHA-256 and produces a valid pass on `validate_survey_for_open` for the side file.
10. **`test_storage_generation.cpp:183` (`fingerprint_seesReplacementButNotInPlaceWrites`)** — *PARTIAL*: `crates/strata-storage/src/session_save.rs:77`. Assert external file replacement triggers conflict guard while SQLite WAL/SHM activity or unchanged content matches own fingerprint.
11. **`test_storage_generation.cpp:220` (`persistWorkspace_ownSaveIsNotSeenAsElsewhere`)** — *PARTIAL*: `crates/strata-storage/src/survey_gpkg_save.rs:71`. Assert two consecutive app saves update baseline/fingerprint so the second save does not falsely abort as modified elsewhere.
12. **`test_storage_hygiene.cpp:53` (`staleStaging_listsOnlyAppLeftovers`)** — *PARTIAL*: `crates/strata-storage/src/session_save.rs:116` (`sweep_stale_temp_files`). Assert temp-file sweep deletes only Strata staging leftovers and never deletes user surveys or `-저장.gpkg` backups.
13. **`test_storage_hygiene.cpp:94` (`editSignature_changesOnlyWithEdits`)** — *MISSING*: `crates/strata-storage/src/session_save.rs:109` (`recovery_snapshot_needed`). Assert signature stays invariant across no-op opens/focus changes and changes immediately upon geometry or attribute edits.
14. **`test_storage_safety.cpp:352` (`atomicWorkspace_preservesRelativeSourceAndPreviousGeneration`)** — *PARTIAL*: `crates/strata-storage/src/gpkg_repository_write.rs:23`. Assert atomic workspace file writes clean temp files and retain valid previous state on simulated replace error.
15. **`test_save_handles.cpp:57` (`removedLayerHeldForUndo_doesNotBlockTheSave`)** — *PARTIAL*: `crates/strata-app/src/app_state_layer_remove.rs:27`. Assert saving after removing a survey layer whose undo entry stays on the undo stack succeeds with no Windows SQLite lock error.
16. **`test_save_handles.cpp:90` (`removedLayerWithUnsavedEdits_doesNotBlockTheSave`)** — *PARTIAL*: `crates/strata-app/src/app_state_layer_remove.rs:27`. Assert removing a layer with unsaved edits does not block saving other layers or lose undo restoration afterward.
17. **`test_save_handles.cpp:118` (`singleLayerReadThenSaveWithoutEdits_replacesInPlace`)** — *PARTIAL*: `crates/strata-storage/src/sqlite_read.rs:107` & `survey_gpkg_save.rs:42`. Assert opening a survey read-only and immediately running save replaces/keeps the file without sharing violations.
18. **`test_save_open.cpp:5645` (`failedCommit_doesNotOverwriteSavedWorkspace`)** — *PARTIAL*: `crates/strata-app/src/app_state_projects.rs:136`. Assert save error in app keeps saved file bytes, dirty indicator, and all edits intact across subsequent retry.
19. **`test_save_open.cpp:5822` (`saveAs_preservesActualTablesAndReopensCopy`)** — *PARTIAL*: `crates/strata-app/src/app_state_projects.rs:152`. Assert `save_survey_project_as` copies non-empty domain and auxiliary GPKG tables cleanly and `open_project` reopens the copy with exact row counts.
20. **`test_save_open.cpp:5916` (`newSurvey_existingNamePreservesUnsavedCurrentSurvey`)** — *PARTIAL*: `crates/strata-app/src/app_state_projects.rs:23`. Assert refusing New Survey on existing filename leaves an currently open dirty survey's unsaved features and `*` state completely intact.
21. **`test_save_open.cpp:5780` (`open_unsavedWorkRespectsAnswer`)** — *PARTIAL*: `crates/strata-app/src/shell_dialogs.rs:67` & `app_state_projects.rs:93`. Assert Cancel keeps current dirty edits open, Discard opens the target discarding edits, and Save commits current work before opening target.
22. **`test_recent.cpp:230` (`closeEvent_asksBeforeDiscardingUnsavedWork`)** — *PARTIAL*: `crates/strata-app/src/shell_dialogs.rs:67`. Assert window close request while dirty opens prompt, Cancel aborts close, Discard closes without write, and Save persists before closing.
23. **`test_save_open.cpp:5505` (`open_blocksNestedOpenAndAutosave`)** — *PARTIAL*: `crates/strata-app/src/gui_tab_home_open.rs:30`. Assert a concurrent/nested `open_survey_gpkg` call while already opening is ignored without corrupting state.
24. **`test_save_open.cpp:6126` (`save_qgisExceptionStaysInsideSlot`)** — *PARTIAL*: `crates/strata-storage/src/survey_gpkg_save.rs:42`. Assert simulated storage error inside `save_survey_project` surfaces status warning cleanly without panicking the UI loop.

### 2. Wrong coordinates or spatial/schema mismatch (6 items)

25. **`test_storage_safety.cpp:331` (`newSurvey_schemaExistsButWorkspaceIsEmpty`)** — *PARTIAL*: `crates/strata-storage/src/survey_gpkg_create.rs:24`. Assert both EPSG:5186 and EPSG:5187 newly created surveys initialize empty domain tables with matching `gpkg_geometry_columns` SRS IDs.
26. **`test_schema_version.cpp:114` (`schemaTable_isShapefileSafeAndOptional`)** — *PARTIAL*: `crates/strata-types/src/schema.rs:131` & `crates/strata-engine/src/submit_shp.rs:120`. Assert SHP submission writer omits `ka_schema_version` table and keeps every exported DBF field name <= 10 bytes.
27. **`test_save_open.cpp:5052` (`saveReopenSubmit_preservesWorkAndPackage`)** — *PARTIAL*: `crates/strata-engine/src/submit_shp.rs:120`. Assert full save -> reopen -> submit SHP export transforms features accurately into EPSG:5179 for both 5186 and 5187 surveys.
28. **`test_save_open.cpp:4444` (`surveyReopensAfterUsbDriveOrFolderChange`)** — *PARTIAL*: `crates/strata-app/src/gui_tab_home_open.rs:30`. Assert moving a saved survey folder to another root/drive reopens cleanly with intact feature coordinates and zoom extent.
29. **`test_save_open.cpp:4850` (`oldVersionSurveysStillOpen`)** — *PARTIAL*: `crates/strata-storage/src/survey_gpkg_migrate.rs:24`. Assert `open_project` loads legacy C++ `.gpkg` survey versions cleanly in EPSG:5186/5187 without geometry loss and migrates safely on save.
30. **`test_save_open.cpp:2540` (`drawingStudio_centerLayerMovesAboveLabelsMapToo`)** — *PARTIAL*: `crates/strata-app/src/gui_tab_drawing_frame.rs:92` (`center_layer_in_sheet`). Assert centering Drawing Studio frame on a layer computes exact world center matching canvas projection without coordinate drift.

### 3. Wrong behaviour (31 items)

31. **`test_save_open.cpp:3059` (`layerDeleteKeyRemovesReferenceFromMap`)** — *MISSING*: `crates/strata-app/src/app_state_layer_remove.rs:27` (`remove_layer`). Assert Delete key on a reference layer removes it from legend/map without deleting its backing disk file.
32. **`test_save_open.cpp:3089` (`layerDeleteKeyRemovesCadastralAndKeepsItGone`)** — *MISSING*: `crates/strata-app/src/app_state_layer_remove.rs:27`. Assert deleting Cadastral WMS layer leaves it absent after saving and reopening the survey.
33. **`test_save_open.cpp:3337` (`emptyGroupDeleteKeyRemovesRowAndCtrlZRestores`)** — *MISSING*: `crates/strata-app/src/app_state_layer_remove.rs:85` (`remove_group_layers`). Assert pressing Delete on an empty group removes the group row and Ctrl+Z restores it at its original position.
34. **`test_edit_buffer_summary.cpp:119` (`tenThousandEditsUnderTenMs`)** — *MISSING*: `crates/strata-storage/src/edit_buffer.rs:55`. Assert summarizing/counting 10,000 buffered edits completes within 10 ms in release/test budget.
35. **`test_log_session.cpp:144` (`secretMask_keepsOrdinaryText`)** — *MISSING*: `crates/strata-secrets/src/mask.rs:5` (`redact`). Assert ordinary Korean layer names, local file paths, and non-secret URLs pass through `redact()` byte-unchanged.
36. **`test_recent.cpp:74` (`remember_keepsOfflineSurveysForLater`)** — *PARTIAL*: `crates/strata-storage/src/recent_surveys.rs:144` (`remember`). Assert opening survey B retains offline survey A in `load_all` for UI display while `last_path` skips A.
37. **`test_edit_buffer_summary.cpp:47` (`countsUniqueFeatureIds`)** — *PARTIAL*: `crates/strata-app/src/app_state_status_bar.rs`. Assert editing the same feature's geometry and attributes twice increases the unsaved count by 1, not 2.
38. **`test_edit_buffer_summary.cpp:108` (`projectDirtyFlagPassesThrough`)** — *PARTIAL*: `crates/strata-app/src/app_state_status_bar.rs:103`. Assert project-level metadata edits mark the project dirty even when zero features differ.
39. **`test_editing_history.cpp:66` (`commandStateFollowsUndoCommitAndDiscard`)** — *PARTIAL*: `crates/strata-storage/src/edit_buffer.rs:83`. Assert undo clears after save and redo stack is purged when a new edit occurs after undo.
40. **`test_undo_group.cpp:77` (`nothingInside_leavesNoStep`)** — *PARTIAL*: `crates/strata-app/src/app_state_draw_commit_b.rs:31`. Assert cancelling an empty sketch or record dialog pushes zero entries onto undo history.
41. **`test_log_session.cpp:114` (`sessionLog_masksKeysAndAccounts`)** — *PARTIAL*: `crates/strata-secrets/src/mask.rs:5`. Assert any formatted API error/log strings mask VWorld/account tokens before surfacing to UI/logs.
42. **`test_save_open.cpp:5686` (`emptyGpkgWithoutWorkspace_opensWithoutDomainLayers`)** — *PARTIAL*: `crates/strata-app/src/gui_tab_home_open.rs:30`. Assert opening a valid empty `.gpkg` via `open_survey_gpkg` populates zero empty domain layers in the left panel.
43. **`test_save_open.cpp:5737` (`openWhileDrawing_resetsCaptureBeforeReplacingLayers`)** — *PARTIAL*: `crates/strata-app/src/gui_tab_home_open.rs:30`. Assert opening another survey mid-polygon cancels active sketch vertices and resets tool to Pan before swapping layers.
44. **`test_save_open.cpp:5013` (`saveAndReopen_keepsTreeGeometryAttributesAndStyle`)** — *PARTIAL*: `crates/strata-app/src/app_state_projects.rs:136`. Assert save and reopen keeps all domain geometry, attribute fields, and layer tree membership accurately.
45. **`test_save_open.cpp:4950` (`samplesOpenAsSurveys`)** — *PARTIAL*: `crates/strata-app/src/gui_tab_home_open.rs:30`. Assert all tracked demo/sample `.gpkg` fixtures open via `open_survey_gpkg` with zero load errors.
46. **`test_save_open.cpp:4110` (`mapContextMenu_onlyOffersRecordEditingForSurveyFeatures`)** — *PARTIAL*: `crates/strata-app/src/gui_tab_map_context.rs:47`. Assert right-clicking over a non-survey reference polygon omits 「이 도형 기록 입력」.
47. **`test_save_open.cpp:3807` (`selectTool_picksTheShapeDrawnOnTopAndKeyAStartsIt`)** — *PARTIAL*: `crates/strata-app/src/feature_pick.rs:80`. Assert pressing shortcut `A` switches to Select/Attribute mode and picks the topmost overlapping polygon.
48. **`test_save_open.cpp:3227` (`cadastralGroupContextMenuCanRemove`)** — *PARTIAL*: `crates/strata-app/src/app_state_layer_remove.rs:85`. Assert triggering Cadastral group context-menu Remove action deletes the group and its children.
49. **`test_save_open.cpp:3258` (`referenceGroupMenuRemovesEveryRowAndCtrlZRestores`)** — *PARTIAL*: `crates/strata-app/src/app_state_layer_remove.rs:85`. Assert Reference group context-menu Remove deletes every child row and single Ctrl+Z restores the full group.
50. **`test_save_open.cpp:3153` (`drawnShapeShowsItsRecordInTheInspector`)** — *PARTIAL*: `crates/strata-app/src/app_state_feature_card.rs:26`. Assert finishing a shape populates inspector card fields and synchronizes changes into record form window.
51. **`test_save_open.cpp:3174` (`cardKeysLeaveTheSketchAlone`)** — *PARTIAL*: `crates/strata-app/src/app_state_feature_card.rs:26`. Assert typing `Z`/`Escape`/digits in the right card input leaves an active polygon sketch's vertices untouched.
52. **`test_save_open.cpp:2935` (`trenchPresetRespectsAreaAndDimensions`)** — *PARTIAL*: `crates/strata-geo/src/trench_generate.rs:25`. Assert selecting each trench dialog preset updates width, length, and count inputs coherently.
53. **`test_save_open.cpp:2868` (`changingLabelFontKeepsEveryLayerAndItsVisibility`)** — *PARTIAL*: `crates/strata-app/src/app_state_layer_menu.rs:64`. Assert changing label/display font settings preserves every layer row and its check state.
54. **`test_save_open.cpp:1778` (`provinceChipMovesMapWithoutLoadingLayers`)** — *PARTIAL*: `crates/strata-app/src/gui_shell_tests_b.rs:55`. Assert clicking each province locator chip moves map center without mutating `left_layers.entries.len()`.
55. **`test_save_open.cpp:1517` (`mapControlsZoomTheCanvasAndShowAScaleBar`)** — *PARTIAL*: `crates/strata-app/src/gui_tab_map_geom.rs:20`. Assert clicking +/- overlay controls alters canvas meters-per-pixel and updates scale bar text coherently.
56. **`test_save_open.cpp:1456` (`recordToolsWaitForASurvey`)** — *PARTIAL*: `crates/strata-app/src/app_state_draw_entry.rs:60`. Assert all record-editing chips/menus remain disabled and ignore clicks before any survey is open.
57. **`test_save_open.cpp:5286` (`drawingStudioPrintSplitsTheDrawingIntoSheets`)** — *PARTIAL*: `crates/strata-layout/src/sheet_scalebar.rs:37`. Assert print tiling dialog matches `compute_tile_grid` rows/cols cleanly when switching paper sizes.
58. **`test_save_open.cpp:5351` (`ribbonPrintOpensTheDrawingThenThePrintWindow`)** — *PARTIAL*: `crates/strata-app/src/dlg_drawing_print.rs:24`. Assert clicking ribbon Print chip navigates from Map to Drawing tab and opens Print modal.
59. **`test_save_open.cpp:5394` (`scaleControlFollowsActiveDrawingWithoutChangingMapCanvas`)** — *PARTIAL*: `crates/strata-app/src/app_state_status_bar.rs`. Assert changing sheet scale in Drawing Studio updates status bar scale without altering map viewport zoom.
60. **`test_save_open.cpp:5478` (`scaleControl_isSingleWidgetAcceptingBothForms`)** — *PARTIAL*: `crates/strata-app/src/status_bar_text.rs:258`. Assert scale text parser accepts both `"1:1000"` and `"1000"` and applies the same scale denominator.
61. **`test_save_open.cpp:2606` (`drawingStudio_largeScaleChipsApplyToMapAndInput`)** — *PARTIAL*: `crates/strata-layout/src/sheet_drawing.rs:119`. Assert clicking Drawing Studio large-scale preset chips updates both frame scale and text field.

### 4. Looks and layout (17 items)

62. **`test_save_open.cpp:1484` (`ribbonButtonsAllHaveDifferentIcons`)** — *MISSING*: `crates/strata-app/src/svg_path_paint.rs:82`. Assert no two distinct ribbon action buttons share the same SVG icon identifier.
63. **`test_save_open.cpp:573` (`compactLayerListKeepsHierarchyAndMapLabelSize`)** — *PARTIAL*: `crates/strata-app/src/left_layer_panel.rs`. Assert narrow left panel preserves parent/child hierarchy indent and map label text sizing.
64. **`test_save_open.cpp:648` (`layerInformationControlsIntegrateWithMapAndDrawingStudio`)** — *PARTIAL*: `crates/strata-app/src/app_state_layer_menu.rs:64`. Assert layer color/opacity/label changes synchronize visually with Map and Drawing legend swatch state.
65. **`test_save_open.cpp:888` (`everyTabFitsA1920ScreenInPaper`)** — *PARTIAL*: `crates/strata-app/src/gui_ribbon_consts.rs:51`. Assert every top-level tab fits inside a 1920x1080 headless pass with zero negative/clipped panels.
66. **`test_save_open.cpp:1012` (`monitorFitDialogs`)** — *PARTIAL*: `crates/strata-app/src/dialogs_download_print.rs`. Assert all modal windows clamp inside viewport rect cleanly on 1280x720 and 1920x1080 frames.
67. **`test_save_open.cpp:1043` (`homePageShowsStrataHeroWithoutOpeningASurvey`)** — *PARTIAL*: `crates/strata-app/src/app_state_lifecycle.rs`. Assert cold boot Home frame paints Strata hero heading cleanly with no open survey.
68. **`test_save_open.cpp:1085` (`drawingToolsGetTheirOwnRowBelowTheRibbon`)** — *PARTIAL*: `crates/strata-app/src/gui_ribbon_draw_row.rs:126`. Assert opened draw sub-toolbar row paints strictly below ribbon bottom Y without overlapping map top.
69. **`test_save_open.cpp:1199` (`narrowWindowKeepsSearchOnTheRibbonRow`)** — *PARTIAL*: `crates/strata-app/src/gui_ribbon_consts.rs:51`. Assert at compact width the ribbon search control stays on the main ribbon band.
70. **`test_save_open.cpp:1266` (`narrowWindowKeepsDrawingAndPrintVisible`)** — *PARTIAL*: `crates/strata-app/src/gui_ribbon_consts.rs:51`. Assert at narrow width both Drawing and Print ribbon chips remain inside screen width with positive width.
71. **`test_save_open.cpp:1332` (`subToolbarFitsAt1904WhileSketching`)** — *PARTIAL*: `crates/strata-app/src/gui_ribbon_draw_row.rs:126`. Assert sub-toolbar width stays within 1904 px during active polygon/line drawing.
72. **`test_save_open.cpp:1422` (`unsavedSaveChipLabelTurnsBoldAndBack`)** — *PARTIAL*: `crates/strata-app/src/app_state_status_bar.rs:103`. Assert dirty state changes Save chip emphasis (`*` / bold styling) and restores normal styling after save.
73. **`test_save_open.cpp:1561` (`topographicActualWindowRemainsStableAfterCompletion`)** — *PARTIAL*: `crates/strata-app/src/dialogs_download_print.rs`. Assert topographic dialog bounds remain stable when download finishes.
74. **`test_save_open.cpp:1699` (`topographicDownloadKeepsMapVisibleAndUsesSurveyFolder`)** — *PARTIAL*: `crates/strata-app/src/dialogs_download_print.rs`. Assert open topographic dialog keeps map canvas visible and targets `<survey_dir>/수치지형도`.
75. **`test_save_open.cpp:1878` (`drawingStudio_decorationsMoveAndShrink`)** — *PARTIAL*: `crates/strata-layout/src/sheet_drawing.rs:125`. Assert interactive drag/shrink clamps decorations inside valid paper margins.
76. **`test_save_open.cpp:2019` (`drawingStudio_northArrowRendersOnOpen`)** — *PARTIAL*: `crates/strata-layout/src/sheet_layout.rs:266`. Assert opening Drawing Studio tab emits non-empty painter geometry for the north arrow.
77. **`test_save_open.cpp:2667` (`drawingInspectorUsesHeightWithoutOverlapping`)** — *PARTIAL*: `crates/strata-app/src/gui_tab_drawing_frame.rs:73`. Assert Drawing Studio right inspector cards have non-overlapping rects across short and tall heights.

---

## Blocked

Grouped by missing feature (90 tests total):

- **Companion `.qgz` workspace read/write, recovery snapshots, and path repair (23)**:
  - `test_storage_safety.cpp:111` `recoverySnapshot_preservesPendingEditsAndSource`
  - `test_storage_safety.cpp:383` `atomicWorkspace_lockedTargetPreservesFileAndUnsavedState`
  - `test_storage_safety.cpp:409` `atomicWorkspace_lockedBackupPreservesBothFiles`
  - `test_storage_safety.cpp:436` `atomicWorkspace_serializationExceptionKeepsFileAndState`
  - `test_storage_safety.cpp:457` `recoverySnapshot_layerFilterSkipsUnlistedVectors`
  - `test_storage_safety.cpp:494` `recoveryPending_notesNewestAndPrunesOldCopies`
  - `test_storage_safety.cpp:527` `absorbLeavesReferenceVectorsOutsideTheSurvey`
  - `test_storage_safety.cpp:586` `persistWorkspace_secondLayerCommitFails_keepsEditsAndRecovery`
  - `test_storage_safety.cpp:674` `persistWorkspace_missingExternal_keepsEditsAndRecovery`
  - `test_storage_safety.cpp:794` `persistWork_writesCompanionQgzWithoutMainWindow`
  - `test_storage_safety.cpp:880` `persistWork_missingParentRelocatesToFallback`
  - `test_storage_safety.cpp:913` `extractEmbeddedReferenceVectors_shrinksSurveyAndKeepsDomain`
  - `test_storage_safety.cpp:995` `repairPersisted_restoresDroppedWindowsHome`
  - `test_storage_safety.cpp:1036` `persistWorkspace_keepsExternalHeritagePathOnReopen`
  - `test_storage_generation.cpp:94` `persistWorkspace_lateFailure_neverCommitsOriginal`
  - `test_storage_generation.cpp:124` `persistWorkspace_blockedLayer_reportsPartialOriginalUpdate`
  - `test_storage_generation.cpp:240` `persistWork_companionFailureMarksStaleWorkspace`
  - `test_save_open.cpp:4356` `embeddedOpenRestoresDemInActualTree`
  - `test_save_open.cpp:4505` `savedWorkspaceNeverPointsIntoTheSaveGenerationFolder`
  - `test_save_open.cpp:5241` `open_repairsRegistryOnlyLayers`
  - `test_save_open.cpp:5542` `partialCommit_preservesRemainingBufferAndCreatesIndependentRecovery`
  - `test_save_open.cpp:6027` `saveAs_embeddedFailureRestoresSourcesAndRemainsUnsaved`
  - `test_save_open.cpp:6173` `save_companionFailureKeepsEmbeddedWorkspaceAndWarns`
- **SessionLog, GDAL CPL error logger, and C++ static source guards (15)**:
  - `test_catch_log.cpp:236` `defaultMaxBytes_is10MiB`
  - `test_catch_log.cpp:246` `sessionLog_writesLine`
  - `test_catch_log.cpp:261` `sessionLog_rotatesWhenOverMax`
  - `test_catch_log.cpp:279` `srcCatchHandlers_allLog`
  - `test_catch_log.cpp:286` `srcAppNestedEventLoops_onlyAllowlisted`
  - `test_gdal_error_log.cpp:53` `dumpHint_namesCrashDump`
  - `test_gdal_error_log.cpp:60` `onlyFailuresAreRecorded`
  - `test_gdal_error_log.cpp:71` `failuresReachTheSessionLogAndRepeatsFold`
  - `test_gdal_error_log.cpp:91` `failuresAreNotRepeatedOnTheTerminal`
  - `test_gdal_error_log.cpp:109` `uninstallRestoresPreviousAndInstallWorksAgain`
  - `test_gdal_error_log.cpp:130` `nestedCplErrorFromPreviousDoesNotRecurse`
  - `test_log_session.cpp:61` `sessionLog_keepsSeveralRotations`
  - `test_log_session.cpp:103` `sessionLog_headerNamesProcess`
  - `test_log_session.cpp:155` `portableBundle_logsBesideItsConfig`
  - `test_log_session.cpp:172` `logExcept_namesSourceAndMessage`
- **Map overlap-erase / split / vertex-edit / clear-all GUI commands & UndoGroup cross-layer tracking (14)**:
  - `test_editing_history.cpp:42` `stampsOrderEditsAcrossLayers`
  - `test_editing_history.cpp:97` `saveThatDropsTheBufferKeepsWrittenCommandsCommitted`
  - `test_editing_history.cpp:130` `redoPrefersTheLastUndone`
  - `test_storage_generation.cpp:154` `persistWorkspace_saveLeavesCommandsCommittedNotDiscarded`
  - `test_undo_group.cpp:63` `closingBeforeAnotherEdit_keepsTwoSteps`
  - `test_save_open.cpp:3364` `ctrlZRestoresVertexEditsAndGroupedFeatureDeletion`
  - `test_save_open.cpp:3435` `clearDrawnFeaturesLeavesOtherLayersSelectionAlone`
  - `test_save_open.cpp:3494` `eraseOverlap_cutsTheShapeUnderTheSelectedOneAndCtrlZRestores`
  - `test_save_open.cpp:3592` `eraseOverlap_acrossLayersUndoesAndRedoesTogether`
  - `test_save_open.cpp:3647` `eraseOverlap_severalShapesAskBeforeRemovingThem`
  - `test_save_open.cpp:3700` `eraseOverlap_secondPressUsesTheShapeJustDrawn`
  - `test_save_open.cpp:3745` `drawnShapeOnTopOffersEraseWhenFinished`
  - `test_save_open.cpp:3854` `selectTool_rightClickErasesAndKeyAEndsSplitMode`
  - `test_save_open.cpp:3922` `polygonCommands_leaveReferenceAndCadastralShapesAlone`
- **GeorefBackup scan backup/restore & pixel window reader (6)**:
  - `test_georef_backup.cpp:69` `plainScan_moveThenRestore_leavesNoTrace`
  - `test_georef_backup.cpp:121` `geotiff_restoreBringsBackEmbeddedCoordinates`
  - `test_georef_backup.cpp:195` `pamWithGeoref_isBackedUpAndRestored`
  - `test_georef_backup.cpp:203` `uniqueOutputPath_neverReusesAnExistingFile`
  - `test_georef_backup.cpp:216` `backupRoot_isInsideTheSurveyFolder`
  - `test_georef_backup.cpp:225` `readPixelWindow_readsOriginalPixels`
- **Multi-monitor Qt window clamping & UI state INI persistence (5)**:
  - `test_save_open.cpp:864` `windowGeometryFitsLogicalMonitors`
  - `test_save_open.cpp:873` `mainWindowFitsAvailableScreenOnShow`
  - `test_save_open.cpp:927` `monitorFitHomeMapDrawing`
  - `test_save_open.cpp:1140` `firstStartIsMaximizedAndAChosenSizeIsKept`
  - `test_save_open.cpp:1153` `filesBoxStartsOpenAndKeepsTheChosenState`
- **Portable USB drive-letter tracking, recent-bundle migration & removed auto-restore flag (5)**:
  - `test_recent.cpp:27` `recentSurveyFollowsUsbDriveLetter`
  - `test_recent.cpp:135` `remember_tabInNameDoesNotShiftFields`
  - `test_recent.cpp:151` `forget_removesSurveyShownOnNewDriveLetter`
  - `test_recent.cpp:177` `takeSkipAutoRestore_clearsOneShot`
  - `test_save_open.cpp:4541` `newPortableFolderInheritsRecentSurveys`
- **SurveyDurability Windows Restart Manager & lock diagnosis (4)**:
  - `test_survey_durability.cpp:67` `processesHolding_namesThisAppWhileItHoldsTheFile`
  - `test_survey_durability.cpp:88` `copySurvey_renameRefused_logsWhoHoldsTheFile`
  - `test_survey_durability.cpp:117` `lockDiagnosis_saysWhenItCannotTell`
  - `test_survey_durability.cpp:128` `lockDiagnosis_namesAReadOnlyTarget`
- **FileCleanup::removeWhenFree deferred deletion registry (3)**:
  - `test_file_cleanup.cpp:38` `removeWhenFree_removesAtOnceWhenNobodyHoldsIt`
  - `test_file_cleanup.cpp:46` `removeWhenFree_removesLaterOnceTheReaderCloses`
  - `test_save_handles.cpp:136` `readerAwaitingDeletion_doesNotBlockTheSave`
- **LayerTreeUndo tree snapshot/restore stack & coalesced refresh (3)**:
  - `test_layer_tree_undo.cpp:30` `restorePutsOrderAndChecksBack`
  - `test_layer_tree_undo.cpp:57` `onlyTheUsersChangesBecomeUndoSteps`
  - `test_layer_tree_undo.cpp:92` `layerChangesMergeIntoOneRefresh`
- **Field package export/restore, bundle asset collector, and memory reference vector serialization (4)**:
  - `test_storage_hygiene.cpp:123` `collect_sameSizeDifferentContentIsCopiedAgain`
  - `test_save_open.cpp:5883` `save_fieldPackageKeepsSurveyAreaReadable`
  - `test_save_open.cpp:5983` `closeSave_preservesMemoryReferenceVectorOnReopen`
  - `test_save_open.cpp:6200` `fieldCopy_restoresAllValidLayersAndSavesTree`
- **EditBufferSummary added/changed/deleted breakdown & layer filter (2)**:
  - `test_edit_buffer_summary.cpp:66` `addedChangedDeleted`
  - `test_edit_buffer_summary.cpp:87` `ignoresLayersWithoutEditBuffer`
- **Drawing Studio async heritage cache & split above-labels renderer (2)**:
  - `test_save_open.cpp:2125` `drawingStudio_heritageRefreshRequestsCoalesceAndReuseOnRevisit`
  - `test_save_open.cpp:2434` `drawingStudioAboveLabelsMapDrawsGeometryWithoutDuplicateNumberLabels`
- **BlockingTaskDialog async cancel-wait UI (2)**:
  - `test_blocking_task.cpp:62` `cancel_keepsTheDialogUntilTheWorkStops`
  - `test_blocking_task.cpp:77` `escape_cancelsAndKeepsTheDialog`
- **Deferred GeoTIFF ribbon export & compile-time closed WorkCrs enum (2)**:
  - `test_save_open.cpp:838` `geoTiffButtonOnlyEnabledOnMapTab`
  - `test_storage_safety.cpp:298` `newSurvey_invalidCrsLeavesNoPartialFiles` (`WorkCrs` enum forbids unknown CRS strings at compile time)

## Could not determine

1. **`check-tests.py` reports `count test_save_open.cpp: 91 rows, 2 test slots in the file`**: `docs/senior/tools/check-tests.py` line 33 breaks out of its `private slots:` scan on the first line starting with `};` (`if in_slots and s.startswith("};"): break`). In `target/cpp-ref/tests/test_save_open.cpp`, line 632 contains a closing struct literal `};` inside an inline test helper method before line 648 (`layerInformationControlsIntegrateWithMapAndDrawingStudio`), so the helper stops counting after the first 2 slots (`compactLayerListKeepsHierarchyAndMapLabelSize` and `layerInformationControlsIntegrateWithMapAndDrawingStudio`). All 91 C++ test slots in `test_save_open.cpp` are listed above.
2. **`test_storage_safety.cpp:298` (`newSurvey_invalidCrsLeavesNoPartialFiles`)**: Classified as `BLOCKED` rather than `MISSING` because `strata_types::crs::WorkCrs` is a closed 2-variant enum (`Epsg5186 | Epsg5187`), so arbitrary invalid CRS strings cannot cross the `create_new_survey` API boundary at runtime (unlike C++ `QString`).
3. **`test_save_open.cpp:838` (`geoTiffButtonOnlyEnabledOnMapTab`) and `test_recent.cpp:177` (`takeSkipAutoRestore_clearsOneShot`)**: Classified as `BLOCKED` because both reflect intentional Rust port differences: GeoTIFF export is currently deferred/disabled on all tabs (`지시 R7`, verified in `geotiff_chip_is_deferred_r7_and_click_does_nothing`), and auto-restore was dropped in favor of unconditional startup on Home (`AGENTS.md`).
4. **`test_storage_safety.cpp:827` (`persistWorkspace_missingSurveyFile_writesLocalCopy`)**: Classified as `PARTIAL` rather than `BLOCKED` because `app_state_projects.rs:140` handles a vanished `.gpkg` by refusing the save cleanly (`saving_a_vanished_survey_file_creates_nothing_and_keeps_edits`, `지시 2E`) instead of silently writing a local copy as C++ does.

## Counts

| C++ test file | C++ slots | PORTED | PARTIAL | MISSING | BLOCKED |
| --- | --- | --- | --- | --- | --- |
| test_blocking_task.cpp | 2 | 0 | 0 | 0 | 2 |
| test_catch_log.cpp | 5 | 0 | 0 | 0 | 5 |
| test_edit_buffer_summary.cpp | 5 | 0 | 2 | 1 | 2 |
| test_editing_history.cpp | 7 | 3 | 1 | 0 | 3 |
| test_file_cleanup.cpp | 2 | 0 | 0 | 0 | 2 |
| test_gdal_error_log.cpp | 6 | 0 | 0 | 0 | 6 |
| test_georef_backup.cpp | 6 | 0 | 0 | 0 | 6 |
| test_layer_tree_undo.cpp | 3 | 0 | 0 | 0 | 3 |
| test_log_session.cpp | 6 | 0 | 1 | 1 | 4 |
| test_recent.cpp | 13 | 7 | 2 | 0 | 4 |
| test_save_handles.cpp | 4 | 0 | 3 | 0 | 1 |
| test_save_open.cpp | 91 | 15 | 45 | 4 | 27 |
| test_schema_version.cpp | 4 | 3 | 1 | 0 | 0 |
| test_storage_generation.cpp | 7 | 0 | 3 | 0 | 4 |
| test_storage_hygiene.cpp | 3 | 0 | 1 | 1 | 1 |
| test_storage_safety.cpp | 25 | 2 | 6 | 2 | 15 |
| test_survey_durability.cpp | 4 | 0 | 0 | 0 | 4 |
| test_survey_save_as.cpp | 2 | 0 | 1 | 1 | 0 |
| test_undo_group.cpp | 3 | 1 | 1 | 0 | 1 |
| **Total** | **198** | **31** | **67** | **10** | **90** |
