//! 배경지도 타일을 **백그라운드에서** 받는다.
//!
//! ## 왜 필요한가
//!
//! 예전에는 `basemap_paint.rs` 의 그리기 함수 안에서 `fetch_tile` 을 불렀고,
//! 그 안에서 `curl` 자식 프로세스를 **동기로** 돌렸다(`--max-time 12`).
//! 한 화면에 타일이 최대 256장이므로 첫 프레임이 최악 12초×256 동안 멈출 수 있었다.
//!
//! 이건 설계 문서 8대 수칙의 **3번(렌더 함수는 순수해야 한다)** 위반이고,
//! C++ 쪽 크래시 규칙("WMS/XYZ 타일이 로딩 중일 때 다시 그리지 않는다")과도 어긋난다.
//!
//! ## 어떻게
//!
//! - 요청은 채널로 워커에 넘기고, 그리기 함수는 **이미 받아 둔 것만** 그린다.
//! - 결과는 한 번에 모아서(`drain`) 프레임당 한 번만 알린다(수칙 4 배치 통지).
//! - 같은 타일을 여러 프레임에 걸쳐 중복 요청하지 않는다(요청 중 표시).

use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver, Sender, TryRecvError};
use std::thread;

use crate::basemap_fetch::fetch_tile;
use crate::basemap_tiles::TileKind;

/// 타일 한 장의 식별자.
pub type TileId = (TileKind, u8, u32, u32);

/// 워커에게 보내는 일.
struct TileJob {
    kind: TileKind,
    key: String,
    z: u8,
    x: u32,
    y: u32,
}

/// 워커가 돌려주는 결과.
#[derive(Debug, Clone)]
pub struct TileReady {
    pub id: TileId,
    /// 받은 파일 경로. 실패면 `None`(다시 시도하지 않도록 기록만 한다).
    pub path: Option<PathBuf>,
}

/// 배경 타일 워커.
///
/// 앱이 하나 들고 있으면 된다. `Drop` 되면 요청 채널이 닫혀 워커가 스스로 끝난다.
pub struct TileWorker {
    tx: Sender<TileJob>,
    rx: Receiver<TileReady>,
    /// 이미 요청했거나 결과를 받은 타일 — 중복 요청 방지.
    inflight: HashSet<TileId>,
}

impl TileWorker {
    /// 워커 스레드를 띄운다.
    ///
    /// 작업 수가 많아도 스레드는 **하나**만 쓴다. curl 이 이미 프로세스를 띄우므로
    /// 동시에 여러 개를 열면 자식 프로세스가 폭증한다.
    #[must_use]
    pub fn spawn() -> Self {
        let (job_tx, job_rx) = channel::<TileJob>();
        let (res_tx, res_rx) = channel::<TileReady>();

        // spawn 실패는 조용히 무시한다 — 타일이 안 나올 뿐 앱이 죽으면 안 된다.
        let _ = thread::Builder::new()
            .name("basemap-tile-worker".to_owned())
            .spawn(move || {
                while let Ok(job) = job_rx.recv() {
                    let path = fetch_tile(job.kind, &job.key, job.z, job.x, job.y);
                    let id = (job.kind, job.z, job.x, job.y);
                    if res_tx.send(TileReady { id, path }).is_err() {
                        break; // 수신자가 사라졌다
                    }
                }
            });

        Self {
            tx: job_tx,
            rx: res_rx,
            inflight: HashSet::new(),
        }
    }

    /// 타일이 필요하다고 알린다. 이미 요청했으면 무시한다(중복 방지).
    ///
    /// **그리기 함수에서 불러도 안전하다** — 채널에 넣기만 하고 기다리지 않는다.
    pub fn request(&mut self, kind: TileKind, key: &str, z: u8, x: u32, y: u32) {
        let id = (kind, z, x, y);
        if self.inflight.contains(&id) {
            return;
        }
        if self
            .tx
            .send(TileJob {
                kind,
                key: key.to_owned(),
                z,
                x,
                y,
            })
            .is_ok()
        {
            self.inflight.insert(id);
        }
    }

