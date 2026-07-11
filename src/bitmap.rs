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

pub use self::img::Image;

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
    pub fn get(&self, handle: &BitmapHandle) -> Result<Arc<Image>, BitmapGetError> {
        {
            match self.get_load_mode(handle)? {
                LoadMode::Embedded => unimplemented!(),
                LoadMode::Linked => {
                    Ok(self
                        .data
                        .linked_image_data
                        .read()
                        .unwrap()
                        .get(handle)
                        // Hashmap should have this key.
                        .unwrap()
                        .clone())
                }
            }
        }
    }

    pub fn load(&self, path: impl AsRef<Path>, load_mode: LoadMode) -> BitmapHandle {
        if !matches!(load_mode, LoadMode::Linked) {
            unimplemented!("embedded mode is not implemented yet");
        }

        let handle = {
            let mut images = self.data.images.write().unwrap();
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
            let mut linked_inage_data = self.data.linked_image_data.write().unwrap();
            linked_inage_data.insert(handle.clone(), Arc::new(Image::load_from_disk(path)));
        }

        handle
    }

    pub fn replace_in_world(self, world: &mut World) {
        // We can reuse already loaded linked images.
        let old_self = world.remove_resource::<Self>();

        {
            let mut linked_inage_data = self.data.linked_image_data.write().unwrap();
            for bitmap_data in self.data.images.read().unwrap().iter() {
                match bitmap_data {
                    BitmapData::Embedded { .. } => (),
                    BitmapData::Linked { handle, path } => {
                        // FIXME: WTF is this monstrosity!!!!!!11111
                        if let Some(image) = old_self
                            .clone()
                            .map(|old_self| {
                                let maybe_old_handle = old_self
                                    .data
                                    .images
                                    .read()
                                    .unwrap()
                                    .iter()
                                    .filter_map(|data| {
                                        if let BitmapData::Linked {
                                            handle: old_handle,
                                            path: old_path,
                                        } = data
                                        {
                                            if old_path == path {
                                                Some(old_handle.clone())
                                            } else {
                                                None
                                            }
                                        } else {
                                            None
                                        }
                                    })
                                    .next();
                                if let Some(old_handle) = maybe_old_handle {
                                    old_self
                                        .data
                                        .linked_image_data
                                        .write()
                                        .unwrap()
                                        .remove(&old_handle)
                                } else {
                                    None
                                }
                            })
                            .flatten()
                        {
                            debug!("Reusing image data for {handle:?}.");
                            linked_inage_data.insert(handle.clone(), image.clone());
                        } else {
                            debug!("Loading image data from disk for {handle:?}.");
                            linked_inage_data
                                .insert(handle.clone(), Arc::new(Image::load_from_disk(path)));
                        }
                    }
                }
            }
        }

        world.insert_resource(self);
    }

    fn get_load_mode(&self, handle: &BitmapHandle) -> Result<LoadMode, BitmapGetError> {
        self.data
            .images
            .read()?
            .iter()
            .find(|&data| data.handle() == *handle)
            .map(|data| match data {
                BitmapData::Embedded { .. } => LoadMode::Embedded,
                BitmapData::Linked { .. } => LoadMode::Linked,
            })
            .ok_or(BitmapGetError::InvalidHandle(handle.clone()))
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
pub enum BitmapGetError {
    #[error("cannot access BitmapServerData.images")]
    CannotAccessImages,
    #[error("invalid handle: {0:?}")]
    InvalidHandle(BitmapHandle),
}

impl<T> From<PoisonError<T>> for BitmapGetError {
    fn from(_value: PoisonError<T>) -> Self {
        Self::CannotAccessImages
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
        let handle: BitmapHandle = bitmap_server.load(&file_path, LoadMode::Linked);
        // Two in the store, one we are holding onto.
        assert_eq!(Arc::strong_count(&handle.id), 3);
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
        let handle: BitmapHandle = bitmap_server.load(&file_path, LoadMode::Linked);
        let maybe_image: Result<Arc<Image>, BitmapGetError> = bitmap_server.get(&handle);
        assert!(maybe_image.is_ok())
    }
}
