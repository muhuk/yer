// Copyright © 2024-2026 Atamert Ölçgen.
// This file is part of Yer.
//
// Yer is free software: you can redistribute it and/or modify it under the
// terms of the GNU General Public License as published by the Free Software
// Foundation, either version 3 of the License, or (at your option) any later
// version.
//
// Yer is distributed in the hope that it will be useful, but WITHOUT ANY
// WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS
// FOR A PARTICULAR PURPOSE.  See the GNU General Public License for more
// details.
//
// You should have received a copy of the GNU General Public License along
// with Yer.  If not, see <https://www.gnu.org/licenses/>.

use std::path::{Path, PathBuf};
use std::sync::{Arc, PoisonError, RwLock};

use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::id::BitmapId;

mod img;

pub use self::img::{Image, ImageLoadError};

// PLUGIN

pub struct BitmapPlugin;

impl Plugin for BitmapPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<BitmapServer>();
    }
}

// RESOURCES

#[derive(Clone, Debug, Deserialize, Reflect, Resource, Serialize)]
#[reflect(Clone, Resource)]
pub struct BitmapServer {
    #[reflect(ignore)]
    data: Arc<BitmapServerData>,
}

impl BitmapServer {
    pub fn get(&self, handle: &BitmapHandle) -> Result<Arc<Image>, BitmapError> {
        {
            match self.get_load_mode(handle)? {
                LoadMode::Embedded => unimplemented!(),
                LoadMode::Linked => {
                    Ok(self
                        .data
                        .linked_image_data
                        .read()?
                        .get(handle)
                        // Hashmap should have this key.
                        .unwrap()
                        .clone())
                }
            }
        }
    }

    // TODO: Test if loading the same path would create two entries.
    pub fn load(
        &self,
        path: impl AsRef<Path>,
        load_mode: LoadMode,
    ) -> Result<BitmapHandle, BitmapError> {
        if !matches!(load_mode, LoadMode::Linked) {
            unimplemented!("embedded mode is not implemented yet");
        }

        let handle = {
            let mut images = self.data.images.write()?;
            match images.iter().find(|bitmap| {
                if let BitmapData::Linked { path: p, .. } = bitmap {
                    p == path.as_ref()
                } else {
                    unreachable!()
                }
            }) {
                Some(bitmap_data) => bitmap_data.handle().clone(),
                None => {
                    let handle = BitmapHandle {
                        id: Arc::new(BitmapId::new()),
                    };

                    images.push(BitmapData::Linked {
                        handle: handle.clone(),
                        path: path.as_ref().to_path_buf(),
                    });
                    handle
                }
            }
        };

        {
            let mut linked_inage_data = self.data.linked_image_data.write()?;
            linked_inage_data.insert(handle.clone(), Arc::new(Image::load_from_disk(path)?));
        }

        Ok(handle)
    }

    /// Replace BitmapServer in the world with this one.
    pub fn replace_in_world(self, world: &mut World) {
        // We can reuse already loaded linked images.
        let old_self = world.remove_resource::<Self>();

        for bitmap_data in self.data.images.read().unwrap().iter() {
            match bitmap_data {
                // Embedded images' data are already loaded.
                BitmapData::Embedded { .. } => (),
                BitmapData::Linked { handle, path } => {
                    // Try to yank the image data from the old instance.
                    let maybe_old_image = old_self
                        .clone()
                        .map(|mut old_self| {
                            old_self
                                .find_handle_from_path(path)
                                .map(|old_handle| old_self.remove(&old_handle))
                                .flatten()
                                .ok()
                        })
                        .flatten();

                    if let Some(image) = maybe_old_image {
                        debug!("Reusing image data for {handle:?}.");
                        let mut linked_image_data = self.data.linked_image_data.write().unwrap();
                        linked_image_data.insert(handle.clone(), image.clone());
                    } else {
                        match Image::load_from_disk(path) {
                            Ok(image) => {
                                debug!("Loaded image data from disk for {handle:?}.");
                                let mut linked_image_data =
                                    self.data.linked_image_data.write().unwrap();
                                linked_image_data.insert(handle.clone(), Arc::new(image));
                            }
                            Err(err) => {
                                error!("Failed to load image at '{0:?}': {1}", path, err);
                            }
                        }
                    }
                }
            }
        }

        world.insert_resource(self);
    }

    fn find_handle_from_path(&self, path: impl AsRef<Path>) -> Result<BitmapHandle, BitmapError> {
        self.data
            .images
            .read()?
            .iter()
            .find_map(|data| match data {
                BitmapData::Embedded {
                    original_path: _, ..
                } => {
                    // We need to think carefully about what original_path means.
                    //
                    // When a file is saved on one system and opened on another system
                    // the exact same path may point to different images.
                    unimplemented!()
                }
                BitmapData::Linked { handle, path: p } => {
                    if p == path.as_ref() {
                        Some(handle.clone())
                    } else {
                        None
                    }
                }
            })
            .ok_or(BitmapError::InvalidPath(path.as_ref().to_path_buf()))
    }

