use bitvec::prelude::{BitVec, Lsb0};
use image::RgbaImage;
use std::fs::{self, File};
use std::io::Read;
use std::io::Write;
use std::path::Path;

use super::common::{check_capacity_images, check_image_png, convert_bytes_to_bits};
use super::io::{read_image, save_rgba_with_metadata};
use super::pixel::{get_bits_reader_images, read_bits, set_bits_image};
use super::resize::resize_image;

pub fn encode_from_file(
    input_image: &str,
    message_file: &str,
    output_image: &str,
    auto_resize: bool,
) -> Result<(), String> {
    check_image_png(output_image)?;

    let mut img = read_image(input_image)?;

    let mut message_file_handle = File::open(message_file)
        .map_err(|e| format!("Failed to open message file '{}': {}", message_file, e))?;
    let mut message_bytes = Vec::new();
    message_file_handle
        .read_to_end(&mut message_bytes)
        .map_err(|e| format!("Failed to read message file: {}", e))?;

    if auto_resize {
        img = resize_image(&mut img, message_bytes.len(), 600)?;
    }

    let bits = get_message_bits(&message_bytes);
    set_bits_image(&mut img, &bits)?;

    if let Some(parent) = Path::new(output_image).parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create output directory: {}", e))?;
    }

    save_rgba_with_metadata(&img, output_image, input_image)?;

    Ok(())
}

pub fn encode_from_files(
    input_images: &[String],
    message_file: &str,
    output_dir: &str,
) -> Result<(), String> {
    if input_images.is_empty() {
        return Err("No input images provided".to_string());
    }

    let mut message_file_handle = File::open(message_file)
        .map_err(|e| format!("Failed to open message file '{}': {}", message_file, e))?;
    let mut message_bytes = Vec::new();
    message_file_handle
        .read_to_end(&mut message_bytes)
        .map_err(|e| format!("Failed to read message file: {}", e))?;

    let mut images: Vec<(String, RgbaImage)> = input_images
        .iter()
        .map(|image_path| read_image(image_path).map(|img| (image_path.clone(), img)))
        .collect::<Result<Vec<_>, _>>()?;

    fs::create_dir_all(output_dir)
        .map_err(|e| format!("Failed to create output directory: {}", e))?;

    let bits = get_message_bits(&message_bytes);

    check_capacity_images(
        &images.iter().map(|(_, img)| img).collect::<Vec<_>>(),
        &bits,
    )?;

    let total_bits = bits.len();
    let images_count = images.len();
    let mut cursor = 0usize;

    for (i, (image_path, img)) in images.iter_mut().enumerate() {
        if cursor >= total_bits {
            break;
        }

        let (width, height) = img.dimensions();
        let image_capacity_bits = (width * height * 4) as usize;

        let bits_to_encode = std::cmp::min(image_capacity_bits, total_bits - cursor);
        let next_cursor = cursor + bits_to_encode;

        set_bits_image(img, &bits[cursor..next_cursor])?;
        cursor = next_cursor;

        let filename = Path::new(image_path)
            .file_name()
            .ok_or_else(|| format!("Invalid input path: {}", image_path))?;

        let mut output_filename = filename.to_string_lossy().to_string();
        if output_filename.to_lowercase().ends_with(".jpg")
            || output_filename.to_lowercase().ends_with(".jpeg")
        {
            if let Some(pos) = output_filename.rfind('.') {
                output_filename = format!("{}.png", &output_filename[..pos]);
            }
        }

        let output_path = Path::new(output_dir).join(&output_filename);
        let output_path_str = output_path.to_string_lossy().to_string();

        save_rgba_with_metadata(&img, &output_path_str, image_path)?;
        println!(
            "Saved encoded image {}/{}: {}",
            i + 1,
            images_count,
            output_path_str
        );
    }

    Ok(())
}

pub fn decode_from_files(image_paths: &[String], output_file: &str) -> Result<(), String> {
    if image_paths.is_empty() {
        return Err("No input images provided".to_string());
    }

    let mut images: Vec<RgbaImage> = Vec::new();
    for image_path in image_paths {
        check_image_png(image_path)?;
        let img = image::open(image_path)
            .map_err(|e| format!("Failed to open image '{}': {}", image_path, e))?
            .to_rgba8();
        images.push(img);
    }

    let mut reader = get_bits_reader_images(&images);
    let message_count = {
        let header_count = 4;
        let bits = read_bits(&mut reader, header_count * 8)?;
        let len_bytes: [u8; 4] = std::array::from_fn(|i| {
            (0..8).fold(0u8, |acc, j| {
                acc | ((*bits.get(i * 8 + j).unwrap() as u8) << j)
            })
        });
        u32::from_be_bytes(len_bytes)
    };

    let message_bytes: Vec<_> = {
        let bits = read_bits(&mut reader, message_count as usize * 8)?;
        bits.chunks(8)
            .map(|chunk| {
                chunk
                    .iter()
                    .enumerate()
                    .fold(0u8, |acc, (i, bit)| acc | ((*bit as u8) << i))
            })
            .collect()
    };

    if let Some(parent) = Path::new(output_file).parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create output directory: {}", e))?;
    }

    let mut file = File::create(output_file)
        .map_err(|e| format!("Failed to create output file '{}': {}", output_file, e))?;
    file.write_all(&message_bytes)
        .map_err(|e| format!("Failed to write to output file: {}", e))?;

    Ok(())
}

fn get_message_bits(message_bytes: &[u8]) -> BitVec<u8, Lsb0> {
    let message_len = message_bytes.len() as u32;
    let message_len_bytes = message_len.to_be_bytes();

    let mut data = Vec::new();
    data.extend_from_slice(&message_len_bytes);
    data.extend_from_slice(message_bytes);

    convert_bytes_to_bits(&data)
}
