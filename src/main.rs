use image::RgbaImage;
use png::{BitDepth, ColorType, Encoder};
use std::env;
use std::fs::File;
use std::io::{BufReader, BufWriter};

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        println!("Usage: lowkey <command> [args]");
        println!("Commands:");
        println!("  encode <input_image> <message> <output_image>");
        println!("  decode <image>");
        return;
    }

    let command = &args[1];

    match command.as_str() {
        "encode" => {
            if args.len() != 5 {
                println!("Usage: lowkey encode <input_image> <message> <output_image>");
                return;
            }
            let input_image = &args[2];
            let message = &args[3];
            let output_image = &args[4];
            match encode_from_file(input_image, message, output_image) {
                Ok(_) => println!("Encoded message into {}", output_image),
                Err(e) => eprintln!("Error encoding message: {}", e),
            }
        }
        "decode" => {
            if args.len() != 3 {
                println!("Usage: lowkey decode <image>");
                return;
            }
            let image_path = &args[2];
            match decode_from_file(image_path) {
                Ok(message) => println!("Decoded message: {}", message),
                Err(e) => eprintln!("Error decoding message: {}", e),
            }
        }
        _ => {
            println!("Unknown command: {}", command);
            println!("Usage: lowkey <command> [args]");
        }
    }
}

struct PngChunk {
    chunk_type: [u8; 4],
    data: Vec<u8>,
}

impl PngChunk {
    fn calculate_crc(&self) -> u32 {
        let mut crc_data = Vec::new();
        crc_data.extend_from_slice(&self.chunk_type);
        crc_data.extend_from_slice(&self.data);
        crc32(&crc_data)
    }

    fn write<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<()> {
        writer.write_all(&(self.data.len() as u32).to_be_bytes())?;
        writer.write_all(&self.chunk_type)?;
        writer.write_all(&self.data)?;
        writer.write_all(&self.calculate_crc().to_be_bytes())?;
        Ok(())
    }
}

fn crc32(data: &[u8]) -> u32 {
    let mut crc = 0xFFFFFFFFu32;
    for &byte in data {
        crc ^= byte as u32;
        for _ in 0..8 {
            if crc & 1 != 0 {
                crc = (crc >> 1) ^ 0xEDB88320;
            } else {
                crc >>= 1;
            }
        }
    }
    !crc
}