    fn get_load_mode(&self, handle: &BitmapHandle) -> Result<LoadMode, BitmapError> {
        self.data
            .images
            .read()?
            .iter()
            .find(|&data| data.handle() == *handle)
            .map(|data| match data {
                BitmapData::Embedded { .. } => LoadMode::Embedded,
                BitmapData::Linked { .. } => LoadMode::Linked,
            })
            .ok_or(BitmapError::InvalidHandle(handle.clone()))
    }

    fn remove(&mut self, handle: &BitmapHandle) -> Result<Arc<Image>, BitmapError> {
        let image = match self.get_load_mode(handle)? {
            LoadMode::Embedded => unimplemented!(),
            LoadMode::Linked => self.data.linked_image_data.write()?.remove(handle).unwrap(),
        };

        self.data
            .images
            .write()?
            .retain_mut(|data| data.handle() != *handle);

        Ok(image)
    }
}

// We need a manual implementation of FromWorld as deriving it will
// be like calling Default::default and not returning a clone of the
// resource stored in the world.
impl FromWorld for BitmapServer {
    fn from_world(world: &mut World) -> Self {
        world
            .get_resource::<Self>()
            .cloned()
            .unwrap_or_else(|| BitmapServer {
                data: Arc::new(BitmapServerData::default()),
            })
    }
}

// LIB

#[derive(Debug, Error)]
pub enum BitmapError {
    #[error("Image load error: {0}")]
    ImageLoadError(ImageLoadError),
    #[error("Invalid handle: {0:?}")]
    InvalidHandle(BitmapHandle),
    #[error("Invalid path: {0:?}")]
    InvalidPath(PathBuf),
    #[error("Cannot access lock.")]
    LockPoisonError,
}

impl From<ImageLoadError> for BitmapError {
    fn from(err: ImageLoadError) -> Self {
        Self::ImageLoadError(err)
    }
}

impl<T> From<PoisonError<T>> for BitmapError {
    fn from(_err: PoisonError<T>) -> Self {
        Self::LockPoisonError
    }
}

#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Reflect, Serialize)]
pub struct BitmapHandle {
    id: Arc<BitmapId>,
}

#[derive(Clone, Copy, Debug, Default, Reflect)]
pub enum LoadMode {
    Embedded,
    #[default]
    Linked,
}

#[derive(Debug, Deserialize, PartialEq, Reflect, Serialize)]
enum BitmapData {
    Embedded {
        handle: BitmapHandle,
        original_path: PathBuf,
    },
    Linked {
        handle: BitmapHandle,
        path: PathBuf,
    },
}

impl BitmapData {
    fn handle(&self) -> BitmapHandle {
        match self {
            Self::Embedded { handle, .. } => handle.clone(),
            Self::Linked { handle, .. } => handle.clone(),
        }
    }
}

#[derive(Debug, Default, Deserialize, Serialize)]
struct BitmapServerData {
    images: RwLock<Vec<BitmapData>>,
    embedded_image_data: RwLock<HashMap<BitmapHandle, Arc<Image>>>,
    #[serde(skip)]
    linked_image_data: RwLock<HashMap<BitmapHandle, Arc<Image>>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    const SMILEY_FILE_PATH: &str = "test_assets/smiley_heightmap.png";

    #[test]
    fn bitmap_server_load_returns_handle() {
        let file_path: PathBuf = {
            let mut file_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
            file_path.push(SMILEY_FILE_PATH);
            file_path
        };

        let mut app = App::new();
        app.add_plugins(BitmapPlugin);

        let bitmap_server = app.world().resource::<BitmapServer>();
        let result = bitmap_server.load(&file_path, LoadMode::Linked);
        assert!(result.is_ok());
        // Two in the store, one we are holding onto.
        assert_eq!(Arc::strong_count(&result.unwrap().id), 3);
    }

    #[test]
    fn bitmap_server_get_returns_a_reference_to_image_data() {
        let file_path: PathBuf = {
            let mut file_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
            file_path.push(SMILEY_FILE_PATH);
            file_path
        };

        let mut app = App::new();
        app.add_plugins(BitmapPlugin);

        let bitmap_server = app.world().resource::<BitmapServer>();
        let result = bitmap_server.load(&file_path, LoadMode::Linked);
        assert!(result.is_ok());
        let maybe_image: Result<Arc<Image>, BitmapError> = bitmap_server.get(&result.unwrap());
        assert!(maybe_image.is_ok())
    }
}
