use bitvec::prelude::{BitVec, Lsb0};
use image::RgbaImage;
use std::fs::{self, File};
use std::io::Read;
use std::io::Write;
use std::path::Path;

use super::common::{check_capacity_images, check_image_png, convert_bytes_to_bits};
use super::io::{read_image, read_sequence_info, save_rgba_with_metadata};
use super::pixel::{get_bits_reader_images, read_bits, set_bits_image};
use super::resize::resize_image;
use crate::crypto;

/// Protocol version for the steganography format
/// Version 0: [1 byte version] + [4 bytes message length] + [encrypted message data]
const PROTOCOL_VERSION: u8 = 0;

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

    let bits = get_message_bits(&message_bytes)?;
    set_bits_image(&mut img, &bits)?;

    if let Some(parent) = Path::new(output_image).parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create output directory: {}", e))?;
    }

    save_rgba_with_metadata(&img, output_image, input_image, None)?;

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

    let bits = get_message_bits(&message_bytes)?;

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

        let sequence_info = Some((i as u32, images_count as u32));
        save_rgba_with_metadata(&img, &output_path_str, image_path, sequence_info)?;
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

    let mut paths_with_sequence: Vec<(String, Option<(u32, u32)>)> = image_paths
        .iter()
        .map(|path| {
            let seq_info = read_sequence_info(path).unwrap_or(None);
            (path.clone(), seq_info)
        })
        .collect();

    let all_have_sequence = paths_with_sequence
        .iter()
        .all(|(_, seq_info)| seq_info.is_some());

    if all_have_sequence {
        paths_with_sequence.sort_by_key(|(_, seq_info)| seq_info.unwrap().0);
        println!("Detected sequence information in PNG metadata, using automatic ordering");
    }

    let sorted_paths: Vec<String> = paths_with_sequence
        .into_iter()
        .map(|(path, _)| path)
        .collect();

    let mut images: Vec<RgbaImage> = Vec::new();
    for image_path in &sorted_paths {
        check_image_png(image_path)?;
        let img = image::open(image_path)
            .map_err(|e| format!("Failed to open image '{}': {}", image_path, e))?
            .to_rgba8();
        images.push(img);
    }

    let mut reader = get_bits_reader_images(&images);

    let message_count = {
        let header_count = 5;
        let bits = read_bits(&mut reader, header_count * 8)?;
        let header_bytes: [u8; 5] = std::array::from_fn(|i| {
            (0..8).fold(0u8, |acc, j| {
                acc | ((*bits.get(i * 8 + j).unwrap() as u8) << j)
            })
        });

        let version = header_bytes[0];
        let len_bytes: [u8; 4] = header_bytes[1..5].try_into().unwrap();
        let count = u32::from_be_bytes(len_bytes);

        if version != PROTOCOL_VERSION {
            return Err(format!(
                "Unsupported protocol version {}. Expected version {}",
                version, PROTOCOL_VERSION
            ));
        }

        count
    };

    let encrypted_bytes: Vec<_> = {
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

    let message_bytes = crypto::decrypt(&encrypted_bytes)?;

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

fn get_message_header_bytes(body_bytes: &[u8]) -> [u8; 5] {
    let message_len = body_bytes.len() as u32;
    let message_len_bytes = message_len.to_be_bytes();

    let mut head = [0u8; 5];
    head[0] = PROTOCOL_VERSION;
    head[1..5].copy_from_slice(&message_len_bytes);

    head
}

fn get_message_body_bytes(message_bytes: &[u8]) -> Result<Vec<u8>, String> {
    let encrypted_bytes = crypto::encrypt(message_bytes)?;
    Ok(encrypted_bytes)
}

fn get_message_bits(message_bytes: &[u8]) -> Result<BitVec<u8, Lsb0>, String> {
    let body_bytes = get_message_body_bytes(message_bytes)?;
    let header_bytes = get_message_header_bytes(&body_bytes);

    let mut data = Vec::new();
    data.extend_from_slice(&header_bytes);
    data.extend_from_slice(&body_bytes);

    Ok(convert_bytes_to_bits(&data))
}