/// Save RGBA image with all PNG metadata chunks preserved from the original file.
///
/// # Why preserve metadata?
///
/// This function ensures that PNG ancillary chunks (metadata) from the original image
/// are preserved in the output image. This is critical for maintaining visual consistency.
///
/// ## ICC Color Profile (iCCP chunk)
///
/// The most important metadata is the **ICC color profile** (iCCP chunk). This defines
/// how RGB numeric values should be interpreted and displayed as actual colors.
///
/// ### The Problem Without ICC Preservation:
/// - Original image: RGB(255,0,0) + Apple Display P3 profile → vibrant red
/// - Output without ICC: RGB(255,0,0) + default sRGB → duller red
/// - **Result: Same pixel values, different displayed colors!**
///
/// When the `image` crate's simple `save()` method is used, it strips ALL metadata,
/// including the ICC profile. This causes the output image to be interpreted as sRGB
/// by default, leading to visible color shifts on ICC-aware viewers (macOS Preview,
/// Photoshop, Safari, etc.).
///
/// ## Other Preserved Metadata:
/// - **eXIf**: Camera settings, timestamps, GPS coordinates
/// - **iTXt**: Image description, copyright, keywords
/// - **tIME**: Last modification time
/// - **pHYs**: Physical pixel dimensions (DPI)
///
/// ## Implementation Strategy:
///
/// 1. **Extract**: Read all ancillary chunks from the input PNG file
/// 2. **Generate**: Use `png` crate to encode the pixel data (IHDR + IDAT + IEND)
/// 3. **Inject**: Insert the extracted metadata chunks between IHDR and IDAT
/// 4. **Output**: Write the complete PNG with metadata preserved
///
/// This ensures the steganography process is truly "invisible" - not just in terms
/// of the hidden data, but also in maintaining the exact visual appearance of the
/// original image.
fn save_rgba_with_metadata(
    img: &RgbaImage,
    output_path: &str,
    input_path: &str,
) -> Result<(), String> {
    use std::io::{Read, Write};

    // Step 1: Extract metadata chunks from original file
    let input_file = File::open(input_path).map_err(|e| e.to_string())?;
    let mut input_reader = BufReader::new(input_file);

    // Skip PNG signature
    let mut signature = [0u8; 8];
    input_reader
        .read_exact(&mut signature)
        .map_err(|e| e.to_string())?;

    // Collect metadata chunks
    let mut metadata_chunks = Vec::new();
    loop {
        let mut length_bytes = [0u8; 4];
        if input_reader.read_exact(&mut length_bytes).is_err() {
            break;
        }
        let length = u32::from_be_bytes(length_bytes) as usize;

        let mut chunk_type = [0u8; 4];
        input_reader
            .read_exact(&mut chunk_type)
            .map_err(|e| e.to_string())?;

        let mut chunk_data = vec![0u8; length];
        input_reader
            .read_exact(&mut chunk_data)
            .map_err(|e| e.to_string())?;

        let mut _crc = [0u8; 4];
        input_reader
            .read_exact(&mut _crc)
            .map_err(|e| e.to_string())?;

        // Save ancillary chunks (not IHDR, IDAT, IEND, PLTE)
        let chunk_type_str = std::str::from_utf8(&chunk_type).unwrap_or("");
        match chunk_type_str {
            "IHDR" | "IDAT" | "IEND" | "PLTE" => {
                if chunk_type_str == "IEND" {
                    break;
                }
            }
            _ => {
                metadata_chunks.push(PngChunk {
                    chunk_type,
                    data: chunk_data,
                });
            }
        }
    }

    // Step 2: Write new PNG with metadata using temp buffer
    let mut temp_buffer = Vec::new();
    {
        let temp_writer = BufWriter::new(&mut temp_buffer);
        let (width, height) = img.dimensions();
        let mut encoder = Encoder::new(temp_writer, width, height);
        encoder.set_color(ColorType::Rgba);
        encoder.set_depth(BitDepth::Eight);

        let mut writer = encoder.write_header().map_err(|e| e.to_string())?;
        writer
            .write_image_data(img.as_raw())
            .map_err(|e| e.to_string())?;
    }

    // Step 3: Parse temp buffer and inject metadata chunks after IHDR
    let output_file = File::create(output_path).map_err(|e| e.to_string())?;
    let mut output_writer = BufWriter::new(output_file);

    // Write PNG signature
    output_writer
        .write_all(&temp_buffer[0..8])
        .map_err(|e| e.to_string())?;

    let mut pos = 8;
    // Read and write IHDR
    let ihdr_length = u32::from_be_bytes([
        temp_buffer[pos],
        temp_buffer[pos + 1],
        temp_buffer[pos + 2],
        temp_buffer[pos + 3],
    ]) as usize;
    let ihdr_end = pos + 4 + 4 + ihdr_length + 4; // length + type + data + crc
    output_writer
        .write_all(&temp_buffer[pos..ihdr_end])
        .map_err(|e| e.to_string())?;
    pos = ihdr_end;

    // Write metadata chunks after IHDR
    for chunk in metadata_chunks {
        chunk.write(&mut output_writer).map_err(|e| e.to_string())?;
    }

    // Write remaining chunks (IDAT and IEND)
    output_writer
        .write_all(&temp_buffer[pos..])
        .map_err(|e| e.to_string())?;

    Ok(())
}

