// Copyright © 2024 Atamert Ölçgen.
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

use bevy::prelude::*;
use rmp_serde;
use serde::{Deserialize, Serialize};

use crate::layer;

// LIB

#[derive(Deserialize, Serialize)]
struct LayerComponents {
    layer: layer::Layer,
    height_map: layer::HeightMap,
}

#[derive(Debug, Deserialize, Serialize, PartialEq)]
struct SaveContainer {
    version: u16,
    #[serde(with = "serde_bytes")]
    data: Vec<u8>,
}

impl SaveContainer {
    fn to_bytes(&self) -> Result<Vec<u8>, SaveError> {
        rmp_serde::encode::to_vec_named(self).map_err(|e| SaveError::EncodeError(e))
    }

    fn from_bytes(bytes: &[u8]) -> Result<Self, SaveError> {
        rmp_serde::decode::from_slice(bytes).map_err(|e| SaveError::DecodeError(e))
    }
}

#[derive(Deserialize, Serialize)]
struct SaveV1 {
    layers: Vec<LayerComponents>,
    // TODO: Store preview config
    // TODO: Store bake config
    // TODO: Store cached preview mesh
}

#[derive(Debug)]
enum SaveError {
    DecodeError(rmp_serde::decode::Error),
    EncodeError(rmp_serde::encode::Error),
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
