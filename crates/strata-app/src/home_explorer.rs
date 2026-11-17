//! Home folder navigation, based on target/cpp-ref/src/app/KaFileBrowserPanel.cpp:283-387.
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver, TryRecvError};

#[path = "home_explorer_io.rs"]
mod io;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) enum Location {
    #[default]
    Recent,
    Computer,
    Folder(PathBuf),
}

#[derive(Debug, Clone)]
pub(crate) struct Entry {
    pub path: PathBuf,
    pub name: String,
    pub directory: bool,
    pub bytes: Option<u64>,
    pub modified: Option<std::time::SystemTime>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum Sort {
    #[default]
    Name,
    Size,
    Modified,
    Kind,
}

#[derive(Debug)]
struct Slot<T>(Option<Receiver<Result<T, String>>>);
impl<T> Default for Slot<T> {
    fn default() -> Self {
        Self(None)
    }
}
impl<T> Slot<T> {
    fn take_ready(&mut self) -> Option<Result<T, String>> {
        let receive = self.0.take()?;
        match receive.try_recv() {
            Ok(result) => Some(result),
            Err(TryRecvError::Empty) => {
                self.0 = Some(receive);
                None
            }
            Err(TryRecvError::Disconnected) => Some(Err("파일을 열지 못했습니다".to_owned())),
        }
    }
}

#[derive(Debug)]
enum Preferences {
    Loaded(Vec<PathBuf>),
    Saved,
}

#[derive(Debug, Default)]
pub(crate) struct ExplorerState {
    pub location: Location,
    pub entries: Vec<Entry>,
    pub pins: Vec<PathBuf>,
    pub filter: String,
    pub selected: Option<PathBuf>,
    pub address: String,
    pub notice: Option<String>,
    pub sort: Sort,
    pub descending: bool,
    history: Vec<Location>,
    position: usize,
    listing: Slot<Vec<Entry>>,
    picker: Slot<Option<Vec<PathBuf>>>,
    preferences: Slot<Preferences>,
    preference_file: Option<PathBuf>,
    write_pending: bool,
}
impl Clone for ExplorerState {
    fn clone(&self) -> Self {
        Self {
            location: self.location.clone(),
            entries: self.entries.clone(),
            pins: self.pins.clone(),
            filter: self.filter.clone(),
            selected: self.selected.clone(),
            address: self.address.clone(),
            notice: self.notice.clone(),
            sort: self.sort,
            descending: self.descending,
            history: self.history.clone(),
            position: self.position,
            ..Self::default()
        }
    }
}
impl ExplorerState {
    pub fn set_sort(&mut self, sort: Sort, descending: bool) {
        self.sort = sort;
        self.descending = descending;
        self.refresh();
    }
    pub fn refresh(&mut self) {
        let filter = std::mem::take(&mut self.filter);
        let selected = self.selected.take();
        self.load(self.location.clone());
        self.filter = filter;
        self.selected = selected;
    }
    pub fn navigate(&mut self, location: Location) {
        if self.history.is_empty() {
            self.history.push(self.location.clone());
        }
        if location != self.location {
            self.history.truncate(self.position + 1);
            self.history.push(location.clone());
            self.position += 1;
        }
        self.load(location);
    }
    fn load(&mut self, location: Location) {
        self.location = location.clone();
        self.address = match &location {
            Location::Folder(path) => path.display().to_string(),
            _ => String::new(),
        };
        self.entries.clear();
        self.selected = None;
        self.filter.clear();
        self.notice = None;
        let (send, receive) = channel();
        let (sort, descending) = (self.sort, self.descending);
        std::thread::spawn(move || drop(send.send(io::read(&location, sort, descending).map_err(|e| e.to_string()))));
        self.listing.0 = Some(receive);
    }
    pub fn poll(&mut self) {
        if let Some(result) = self.listing.take_ready() {
            match result {
                Ok(rows) => self.entries = rows,
                Err(why) => self.notice = Some(why),
            }
        }
        if let Some(result) = self.picker.take_ready() {
            match result {
                Ok(Some(paths)) => {
                    for path in &paths {
                        self.pin(path.clone());
                    }
                    if let Some(path) = paths.first() {
                        self.navigate(Location::Folder(path.clone()));
                    }
                }
                Ok(None) => {}
                Err(why) => self.notice = Some(why),
            }
        }
        if let Some(result) = self.preferences.take_ready() {
            match result {
                Ok(Preferences::Loaded(pins)) if !self.write_pending => self.pins = pins,
                Ok(_) => {}
                Err(why) => self.notice = Some(why),
            }
        }
        if self.write_pending && self.preferences.0.is_none() {
            if let Some(file) = self.preference_file.clone() {
                let pins = self.pins.clone();
                let (send, receive) = channel();
                std::thread::spawn(move || drop(send.send(io::save_pins(&file, &pins).map(|()| Preferences::Saved).map_err(|e| e.to_string()))));
                self.preferences.0 = Some(receive);
                self.write_pending = false;
            }
        }
    }
    pub fn busy(&self) -> bool {
        self.listing.0.is_some() || self.picker.0.is_some() || self.preferences.0.is_some() || (self.write_pending && self.preference_file.is_some())
    }
    pub fn listing_busy(&self) -> bool {
        self.listing.0.is_some()
    }
    pub fn can_travel(&self, delta: isize) -> bool {
        self.position.checked_add_signed(delta).is_some_and(|p| p < self.history.len())
    }
    pub fn travel(&mut self, delta: isize) {
        if self.can_travel(delta) {
            self.position = self.position.checked_add_signed(delta).unwrap_or(self.position);
            self.load(self.history[self.position].clone());
        }
    }
    pub fn up(&mut self) {
        if let Location::Folder(path) = &self.location {
            let parent = path.parent().map_or(Location::Computer, |path| Location::Folder(path.to_path_buf()));
            self.navigate(parent);
        }
    }
    pub fn pin(&mut self, path: PathBuf) {
        if !self.pins.iter().any(|p| io::same_path(p, &path)) {
            self.pins.push(path);
            self.write_pending = true;
        }
    }
    pub fn unpin(&mut self, path: &Path) {
        self.pins.retain(|p| !io::same_path(p, path));
        self.write_pending = true;
    }
    pub fn configure(&mut self, path: PathBuf) {
        if self.preference_file.is_some() {
            return;
        }
        let (send, receive) = channel();
        let file = path.clone();
        std::thread::spawn(move || drop(send.send(io::load_pins(&file).map(Preferences::Loaded).map_err(|e| e.to_string()))));
        self.preference_file = Some(path);
        self.preferences.0 = Some(receive);
    }
    pub fn pick_folders(&mut self) {
        if self.picker.0.is_some() {
            return;
        }
        let (send, receive) = channel();
        std::thread::spawn(move || {
            let picked = if cfg!(test) { None } else { rfd::FileDialog::new().set_title("조사 데이터 폴더 선택").pick_folders() };
            drop(send.send(Ok(picked)));
        });
        self.picker.0 = Some(receive);
    }
    pub fn set_filter(&mut self, text: String) {
        self.filter = text;
        self.selected = None;
    }
    pub fn visible(&self) -> Vec<&Entry> {
        let filter = self.filter.trim().to_lowercase();
        self.entries.iter().filter(|entry| filter.is_empty() || entry.name.to_lowercase().contains(&filter)).collect()
    }
}

#[cfg(test)]
#[path = "home_explorer_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "home_explorer_action_tests.rs"]
mod action_tests;
