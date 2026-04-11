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
use image::{DynamicImage, GenericImageView, ImageFormat, Pixel};
use serde::{Deserialize, Serialize};

use crate::id::LayerId;
use crate::math::{Alpha, ApproxEq, Sample, Sampler2D, Transform2D};

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
    Bitmap {
        bitmap: Bitmap,
        transform: Transform2D,
        repeat_mode: BitmapRepeatMode,
    },
    Constant(f32),
}

impl Default for HeightMap {
    fn default() -> Self {
        Self::Constant(0.0)
    }
}

impl Sampler2D for HeightMap {
    fn sample(&self, position: Vec2, _base: &Sample) -> Sample {
        match self {
            Self::Bitmap {
                bitmap,
                transform,
                repeat_mode,
            } => {
                let offset = bitmap.size().as_vec2() * 0.5;
                // FIXME: Remove 20.0 multiplier, use height param.
                let value = bitmap.sample(transform.apply(position) + offset, *repeat_mode) * 20.0;
                Sample::new(value, Alpha::Opaque)
            }
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

impl Bitmap {
    const NORMALIZE_HEIGHT: f32 = 1.0 / 256.0;

    pub fn sample(&self, position: Vec2, repeat_mode: BitmapRepeatMode) -> f32 {
        // FIXME: Apply non-uniform scaling.

        let image_position = repeat_mode.apply(position, self.size());

        // TODO: We need a setting for which channel to sample in Bitmap.
        //       Currently we are assuming it is grayscale.
        f32::from(
            self.image()
                .1
                .get_pixel(
                    image_position.x.round() as u32,
                    image_position.y.round() as u32,
                )
                .to_luma()
                .0[0],
        ) * Self::NORMALIZE_HEIGHT
    }

    fn image(&self) -> &Image {
        match self {
            Self::Embedded { image, .. } => &image,
            Self::Linked { image, .. } => image.as_ref().unwrap(),
        }
    }

    fn size(&self) -> UVec2 {
        let (width, height) = self.image().1.dimensions();
        UVec2::new(width, height)
    }
}

#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Reflect, Serialize)]
pub enum BitmapRepeatMode {
    Extend,
    Fade,
    #[default]
    Repeat,
}

impl BitmapRepeatMode {
    pub fn apply(&self, position: Vec2, size: UVec2) -> Vec2 {
        match self {
            Self::Extend => Vec2::new(
                position.x.max(0.0).min((size.x - 1) as f32),
                position.y.max(0.0).min((size.y - 1) as f32),
            ),
            Self::Fade => unimplemented!(),
            Self::Repeat => Vec2::new(
                Self::normalize(position.x, size.x as f32),
                Self::normalize(position.y, size.y as f32),
            ),
        }
    }

    fn normalize(x: f32, boundary: f32) -> f32 {
        let mut n = x;
        while n > boundary || n.approx_eq(boundary, None) {
            n -= boundary;
        }
        while n < 0.0 {
            n += boundary;
        }
        n
    }
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

    use crate::math::ApproxEq;

    use super::*;

    #[test]
    fn bitmap_serialization_roundtrip() {
        const TEST_FILE_PATH: &str = "test_assets/smiley_heightmap.png";

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

    #[test]
    fn bitmap_repeat_mode_repeating() {
        // (input,  output)
        const CASES: &[(Vec2, Vec2)] = &[
            // Within boundaries, unchanged.
            (Vec2::new(24.0, 18.0), Vec2::new(24.0, 18.0)),
            // Out of boundaries in positive direction.
            (Vec2::new(68.0, 62.0), Vec2::new(4.0, 62.0)),
            (Vec2::new(3.0, 80.0), Vec2::new(3.0, 16.0)),
            // Exact boundary is converted to 0.0
            (Vec2::new(7.0, 128.0), Vec2::new(7.0, 0.0)),
            (Vec2::new(128.0, 8.0), Vec2::new(0.0, 8.0)),
            (Vec2::new(256.0, 192.0), Vec2::new(0.0, 0.0)),
            // Out of boundaries in negative direction.
            (Vec2::new(-4.0, -61.5), Vec2::new(60.0, 2.5)),
            (Vec2::new(-129.0, -0.995), Vec2::new(63.0, 63.005)),
        ];
        const SIZE: UVec2 = UVec2::new(64, 64);

        let mode = BitmapRepeatMode::Repeat;

        for (input, output) in CASES.iter() {
            eprintln!("{:?}", mode.apply(*input, SIZE));
            assert!(mode.apply(*input, SIZE).approx_eq(*output, None));
        }
    }
}
