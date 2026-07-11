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
use std::path::Path;

use bevy::prelude::*;
use image::{DynamicImage, GenericImageView, ImageFormat, ImageReader, Pixel};
use serde::{
    de::{self, Deserializer, Visitor},
    ser::{self, Serializer},
    Deserialize, Serialize,
};

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

    pub(super) fn load_from_disk(file_path: impl AsRef<Path>) -> Self {
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
    use std::path::PathBuf;

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
}
