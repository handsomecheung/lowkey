use image::ImageBuffer;
use image::RgbaImage;
use png::{BitDepth, ColorType, Encoder};
use std::fs::File;
use std::fs::{self};
use std::io::{BufReader, BufWriter};
use std::path::Path;

pub fn read_image(path: &str) -> Result<ImageBuffer<image::Rgba<u8>, Vec<u8>>, String> {
    // Read any image format - JPEG inputs are fine, they get converted to RGBA
    let img = image::open(path)
        .map_err(|e| format!("Failed to open image '{}': {}", path, e))?
        .to_rgba8();

    Ok(img)
}

pub fn collect_images_from_dir(dir: &str) -> Result<Vec<String>, String> {
    let path = Path::new(dir);
    if !path.is_dir() {
        return Err(format!("'{}' is not a directory", dir));
    }

    let entries =
        fs::read_dir(path).map_err(|e| format!("Failed to read directory '{}': {}", dir, e))?;

    let mut image_files = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|e| format!("Failed to read directory entry: {}", e))?;
        let path = entry.path();

        if path.is_file() {
            if let Some(ext) = path.extension() {
                let ext_str = ext.to_string_lossy().to_lowercase();
                if ext_str == "png" || ext_str == "jpg" || ext_str == "jpeg" {
                    image_files.push(path.to_string_lossy().to_string());
                }
            }
        }
    }

    if image_files.is_empty() {
        return Err(format!("No image files found in directory '{}'", dir));
    }

    image_files.sort();
    Ok(image_files)
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
pub fn save_rgba_with_metadata(
    img: &RgbaImage,
    output_path: &str,
    input_path: &str,
) -> Result<(), String> {
    use std::io::{Read, Write};

    // Check if input file is PNG by reading signature
    let input_file = File::open(input_path).map_err(|e| e.to_string())?;
    let mut input_reader = BufReader::new(input_file);

    let mut signature = [0u8; 8];
    let png_signature: [u8; 8] = [137, 80, 78, 71, 13, 10, 26, 10];

    // Try to read signature
    if input_reader.read_exact(&mut signature).is_err() || signature != png_signature {
        // Not a PNG file, just save without metadata preservation
        drop(input_reader);
        return save_rgba_simple(img, output_path);
    }

    // Step 1: Extract metadata chunks from original PNG file
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

/// Save RGBA image as PNG without metadata preservation.
fn save_rgba_simple(img: &RgbaImage, output_path: &str) -> Result<(), String> {
    let output_file = File::create(output_path).map_err(|e| e.to_string())?;
    let output_writer = BufWriter::new(output_file);
    let (width, height) = img.dimensions();
    let mut encoder = Encoder::new(output_writer, width, height);
    encoder.set_color(ColorType::Rgba);
    encoder.set_depth(BitDepth::Eight);

    let mut writer = encoder.write_header().map_err(|e| e.to_string())?;
    writer
        .write_image_data(img.as_raw())
        .map_err(|e| e.to_string())?;

    Ok(())
}
