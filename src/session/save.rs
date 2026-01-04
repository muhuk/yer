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

use std::fs;
use std::path::Path;

use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use rmp_serde;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::id::LayerId;
use crate::layer;
use crate::preview;

const CURRENT_SAVE_VERSION: u16 = 1;

// LIB

#[derive(Debug, Error)]
pub enum SaveError {
    #[error("decode error: {0}")]
    DecodeError(rmp_serde::decode::Error),
    #[error("encode error: {0}")]
    EncodeError(rmp_serde::encode::Error),
    #[error("io error: {0}")]
    IoError(std::io::Error),
}

pub fn load(path: &Path, world: &mut World) -> Result<(), SaveError> {
    let save_container: SaveContainer = fs::read(path)
        .map_err(|e| SaveError::IoError(e))
        .map(|bytes| SaveContainer::from_bytes(&bytes))??;
    assert!(save_container.version == CURRENT_SAVE_VERSION);
    let save_data = SaveV1::from_bytes(&save_container.data)?;
    layer::LayerBundle::insert_all(world, save_data.layers);
    world.spawn_batch(save_data.preview_regions);
    layer::MaskBundle::insert_all(world, save_data.masks);
    Ok(())
}

pub fn save(path: &Path, world: &mut World) -> Result<(), SaveError> {
    let container = SaveContainer {
        version: CURRENT_SAVE_VERSION,
        data: (SaveV1 {
            layers: layer::LayerBundle::extract_all(world),
            preview_regions: preview::PreviewBundle::extract_all(world),
            masks: layer::MaskBundle::extract_all(world),
        })
        .to_bytes()?,
    };
    fs::write(path, container.to_bytes()?.as_slice()).map_err(|e| SaveError::IoError(e))
}

#[derive(Debug, Deserialize, Serialize, PartialEq)]
struct SaveContainer {
    version: u16,
    #[serde(with = "serde_bytes")]
    data: Vec<u8>,
}

impl SaveContainer {
    #[inline]
    fn to_bytes(&self) -> Result<Vec<u8>, SaveError> {
        rmp_serde::encode::to_vec_named(self).map_err(|e| SaveError::EncodeError(e))
    }

    #[inline]
    fn from_bytes(bytes: &[u8]) -> Result<Self, SaveError> {
        rmp_serde::decode::from_slice(bytes).map_err(|e| SaveError::DecodeError(e))
    }
}

// TODO: Store bake config
// TODO: Store cached preview mesh
#[derive(Deserialize, Serialize)]
struct SaveV1 {
    layers: Vec<layer::LayerBundle>,
    preview_regions: Vec<preview::PreviewBundle>,
    masks: HashMap<LayerId, Vec<layer::MaskBundle>>,
}

impl SaveV1 {
    #[inline]
    fn to_bytes(&self) -> Result<Vec<u8>, SaveError> {
        rmp_serde::encode::to_vec_named(self).map_err(|e| SaveError::EncodeError(e))
    }

    #[inline]
    fn from_bytes(bytes: &[u8]) -> Result<Self, SaveError> {
        rmp_serde::decode::from_slice(bytes).map_err(|e| SaveError::DecodeError(e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decoding_and_encoding_arbitrary_data() {
        let save_data = SaveContainer {
            version: 0,
            data: {
                let mut data: Vec<u8> = vec![];
                for i in 0..1000 {
                    data.insert(i, (i % 256) as u8);
                }
                data
            },
        };
        let save_result = save_data.to_bytes();
        assert!(save_result.is_ok());
        // if let Ok(ref save_result) = save_result {
        //     println!("Size = {}", save_result.len());
        // }
        let load_result = SaveContainer::from_bytes(&save_result.unwrap());
        assert!(load_result.is_ok());
        assert_eq!(save_data, load_result.unwrap());
    }
}
