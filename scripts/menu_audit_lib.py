"""First-party source discovery and audit evidence validation."""
import hashlib
import re

CPP_CALLS = r'addMenuAction|addSubmenu|addIcon|addAction|addMenu|addItem|addItems|addButton|setText|setToolTip|setPlaceholderText|setShortcut|setShortcuts|setEnabled|setDisabled|setCheckable|setChecked|setContextMenuPolicy|setWindowTitle|QAction|QMenu|QPushButton|QToolButton|QComboBox|QShortcut|QCheckBox|QRadioButton|QDialogButtonBox|QLineEdit|QSpinBox|QDoubleSpinBox'
RUST_CALLS = r'(?:Window|ComboBox|Button)::(?:new|from_id_salt|from_label)|native_menu_button|native_popup_below_widget|menu_button|context_menu|popup_below_widget|popup_above_or_below_widget|button|small_button|text_button|icon_button|checkbox|radio_value|selectable_label|selectable_value|color_edit_button_[a-z_]+|set_enabled|add_enabled_ui|add_enabled|draw_ribbon_chip|action'


def table_rows(text):
    rows = []
    for number, line in enumerate(text.splitlines(), 1):
        if not line.lstrip().startswith('|'):
            continue
        cells, cell, ticks, escaped = [], '', False, False
        for char in line.strip()[1:]:
            if char == '`' and not escaped:
                ticks = not ticks
            if char == '|' and not ticks and not escaped:
                cells.append(cell.strip())
                cell = ''
            else:
                cell += char
            escaped = char == '\\' and not escaped
        if cell.strip():
            cells.append(cell.strip())
        if cells and re.fullmatch(r'`?[A-Za-z0-9]+[-.][A-Za-z0-9.-]+`?', cells[0]):
            rows.append((number, cells))
    return rows


def _lex(text, language):
    masked, literals, i = list(text), [], 0
    raw_pattern = re.compile(r'(?:br|r)(#*)"' if language == 'rust' else r'R"([^ ()\\\t\n]{0,16})\(')
    char_pattern = re.compile(r"'(?:\\.|[^'\\\n])'")
    while i < len(text):
        start, value = i, None
        if text.startswith('//', i):
            end = text.find('\n', i)
            i = len(text) if end < 0 else end
        elif text.startswith('/*', i):
            i, depth = i + 2, 1
            while i < len(text) and depth:
                if language == 'rust' and text.startswith('/*', i):
                    depth, i = depth + 1, i + 2
                elif text.startswith('*/', i):
                    depth, i = depth - 1, i + 2
                else:
                    i += 1
        else:
            raw = raw_pattern.match(text, i)
            if raw:
                closing = '"' + raw[1] if language == 'rust' else ')' + raw[1] + '"'
                end = text.find(closing, raw.end())
                end = len(text) if end < 0 else end
                value = text[raw.end():end]
                i = min(end + len(closing), len(text))
            elif text[i] == '"':
                i += 1
                while i < len(text) and text[i] != '"':
                    i += 2 if text[i] == '\\' else 1
                value, i = text[start + 1:i], min(i + 1, len(text))
            else:
                char = char_pattern.match(text, i)
                if not char:
                    i += 1
                    continue
                i = char.end()
        if value is not None:
            literals.append((start, i, value.replace(r'\"', '"').replace(r'\n', ' ')))
        for j in range(start, i):
            if masked[j] != '\n':
                masked[j] = ' '
    return ''.join(masked), literals


def _close(code, start, opening='(', closing=')'):
    depth = 0
    for i in range(start, len(code)):
        depth += code[i] == opening
        depth -= code[i] == closing
        if depth == 0:
            return i + 1
    return len(code)


def source_candidates(text, language):
    code, literals = _lex(text, language)
    excluded = []
    if language == 'rust':
        for m in re.finditer(r'#\[cfg\(test\)\]\s*(?:#\[[^\]]*\]\s*)*mod\s+\w+\s*\{', code):
            excluded.append((m.start(), _close(code, m.end() - 1, '{', '}')))
    ignored = lambda pos: any(a <= pos < b for a, b in excluded)
    pattern = CPP_CALLS if language == 'cpp' else RUST_CALLS
    rows = []
    for found in re.finditer(r'\b(' + pattern + r')\s*\(', code):
        if ignored(found.start()):
            continue
        end = _close(code, found.end() - 1)
        labels = [value[:200] for a, b, value in literals if found.start() < a and b <= end and re.search('[가-힣]', value)]
        rows.append({'kind': 'call', 'callee': found[1], 'line': text.count('\n', 0, found.start()) + 1, 'labels': labels[:12]})
    # Korean literals outside known widget factories remain visible for manual triage.
    for start, _, value in literals:
        if not ignored(start) and re.search('[가-힣]', value):
            rows.append({'kind': 'text', 'callee': 'literal', 'line': text.count('\n', 0, start) + 1, 'labels': [value[:200]]})
    return sorted(rows, key=lambda x: (x['line'], x['kind'], x['callee']))


def ref_file(ref, root):
    name, _, line = ref.rpartition(':')
    path = root / name
    if not line.isdigit() or not path.is_file():
        raise ValueError('absent source evidence: ' + ref)
    if not 1 <= int(line) <= len(path.read_text(encoding='utf-8-sig').splitlines()):
        raise ValueError('absent source line: ' + ref)
    return path


def validate_cases(cases, root):
    ids = set()
    for case in cases:
        if case['id'] in ids:
            raise ValueError('duplicate case: ' + case['id'])
        ids.add(case['id'])
        if case['status'] not in ('완료', '부분 구현', '미구현', '검증 안 됨'):
            raise ValueError('unknown verdict: ' + case['id'])
        for ref in case['cpp'] + case['rust']:
            ref_file(ref, root)
        if case['status'] == '완료':
            if not case['cpp'] or not case['rust']:
                raise ValueError('both implementation source references required: ' + case['id'])
            runtime = case['runtime']
            if not runtime or any(x['kind'] not in ('headless_production_ui', 'native_ui') for x in runtime):
                raise ValueError('runtime UI evidence required: ' + case['id'])
            if case['remaining']:
                raise ValueError('remaining work prevents completion: ' + case['id'])
            if not case.get('conditions') or not case.get('cancel_return'):
                raise ValueError('state and cancellation scope required: ' + case['id'])
            digests = case.get('reviewed_digests', {})
            for ref in case['cpp'] + case['rust']:
                path = ref_file(ref, root)
                relative = path.relative_to(root).as_posix()
                if digests.get(relative) != hashlib.sha256(path.read_bytes()).hexdigest():
                    raise ValueError('stale completion evidence: ' + relative)
            for evidence in runtime:
                ref_file(evidence['evidence'], root)
