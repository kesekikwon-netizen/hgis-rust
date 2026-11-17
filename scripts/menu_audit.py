"""Rebuild/check the first-party UI audit ledger without running the application.

Use --write to update generated files; default is a read-only freshness check.
Source discovery is deliberately broader than a feature count. Unmapped widget
factories and Korean strings stay in the pending queue, never disappear as DONE.
"""
import argparse
import hashlib
import json
import runpy
import subprocess
from collections import Counter
from pathlib import Path

from menu_audit_lib import source_candidates, table_rows, validate_cases

ROOT = Path(__file__).resolve().parents[1]
DATA = ROOT / 'docs/research/menus'
OUT = ROOT / '.codex/team/menu-audit'
HISTORICAL = ['more/MORE0.md', 'menus/MENU0.md', 'edit/EDIT0.md', 'sheet/SHEET0.md', 'align/ALIGN0.md', 'cadastral/CADMENU0.md']


def git(*args):
    return subprocess.check_output(['git', *args], cwd=ROOT, text=True, encoding='utf-8').strip()


def discover():
    manifest, candidates = [], []
    for language, folder, glob in [('cpp', 'target/cpp-ref/src', '*.cpp'), ('cpp', 'target/cpp-ref/src', '*.h'), ('rust', 'crates', '*.rs')]:
        names = subprocess.check_output(['rg', '--files', folder, '-g', glob], cwd=ROOT, text=True, encoding='utf-8').splitlines()
        for name in sorted(names):
            path = ROOT / name
            relative = path.relative_to(ROOT).as_posix()
            if 'test' in path.stem.lower():
                manifest.append({'file': relative, 'sha256': hashlib.sha256(path.read_bytes()).hexdigest(),
                                 'candidates': 0, 'excluded': 'test-name hint; naming/dynamic-surface classification remains reviewable'})
                continue
            source = path.read_text(encoding='utf-8-sig')
            found = source_candidates(source, language)
            manifest.append({'file': relative, 'sha256': hashlib.sha256(path.read_bytes()).hexdigest(), 'candidates': len(found)})
            occurrences = Counter()
            for row in found:
                signature = json.dumps([relative, row['kind'], row['callee'], row['labels']], ensure_ascii=False)
                occurrences[signature] += 1
                key = hashlib.sha256((signature + ':' + str(occurrences[signature])).encode()).hexdigest()[:16]
                candidates.append({'id': language + '-' + key, 'language': language, 'source': relative + ':' + str(row['line']), **row})
    return manifest, candidates


def history():
    rows = []
    for name in HISTORICAL:
        path = ROOT / 'docs/research' / name
        for line, cells in table_rows(path.read_text(encoding='utf-8')):
            label_index = 2 if name == 'menus/MENU0.md' else 1
            rows.append({'id': name + ':' + cells[0].strip('`') + '@' + str(line), 'label': cells[label_index] if len(cells) > label_index else '',
                         'historical': path.relative_to(ROOT).as_posix() + ':' + str(line), 'status': '검증 안 됨'})
    return rows


