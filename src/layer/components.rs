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

use std::fmt::{self, Display};
use std::ops::RangeInclusive;
use std::path::PathBuf;

use bevy::ecs::{lifecycle::HookContext, world::DeferredWorld};
use bevy::prelude::*;
use image::{DynamicImage, ImageFormat};
use serde::{Deserialize, Serialize};

use crate::id::LayerId;
use crate::math::{Alpha, Sample, Sampler2D};

pub const HEIGHT_RANGE: RangeInclusive<f32> = -16000.0..=64000.0;
pub const LAYER_SPACING: u32 = 100;

const DEFAULT_LAYER_NAME: &str = "<unnamed>";

// We need to store ImageFormat because DynamicImage is not
// serializable.  So we convert the DynamicImage to an image file and
// serialize its bytes instead.
//
// See serde_image module.
type Image = (ImageFormat, DynamicImage);

// PLUGIN

pub struct LayerComponentsPlugin;

impl Plugin for LayerComponentsPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<HeightMap>()
            .register_type::<Layer>()
            .register_type::<LayerOrder>()
            .register_type::<LayerId>();
        app.register_type::<NeedsLayerOrderNormalization>();
    }
}

// BUNDLES

#[derive(Bundle, Clone, Debug, Deserialize, Reflect, Serialize)]
pub struct LayerBundle {
    pub layer: Layer,
    pub name: Name,
    pub height_map: HeightMap,
}

impl LayerBundle {
    pub fn extract_all(world: &mut World) -> Vec<Self> {
        let mut layer_bundles = vec![];
        for (layer, name, height_map) in world
            .query::<(&Layer, &LayerOrder, &Name, &HeightMap)>()
            .iter(world)
            .sort::<&LayerOrder>()
            .map(|(l, _, n, h)| (l, n, h))
        {
            layer_bundles.push(Self {
                layer: layer.clone(),
                name: name.clone(),
                height_map: height_map.clone(),
            });
        }
        layer_bundles
    }

    pub fn insert_all(world: &mut World, layer_bundles: Vec<Self>) {
        layer_bundles
            .into_iter()
            .enumerate()
            .for_each(|(idx, layer_bundle)| {
                world.spawn((layer_bundle, LayerOrder(idx as u32 * LAYER_SPACING)));
            });
    }
}

impl Default for LayerBundle {
    fn default() -> Self {
        let layer = Layer::default();
        Self {
            name: layer.name_component(),
            layer,
            height_map: HeightMap::default(),
        }
    }
}

// COMPONENTS

#[derive(Component, Clone, Debug, Deserialize, PartialEq, Reflect, Serialize)]
#[require(Layer)]
pub enum HeightMap {
    Bitmap(Bitmap),
    Constant(f32),
}

impl Default for HeightMap {
    fn default() -> Self {
        Self::Constant(0.0)
    }
}

impl Sampler2D for HeightMap {
    fn sample(&self, _position: Vec2, _base: &Sample) -> Sample {
        match self {
            Self::Bitmap { .. } => todo!(),
            Self::Constant(value) => Sample::new(*value, Alpha::Opaque),
        }
    }
}

#[derive(Component, Clone, Debug, Deserialize, Reflect, Serialize)]
pub struct Layer {
    pub name: String,
    pub enable_baking: bool,
    pub enable_preview: bool,
    id: LayerId,
}

impl Layer {
    pub fn id(&self) -> LayerId {
        self.id
    }

    pub fn name_component(&self) -> Name {
        Name::new(format!("Layer 0x{}", &self.id.simple().to_string()[25..32]))
    }

    pub(super) fn new(id: LayerId) -> Self {
        Self {
            name: DEFAULT_LAYER_NAME.to_owned(),
            enable_baking: true,
            enable_preview: true,
            id,
        }
    }

    pub(super) fn new_id() -> LayerId {
        LayerId::now_v7()
    }
}

impl Default for Layer {
    fn default() -> Self {
        Self::new(Self::new_id())
    }
}

impl Display for Layer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            // Last 7 digits of uuid.
            &self.id.simple().to_string()[25..32]
        )
    }
}

#[derive(Component, Copy, Clone, Debug, Deref, Eq, Ord, PartialEq, PartialOrd, Reflect)]
#[component(on_remove = layer_order_on_remove_hook)]
#[require(Layer)]
pub struct LayerOrder(#[deref] pub(super) u32);

/// When a layer is despawned, and its [LayerOrder] is removed, this component
/// signals a layer reordering.
#[derive(Clone, Component, Debug, Reflect)]
pub struct NeedsLayerOrderNormalization;

// LIB

#[derive(Clone, Debug, Deserialize, PartialEq, Reflect, Serialize)]
pub enum Bitmap {
    Embedded {
        #[reflect(ignore)]
        #[reflect(default = "default_image")]
        #[serde(with = "serde_image")]
        image: Image,
        original_path: PathBuf,
    },
    Linked {
        #[reflect(ignore)]
        #[serde(skip)]
        image: Option<Image>,
        path: PathBuf,
    },
}

fn layer_order_on_remove_hook(mut world: DeferredWorld, HookContext { .. }: HookContext) {
    world.commands().spawn(NeedsLayerOrderNormalization);
}

fn default_image() -> Image {
    (ImageFormat::Png, DynamicImage::default())
}

mod serde_image {
    use std::io::{BufReader, BufWriter, Cursor};

    use image::ImageReader;
    use serde::{
        de::{self, Visitor},
        ser, Deserializer, Serializer,
    };

    use super::Image;

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Image, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_bytes(ImageVisitor)
    }

    pub fn serialize<S>(image: &Image, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let (image_format, dynamic_image) = image;
        let mut data: Vec<u8> = vec![];
        let writer = BufWriter::new(Cursor::new(&mut data));
        dynamic_image
            .write_to(writer, *image_format)
            .map_err(ser::Error::custom)?;
        serializer.serialize_bytes(&data[..])
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
            Ok((image_format, image))
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use image::ImageReader;
    use rmp_serde;

    use super::*;

    #[test]
    fn bitmap_serialization_roundtrip() {
        const TEST_FILE_PATH: &str = "test_assets/soft_circle_mask_grayscale.png";

        let file_path: PathBuf = {
            let mut file_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
            file_path.push(TEST_FILE_PATH);
            file_path
        };

        let (image_format, dynamic_image) = {
            let reader = ImageReader::open(&file_path).unwrap();
            (reader.format().unwrap(), reader.decode().unwrap())
        };

        let bitmap = Bitmap::Embedded {
            image: (image_format, dynamic_image),
            original_path: file_path,
        };

        let serialized = rmp_serde::to_vec(&bitmap).unwrap();
        let deserialized: Bitmap = rmp_serde::from_slice(serialized.as_slice()).unwrap();

        assert_eq!(deserialized, bitmap);
    }
}
