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

use std::io::{BufReader, BufWriter, Cursor};
use std::path::{Path, PathBuf};
use std::sync::{Arc, PoisonError, RwLock};

use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use image::{DynamicImage, GenericImageView, ImageFormat, ImageReader, Pixel};
use serde::{
    de::{self, Deserializer, Visitor},
    ser::{self, Serializer},
    Deserialize, Serialize,
};
use thiserror::Error;

use crate::id::BitmapId;

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
            if self.is_embedded(handle)? {
                unimplemented!();
            } else {
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

    fn is_embedded(&self, handle: &BitmapHandle) -> Result<bool, BitmapGetError> {
        if let Some(is_embedded) = self
            .data
            .images
            .read()?
            .iter()
            .find(|&data| data.handle() == *handle)
            .map(|data| match data {
                BitmapData::Embedded { .. } => true,
                BitmapData::Linked { .. } => false,
            })
        {
            Ok(is_embedded)
        } else {
            Err(BitmapGetError::InvalidHandle(handle.clone()))
        }
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

#[derive(Debug, PartialEq)]
pub struct Image {
    format: ImageFormat,
    image: DynamicImage,
}

impl Image {
    // FIXME: What if the image is not 8-bit?
    const NORMALIZE_HEIGHT: f32 = 1.0 / 256.0;

    pub fn get_pixel_luma(&self, position: UVec2) -> f32 {
        f32::from(self.image.get_pixel(position.x, position.y).to_luma().0[0])
            * Self::NORMALIZE_HEIGHT
    }

    pub fn size(&self) -> UVec2 {
        UVec2::from(self.image.dimensions())
    }

    fn load_from_disk(file_path: impl AsRef<Path>) -> Self {
        let (format, image) = {
            let reader = image::ImageReader::open(file_path).unwrap();
            (reader.format().unwrap(), reader.decode().unwrap())
        };

        Self { format, image }
    }
}

impl<'de> Deserialize<'de> for Image {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_bytes(ImageVisitor)
    }
}

impl Serialize for Image {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut data: Vec<u8> = vec![];
        let writer = BufWriter::new(Cursor::new(&mut data));
        self.image
            .write_to(writer, self.format)
            .map_err(ser::Error::custom)?;
        serializer.serialize_bytes(&data[..])
    }
}

struct ImageVisitor;

impl<'de> Visitor<'de> for ImageVisitor {
    type Value = Image;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(formatter, "bytes content of an image file")
    }

    fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        let reader = BufReader::new(Cursor::new(v));

        let image_reader = ImageReader::new(reader)
            .with_guessed_format()
            .map_err(de::Error::custom)?;
        let image_format = image_reader
            .format()
            .ok_or("Cannot determine image format.")
            .map_err(de::Error::custom)?;
        let image = image_reader.decode().map_err(de::Error::custom)?;
        Ok(Image {
            format: image_format,
            image,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SMILEY_FILE_PATH: &str = "test_assets/smiley_heightmap.png";

    #[test]
    fn bitmap_serialization_roundtrip() {
        let file_path: PathBuf = {
            let mut file_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
            file_path.push(SMILEY_FILE_PATH);
            file_path
        };

        let image = {
            let reader = ImageReader::open(&file_path).unwrap();
            Image {
                format: reader.format().unwrap(),
                image: reader.decode().unwrap(),
            }
        };

        let serialized = rmp_serde::to_vec(&image).unwrap();
        let deserialized: Image = rmp_serde::from_slice(serialized.as_slice()).unwrap();

        assert_eq!(deserialized, image);
    }

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
