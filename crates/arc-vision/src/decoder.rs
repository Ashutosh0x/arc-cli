// SPDX-License-Identifier: MIT
use anyhow::Result;
use base64::{Engine, engine::general_purpose::STANDARD};
use image::DynamicImage;
use std::path::Path;

pub struct ImageDecoder;

impl ImageDecoder {
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<DynamicImage> {
        Ok(image::open(path)?)
    }

    pub fn to_base64(image: &DynamicImage) -> Result<String> {
        let mut bytes: Vec<u8> = Vec::new();
        image.write_to(
            &mut std::io::Cursor::new(&mut bytes),
            image::ImageFormat::Jpeg,
        )?;
        Ok(STANDARD.encode(&bytes))
    }

    pub fn load_and_encode<P: AsRef<Path>>(path: P) -> Result<String> {
        let img = Self::from_file(path)?;
        Self::to_base64(&img)
    }
}
