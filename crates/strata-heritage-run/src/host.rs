//! The pages as the stage logic sees them during one frame: a list taken from the browser, and
//! a queue of what the logic wants done. Nothing here touches the browser, so the logic can
//! also be asked from inside one of the browser's own callbacks, where the browser cannot be
//! used. The shape is `strata-ngii-run/src/host.rs`.

use strata_net::heritage_auto::{PageHost, PageId, PageRef};

/// Something the stage logic wants the browser to do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Wanted {
    Script {
        page: PageId,
        script: String,
        token: u64,
    },
    Navigate(String),
}

#[derive(Debug, Default)]
pub(crate) struct QueuedHost {
    pub(crate) pages: Vec<PageRef>,
    pub(crate) wanted: Vec<Wanted>,
    tokens: u64,
}

impl PageHost for QueuedHost {
    fn pages(&self) -> Vec<PageRef> {
        self.pages.clone()
    }

    fn run_script(&mut self, page: PageId, script: &str) -> Option<u64> {
        self.pages.iter().any(|p| p.id == page).then(|| {
            self.tokens += 1;
            self.wanted.push(Wanted::Script {
                page,
                script: script.to_owned(),
                token: self.tokens,
            });
            self.tokens
        })
    }

    fn navigate(&mut self, url: &str) {
        self.wanted.push(Wanted::Navigate(url.to_owned()));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scripts_are_queued_for_open_pages_only_and_every_one_gets_its_own_token() {
        let mut host = QueuedHost {
            pages: vec![PageRef {
                id: PageId(1),
                url: "https://a.example/".into(),
                opener: None,
            }],
            ..QueuedHost::default()
        };
        assert_eq!(host.run_script(PageId(1), "1"), Some(1));
        assert_eq!(host.run_script(PageId(7), "2"), None, "a page that is gone");
        assert_eq!(host.run_script(PageId(1), "3"), Some(2));
        host.navigate("https://b.example/");
        assert_eq!(
            host.wanted,
            vec![
                Wanted::Script {
                    page: PageId(1),
                    script: "1".into(),
                    token: 1
                },
                Wanted::Script {
                    page: PageId(1),
                    script: "3".into(),
                    token: 2
                },
                Wanted::Navigate("https://b.example/".into()),
            ]
        );
        // The tokens go on after the queue was taken: an old answer can never be mistaken for
        // a new one.
        host.wanted.clear();
        assert_eq!(host.run_script(PageId(1), "4"), Some(3));
    }
}
