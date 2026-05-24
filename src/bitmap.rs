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
use std::sync::{Arc, RwLock};

use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use image::{DynamicImage, GenericImageView, ImageFormat, ImageReader, Pixel};
use serde::{
    de::{self, Deserializer, Visitor},
    ser::{self, Serializer},
    Deserialize, Serialize,
};

use crate::id::BitmapId;

// PLUGIN

pub struct BitmapPlugin;

impl Plugin for BitmapPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<BitmapServer>();
    }
}

// RESOURCES

#[derive(Clone, Debug, Deserialize, FromWorld, Reflect, Resource, Serialize)]
#[reflect(Clone, Resource)]
pub struct BitmapServer {
    data: Arc<BitmapServerData>,
}

impl BitmapServer {
    pub fn get(&self, handle: &BitmapHandle) -> Option<Arc<Image>> {
        let maybe_embedded_or_linked = self
            .data
            .images
            .read()
            .map(|images| {
                images
                    .iter()
                    .find(|&data| data.handle() == *handle)
                    .map(|data| match data {
                        BitmapData::Embedded { .. } => true,
                        BitmapData::Linked { .. } => false,
                    })
            })
            .ok()
            .flatten();

        match maybe_embedded_or_linked {
            None => {
                // TODO: Create a proper result type for this.
                //       Do not handle 2 error cases in one.
                //
                //       Do not use bools too.
                error!("Invalid handle or we cannot access images.");
                None
            }
            // Embedded
            Some(true) => unimplemented!(),
            // Linked
            Some(false) => Some(
                self.data
                    .linked_image_data
                    .read()
                    .unwrap()
                    .get(handle)
                    // TODO: hashmap should have this key, but in case it does
                    //       not, handle this case.
                    .unwrap()
                    .clone(),
            ),
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
}

// LIB

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

#[derive(Debug, PartialEq, Reflect)]
pub struct Image {
    #[reflect(ignore)]
    #[reflect(default = "default_image_format")]
    format: ImageFormat,
    #[reflect(ignore)]
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

#[derive(Debug, Default, Deserialize, Reflect, Serialize)]
struct BitmapServerData {
    #[reflect(ignore)]
    images: RwLock<Vec<BitmapData>>,
    #[reflect(ignore)]
    embedded_image_data: RwLock<HashMap<BitmapHandle, Arc<Image>>>,
    #[serde(skip)]
    #[reflect(ignore)]
    linked_image_data: RwLock<HashMap<BitmapHandle, Arc<Image>>>,
}

fn default_image_format() -> ImageFormat {
    ImageFormat::Png
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
        let maybe_image: Option<Arc<Image>> = bitmap_server.get(&handle);
        assert!(maybe_image.is_some())
    }
}
