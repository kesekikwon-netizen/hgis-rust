use std::sync::Mutex;
use std::thread::ThreadId;

static OWNER: Mutex<Option<ThreadId>> = Mutex::new(None);

pub(crate) fn claim() {
    *OWNER
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(std::thread::current().id());
}

pub(super) fn is_owner() -> bool {
    *OWNER
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        == Some(std::thread::current().id())
}
