"""Run with python -m unittest discover -s scripts -p test_menu_audit.py."""
import tempfile
import unittest
from pathlib import Path

from menu_audit_lib import source_candidates, table_rows, validate_cases


class MenuAuditTests(unittest.TestCase):
    def test_preserves_inline_code_pipes_and_historical_row_identity(self):
        text = '| id | control | contract |\n|---|---|---|\n| M4-19 | Delete | `Vertex|Segment` and A\\|B |\n'
        self.assertEqual(table_rows(text), [(3, ['M4-19', 'Delete', '`Vertex|Segment` and A\\|B'])])

    def test_multiline_action_is_found_without_fake_calls_inside_comments_or_strings(self):
        text = '// menu->addAction("가짜");\nQString note="addAction(가짜)";\nvoid X::menu() {\n menu->addAction(\n QStringLiteral("저장"), [&] { run(1); });\n}'
        calls = [x for x in source_candidates(text, 'cpp') if x['kind'] == 'call']
        self.assertEqual([(x['callee'], x['line'], x['labels']) for x in calls], [('addAction', 4, ['저장'])])

    def test_raw_rust_labels_and_repeated_same_name_actions_are_not_lost(self):
        text = 'fn menu() { ui.button(r#"같은 (이름)"#); ui.button("같은 (이름)"); }'
        calls = [x for x in source_candidates(text, 'rust') if x['kind'] == 'call']
        self.assertEqual([x['labels'] for x in calls], [['같은 (이름)'], ['같은 (이름)']])

    def test_first_party_context_menu_factory_is_discovered(self):
        rows = source_candidates('addMenuAction(menu, "map.attributes", QStringLiteral("이 도형 기록 입력"), reason, callback);', 'cpp')
        self.assertEqual([x['callee'] for x in rows if x['kind'] == 'call'], ['addMenuAction'])

    def test_completion_without_runtime_or_with_remaining_work_is_rejected(self):
        case = {'id': 'save', 'status': '완료', 'cpp': ['cpp.cpp:1'], 'rust': ['app.rs:1'], 'runtime': [], 'remaining': []}
        with tempfile.TemporaryDirectory() as d:
            root = Path(d)
            (root / 'cpp.cpp').write_text('source', encoding='utf-8')
            (root / 'app.rs').write_text('source', encoding='utf-8')
            with self.assertRaisesRegex(ValueError, 'runtime'):
                validate_cases([case], root)
            case['runtime'] = [{'kind': 'compile', 'evidence': 'app.rs:1'}]
            with self.assertRaisesRegex(ValueError, 'runtime'):
                validate_cases([case], root)
            case['runtime'] = [{'kind': 'headless_production_ui', 'evidence': 'app.rs:1'}]
            case['remaining'] = ['cancel path not checked']
            with self.assertRaisesRegex(ValueError, 'remaining'):
                validate_cases([case], root)

    def test_duplicate_cases_and_missing_source_evidence_are_rejected(self):
        case = {'id': 'save', 'status': '검증 안 됨', 'cpp': ['absent.cpp:1'], 'rust': [], 'runtime': [], 'remaining': ['inspect']}
        with tempfile.TemporaryDirectory() as d:
            with self.assertRaisesRegex(ValueError, 'absent'):
                validate_cases([case], Path(d))
            case['cpp'] = []
            with self.assertRaisesRegex(ValueError, 'duplicate'):
                validate_cases([case, case], Path(d))

    def test_completed_case_with_changed_source_is_rejected(self):
        with tempfile.TemporaryDirectory() as d:
            root = Path(d)
            (root / 'app.rs').write_text('changed', encoding='utf-8')
            case = {'id': 'save', 'status': '완료', 'cpp': ['app.rs:1'], 'rust': ['app.rs:1'],
                    'runtime': [{'kind': 'headless_production_ui', 'evidence': 'app.rs:1'}],
                    'remaining': [], 'conditions': ['editing'], 'cancel_return': ['Escape retains data'],
                    'reviewed_digests': {'app.rs': 'old-digest'}}
            with self.assertRaisesRegex(ValueError, 'stale'):
                validate_cases([case], root)

    def test_completed_case_cannot_omit_both_implementations(self):
        with tempfile.TemporaryDirectory() as d:
            root = Path(d)
            (root / 'proof.txt').write_text('observed', encoding='utf-8')
            case = {'id': 'save', 'status': '완료', 'cpp': [], 'rust': [],
                    'runtime': [{'kind': 'headless_production_ui', 'evidence': 'proof.txt:1'}],
                    'remaining': [], 'conditions': ['editing'], 'cancel_return': ['cancelled'], 'reviewed_digests': {}}
            with self.assertRaisesRegex(ValueError, 'source'):
                validate_cases([case], root)


if __name__ == '__main__':
    unittest.main()
