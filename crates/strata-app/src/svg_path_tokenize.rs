//! SVG path 명령 문자열 토크나이저.
//!
//! `svg_path_paint.rs` 가 300줄을 넘어 이 파일로 나눴다.

#[derive(Debug, Clone, PartialEq)]
pub enum Tok {
    Cmd(char),
    Num(f32),
}

pub fn tokenize(d: &str) -> Vec<Tok> {
    let mut out = Vec::new();
    let b: Vec<char> = d.chars().collect();
    let mut i = 0usize;
    let mut cur_cmd = 'M';
    let mut arg_idx = 0usize;
    while i < b.len() {
        let c = b[i];
        if c.is_ascii_alphabetic() {
            out.push(Tok::Cmd(c));
            cur_cmd = c;
            arg_idx = 0;
            i += 1;
            continue;
        }
        if c.is_whitespace() || c == ',' {
            i += 1;
            continue;
        }
        // SVG 호(A/a) 명령의 4번째·5번째 인자(large-arc-flag, sweep-flag)는 단일 문자 '0' 또는 '1'
        if (cur_cmd == 'A' || cur_cmd == 'a')
            && (arg_idx % 7 == 3 || arg_idx % 7 == 4)
            && (c == '0' || c == '1')
        {
            out.push(Tok::Num(if c == '1' { 1.0 } else { 0.0 }));
            arg_idx += 1;
            i += 1;
            continue;
        }
        let start = i;
        let mut seen_dot = false;
        if c == '-' || c == '+' {
            i += 1;
        }
        while i < b.len() {
            if b[i].is_ascii_digit() {
                i += 1;
            } else if b[i] == '.' && !seen_dot {
                seen_dot = true;
                i += 1;
            } else {
                break;
            }
        }
        if i < b.len() && (b[i] == 'e' || b[i] == 'E') {
            i += 1;
            if i < b.len() && (b[i] == '-' || b[i] == '+') {
                i += 1;
            }
            while i < b.len() && b[i].is_ascii_digit() {
                i += 1;
            }
        }
        let s: String = b[start..i].iter().collect();
        if let Ok(v) = s.parse::<f32>() {
            out.push(Tok::Num(v));
            arg_idx += 1;
        }
        if i == start {
            i += 1;
        }
    }
    out
}
