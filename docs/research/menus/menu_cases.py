"""Reviewed functional cases. Generated discovery/reports stay under .codex.

File-level references in unverified cases are starting points, not parity proof.
Conditions and return behavior describe the scope still needing investigation.
Only explicitly scoped production-UI cases can be complete; digests are frozen
when their evidence is reviewed, never recomputed by the report generator.
"""
import re

REVIEWED_DIGESTS = {
    "target/cpp-ref/src/app/MainWindowOffline.cpp": "d9d3a872cb39ab21a45f910703838441699d78bd3d7abdb77c49b2f1ba3df9f2",
    "crates/strata-app/src/gui_ribbon_menu_actions.rs": "6e9f51cc99384fb501703fd07a4751ca9f0324a275845a8998b53b7b9b4143cc",
    "crates/strata-app/src/gui_ribbon_menus_a.rs": "fd52bf69b813365fe2c08db65baa92eecb8f9ba8136b818cb1f71a4861e7dd39",
    "crates/strata-app/src/gui_ribbon_menus_b.rs": "d03a1d4fcfe7a325973ee389e1f4466b8465c63e1925b78d875d4b9fe453df48",
    "crates/strata-app/src/ribbon_state.rs": "5c6d8158c2c27c0e9a3af3e3e2706b25826dcb7f6418d1fb23c35f1cbfc7d849",
    "crates/strata-app/src/app_state_lifecycle.rs": "6a101d0cbb00120fba385d37f284a1b2288095dd3fa79363815ce0d9bb20a6c8",
    "crates/strata-app/src/ref_tile_history.rs": "9604e7e16f6df86d117c5d1daff3392083616fe026030bb10f96b49fbb9fb303",
    "crates/strata-app/src/old_map_input_tests.rs": "e434d2a27536d05bf45b3791c2c5c2207220fecd30776f5212c7c32ee7da6423",
    "crates/strata-app/src/map_wheel_zoom_tests_c.rs": "ff71283f35722327c763eef80909120b2afbfd3eddd32af5dc63ba9e98887f1f",
    "crates/strata-app/src/app_state_buffer_save.rs": "7caffda784665f3bdba5389250ad9cbb95fabd096258dc58cfa9e56ab3c62e15",
    "crates/strata-app/src/app_state.rs": "065816726ef2adda4f4a845ef634d4e32bb23cf6f26d822038b7f43738cda3d4",
    "crates/strata-app/src/real_survey_types.rs": "17a17ae02492972420f85643d45eb02a789a96be1a0854f38bdcbf29cbab99f9",
    "crates/strata-app/src/buffer_save_input_tests.rs": "b947a5cdf11c836127e512c35bc35dd4fde45f534021490bb7ed07d4ce96d427",
    "crates/strata-app/src/buffer_save_input_tests_a.rs": "4b92a76601d9e844bbbb92bd8a7d88725596ff42588aaa13a139d191dcdbf0ac",
    "crates/strata-types/src/buffer.rs": "f5582a77149d2ce8e019f3c387386a66ccf48ce94eeb43611c1ec31d9a0f5492",
    "crates/strata-storage/src/survey_gpkg_save.rs": "47a0685dcea61a7ab1f6dc3b9e8ac52d9f4ab67cc70218268bfa64447c79aeff",
    "crates/strata-storage/src/survey_gpkg_buffers.rs": "036a8f12cfef2b7e26fc152970aefe8093bd3b862bfbd967de483ecb607834ea",
    "crates/strata-storage/src/survey_gpkg_buffers_tests.rs": "47f55d4d4d2175f52d2d4ec713e2d69dbba08fe58e057fa78bc9f645f1a6daed",
    "target/cpp-ref/src/core/SurveyStorage.cpp": "83efc38106c63424c05a2c6f95c66a8538d1da61872c675f08bbe92a1a3dce57",
    "crates/strata-app/src/app_state_buffer.rs": "0770a23334564133a086287aabbf9617819430f7f37b19c614f93aa4756835a4",
    "crates/strata-app/src/buffer_crs_input_tests.rs": "64fc108063a810c1a0fce5615a295929a9998882a77b312bcd19f001312ba27b",
    "crates/strata-app/src/buffer_crs_input_tests_a.rs": "90f67775090b1407983d9337a6d22651b658051db9fd378134ad5622bec3b7f0",
    "crates/strata-app/src/gui_ribbon_subtoolbar.rs": "467c2d2fdc9fe4f3a903fc4adce1bc76f42e611237ffab927262c57abbd67625",
    "target/cpp-ref/src/app/MainWindowRibbon.cpp": "c954bede33a7ca5ae6bf09d461bcaa0bb258b0376164bdd5b9e9e587e9bbc39d",
    "target/cpp-ref/src/core/BufferAnalysis.cpp": "9795f866d6fbc023a64c742a29e1a61bbb4922af04f900cf586935c961930f58",
    "target/cpp-ref/src/core/NearbyHeritageCollect.cpp": "12623197b9872e362d5dd174dbdf7c1938e19d223ee2bbb8af1614645699b7fc",
    "target/cpp-ref/src/app/MainWindowEditing.cpp": "bf15c3e25220aae49894159fe0d314249ec82709e61dc9b5a5c60436bd33492c",
    "crates/strata-app/src/app_state_draw_commit.rs": "033d2c1f7207a56fc3369799df0b0b9b012758c8d0caf1586e82d744de846eba",
    "crates/strata-app/src/app_state_split.rs": "98b3643ea77207403f7c1ccb6c1f1d5e7966a9046fa0df4855828766d06fb233",
    "crates/strata-app/src/split_crs_input_tests.rs": "59ec0c651a218a4e790f85aeec94774ebb01cb9ffdd16236d41926054a1be15c",
    "crates/strata-app/src/gui_ribbon_draw_row.rs": "b2f12457aee3cc8a4666db4ae9cdb2004e55fd25d460c7a313f1eada73cf11a8",
    "crates/strata-app/src/app_state_draw_row.rs": "5b951af734c6f5fcd6fd4d73f19f9e0030d0fce528a15cebf232c654cbae3de9",
    "crates/strata-app/src/gui_layer_panel_rows.rs": "1955ac7e8659910c4854ae3a5684db2434175d922b961cd4d8b15ad5be5955ea",
    "target/cpp-ref/src/core/LayerOps.cpp": "73a209387bcb18992e18016c6610092f7c8a1d9fcedd0da7fcaa03bb9d568afb",
    "target/cpp-ref/tests/test_buffer.cpp": "dd3489849ae884803e2f10a69e87cf82afcfa2829215c7a45edcef108440880c",
    "crates/strata-app/src/app_state_draw_commit_b.rs": "5bbaafac5122c0e71f7ea0d04a08c25d550cbf6eac2148bf5717376d91102bc6",
    "crates/strata-app/src/app_state_draw_entry.rs": "51eaffd7e767f9f2d09c34b9aed575ac8c6468a46328ab23352d0a6cfe749454",
    "crates/strata-app/src/gui_tab_map_capture.rs": "b474189a44b9865429f5cbf07e8c22a5800f8330741f8435566e517f2760e530",
    "crates/strata-app/src/app_state_feature_form.rs": "f5fbe97534c6feee1c3f329352cf782c94be126ad2caff390310030ddd05e999",
    "crates/strata-app/src/dlg_feature_form.rs": "a15cc6b65ee56af0b436034fb7cd4d878b1c461360c83913fa612a165cb262bb",
    "crates/strata-app/src/capture_crs_input_tests.rs": "6d8bbc6ab7f292ebb32d85f7bb526600119ef4751c0ae3cd29dfae8088968a1f",
    "crates/strata-app/src/feature_pick.rs": "9c447528a2ed904073eb7b186c3f56cfe1c04586281856c98f10c18a206f4c80",
    "crates/strata-app/src/gui_tab_map_select_box.rs": "88749469a5b8951255995f05ec2d52f302910dd2347e20061aff38e702db1a74",
    "crates/strata-app/src/vertex_crs_input_tests.rs": "83912408f3dafe361155c53d84afa04302d19c71c0f65817befb6695771ffdf5",
    "crates/strata-app/src/vertex_crs_boundary_tests.rs": "402db9986108cd79d44ac7977a91225e79507ac78f7522ae2566800993d13754",
    "crates/strata-engine/src/crs.rs": "13d686ca704937b161e506a0cb66866f6f193d7bca0d256a520ade9d4b036050",
    "target/cpp-ref/src/app/KaClosePrompt.cpp": "021bedf877bdf48a831e443be4849073e145c68dff3645cc9f640f8cdd948239",
    "target/cpp-ref/src/app/KaFeatureCardEditors.cpp": "dee08bcfaa93e932c7d36b8b4122c852a035c503fbcefb777ee07fbf66f4611c",
    "crates/strata-app/src/shell_dialogs.rs": "7124bfa16caafa356a31b7cd661c7b985c50bc26cf16e2cb641ab956efb05ec1",
    "crates/strata-app/src/close_prompt_input_tests.rs": "3529546d1c7a0ba19e47aabf75d473c2b447f86e4b43f497462b16dcecc84a0f",
    "crates/strata-app/src/close_prompt_input_tests_a.rs": "a30f97f722046d75d4b339b44cfa01651f61df92c0da9e58f0fc2f7b23b5b422",
    "target/cpp-ref/src/app/KaFeatureCard.cpp": "68c12c471bb1005b5df50e2101fefb5fa17f8032de080b440494607fe3a02a51",
    "target/cpp-ref/src/app/MainWindowSession.cpp": "db442da24cf55cee4c4c92c1b8a8dc170a9cdff32579b092b429f7a6d72d5600",
    "crates/strata-app/src/save_before_input_tests.rs": "4ac9700b2a45e6b89a1d8b529bf4bc856f2308acb26df733f3368ef0eb763624",
    "crates/strata-app/src/gui_tab_home_open.rs": "aedb5d4671946a64d2ebb6b00cf53796923e7a1ae4560bedc3409c9f6d1bac2d",
    "crates/strata-app/src/app_state_projects.rs": "8bc0b3e652dbf1784c91b32afd32e12adeb6b528e1ecb047e5f75fb67bff24c3",
    "crates/strata-app/src/app_state_feature_card.rs": "632cc06bc925503098433bb043a469edfb824961970ae39b357ed06b5fd11633",
    "crates/strata-app/src/app_state_feature_record.rs": "24cb20bba6a40851e47eee8d7f168a28146af90e60529f09c5ed2947501bd088",
    "crates/strata-app/src/app_state_save_before.rs": "c971003cf080619fbf81efa7a1a7ccd0ab209ab8534ac7626c08f372699c89c8",
    "target/cpp-ref/tests/test_editing_tools.cpp": "1ec635252608aced322649d6b560cdd92a51d8ac9e423fe012f22eafe839d7df",
    "target/cpp-ref/src/app/KaVertexEditTool.cpp": "55d536e65c3e67cf4e4294dbe46b7f6361290573abcd7a274f368cb833d8d8c8",
    "crates/strata-app/src/vertex_key_rejection_tests.rs": "b99c96aae266debd7c8859affa684ee4f8bd8378050486ea091ff0ecbd51e186",
    "crates/strata-app/src/app_state_vertex_edit_tests.rs": "2b4304098e17bf9965bfce1e00135483efec3c679ad93a87533e6d3c3b0097c7",
    "crates/strata-app/src/app_state_vertex_edit.rs": "47e19c3141c76ce214e3694bc3056926a2baf4bd11e9195f50004069787c5c20",
    "target/cpp-ref/src/app/MainWindowUndo.cpp": "2c11706d9c729bad7f54cca430f187d52558cb0787011e24351dd964817fe0c3",
    "target/cpp-ref/src/app/KaFeatureSelectTool.cpp": "0bc78d26e70186422c02204c0297aca181953671048efaa7a47ebecf70381a69",
    "target/cpp-ref/tests/test_save_open.cpp": "2f84bbad9cf53fca98f07fd98117f3234eeeb450f2864bef33bd20c5592fec2b",
    "target/cpp-ref/src/app/MainWindow.cpp": "017360b61d6204525a527591140169fc1fb64596335cdf920a5a8dd6fe75eb19",
    "crates/strata-app/src/menu_selection_keys_tests.rs": "c54de8904932f32f702074cdeec77f8d4ae33e118386ce66d5b7aff8e5d09021",
    "crates/strata-app/src/menu_capture_keys_tests.rs": "4b60110121a0c63830d04308e281d67779f9f0ee35265ef2b4e260801bda8658",
    "crates/strata-app/src/popup_input_harness.rs": "12bc498a781ee060c639507ff22bd1ee5bf67603f2bbc96a5e103070f81a4371",
    "crates/strata-app/src/shell_workspace.rs": "b2d833287f100313db7a19a66ebb4cb78b283b1ed123afbd5017471edccc8142",
    "crates/strata-app/src/gui_shell.rs": "1d34bf95ec64d486e90247513e989873ab4beaaac8246279ffaed1278d77931e",
    "crates/strata-app/src/gui_shell_escape.rs": "37a738f061a24b7bc01fac480a840e09dc93abf53162e56d1da0b70cbd2c45e7",
    "crates/strata-app/src/native_menu_input.rs": "03e8b7fe31bee033005d145c058f6d59779593758ba2245f4bb51e7c6704f216",
    "crates/strata-app/src/native_popup_escape_tests.rs": "cc6778efc5f5aa6dd97f2b2bd2ce1ffcd7d282fd30964de874f6e503e90b0f9c",
    "crates/strata-app/src/popup_escape_tests.rs": "2ea0dca3e1aaaf32e2deda81d43ce77a8613ab2cf543c0a8c3bea71e8fae5323",
    "target/cpp-ref/src/app/KaCaptureMapTool.cpp": "d0da08cc04d7cc2bd829a4586ebddd4b25edf2a230497460feb4010fac5e555c",
    "target/cpp-ref/src/app/KaSnapSettingsWidget.cpp": "865ded2f58213293eff9a0522a89bf58362a496ab0d8c1a7dac0824fd8b7e990",
    "target/cpp-ref/src/app/MainWindowContextMenus.cpp": "674a137ae3530ba8811d7d519d91954ce451ac3b8887a93ae40006b0ed7f4362"
}


