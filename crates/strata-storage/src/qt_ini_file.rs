//! ini 파일 안에서 「절/키」 한 줄 찾기·바꾸기 — 나머지 바이트는 그대로 둔다.
//!
//! 줄 읽기는 Qt `readIniLine`·`readIniSection` 을 따른다: `[절]` 머리, `;` 설명 줄, `키=값`.
//! 같은 키가 여러 번이면 Qt 처럼 마지막 것이 값이다. Windows 의 ini 키는 대소문자를 가리지 않는다.

use std::io;
use std::path::Path;

/// ini 파일 전체 글(UTF-8, Qt 6 과 같다).
#[derive(Debug, Clone, Default)]
pub struct IniFile {
    text: String,
}

/// 줄 하나: 시작, 줄바꿈 앞 끝(`\r` 제외), 다음 줄 시작.
#[derive(Debug, Clone, Copy)]
struct Line {
    start: usize,
    end: usize,
    next: usize,
}

/// 찾은 자리 — 키 줄의 값 범위, 그리고 그 절의 마지막 내용 줄(새 키를 넣을 자리).
#[derive(Debug, Default)]
struct Found {
    value: Option<(usize, usize)>,
    section_last: Option<Line>,
}

impl IniFile {
    #[must_use]
    pub fn from_text(text: String) -> Self {
        Self { text }
    }

    /// 파일을 읽는다. 없으면 빈 글. UTF-8 이 아니면 거절한다(그 파일에는 쓰지 않는다).
    ///
    /// # Errors
    /// 읽기 실패나 UTF-8 이 아닌 내용.
    pub fn read(path: &Path) -> io::Result<Self> {
        match std::fs::read(path) {
            Ok(bytes) => String::from_utf8(bytes)
                .map(Self::from_text)
                .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "설정 파일이 UTF-8 글이 아닙니다")),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(Self::default()),
            Err(e) => Err(e),
        }
    }

    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }

    /// 파일의 줄바꿈 — 없으면 Windows Qt 가 쓰는 CRLF.
    fn newline(&self) -> &'static str {
        if self.text.contains("\r\n") {
            "\r\n"
        } else if self.text.contains('\n') {
            "\n"
        } else if cfg!(windows) {
            "\r\n"
        } else {
            "\n"
        }
    }

    fn lines(&self) -> Vec<Line> {
        let mut out = Vec::new();
        let mut start = 0;
        while start < self.text.len() {
            let next = self.text[start..].find('\n').map_or(self.text.len(), |k| start + k + 1);
            let mut end = if self.text[..next].ends_with('\n') { next - 1 } else { next };
            if self.text[start..end].ends_with('\r') {
                end -= 1;
            }
            out.push(Line { start, end, next });
            start = next;
        }
        out
    }

    fn find(&self, section: &str, key: &str) -> Found {
        let mut found = Found::default();
        let mut in_section = false;
        for line in self.lines() {
            let content = &self.text[line.start..line.end];
            let content = if line.start == 0 { content.trim_start_matches('\u{feff}') } else { content };
            let trimmed = content.trim_start();
            if trimmed.is_empty() || trimmed.starts_with(';') {
                continue;
            }
            if let Some(rest) = trimmed.strip_prefix('[') {
                let name = rest.find(']').map_or(rest, |k| &rest[..k]);
                in_section = name.trim().eq_ignore_ascii_case(section);
                if in_section {
                    found.section_last = Some(line);
                }
                continue;
            }
            if !in_section {
                continue;
            }
            found.section_last = Some(line);
            if let Some(eq) = content.find('=') {
                if content[..eq].trim().eq_ignore_ascii_case(key) {
                    let value_start = line.end - (content.len() - eq - 1);
                    found.value = Some((value_start, line.end));
                }
            }
        }
        found
    }

    /// 「절/키」의 날 값(이스케이프된 글).
    #[must_use]
    pub fn value(&self, section: &str, key: &str) -> Option<&str> {
        self.find(section, key).value.map(|(a, b)| &self.text[a..b])
    }

    /// 「절/키」에 이스케이프된 값을 쓴다. 있으면 그 값만, 없으면 그 절 끝에 한 줄, 절도 없으면
    /// Qt 처럼 빈 줄 하나 띄우고 파일 끝에 절을 붙인다.
    pub fn set_value(&mut self, section: &str, key: &str, escaped: &str) {
        let nl = self.newline();
        let found = self.find(section, key);
        if let Some((a, b)) = found.value {
            self.text.replace_range(a..b, escaped);
            return;
        }
        if let Some(last) = found.section_last {
            let lead = if last.next == last.end { nl } else { "" };
            self.text.insert_str(last.next, &format!("{lead}{key}={escaped}{nl}"));
            return;
        }
        if !self.text.is_empty() {
            if !self.text.ends_with('\n') {
                self.text.push_str(nl);
            }
            if !self.text.ends_with(&format!("{nl}{nl}")) {
                self.text.push_str(nl);
            }
        }
        self.text.push_str(&format!("[{section}]{nl}{key}={escaped}{nl}"));
    }

    /// 같은 폴더의 임시 파일에 쓰고 바꿔 단다(Qt `QSaveFile` 과 같은 방식). 폴더가 없으면 만든다.
    ///
    /// # Errors
    /// 폴더 만들기·쓰기·바꿔 달기 실패.
    pub fn write(&self, path: &Path) -> io::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let name = path.file_name().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
        let tmp = path.with_file_name(format!("{name}.{}.tmp", std::process::id()));
        std::fs::write(&tmp, self.text.as_bytes())?;
        std::fs::rename(&tmp, path).inspect_err(|_| {
            let _ = std::fs::remove_file(&tmp);
        })
    }
}
