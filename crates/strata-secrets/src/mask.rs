#[derive(Debug, Clone, Copy)]
pub struct SecretMask;

impl SecretMask {
    pub fn redact(input: &str, extra_secrets: &[&str]) -> String {
        let mut out = input.to_string();
        for &secret in extra_secrets {
            let trimmed = secret.trim();
            if !trimmed.is_empty() {
                out = out.replace(trimmed, "***");
            }
        }
        for param in ["key=", "KEY=", "pwd=", "password=", "USER_PW="] {
            let mut search_from = 0;
            while let Some(rel_idx) = out[search_from..].find(param) {
                let val_start = search_from + rel_idx + param.len();
                let val_end = out[val_start..]
                    .find(['&', ' ', '\t', '\r', '\n', '"', '\''])
                    .map_or(out.len(), |i| val_start + i);
                if val_end > val_start {
                    out.replace_range(val_start..val_end, "***");
                    search_from = val_start + 3;
                } else {
                    search_from = val_start;
                }
            }
        }
        out
    }
}