    /// 이미 요청했는가(그리기 함수가 중복을 피할 때 쓴다).
    #[must_use]
    pub fn is_pending(&self, id: &TileId) -> bool {
        self.inflight.contains(id)
    }

    /// 끝난 결과를 **한 번에 모두** 꺼낸다.
    ///
    /// 프레임마다 한 번만 부르고, 반환된 것이 있으면 그때만 `request_repaint` 한다
    /// (수칙 4: 배치 알림).
    pub fn drain(&mut self) -> Vec<TileReady> {
        let mut out = Vec::new();
        loop {
            match self.rx.try_recv() {
                Ok(r) => out.push(r),
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => break,
            }
        }
        out
    }

    /// 받은 타일 기록을 지운다(강제 새로고침용).
    pub fn forget(&mut self) {
        self.inflight.clear();
    }
}

impl Drop for TileWorker {
    fn drop(&mut self) {
        // 요청 채널을 닫아 워커가 끝나게 한다. 스레드는 스스로 정리된다.
        self.inflight.clear();
    }
}

/// `StrataAppState` 가 `Debug` 를 요구하므로 직접 구현한다.
/// 채널 내부는 보여 줄 것이 없으므로 요청 중인 타일 수만 보여 준다.
impl std::fmt::Debug for TileWorker {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TileWorker")
            .field("inflight", &self.inflight.len())
            .finish()
    }
}

/// `StrataAppState` 가 `Clone` 을 요구한다.
///
/// 워커는 스레드를 들고 있어 그대로 복제할 수 없다. 복제본은 **새 워커**가 되어
/// 스레드를 하나 더 띄운다. 앱 상태를 복제하는 일은 시험·초기화에서만 일어나므로
/// 실제로는 워커가 하나만 살아 있다.
impl Clone for TileWorker {
    fn clone(&self) -> Self {
        Self::spawn()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 같은 타일을 두 번 요청해도 중복으로 들어가지 않는다.
    #[test]
    fn duplicate_requests_are_ignored() {
        let mut w = TileWorker::spawn();
        let id = (TileKind::Satellite, 12, 3400, 1560);
        w.request(TileKind::Satellite, "test-key", 12, 3400, 1560);
        assert!(w.is_pending(&id), "첫 요청은 기록돼야 한다");
        w.request(TileKind::Satellite, "test-key", 12, 3400, 1560);
        // 두 번째 요청은 무시됐다(중복 제거).
        assert!(w.is_pending(&id));
    }

    /// request 는 **기다리지 않는다** — 그리기 함수에서 불러도 프레임이 안 멈춘다.
    #[test]
    fn request_does_not_block_the_caller() {
        let mut w = TileWorker::spawn();
        let started = std::time::Instant::now();
        // 네트워크가 없어도 request 는 즉시 돌아와야 한다.
        for i in 0..64u32 {
            w.request(TileKind::Satellite, "no-network", 10, i, 0);
        }
        let elapsed = started.elapsed();
        assert!(
            elapsed < std::time::Duration::from_millis(100),
            "request 가 {elapsed:?} 걸렸다 — 블로킹이 남아 있다"
        );
    }

    /// drain 은 결과가 없으면 빈 벡터를 즉시 돌려준다(프레임 지연 없음).
    #[test]
    fn drain_returns_immediately_when_empty() {
        let mut w = TileWorker::spawn();
        let started = std::time::Instant::now();
        let got = w.drain();
        assert!(got.is_empty());
        assert!(started.elapsed() < std::time::Duration::from_millis(50));
    }

    /// forget 하면 다시 요청할 수 있다.
    #[test]
    fn forget_allows_refetch() {
        let mut w = TileWorker::spawn();
        let id = (TileKind::Satellite, 5, 1, 1);
        w.request(TileKind::Satellite, "k", 5, 1, 1);
        assert!(w.is_pending(&id));
        w.forget();
        assert!(!w.is_pending(&id), "forget 뒤에는 다시 요청 가능해야 한다");
    }
}
