//! Keep the previous fit on write failure, target/cpp-ref/src/core/GeorefBackup.cpp:232-277.
use super::{error, sidecars, unique_folder};
use crate::{EngineError, PictureInfo};
use std::path::{Path, PathBuf};

struct Staged {
    folder: PathBuf,
    image: PathBuf,
    previous: Vec<(PathBuf, PathBuf)>,
    installed: Vec<PathBuf>,
    retain: bool,
}

impl Drop for Staged {
    fn drop(&mut self) {
        if self.retain {
            return;
        }
        for file in sidecars(&self.image).into_iter().chain(std::iter::once(self.image.clone())).chain(self.previous.iter().map(|(_, old)| old.clone())) {
            drop(std::fs::remove_file(file));
        }
        drop(std::fs::remove_dir(&self.folder));
    }
}

pub(super) fn replace(source: &Path, output: &Path, write: impl FnOnce(&Path) -> Result<PictureInfo, EngineError>) -> Result<PictureInfo, EngineError> {
    let parent = output.parent().ok_or_else(|| error("Invalid result folder"))?;
    let folder = unique_folder(parent)?;
    let image = folder.join(output.file_name().ok_or_else(|| error("Invalid result filename"))?);
    let mut staged = Staged { folder, image, previous: Vec::new(), installed: Vec::new(), retain: false };
    std::fs::copy(source, &staged.image).map_err(error)?;
    let info = write(&staged.image)?;
    let targets: Vec<PathBuf> = std::iter::once(output.to_path_buf()).chain(sidecars(output)).collect();
    let prepared: Vec<PathBuf> = std::iter::once(staged.image.clone()).chain(sidecars(&staged.image)).collect();
    // Only the owned image and known sidecars take part; a directory is never replaced or removed.
    for target in &targets {
        if target.exists() && !std::fs::symlink_metadata(target).map_err(error)?.file_type().is_file() {
            return Err(error("Result path is not a regular file"));
        }
    }
    let committed = (|| {
        for (index, target) in targets.iter().enumerate() {
            if target.exists() {
                let old = staged.folder.join(format!("previous-{index}"));
                std::fs::rename(target, &old).map_err(error)?;
                staged.previous.push((target.clone(), old));
            }
        }
        for (prepared, target) in prepared.iter().zip(&targets) {
            if prepared.exists() {
                std::fs::rename(prepared, target).map_err(error)?;
                staged.installed.push(target.clone());
            }
        }
        Ok(())
    })();
    if let Err(why) = committed {
        for installed in &staged.installed {
            drop(std::fs::remove_file(installed));
        }
        for (target, old) in &staged.previous {
            if let Err(rollback) = std::fs::rename(old, target) {
                staged.retain = true;
                return Err(error(format!("{why}; restore failed: {rollback}; backup: {}", staged.folder.display())));
            }
        }
        return Err(why);
    }
    Ok(info)
}
