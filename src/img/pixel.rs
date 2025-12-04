use bitvec::prelude::{BitSlice, BitVec, Lsb0};
use image::RgbaImage;

use super::common::check_capacity_image;

pub fn set_bits_image(img: &mut RgbaImage, bits: &BitSlice<u8, Lsb0>) -> Result<(), String> {
    check_capacity_image(img, bits)?;

    let mut iter = img.iter_mut();
    for bit in bits {
        if let Some(channel) = iter.next() {
            *channel = (*channel & 0xFE) | (*bit as u8);
        }
    }

    Ok(())
}

pub fn get_bits_reader_images<'a>(imgs: &'a [RgbaImage]) -> impl Iterator<Item = &'a u8> + 'a {
    imgs.iter().flat_map(|img| img.iter())
}

pub fn read_bits<'a>(
    reader: &mut impl Iterator<Item = &'a u8>,
    length: usize,
) -> Result<BitVec<u8, Lsb0>, String> {
    let bytes: Vec<u8> = reader.take(length).map(|&v| v).collect();

    let batch_len = bytes.len();
    if batch_len < length {
        Err(format!(
            "Count of channels ({}) is fewer than length ({})",
            batch_len, length
        ))
    } else {
        let bits: BitVec<u8, Lsb0> = bytes.into_iter().map(|c| (c & 1) == 1).collect();
        Ok(bits)
    }
}