pub fn encode_from_file(
    input_image: &str,
    message: &str,
    output_image: &str,
) -> Result<(), String> {
    let output_lower = output_image.to_lowercase();
    if output_lower.ends_with(".jpg") || output_lower.ends_with(".jpeg") {
        return Err("JPEG format is not supported for output. JPEG's lossy compression will destroy the hidden data. Please use PNG format instead.".to_string());
    }

    let mut img = image::open(input_image)
        .map_err(|e| e.to_string())?
        .to_rgba8();

    encode(&mut img, message)?;

    save_rgba_with_metadata(&img, output_image, input_image)?;
    Ok(())
}

pub fn decode_from_file(image_path: &str) -> Result<String, String> {
    let img = image::open(image_path)
        .map_err(|e| e.to_string())?
        .to_rgba8();
    decode(&img)
}

fn encode(img: &mut RgbaImage, message: &str) -> Result<(), String> {
    let (width, height) = img.dimensions();
    let message_bytes = message.as_bytes();
    let message_len = message_bytes.len() as u32;
    let message_len_bytes = message_len.to_be_bytes();

    let total_capacity = (width * height * 4) / 8;
    if (message_len + 4) as u32 > total_capacity {
        return Err(format!(
            "Message is too long for the image. Capacity: {} bytes",
            total_capacity
        ));
    }

    let mut data_to_hide = Vec::new();
    data_to_hide.extend_from_slice(&message_len_bytes);
    data_to_hide.extend_from_slice(message_bytes);

    let mut bit_index = 0usize;

    for &byte in &data_to_hide {
        for bit_pos in 0..8 {
            let bit = (byte >> bit_pos) & 1;

            let pixel_index = bit_index / 4;
            let channel_index = bit_index % 4;

            let x = (pixel_index % width as usize) as u32;
            let y = (pixel_index / width as usize) as u32;

            let pixel = img.get_pixel_mut(x, y);
            pixel[channel_index] = (pixel[channel_index] & 0xFE) | bit;

            bit_index += 1;
        }
    }

    Ok(())
}

fn decode(img: &RgbaImage) -> Result<String, String> {
    let (width, height) = img.dimensions();

    let mut bits = Vec::new();
    let mut byte_count = 0;
    let mut finished = false;

    for y in 0..height {
        for x in 0..width {
            let pixel = img.get_pixel(x, y);
            for i in 0..4 {
                bits.push(pixel[i] & 1);
                if !finished && bits.len() >= 32 {
                    let mut len_bytes = [0u8; 4];
                    for i in 0..4 {
                        let mut byte = 0u8;
                        for j in 0..8 {
                            byte |= bits[i * 8 + j] << j;
                        }
                        len_bytes[i] = byte;
                    }
                    byte_count = u32::from_be_bytes(len_bytes);
                    finished = true;
                }

                if finished && bits.len() >= 32 + (byte_count as usize * 8) {
                    let mut message_bytes = Vec::new();
                    let message_bits = &bits[32..];
                    for chunk in message_bits.chunks(8) {
                        let mut byte = 0u8;
                        for (i, &bit) in chunk.iter().enumerate() {
                            byte |= bit << i;
                        }
                        message_bytes.push(byte);
                        if message_bytes.len() >= byte_count as usize {
                            break;
                        }
                    }
                    return String::from_utf8(message_bytes).map_err(|e| e.to_string());
                }
            }
        }
    }

    Err("Could not decode message from image.".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::RgbaImage;

    #[test]
    fn test_encode_decode() {
        let mut img = RgbaImage::new(100, 100);
        let message = "This is a secret message.";

        encode(&mut img, message).unwrap();
        let decoded_message = decode(&img).unwrap();

        assert_eq!(message, decoded_message);
    }

    #[test]
    fn test_message_too_long() {
        let mut img = RgbaImage::new(1, 1);
        let message = "This message is definitely too long.";
        let result = encode(&mut img, message);
        assert!(result.is_err());
    }

    #[test]
    fn test_empty_message() {
        let mut img = RgbaImage::new(100, 100);
        let message = "";
        encode(&mut img, message).unwrap();
        let decoded_message = decode(&img).unwrap();
        assert_eq!(message, decoded_message);
    }
}