def build_cases(root):
    def ref(file, needle=None):
        if '/' not in file:
            folder = 'target/cpp-ref/src' if file.endswith(('.cpp', '.h')) else 'crates/strata-app/src'
            found = list((root / folder).rglob(file))
            if len(found) != 1:
                raise ValueError('ambiguous/absent reference: ' + file)
            path = found[0]
        else:
            path = root / file
        lines = path.read_text(encoding='utf-8-sig').splitlines()
        number = 1 if needle is None else next((i for i, line in enumerate(lines, 1) if needle in line), None)
        if number is None:
            raise ValueError('absent anchor: ' + file + ' / ' + needle)
        return path.relative_to(root).as_posix() + ':' + str(number)

    rows = []

    def add(id, path, priority, status, cpp, rust, conditions, remaining, cancel='취소·닫기·이전 도구 복귀 및 데이터 보존 재대조 필요', runtime=None):
        rows.append({'id': id, 'path': path, 'priority': priority, 'status': status,
                     'cpp': [ref(*x) if isinstance(x, tuple) else ref(x) for x in cpp],
                     'rust': [ref(*x) if isinstance(x, tuple) else ref(x) for x in rust],
                     'conditions': conditions, 'cancel_return': [cancel], 'remaining': remaining,
                     'runtime': runtime or [], 'candidate_ids': []})

    add('file.new', '조사 > 새 조사 / Ctrl+N', 0, '부분 구현', [('MainWindowRibbon.cpp', 'actNew')],
        ['app_state_save_before.rs', 'app_state_save_before_tests.rs'], ['미저장 변경 유무', '저장 성공·실패', '처리 중 중복 요청'],
        ['MENU9 저장 질문의 입력 소유권·응답·미저장 글 보존 확인; 새 조사 작성·파일 선택 취소·반복 전환의 전체 UI는 남음'])
    add('file.open', '조사 > 열기 / Ctrl+O / 파일 끌어놓기', 0, '부분 구현', [('MainWindowRibbon.cpp', 'actOpen')],
        ['gui_tab_home_open.rs', 'app_state_save_before.rs', 'user_files.rs'], ['정상·잘못된 GPKG', '미저장 변경', '읽기 전용·없는 경로'],
        ['MENU9 저장 질문의 입력·응답과 임시 파일 열기 요청의 버리기·취소 확인; 저장 후 열기·파일 선택기·모든 진입점·열기 실패 UI는 남음'])
    add('file.recent', '홈 > 최근 조사 / 이어서 열기', 0, '부분 구현', ['KaHomeRecentCard.cpp'],
        ['gui_tab_home.rs', 'app_state_save_before_tests.rs'], ['원본 존재·없음', '미저장 변경', '폴더 이동'],
        ['홈 탐색기 현재 UI와 기존 최근 조사 검사 연결; 원본 없음·복귀 상태 확인'])
    add('file.save', '조사 > 저장 / Ctrl+S', 0, '부분 구현', [('MainWindowRibbon.cpp', 'actSave')],
        ['app_state_projects.rs', 'app_state_save_tests.rs'], ['편집 유무', '임시 FID', '저장 실패·디스크 부족'],
        ['기존 원자적 저장 검사 연결; 모든 실패 상태의 UI와 수정 표시 보존 확인'])
    add('file.save-as', '조사 > 다른 이름 / Ctrl+Shift+S', 0, '검증 안 됨', [('MainWindowRibbon.cpp', 'actSaveAs')],
        ['app_state_projects.rs', 'gui_shell.rs'], ['원본·새 경로', '대상 존재', '취소'], ['원본 보존·레이어 및 외부 자료 경로·저장 후 현재 조사 전환 재대조'])
    add('file.close', '창 닫기 > 저장 / 버리기 / 취소', 0, '부분 구현', [('MainWindow.cpp', 'void MainWindow::closeEvent'), 'KaClosePrompt.cpp'],
        ['shell_workspace.rs', 'shell_dialogs.rs', 'app_state_feature_record.rs'], ['미저장·입력 중·작업 중·모달', '저장 실패·반복 닫기'],
        ['MENU10 논리적 창 닫기와 질문 입력은 확인; 실제 OS 창 종료·다운로드 정리·다른 팝업 조합·Tab 이동은 남음',
         'C++ 메모리 참조 벡터 저장·다시 열기 검사는 읽었으나 이 질문 검사로 이식한 것은 아님'])
    add('file.close-input', '창 닫기 질문 > 첫 요청·입력 소유권·명시적 응답', 0, '완료',
        [('MainWindow.cpp', 'void MainWindow::closeEvent'), 'KaClosePrompt.cpp',
         ('KaFeatureCard.cpp', 'void KaFeatureCard::commitTyping'), ('KaFeatureCardEditors.cpp', 'QEvent::FocusOut')],
        ['shell_dialogs.rs', 'app_state_feature_record.rs', 'shell_workspace.rs', 'gui_shell.rs', 'popup_input_harness.rs',
         'close_prompt_input_tests.rs', 'close_prompt_input_tests_a.rs'],
        ['1680×960·480×640 생산 UI의 실제 논리적 root Close 이벤트', '추적 데모 임시 사본·표 EPSG:5186·작업 EPSG:5187',
         '첫 요청과 Delete·Backspace·Enter·CtrlN 및 진행 중 홈 클릭', '질문 중 Delete·CtrlZ·CtrlY·Esc·배경 클릭',
         '저장·버리기·취소 실제 단추와 기본 Enter·실패 저장', '취소 뒤 정상 편집·반복 닫기와 기존 저장 질문의 단독 소유권',
         '깨끗한 조사와 현재 메모 입력·명시적 저장 뒤 후속 Close 명령'], [],
        '취소는 질문만 닫고 선택·도구·작성점·자료·이력을 보존. 명시적 저장만 임시 사본에 기록하며 저장 실패는 Close를 보내지 않음. 물리적 OS 종료·작업 정리·네이티브 선택기·Tab 순서는 범위 밖',
        [{'kind': 'headless_production_ui', 'evidence': ref('docs/RESULTS.md', '### 닫기 질문의 입력')}])
    add('file.save-before-input', '새 조사·열기 전 저장 질문 > 입력 소유권·명시적 응답', 0, '완료',
        [('MainWindowSession.cpp', 'bool MainWindow::confirmSaveBeforeOpeningSurvey'), ('KaFeatureCard.cpp', 'void KaFeatureCard::commitTyping'),
         ('target/cpp-ref/tests/test_save_open.cpp', 'void open_unsavedWorkRespectsAnswer()')],
        ['app_state_save_before.rs', 'app_state_feature_record.rs', 'app_state_feature_card.rs', 'app_state_projects.rs', 'gui_tab_home_open.rs',
         'gui_shell.rs', 'gui_shell_escape.rs', 'shell_workspace.rs', 'popup_input_harness.rs', 'menu_selection_keys_tests.rs', 'save_before_input_tests.rs'],
        ['1680×960·480×640 생산 UI', '추적 데모 사본·미저장 속성과 실제 메모 입력', 'Delete·CtrlZ·CtrlY·Esc·Enter',
         '임시 파일 열기 요청·중복 CtrlN과 직접 요청', '창 밖 홈 탭 클릭', '새 조사 요청 저장·취소·저장 실패 / 임시 파일 열기 요청 버리기·취소'], [],
        '질문 중 자료·이력·도구·선택 유지; Esc는 질문만 취소하며 정상 키가 재개됨. 명시적 저장만 임시 파일에 기록; 저장 실패는 현재 Rust 질문을 유지. 네이티브 파일 선택기·닫기 질문·Tab 순서·모든 입력 조합은 이 행의 범위 밖',
        [{'kind': 'headless_production_ui', 'evidence': ref('docs/RESULTS.md', '### 저장 전 질문의 입력')}])
    for id, key, label in [('undo', 'Undo', 'Ctrl+Z'), ('redo', 'Redo', 'Ctrl+Y')]:
        add('edit.' + id, '편집 > ' + label, 0, '부분 구현', ['MainWindowUndo.cpp', ('MainWindowRibbon.cpp', 'act' + key)],
            ['gui_shell.rs', 'app_state_layer_remove.rs'], ['선택 도형·레이어', '그리기·정합·도면', '텍스트 포커스', '되돌릴 항목 없음'],
            ['MENU7 기본 레이어 메뉴의 지도 이력과 삭제 뒤 복원 확인; 정합·도면·다른 메뉴·입력칸·저장 전후 분기는 재대조'])
    add('edit.delete-focus', 'Delete / Backspace > 도형·레이어·그리던 점·정합점', 0, '부분 구현', ['MainWindow.cpp', 'MainWindowUndo.cpp'],
        [('gui_shell.rs', 'Key::Delete'), 'app_state_layer_remove.rs', 'menu_capture_keys_tests.rs'], ['메뉴 열림', '텍스트 입력', '선택 없음·여러 선택', '그리기 중'],
        ['MENU6 작성 중 도형·검색칸, MENU7 선택 자료·이력, MENU8 최소 꼭짓점의 삭제 거부 보존을 확인. 정합점·도면·다른 메뉴의 키와 포커스는 남음'])
    add('capture.real-snap', '그리기 > 자석 > 실제 점·선·면 캡처', 0, '미구현', [('KaCaptureMapTool.cpp', 'snapToMap')],
        [('gui_tab_map_capture.rs', 'self.sketch_press(s2w'), 'app_state_draw_commit.rs'], ['자석 켬·끔', '모든 조사·현재 레이어', '원본 CRS', '취소·조사 전환'],
        ['실제 캡처는 원래 좌표를 전달; 별도 legacy CAD 스냅과 연결되지 않음', 'SNAP1A 소유권 준비를 재사용해 실제 입력·미리보기·오래된 결과 폐기 연결'])
    add('edit.nearest-lines-crs', '그리기 > 가까운 선에 붙이기 > CRS 오류', 0, '부분 구현', [('SnapTraceOps.cpp', 'snapGeometryToNearestLines')],
        ['app_state_snap_lines.rs', 'app_state_snap_lines_tests.rs'], ['EPSG:-1·0·999999', '기준 선·점', '정상 대상', '5186→5187'],
        ['SNAP1B 실제 모델 검사와 정상 변환 확인 완료; 해당 메뉴에서의 오류 안내·반복 UI 확인 남음'],
        '잘못된 대상은 도형·이력·변경 상태를 유지; 잘못된 기준 표는 제외')
    for key, title in [('poly', '유구 면'), ('line', '유구 선'), ('point', '유물 위치')]:
        add('capture.' + key, '그리기 > ' + title, 0, '부분 구현', ['KaCaptureMapTool.cpp', 'MainWindowEditing.cpp'],
            ['gui_tab_map_capture.rs', 'app_state_draw_commit.rs'], ['새 조사·기존 조사', '단발·연속', 'Enter·우클릭·더블클릭', '기록 창 취소'],
            ['기존 실제 캡처 입력 검사 연결; 스냅·그리기 행 작은 화면·모든 복귀 상태 포함한 최종 대조'])
    add('key.escape-native', '레이어·작은 리본 메뉴 > 첫 Esc / 다음 Esc', 0, '완료',
        [('MainWindowContextMenus.cpp', 'menu.exec'), ('KaCaptureMapTool.cpp', 'Qt::Key_Escape')],
        ['native_menu_input.rs', 'gui_shell_escape.rs', 'gui_shell.rs', 'shell_workspace.rs', 'popup_input_harness.rs', 'app_state_feature_record.rs', 'popup_escape_tests.rs'],
        ['1024×768 생산 UI', '그리던 점 3개', '반복 열기·하위 메뉴', '별도 메뉴 없는 캔버스의 같은 입력 묶음 Esc+Enter'], [],
        '첫 Esc는 메뉴만 닫고 점 보존; 다음 Esc는 정상 캔버스 취소. 이 행은 검사한 두 루트에 한정',
        [{'kind': 'headless_production_ui', 'evidence': ref('docs/RESULTS.md', '### 기본 메뉴 Esc')}])
    add('key.escape-target', '자석 대상 선택 > 선택 유지·Esc·다시 열기', 0, '완료',
        ['KaSnapSettingsWidget.cpp', ('KaCaptureMapTool.cpp', 'Qt::Key_Escape')],
        ['native_menu_input.rs', 'gui_shell.rs', 'shell_workspace.rs', 'popup_input_harness.rs', 'app_state_feature_record.rs', 'native_popup_escape_tests.rs'],
        ['1680×960 생산 UI', '현재·모든 조사 실제 선택', '같은 묶음의 클릭 해제+Esc'], [],
        '팝업 닫기 뒤 그리던 점과 선택값 유지; 다음 Esc는 캔버스 취소. 실제 스냅 계산 완료를 뜻하지 않음',
        [{'kind': 'headless_production_ui', 'evidence': ref('docs/RESULTS.md', '### 그리기 대상 팝업')}])
    add('key.escape-other', '속성 팝업·선택 상자·색 선택기 > Esc·Enter·Delete', 0, '검증 안 됨', ['MainWindowContextMenus.cpp'],
        ['dlg_feature_form.rs', 'dlg_feature_record.rs', 'ri_attr_card_rows.rs', 'gui_shell.rs'],
        ['메뉴 종류별', '포커스·기록 입력·그리기 중', '반복 열기'], ['기존 MENU3·MENU4·MENU5와 겹치지 않는 종류만 재현'])
    add('key.capture-menu', '작성 중 도형 > 기본 메뉴·자석 대상·검색칸의 Enter / Delete / Backspace', 0, '완료',
        [('KaCaptureMapTool.cpp', 'void KaCaptureMapTool::keyPressEvent')],
        ['gui_shell.rs', 'gui_shell_escape.rs', 'shell_workspace.rs', 'native_menu_input.rs', 'popup_input_harness.rs', 'app_state_feature_record.rs', 'menu_capture_keys_tests.rs'],
        ['1024×768 레이어·기록 메뉴', '1680×960 자석 대상·실제 검색칸', 'EPSG:5187 그리던 세 점', '메뉴 열기 클릭 해제+Delete 같은 묶음'], [],
        '메뉴에서는 점·도형 수·변경 상태 유지; 닫은 뒤 캔버스 키 재개. 검색칸은 기존 포커스 보호 유지. 사용자 데이터 보존 결정이며 Qt 메뉴 내부 단축키와의 동일성 주장이 아님',
        [{'kind': 'headless_production_ui', 'evidence': ref('docs/RESULTS.md', '### 메뉴의 편집 키')}])
    add('key.selection-menu', '기존 선택 자료 > 레이어 메뉴의 Delete·꼭짓점 Backspace / 우클릭 Delete', 0, '완료',
        [('MainWindow.cpp', 'auto* delAct'), ('KaFeatureSelectTool.cpp', 'void KaFeatureSelectTool::keyPressEvent'), ('target/cpp-ref/tests/test_save_open.cpp', 'void layerDeleteKeyPreservesSourceAndUndoRestoresPendingEdits')],
        ['gui_shell.rs', 'gui_shell_escape.rs', 'native_menu_input.rs', 'shell_workspace.rs', 'popup_input_harness.rs', 'app_state_feature_record.rs', 'menu_selection_keys_tests.rs', 'app_state_vertex_edit.rs'],
        ['1024×768 기본 레이어 메뉴', '1680×960 레이어 우클릭 Delete', '실제 범례 클릭·기존 피처 ID와 꼭짓점 선택 고정물', '저장 전 속성·반복 열기·파일 바이트', '같은 묶음 Delete+Backspace 한 번 편집·Esc+Backspace 편집 없음'], [],
        '메뉴 Delete는 자료 보존; 닫은 뒤 정상 삭제와 CtrlZ가 같은 ID·도형·속성을 복원하고 파일 유지. C++의 원본 보존 계약과 사용자 메뉴 격리 결정이며 Qt 전체 단축키 동일성은 미확인',
        [{'kind': 'headless_production_ui', 'evidence': ref('docs/RESULTS.md', '### 메뉴의 선택 삭제')}])
    add('key.history-native', '지도 편집 이력 > 기본 레이어 메뉴의 CtrlZ·CtrlY / Delete+CtrlZ 순서', 0, '완료',
        [('MainWindowUndo.cpp', 'void MainWindow::undoMapAction'), ('MainWindowUndo.cpp', 'void MainWindow::redoMapAction')],
        ['gui_shell.rs', 'gui_shell_escape.rs', 'native_menu_input.rs', 'shell_workspace.rs', 'popup_input_harness.rs', 'app_state_feature_record.rs', 'menu_selection_keys_tests.rs'],
        ['1024×768 기본 레이어 메뉴', '저장 전 속성이 있는 기존 도형', '메뉴 닫은 뒤 실제 CtrlZ·CtrlY', '같은 묶음 Delete 뒤 CtrlZ가 같은 ID·WKT·속성 복원'], [],
        '메뉴 뒤 이력 유지; 닫은 뒤 정상 이력 명령 재개. 입력칸·도면·정합·다른 메뉴 전체와 Qt 단축키 내부 동일성은 이 행의 범위에 포함하지 않음',
        [{'kind': 'headless_production_ui', 'evidence': ref('docs/RESULTS.md', '### 메뉴의 이력 소유권')}])
    add('key.vertex-rejection', '꼭짓점 수정 > 삭제 거부 시 도형 보존·키 소비', 0, '완료',
        [('KaFeatureSelectTool.cpp', 'bool KaFeatureSelectTool::deleteActiveVertex'), ('KaVertexEditTool.cpp', 'const int minCount'),
         ('target/cpp-ref/tests/test_editing_tools.cpp', 'void TestEditingTools::clickedHandleIsPickedAndDeleteRemovesOnlyThatVertex')],
        ['app_state_vertex_edit.rs', 'app_state_vertex_edit_tests.rs', 'gui_shell.rs', 'gui_shell_escape.rs', 'shell_workspace.rs',
         'popup_input_harness.rs', 'app_state_feature_record.rs', 'menu_selection_keys_tests.rs', 'vertex_key_rejection_tests.rs'],
        ['1680×960 생산 UI', '추적 데모의 삼각형·두 점 선', '표 CRS EPSG:5186·작업 EPSG:5187', '임시 저장한 조사·저장 전 속성',
         '반복 Delete·Backspace', '기존 거부 문구·기록·이력·변경 상태·파일 바이트'], [],
        '삭제 거부는 키를 소비해 전체 도형 삭제로 넘어가지 않음; Esc로 꼭짓점 선택 해제 후 의도적인 도형 삭제와 CtrlZ는 정상. 알고리즘·최소 점수는 변경하지 않음',
        [{'kind': 'headless_production_ui', 'evidence': ref('docs/RESULTS.md', '### 꼭짓점 삭제 거부')}])
    add('edit.vertex-crs-input', '도형선택 상태 > 좌표계 오류의 선택·꼭짓점 편집 진입 거부', 0, '완료',
        [('KaVertexEditTool.cpp', 'QgsPointXY KaVertexEditTool::toLayer'), ('KaFeatureSelectTool.cpp', 'void KaFeatureSelectTool::selectInRect'),
         ('target/cpp-ref/tests/test_editing_tools.cpp', 'void TestEditingTools::clickedHandleIsPickedAndDeleteRemovesOnlyThatVertex')],
        ['app_state_vertex_edit.rs', 'feature_pick.rs', 'gui_tab_map_select_box.rs', 'gui_shell.rs', 'popup_input_harness.rs',
         'vertex_crs_input_tests.rs', 'vertex_crs_boundary_tests.rs', 'crates/strata-engine/src/crs.rs'],
        ['1680×960 생산 UI·추적 합성 데모 임시 사본·도형선택 상태', '메타데이터·머리말을 함께 미정의 0·-1로 바꾼 사본의 실제 열기·클릭·끌기·상자',
         '양수 미등록 999999의 기존 포인터 거부', '표 EPSG:5186·5187과 작업 EPSG:5187의 실제 끌기·CtrlZ',
         'C++의 1e-6 m 기준으로 두 국소 놓기 위치 확인·전체 영역 오차 상한 아님',
         '직접 선택 상태의 내부 경계 5개는 별도 증거이며 UI 도달성으로 해석하지 않음'], [],
        '거부는 기록·이력·변경 상태·사본 바이트 보존; 정상 끌기 뒤 CtrlZ는 원래 ID·WKT·속성 복원. 선택 모드 진입 단추·모든 CRS·작도·버퍼·분할·네이티브 UI는 범위 밖',
        [{'kind': 'headless_production_ui', 'evidence': ref('docs/RESULTS.md', '### 꼭짓점 편집의 좌표계 오류')}])
    add('edit.capture-crs-input', '점·선·면 작도 상태 > 대상 좌표계 오류 거부·초안·취소·되돌리기', 0, '완료',
        [('KaCaptureMapTool.cpp', 'void KaCaptureMapTool::finish'), ('MainWindowEditing.cpp', '&KaCaptureMapTool::captureCanceled'),
         ('target/cpp-ref/tests/test_editing_tools.cpp', 'void TestEditingTools::clickedHandleIsPickedAndDeleteRemovesOnlyThatVertex')],
        ['app_state_draw_commit.rs', 'app_state_draw_commit_b.rs', 'app_state_draw_entry.rs', 'gui_tab_map_capture.rs',
         'app_state_feature_form.rs', 'dlg_feature_form.rs', 'gui_shell.rs', 'gui_shell_escape.rs', 'native_menu_input.rs',
         'popup_input_harness.rs', 'vertex_crs_input_tests.rs', 'capture_crs_input_tests.rs', 'crates/strata-engine/src/crs.rs'],
        ['1680×960 생산 UI·추적 합성 데모 사본·유효한 편집 대상과 작도 상태', '대상 0·-1의 점·선·면 실제 클릭·Enter 거부는 사용자 좌표계 규칙에 따름',
         '미등록 999999 생성 실패와 유효한 5187→5186 변환의 범위 밖 점 실패를 구분', '반복 Enter의 오류 갱신·Esc·실제 재입력·재취소',
         '정상 표 EPSG:5186·5187 / 작업 EPSG:5187의 점·선·면 위치와 실제 속성 창 건너뛰기·CtrlZ',
         'C++ 1e-6 m 국소 위치 기준·전체 영역 오차 상한 아님', '범위 밖 화면 준비는 고정물이며 그 화면에 이르는 모든 이동 제스처 검사는 아님'], [],
        '실패는 초안·기록·이력·변경 상태·사본 바이트 유지; Esc 뒤 같은 도구로 다시 입력 가능. 정상 CtrlZ는 원래 ID·WKT·속성 복원. 상위 단추 전체 진입·속성 저장 조합·다른 작업 CRS·분할 확정·네이티브 입력은 범위 밖',
        [{'kind': 'headless_production_ui', 'evidence': ref('docs/RESULTS.md', '### 작도 확정의 좌표계 오류')}])
    add('edit.split', '그리기 > 폴리곤 나누기 > 선·겹침·멀티파트', 0, '부분 구현', [('MainWindowEditing.cpp', 'void MainWindow::startSplitPolygonTool')],
        ['gui_ribbon_draw_row.rs', 'app_state_split.rs', 'app_state_split_tests.rs'],
        ['범례만 선택·한 도형·두 도형·멀티파트', '읽기 전용·잘못된 CRS', '확정·취소·반복·다른 도구'],
        ['MENU13의 분할선 입력·자료 보존 경계만 확인; 겹침·멀티파트·읽기 전용·다른 작업 CRS의 실제 UI는 남음',
         'C++ MainWindowEditing.cpp:795는 확정 후 분할 플래그를 끄며 Rust는 현재 유지함. 다음 도구·상태 표시까지 전체 복귀 경로 추가 대조 필요'])
    add('edit.split-line-input', '폴리곤 나누기 > 분할선 CRS 오류·초안·저장 전 질문', 0, '완료',
        [('KaCaptureMapTool.cpp', 'void KaCaptureMapTool::finish'), ('MainWindowEditing.cpp', 'if (m_isSplittingPolygon)'),
         ('LayerOps.cpp', 'bool LayerOps::splitPolygonWithLine'), ('MainWindowSession.cpp', 'bool MainWindow::surveyHasUnsavedChanges'),
         ('target/cpp-ref/tests/test_buffer.cpp', 'void TestBuffer::splitPolygonWithLine_splitsGeometry')],
        ['gui_layer_panel_rows.rs', 'gui_ribbon_draw_row.rs', 'app_state_draw_row.rs', 'app_state_split.rs', 'app_state_draw_commit.rs',
         'gui_tab_map_capture.rs', 'app_state_save_before.rs', 'gui_shell.rs', 'popup_input_harness.rs', 'vertex_crs_input_tests.rs',
         'split_crs_input_tests.rs', 'crates/strata-engine/src/crs.rs'],
        ['1680×960 생산 UI·추적 합성 데모 사본·범례 선택→그리기→폴리곤 나누기의 실제 단추',
         '대상 0·-1·미등록 999999와 유효한 5187→5186 변환의 범위 밖 점·실제 클릭·우클릭·Enter',
         '오류 갱신·반복 확정·Esc·실제 재입력과 미정의 거부의 사용자 CRS 규칙',
         '0의 사본에 준비한 다른 표의 미저장 메모·Undo 1·Redo 1, 실패 후 실제 CtrlY·CtrlZ 보존. 사전 메모 작성 UI 검사는 아님',
         '정상 표 EPSG:5186·5187 / 작업 EPSG:5187의 두 54m² 조각·원래 ID·속성·다른 기록과 CtrlZ',
         'C++ 전체 면적 기준 1/10000을 데모 108m²에 적용한 0.0108m² 국소 검사·전체 영역 오차 상한 아님',
         '정상 분할 후 실제 CtrlN 저장 질문·취소·CtrlZ, 교차하지 않는 선의 기존 소비 동작',
         '준비한 범위 밖 화면은 모든 이동 제스처의 검증이 아님'], [],
        '좌표 변환 실패는 분할선·기록·기존 이력·변경 상태·사본 바이트 유지; Esc 뒤 다시 입력 가능. 유효한 선은 교차 여부와 무관하게 소비. 정상 분할은 저장 질문과 취소 후 Undo 보존. 확정 뒤 분할 모드 전체 복귀·겹침·멀티파트·작업5186·명시적 저장·네이티브 UI는 상위 행에 남음',
        [{'kind': 'headless_production_ui', 'evidence': ref('docs/RESULTS.md', '### 분할선의 좌표계와 저장 전 질문')}])
    add('buffer.rings', '버퍼 > 주변유적경계 > 500m·1000m', 0, '부분 구현',
        [('MainWindowRibbon.cpp', 'void MainWindow::runSiteBuffer500'), ('NearbyHeritageCollect.cpp', 'resolveSourceLayer'),
         ('BufferAnalysis.cpp', 'bool addDistanceRing')],
        ['app_state_buffer.rs', 'gui_ribbon_subtoolbar.rs', 'buffer_crs_input_tests.rs', 'buffer_crs_input_tests_a.rs'],
        ['고른 면·보이는 조사구역·첫 조사구역', '두 반경·재실행·기존 경계', '원본·작업 CRS와 잘못된 도형'],
        ['MENU14 좌표계 입구와 MENU15 저장 입력은 별도 완료 행; 모든 소스 선택·빈 자료·큰 자료·처리 중·네이티브 UI는 남음',
         '저장 전체의 남은 분기·전환 직후 표시와 엑셀 진입점은 개별 행으로 추적'])
    add('buffer.crs-input', '버퍼 > 500m·1000m > 원본 CRS 오류·기존 경계 보존', 0, '완료',
        [('MainWindowRibbon.cpp', 'void MainWindow::runSiteBuffer500'), ('NearbyHeritageCollect.cpp', 'resolveSourceLayer'),
         ('BufferAnalysis.cpp', 'unionInCrs'), ('target/cpp-ref/tests/test_buffer.cpp', 'void TestBuffer::ringHasGapAndLabel')],
        ['app_state_buffer.rs', 'gui_ribbon_subtoolbar.rs', 'gui_shell.rs', 'vertex_crs_input_tests.rs',
         'buffer_crs_input_tests.rs', 'buffer_crs_input_tests_a.rs', 'crates/strata-engine/src/crs.rs'],
        ['1680×960 생산 egui·추적 데모 임시 사본·실제 범례·버퍼·두 반경 단추',
         '원본0·-1·999999 / 작업5187·새 경계와 기존500·1000 경계 각각·반복 거부',
         '원본0의 준비한 다른 표 메모·Undo1·Redo1·실패 뒤 실제 CtrlY·CtrlZ',
         '모든 기록·속성·FID·이력·변경 상태·사본 바이트·범례·경계 좌표와 라벨 보존',
         '닫기·재열기·재시도·정상 조사구역 선택 후 복구',
         '원본5186·5187 / 작업5187·각 반경의 정상 실행·반복·1경계와4라벨',
         'C++80×60 사각형의500m 범위·유효 변환의 개별 실패 및 일부 유효 자료 유지',
         '국소0.3m 범위 기준은 반경≤1000·사분원36분할의 최대 오차<0.239m에 반올림 여유·전체 영역 CRS 상한 아님'], [],
        '오류는 하단 문구만 갱신하고 자료·기존 경계·이력·변경 상태를 유지. 실제 닫기·재열기 뒤 같은 거부, 정상 소스 복구는 이전 미저장 자료를 유지. 사전 상태 준비는 메모·도형 작성 UI 검증이 아님. 저장·재열기·다른 소스 선택·작업5186·네이티브 UI는 상위 또는 별도 행 범위',
        [{'kind': 'headless_production_ui', 'evidence': ref('docs/RESULTS.md', '### 버퍼의 원본 좌표계와 기존 경계')}])
    add('buffer.save-reopen', '버퍼 > 경계·거리 라벨의 명시적 저장·재열기 전체', 0, '부분 구현',
        [('target/cpp-ref/tests/test_buffer.cpp', 'void TestBuffer::ringLayers_bakeIntoSurveyOnlyOnSave'),
         ('MainWindowSession.cpp', 'bool MainWindow::persistSurveyWork')],
        ['app_state_buffer.rs', 'app_state_projects.rs', 'app_state_buffer_save.rs', 'crates/strata-storage/src/survey_gpkg_buffers.rs'],
        ['500m·1000m·거리 라벨', '저장 전 질문·CtrlS·실패·재열기', '기존 경계 교체·레이어 키·원본 보존'],
        ['MENU15의 기본 이름·고정 스타일·명시적 저장과 재열기 입력은 별도 완료 행',
         '삭제·이름·표시·스타일 변경 전체 보존, 다른 이름 저장, 네이티브 선택기, C++ QGZ 작업공간과 외부 동시 편집은 남음'])
    add('buffer.save-input', '버퍼 > 기본 경계·거리 글자 > 저장·실패·취소·재열기 입력', 0, '완료',
        [('target/cpp-ref/tests/test_buffer.cpp', 'void TestBuffer::ringLayers_bakeIntoSurveyOnlyOnSave'),
         ('SurveyStorage.cpp', 'AbsorbResult absorbExternalVectors'), ('MainWindowSession.cpp', 'bool MainWindow::persistSurveyWork'),
         ('MainWindowSession.cpp', 'bool MainWindow::openSurveyGpkg(const QString& gpkgPath, OpenSurveyMode mode)')],
        ['app_state_buffer.rs', 'app_state_buffer_save.rs', 'app_state_projects.rs', 'gui_tab_home_open.rs', 'gui_shell.rs',
         'real_survey_types.rs', 'buffer_save_input_tests.rs', 'buffer_save_input_tests_a.rs', 'popup_input_harness.rs',
         'crates/strata-types/src/buffer.rs', 'crates/strata-storage/src/survey_gpkg_save.rs',
         'crates/strata-storage/src/survey_gpkg_buffers.rs', 'crates/strata-storage/src/survey_gpkg_buffers_tests.rs'],
        ['추적 데모 임시 사본·C++ 40×40m 사각형의 준비 상태·1680×960 생산 egui',
         '실제 버퍼·500·1000·저장 단추와 CtrlS·CtrlN·취소',
         '작업5187 두 반경·작업5186 생성500·생성 뒤 표시CRS 전환과 파일 원래CRS 유지·재열기 PROJ 경로',
         '반복 저장·두 반경·한 반경 교체·기존 기록의 속성·FID와 사본 파일 보존',
         '새 경계 작성은 파일을 쓰지 않고 저장 필요 표시·질문을 연결',
         '실패 저장은 이전 파일 바이트·새 경계·원본 편집·이력 유지·실제 CtrlZ·CtrlS 재시도',
         '다른 용도의 같은 이름 표 보존·소유 레이어 중복 방지',
         '미지원 판·CRS·다른 바이너리 머리말CRS·표현 불가 거리 글자·불완전 짝은 현재 미저장 조사 교체 전 거부',
         '3개 저장 경계 검사·두 CRS와 정식 SRS 정의·빈 일반 호출의 기존 경계 보존·원본 편집과 잘못된 경계의 원자적 실패',
         '저장 왕복은 좌표 계산이 없는 동일 f64 값 비교·재투영 검사는 명시한 PROJ 입출력 경로이며 전체 영역 오차 상한 아님'], [],
        '취소는 기존 버퍼와 변경 상태를 유지한다. 실패는 하단 문구로 알리고 현재 자료·이력과 이전 파일을 보존한다. 버퍼는 저장한 실제 선·점에서 복원한다. 재열기는 준비된 경로의 생산 열기 요청이며 네이티브 파일 선택기 검증은 아니다. 사용자 원본·실제 사이트·OS 입력·인쇄기를 쓰지 않음',
        [{'kind': 'headless_production_ui', 'evidence': ref('docs/RESULTS.md', '### 버퍼 저장과 재열기의 데이터 보존')}])
    add('buffer.view-work-crs', '버퍼 > 작업 좌표계 전환 직후 지도 표시', 0, '검증 안 됨',
        [('BufferAnalysis.cpp', 'QgsCoordinateReferenceSystem dest = project->crs()')],
        [('shell_workspace.rs', 'self.work_crs = match self.work_crs'), ('gui_tab_map_heritage.rs', 'for (meters, ring)'),
         ('gui_tab_drawing_map_layers.rs', 'for (meters, ring)')],
        ['경계 생성 뒤5187↔5186 전환', '현재 지도·도면·재열기 전후'],
        ['현재 전환은 work_crs만 바꾸고 두 렌더러는 경계 숫자를 바로 사용한다는 소스 단서. MENU15는 저장 CRS와 재열기 변환을 검사했으며 전환 직후 표시의 별도 실제 재현·수정이 남음'])
    add('buffer.export-excel', '버퍼 > 엑셀 내보내기 진입점', 1, '미구현',
        [('MainWindowRibbon.cpp', 'exportNearbyHeritageExcel'), ('MainWindowExport.cpp', 'void MainWindow::exportNearbyHeritageExcel')],
        [('gui_ribbon_subtoolbar.rs', 'ui.add_enabled_ui(false')],
        ['주변 경계·조사구역·유적', '활성 조건·방향·거리·파일 선택 취소'],
        ['현재 W-H1로 항상 비활성인 단추와 MENU14 PNG를 확인. C++ 수집·저장기 연결 필요; 다른 내보내기 전체가 미구현이라는 뜻은 아님'])
    add('map.scale-labels', '지도 > 축척 막대 > 숫자와 단위', 1, '부분 구현',
        [('KaMapControls.cpp', 'void KaMapScaleBar::paintPlate'), ('KaMapControls.cpp', 'QStringList KaMapScaleBar::ticks')],
        ['gui_tab_map_geom.rs', 'gui_tab_map_overlay.rs'],
        ['화면4m/px의200m 막대·가운데100·끝200m', '확대·축소·소수 눈금·DPI'],
        ['MENU14 실제 PNG에서 숫자 겹침 관찰; C++는 끝 눈금과 단위를 분리. 최소 수정 전 모든 관련 입력·범위를 별도 재현',
         '소수 눈금 반올림 차이는 소스 단서이며 아직 실행 검증하지 않음'])
    add('context.map', '지도 우클릭 > 기록·그리기·선택·보기·좌표 복사', 1, '검증 안 됨', [('MainWindowContextMenus.cpp', 'populateMapContextMenu')],
        ['gui_tab_map_context.rs'], ['도형 아래·빈 지도', '조사 없음·처리 중', '읽기 전용', '키보드 닫기'], ['별도 지도 메뉴 렌더러의 조건·경계·취소 확인; MENU2 레이어 메뉴와 구분'])
    add('context.layer', '레이어 우클릭 > 편집·표시·삭제·라벨·인코딩', 1, '부분 구현', [('MainWindowContextMenus.cpp', 'showLayerTreeContextMenu')],
        ['app_state_layer_menu.rs', 'menu_popup.rs'], ['조사·지적·참조·래스터', '읽기 전용', '하나·여러 선택', '파일 없음'], ['MENU2 경계·긴 이름·하위 메뉴 검사 연결; 각 작업의 활성 조건·실제 결과는 개별 추적'])
    add('layer.sets', '레이어 세트 > 이름 지정·덮어쓰기·적용·삭제', 1, '부분 구현', ['KaDrawingLayerSets.cpp'],
        ['layer_sets.rs', 'layer_sets_io.rs', 'layer_sets_ui.rs', 'layer_sets_ui_tests.rs'], ['같은 이름', '취소·기본 No', '명시적 저장·재열기'],
        ['MENU1 구현·UI 근거 연결; 전체 도면 저장·QGZ 세트 가져오기는 별도'], '취소는 기존 세트 유지; 명시적 저장만 디스크 반영')
    add('layer.sets-qgz', '레이어 세트 > C++ QGZ 세트 가져오기', 1, '미구현', ['KaDrawingLayerSets.cpp'],
        ['layer_sets_io.rs'], ['C++ 조사', '같은 이름·없는 레이어'], ['현재 Rust 저장 형식과 C++ QGZ 세트 가져오기 연결 없음'])
    add('sheet.persist', '도면 목록 > 보관·열기·삭제 > 저장 후 재열기', 1, '부분 구현', ['KaDrawingSheetSet.cpp'],
        ['gui_tab_drawing_sheets.rs', 'tab_drawing.rs'], ['도면 여러 장', '같은 이름', '레이어 세트 연결', '조사 재열기'], ['StoredSheet 메모리 목록은 있으나 도면 전체 디스크 보관 없음'])
    add('export.pdf', '도면 > PDF 내보내기', 1, '부분 구현', ['MainWindowExport.cpp'],
        ['gui_ribbon_click_b.rs', 'tab_drawing.rs'], ['용지·축척·범례', '내보내기 중 취소', '대상 경로 실패'], ['기존 PDF 근거 연결; 모든 용지·항목·실패·복귀 UI 대조'])
    add('export.print', '인쇄 / Ctrl+P > 전용 탭·설정·실행', 1, '부분 구현', ['MainWindowExport.cpp'],
        ['gui_ribbon_click_b.rs', 'print_job.rs'], ['용지·DPI·분할', '인쇄기 없음', '취소·반복'], ['UI 작업 근거 연결; 실제 인쇄기 출력 미검증, 허가 없이 물리 인쇄하지 않음'])
    add('export.submit', '검수·제출 / Ctrl+E > 검사·꾸러미', 1, '검증 안 됨', ['KaSubmitFlow.cpp'],
        ['modal_export.rs', 'modal_export_footer.rs'], ['검수 통과·실패', '변경 중', '경로 실패·취소'], ['원본 보존·EPSG:5179·출력 무결성 및 모든 차단 조건 UI 재대조'])
    add('home.explorer', '홈 탐색기 > 고정 폴더·탐색·검색·우클릭', 1, '부분 구현', ['KaHomeRecentCard.cpp'],
        ['home_explorer_ui.rs', 'home_explorer_rows.rs', 'gui_tab_home.rs'], ['여러 폴더', '없는 경로·권한', '화면 크기·DPI', '선택 유지'], ['UI6 탐색기 구현 근거 연결; 우클릭 키보드 소유권과 모든 폴더 이동 상태 재대조'])
    for key, title in [('align', '정합'), ('cad', '벡터·CAD 정합')]:
        add('align.' + key, title + ' > 대상 선택·짝점·이동·저장·되돌리기', 0, '부분 구현' if key == 'align' else '미구현',
            ['MainWindowAlign.cpp', 'KaAlignMapTool.cpp'], ['align_map_input.rs', 'gui_ribbon_align.rs', ('align_picker.rs', '.add_filter')],
            ['래스터·CAD', '원본 보존', '진행 중·점 선택', '다른 조사 전환'], ['래스터 UI 근거 연결; 스캔/사진 표현·벡터 CAD 및 전체 취소/복귀 재대조'])
    for key, title in [('topo', '수치지형'), ('dem', 'DEM'), ('soil', '토양'), ('cadastral', '지적')]:
        add('fetch.' + key, '자료 받기 > ' + title + ' > 왼쪽·우클릭·하위 설정', 1, '검증 안 됨', ['MainWindowRibbon.cpp'],
            ['gui_ribbon_menus_a.rs', 'gui_ribbon_menu_actions.rs'], ['조사구역·계정 유무', '받는 중·캐시 있음', '설정 취소·실패'], ['MENU0·CADMENU0의 각 하위 행을 현 소스에서 재대조; 실제 사이트 성공으로 로컬 검사를 확대하지 않음'])
    add('fetch.heritage', '자료 받기 > 유산 > 지역·내려받기·취소·재시도', 1, '부분 구현', ['HeritageIntranetFlow.cpp', 'KaHeritageBrowser.cpp'],
        ['dlg_heritage_browser.rs', 'heritage_download_worker_tests.rs'], ['폼 필드 종류', '부모 창 숨김', '전송 실패·재시작·조사 전환'], ['유산 로컬 WebView 근거 연결; 실제 사이트 로그인·완료 미검증'])
    add('map.old', '배경 지도 > 옛 지도 > 대동여지도 / 1919 조선지형도 전체', 2, '부분 구현',
        [('MainWindowRibbon.cpp', 'oldMaps->setPopupMode'), ('MainWindowOffline.cpp', 'void MainWindow::addHistoryGisMap1919')],
        ['gui_ribbon_menus_a.rs', 'gui_ribbon_menus_b.rs', 'gui_ribbon_menu_actions.rs', 'ref_tile_history.rs'],
        ['첫 클릭·반복·선택·해제', '지도·도면·키보드', '계정·실제 지도 영상·저장·재열기'],
        ['최신 사용자 아이콘 불동작은 MENU16에서 실제 재현·수정했고 좁은 포인터 진입은 별도 완료 행',
         '실제 서비스 영상·인증, 전체 키보드 선택·도면 탭·범례 표시와 저장 왕복은 미검증 분기가 남음'])
    add('map.old-input', '옛 지도 > 실제 아이콘·글자 > 두 선택·취소와 그리기 복귀 입력', 1, '완료',
        [('MainWindowRibbon.cpp', 'oldMaps->setPopupMode'), ('MainWindowOffline.cpp', 'void MainWindow::addDaedongyeojidoMap'),
         ('MainWindowOffline.cpp', 'void MainWindow::addHistoryGisMap1919')],
        ['gui_ribbon_menu_actions.rs', 'gui_ribbon_menus_a.rs', 'gui_ribbon_menus_b.rs', 'ribbon_state.rs',
         'app_state.rs', 'app_state_lifecycle.rs', 'ref_tile_history.rs', 'old_map_input_tests.rs', 'popup_input_harness.rs', 'gui_shell.rs'],
        ['추적 데모 임시 사본·생산 egui·1680/2560/1280 폭의 실제 아이콘 기본·보조 클릭',
         '글자 클릭·아이콘 다시 눌러 닫기·800 폭 축약 메뉴',
         '실제 두 선택 경로·중복 없는 범례·표시·변경 표시·조사 기록과 파일 바이트 보존',
         '1919는 새 고정물의 가짜/빈 키 읽기·기본 생산 읽기는 기존 함수 유지',
         'Esc와 실제 지도 바깥 클릭으로 닫기·미완성 세 점·원본·이력 보존·다음 지도 클릭은 네 번째 점',
         '기존 관련 키보드9개·새 UI8개·원래 아이콘2개 실패·포인터 캡처 변이의2개 실패와 정확 복구'], [],
        '첫 클릭은 메뉴 열기만 맡고 이후에는 실제 메뉴 경계로 닫는다. 메뉴가 열려 있을 때 투명한 앱 내부 입력 배경이 지도 클릭을 받는다. 실패 키는 하단 문구이며 별도 창을 만들지 않는다. OS 입력·원본 쓰기·실제 계정/사이트·온라인 WMS/WMTS 영상 검증은 아니다.',
        [{'kind': 'headless_production_ui', 'evidence': ref('docs/RESULTS.md', '### 옛지도 아이콘과 메뉴 닫기의 입력 소유권')}])
    for key, title in [('legend', '범례 이동·크기·글자'), ('paper', '용지·축척'), ('north', '방위표'), ('coords', '좌표점·격자')]:
        add('sheet.' + key, '도면 > ' + title, 2, '부분 구현', ['KaDrawingStudio.cpp'],
            ['gui_tab_drawing.rs', 'tab_drawing.rs'], ['선택 없음·한 항목·여러 항목', '용지 안·밖', '작은 화면·DPI'], ['SHEET0와 UI1~UI6의 기존 근거 연결; 항목별 속성·취소·저장 상태 재대조'])
    for key, title in [('F9', '왼쪽 패널'), ('F10', '오른쪽 패널'), ('F11', '전체 화면·지도 넓게 보기')]:
        add('key.' + key.lower(), title + ' / ' + key, 2, '검증 안 됨', ['MainWindowRibbon.cpp', 'MainWindowChrome.cpp'],
            ['gui_shell.rs'], ['메뉴·모달·텍스트 포커스', '축소·확대 후 복귀', 'Ctrl 등 수식키'], ['MORE0의 과거 판정을 갱신하고 현재 키 처리·복귀 UI 대조'])
    add('chrome.compact', '그리기 도구 줄·작은 메뉴 > 화면 넘침·클릭 영역', 2, '부분 구현', ['MainWindowRibbon.cpp'],
        ['gui_ribbon_draw_row.rs', 'gui_ribbon_compact.rs'], ['480~1920 너비', '200% DPI', '그리기 진행 중'], ['기존 UI6 근거 연결; MENU16의1280 폭 실제 그림에서 상단 긴 이름의 겹침·잘림도 관찰. 그리기 도구 줄 넘침·글꼴 의존 표시와 함께 후속 공통 UI에서 수정'])
    # First-party named actions: target/cpp-ref/src/app/MainWindowContextMenus.cpp:112-448.
    titles = {
        'layer.zoom': '이 레이어로 이동', 'layer.draw': '그리기·편집 시작', 'layer.vertices': '꼭짓점 수정',
        'layer.attributes': '도형 속성', 'layer.rename': '이름 바꾸기', 'layer.style': '면·외곽선 색',
        'layer.labels': '라벨·명칭 속성', 'layer.labelsVisible': '라벨 보이기·숨기기',
        'layer.labelField': '표시할 항목', 'layer.labelFields': '항목 10개씩 묶기', 'layer.labelFieldsNext': '90개 이후 다음 항목',
        'layer.labelFieldValue': '데이터 필드별 선택 항목', 'layer.labelArea': '면적 함께 표시',
        'layer.labelSize': '글자 크기', 'layer.labelSizeValue': '3·4·5·6·7·8·9·10·12·14 pt',
        'layer.encoding': '깨진 한글 바로잡기', 'layer.encodingValue': 'CP949·EUC-KR·UTF-8·이 컴퓨터 설정',
        'layer.area': '면적·조사 비율 확인', 'layer.export': '제출 꾸러미 만들기', 'layer.import': '레이어 불러오기',
        'layer.opacity': '투명도', 'layer.opacityValue': '0·20·40·50·60·80% 투명도', 'layer.bottom': '같은 묶음 맨 아래',
        'layer.offline': '배경 오프라인 저장', 'layer.fullExtent': '전체 범위', 'layer.merge': '조사구역 합치기',
        'layer.clear': '그린 도형 모두 지우기', 'layer.remove': '레이어 삭제', 'layer.trenchCreate': '시굴격자',
        'layer.trench2': '시굴격자 2×2', 'layer.trench10': '시굴격자 2×10', 'layer.trenchCustom': '시굴격자 사용자 설정',
        'map.attributes': '이 도형 기록 입력', 'map.draw': '조사 도형 그리기', 'map.drawArea': '조사구역',
        'map.drawFeature': '유구 면', 'map.drawLine': '유구 선', 'map.drawSection': '단면선',
        'map.previous': '이전 화면', 'map.next': '다음 화면', 'map.full': '전체 범위',
        'map.layerZoom': '선택한 레이어로 이동', 'map.refresh': '새로 고침', 'map.copyCoordinates': '이 위치 좌표 복사',
    }
    conditions = {
        'layer.draw': ['처리 중 금지', '현재 조사 소유', '추가 기능 지원·읽기 전용'],
        'layer.vertices': ['선·면만', '도형 있음', '현재 조사 소유·변경 기능 지원'],
        'layer.attributes': ['벡터·도형 있음', '현재 조사 소유', '속성 변경 가능'],
        'layer.labels': ['벡터·지원하는 이름/번호 필드'],
        'layer.labelFieldValue': ['현재 선택 체크', '필드별 동적 생성', '10·90개 경계'],
        'layer.labelSizeValue': ['도면 유적 번호는 고정 크기', '현재 크기 체크'],
        'layer.export': ['빈 레이어도 조사 전체 내보내기 허용', '조사 없음·처리 중 금지'],
        'layer.clear': ['조사 도메인·현재 조사 소유', '도형 있음·처리 중 금지'],
        'layer.opacityValue': ['자료·색 보존', '지도 표시만 변경', '현재 값 체크'],
        'layer.remove': ['기존 선택과 다른 우클릭 대상', '다중 선택·그룹·처리 중'],
        'map.attributes': ['클릭 위치의 조사 도형', '조사 소유·읽기 전용·처리 중'],
        'map.previous': ['이전 보기 이력 유무'], 'map.next': ['다음 보기 이력 유무'],
        'map.copyCoordinates': ['작업 CRS', 'x·y 순서와 소수 셋째 자리', '우클릭 위치 유지'],
    }
    cpp_file = 'target/cpp-ref/src/app/MainWindowContextMenus.cpp'
    cpp_text = (root / cpp_file).read_text(encoding='utf-8')
    action_ids = sorted(set(re.findall(r'"((?:map|layer)\.[A-Za-z0-9]+)"', cpp_text)))
    missing_context = {
        'layer.clear': '현재 메뉴는 deferred(CM2a-2)로 항상 비활성; C++ clearDrawnFeaturesOfCurrentLayer 연결이 남음',
        'layer.export': '현재 우클릭 메뉴는 deferred(R2)로 항상 비활성; 상위 제출 메뉴의 구현과 구분해 연결해야 함',
    }
    for action_id in action_ids:
        is_layer = action_id.startswith('layer.')
        model = 'layer_menu_model.rs' if is_layer else 'map_context_menu_model.rs'
        needle = '"' + action_id + '"'
        model_text = (root / 'crates/strata-app/src' / model).read_text(encoding='utf-8')
        add('context.action.' + action_id, ('레이어' if is_layer else '지도') + ' 우클릭 > ' + titles.get(action_id, action_id),
            0 if action_id in ['layer.clear', 'layer.remove', 'layer.merge', 'layer.draw', 'layer.vertices'] else 1,
            '미구현' if action_id in missing_context else '검증 안 됨', [('MainWindowContextMenus.cpp', needle)], [(model, needle)] if needle in model_text else [model],
            conditions.get(action_id, ['레이어 종류·선택·도형 유무·처리 중 조건을 원본 분기와 대조']),
            [missing_context[action_id]] if action_id in missing_context else
            ['표시·활성 조건·작업 결과·취소 UI를 개별 검증; 상위 메뉴의 Esc 통과를 이 동작 완료로 확대하지 않음'])
        rows[-1]['cpp'] = [cpp_file + ':' + str(i) for i, line in enumerate(cpp_text.splitlines(), 1) if needle in line]
    for row in rows:
        if row['status'] == '완료':
            row['reviewed_digests'] = REVIEWED_DIGESTS
    return rows
