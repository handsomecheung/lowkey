use bitvec::prelude::{BitSlice, BitVec, Lsb0};
use image::RgbaImage;

fn check_capacity(capacity_bit_count: usize, bit_count: usize) -> Result<(), String> {
    if bit_count > capacity_bit_count {
        return Err(format!(
            "Message is too long for the image. Capacity: {} bits, required: {} bits",
            capacity_bit_count, bit_count
        ));
    }

    Ok(())
}

pub fn check_capacity_image(img: &mut RgbaImage, bits: &BitSlice<u8, Lsb0>) -> Result<(), String> {
    let (width, height) = img.dimensions();
    let capacity_bit_count = width as usize * height as usize * 4;

    check_capacity(capacity_bit_count, bits.len())?;

    Ok(())
}

pub fn check_capacity_images(imgs: &[&RgbaImage], bits: &BitSlice<u8, Lsb0>) -> Result<(), String> {
    let capacity_bit_count: u32 = imgs
        .iter()
        .map(|img| {
            let (width, height) = img.dimensions();
            width * height * 4
        })
        .sum();

    check_capacity(capacity_bit_count as usize, bits.len())?;

    Ok(())
}

pub fn convert_bytes_to_bits(bytes: &[u8]) -> BitVec<u8, Lsb0> {
    bytes
        .iter()
        .flat_map(|&byte| (0..8).map(move |bit_pos| (byte >> bit_pos) & 1 == 1))
        .collect()
}

pub fn check_image_png(path: &str) -> Result<(), String> {
    let path_lower = path.to_lowercase();
    if path_lower.ends_with(".jpg") || path_lower.ends_with(".jpeg") {
        return Err("JPEG format is not supported. JPEG's lossy compression will destroy the hidden data. Please use PNG format instead.".to_string());
    }

    Ok(())
}