def render():
    cases = runpy.run_path(str(DATA / 'menu_cases.py'))['build_cases'](ROOT)
    validate_cases(cases, ROOT)
    manifest, candidates = discover()
    old_rows = history()
    ids = {x['id'] for x in candidates}
    linked = {}
    for case in cases:
        for candidate in case.get('candidate_ids', []):
            if candidate not in ids:
                raise ValueError('stale candidate binding: ' + candidate)
            linked.setdefault(candidate, []).append(case['id'])
    for row in candidates:
        row['cases'] = sorted(set(linked.get(row['id'], []) + [case['id'] for case in cases if row['source'] in case['cpp'] + case['rust']]))
        row['disposition'] = 'linked' if row['cases'] else 'unreviewed'
    payload = {'schema': 1, 'application_tree': git('rev-parse', 'HEAD:crates'), 'source_dirty': bool(git('status', '--porcelain', '--', 'crates')), 'cpp_head': git('-C', 'target/cpp-ref', 'rev-parse', 'HEAD'),
               'limits': 'Lexical discovery, not semantic completeness or runtime proof. Dynamic factories and text may be aliases or noninteractive. Labels show at most200characters,12per call; all Korean literals also have separate source references. Every discovered candidate stays linked or unreviewed. Historical SAME/FAKE claims are not current verdicts.',
               'files': manifest, 'historical_items': old_rows, 'cases': cases, 'candidates': candidates}
    counts = Counter(x['status'] for x in cases)
    lines = ['# 현재 메뉴·상호작용 추적표', '', '이 파일은 `python -X utf8 scripts/menu_audit.py --write`로 생성한다. 원본 판정은 `docs/research/menus/menu_cases.py`에 기록한다.', '',
             f'앱 소스 트리 `{payload["application_tree"]}`, C++ 참조 `{payload["cpp_head"]}`.', '',
             '완료는 해당 행에 적힌 조건과 검증 범위만 뜻한다. 전체 메뉴 또는 네이티브 창의 완료로 확대하지 않는다. 소스 검색이나 컴파일 성공만으로 완료를 판정하지 않는다.', '',
             '상태: 완료 / 부분 구현 / 미구현 / 검증 안 됨. 이전 조사 문서의 판정은 현재 상태와 분리했다.', '',
             '## 현재 대조한 기능', '', '| ID | 경로·기능 | 우선순위 | 상태 | 선택·활성 조건 | 취소·복귀 | 근거·남은 일 |', '|---|---|---|---|---|---|---|']
    for case in cases:
        evidence = '; '.join(case['cpp'] + case['rust'] + [x['evidence'] for x in case['runtime']])
        values = [case['id'], case['path'], str(case['priority']), case['status'], '; '.join(case['conditions']), '; '.join(case['cancel_return']), evidence + ' / 남음: ' + '; '.join(case['remaining'])]
        lines.append('| ' + ' | '.join(x.replace('|', '\\|').replace('\n', ' ') for x in values) + ' |')
    lines += ['', '## 이전 조사에서 이어받은 기능 목록', '',
              '다음 행은 재대조 대기 항목이다. 과거 문서의 SAME/FAKE 판정을 현재 완료·미구현으로 재사용하지 않는다.', '',
              '| ID | 기능 | 이전 근거 | 현재 상태 |', '|---|---|---|---|']
    for row in old_rows:
        lines.append('| ' + ' | '.join(str(row[x]).replace('|', '\\|') for x in ['id', 'label', 'historical', 'status']) + ' |')
    lines += ['', '## 누락 추적과 해석 한계', '',
              f'검토 기능 {len(cases)}행: ' + ', '.join(k + ' ' + str(counts[k]) for k in ['완료', '부분 구현', '미구현', '검증 안 됨']) + '.', '',
              f'이전 조사 {len(old_rows)}행, 소스 파일 {len(manifest)}개를 수집했다. 이는 고유 메뉴 수나 기능 완료율이 아니다.', '',
              '원본의 메뉴·단추·선택 상자·활성 조건·단축키 선언과, 알려진 생성 함수 밖의 한국어 문자열을 `menu_sources.json`에 모두 남긴다. 미연결 행은 `unreviewed`이며 삭제하거나 완료로 계산하지 않는다.', '',
              '문자열은 상태 메시지·중복 별칭일 수 있고 동적 생성 항목은 추가 분류가 필요하다. 모든 후보가 의미별로 분류되기 전까지 목록의 의미적 완전성은 미검증이다.', '',
              '현재 HEAD의 CI 및 배포 상태는 해당 커밋의 기록과 별도로 확인한다. 이 목록 생성은 앱 실행 검증이 아니다.', '']
    pending = ['# C++ 조작 선언 후보', '', '기능 판정이 아닌 원본 조작 선언 목록이다. 각 행의 원본 조건과 호출 결과를 읽어 기능 행에 연결한다. 문자열·동적 생성의 추가 단서는 menu_sources.json에 남아 있다.', '',
               '| ID | 원본 | 선언 | 표시 문자열 | 연결 기능 / 분류 대기 |', '|---|---|---|---|---|']
    for row in candidates:
        if row['language'] == 'cpp' and row['kind'] == 'call':
            fields = [row['id'], row['source'], row['callee'], '; '.join(row['labels']), ', '.join(row['cases']) or '검증 안 됨·미분류']
            pending.append('| ' + ' | '.join(x.replace('|', '\\|').replace('\n', ' ') for x in fields) + ' |')
    return {'menu_sources.json': json.dumps(payload, ensure_ascii=False, separators=(',', ':')) + '\n',
            'CURRENT_MENU_AUDIT.md': '\n'.join(lines), 'PENDING_CPP_CONTROLS.md': '\n'.join(pending) + '\n'}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--write', action='store_true')
    args = parser.parse_args()
    generated = render()
    if args.write:
        OUT.mkdir(parents=True, exist_ok=True)
    for name, content in generated.items():
        path = OUT / name
        if args.write:
            path.write_text(content, encoding='utf-8', newline='\n')
        elif not path.is_file() or path.read_text(encoding='utf-8') != content:
            raise SystemExit('STALE ' + name + '; inspect changes then regenerate')
    data = json.loads(generated['menu_sources.json'])
    print(json.dumps({'files': len(data['files']), 'historical': len(data['historical_items']), 'cases': len(data['cases']),
                      'candidate_kinds': dict(Counter(x['language'] + '/' + x['kind'] for x in data['candidates'])),
                      'linked': sum(bool(x['cases']) for x in data['candidates'])}, ensure_ascii=False))


if __name__ == '__main__':
    main()
