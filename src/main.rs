use clap::{Parser, Subcommand};
use image::RgbaImage;
use png::{BitDepth, ColorType, Encoder};
use std::fs::{self, File};
use std::io::{BufReader, BufWriter};
use std::path::Path;

#[derive(Parser)]
#[command(name = "lowkey")]
#[command(about = "LSB steganography tool for hiding messages in PNG images", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Encode {
        #[arg(long, default_value = "default")]
        variant: String,

        /// Single input image (mutually exclusive with --image-list and --image-dir)
        #[arg(long)]
        image: Option<String>,

        /// Multiple input images (space-separated)
        #[arg(long, num_args = 1..)]
        image_list: Option<Vec<String>>,

        /// Directory containing input images
        #[arg(long)]
        image_dir: Option<String>,

        #[arg(long)]
        message: String,

        /// Single output image (used with --image)
        #[arg(long)]
        output: Option<String>,

        /// Output directory (used with --image-list or --image-dir)
        #[arg(long)]
        output_dir: Option<String>,

        #[arg(long, default_value = "false")]
        auto_resize: bool,
    },
    Decode {
        #[arg(long, default_value = "default")]
        variant: String,

        /// Single input image (mutually exclusive with --image-list and --image-dir)
        #[arg(long)]
        image: Option<String>,

        /// Multiple input images (space-separated)
        #[arg(long, num_args = 1..)]
        image_list: Option<Vec<String>>,

        /// Directory containing input images
        #[arg(long)]
        image_dir: Option<String>,

        #[arg(long)]
        output: String,
    },
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Encode {
            variant,
            image,
            image_list,
            image_dir,
            message,
            output,
            output_dir,
            auto_resize,
        } => {
            // Validate mutually exclusive image parameters
            let image_param_count = [image.is_some(), image_list.is_some(), image_dir.is_some()]
                .iter()
                .filter(|&&x| x)
                .count();

            if image_param_count == 0 {
                eprintln!("Error: Must specify one of --image, --image-list, or --image-dir");
                std::process::exit(1);
            }

            if image_param_count > 1 {
                eprintln!("Error: Only one of --image, --image-list, or --image-dir can be specified");
                std::process::exit(1);
            }

            // Validate output parameters
            if image.is_some() {
                if output.is_none() {
                    eprintln!("Error: --output is required when using --image");
                    std::process::exit(1);
                }
                if output_dir.is_some() {
                    eprintln!("Error: --output-dir cannot be used with --image (use --output instead)");
                    std::process::exit(1);
                }
            } else {
                if output_dir.is_none() {
                    eprintln!("Error: --output-dir is required when using --image-list or --image-dir");
                    std::process::exit(1);
                }
                if output.is_some() {
                    eprintln!("Error: --output cannot be used with --image-list or --image-dir (use --output-dir instead)");
                    std::process::exit(1);
                }
            }

            // Execute encoding based on parameters
            let result = if let Some(single_image) = &image {
                encode_from_files(&variant, single_image, &message, output.as_ref().unwrap(), auto_resize)
            } else if let Some(images) = &image_list {
                encode_from_multiple_files(&variant, images, &message, output_dir.as_ref().unwrap(), auto_resize)
            } else if let Some(dir) = &image_dir {
                let images = match collect_images_from_dir(dir) {
                    Ok(imgs) => imgs,
                    Err(e) => {
                        eprintln!("Error reading image directory: {}", e);
                        std::process::exit(1);
                    }
                };
                encode_from_multiple_files(&variant, &images, &message, output_dir.as_ref().unwrap(), auto_resize)
            } else {
                unreachable!()
            };

            match result {
                Ok(_) => {
                    if let Some(out) = &output {
                        println!("Encoded message into {}", out);
                    } else if let Some(out_dir) = &output_dir {
                        println!("Encoded message into output directory {}", out_dir);
                    }
                }
                Err(e) => eprintln!("Error encoding message: {}", e),
            }
        }
        Commands::Decode {
            variant,
            image,
            image_list,
            image_dir,
            output,
        } => {
            // Validate mutually exclusive image parameters
            let image_param_count = [image.is_some(), image_list.is_some(), image_dir.is_some()]
                .iter()
                .filter(|&&x| x)
                .count();

            if image_param_count == 0 {
                eprintln!("Error: Must specify one of --image, --image-list, or --image-dir");
                std::process::exit(1);
            }

            if image_param_count > 1 {
                eprintln!("Error: Only one of --image, --image-list, or --image-dir can be specified");
                std::process::exit(1);
            }

            // Execute decoding based on parameters
            let result = if let Some(single_image) = &image {
                decode_to_file(&variant, single_image, &output)
            } else if let Some(images) = &image_list {
                decode_from_multiple_files(&variant, images, &output)
            } else if let Some(dir) = &image_dir {
                let images = match collect_images_from_dir(dir) {
                    Ok(imgs) => imgs,
                    Err(e) => {
                        eprintln!("Error reading image directory: {}", e);
                        std::process::exit(1);
                    }
                };
                decode_from_multiple_files(&variant, &images, &output)
            } else {
                unreachable!()
            };

            match result {
                Ok(_) => println!("Decoded message saved to {}", output),
                Err(e) => eprintln!("Error decoding message: {}", e),
            }
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

pub fn encode_from_files(
    variant: &str,
    input_image: &str,
    message_file: &str,
    output_image: &str,
    auto_resize: bool,
) -> Result<(), String> {
    use std::io::Read;

    if variant != "default" {
        return Err("invalid variant".to_string());
    }

    let output_lower = output_image.to_lowercase();
    if output_lower.ends_with(".jpg") || output_lower.ends_with(".jpeg") {
        return Err("JPEG format is not supported for output. JPEG's lossy compression will destroy the hidden data. Please use PNG format instead.".to_string());
    }

    let mut message_file_handle = File::open(message_file)
        .map_err(|e| format!("Failed to open message file '{}': {}", message_file, e))?;
    let mut message_bytes = Vec::new();
    message_file_handle
        .read_to_end(&mut message_bytes)
        .map_err(|e| format!("Failed to read message file: {}", e))?;

    let mut img = image::open(input_image)
        .map_err(|e| e.to_string())?
        .to_rgba8();

    if auto_resize {
        let (original_width, original_height) = img.dimensions();
        let (new_width, new_height) =
            calculate_optimal_dimensions(message_bytes.len(), original_width, original_height, 600);

        if new_width < original_width || new_height < original_height {
            println!(
                "Resizing image from {}x{} to {}x{} to optimize for message size",
                original_width, original_height, new_width, new_height
            );
            img = image::imageops::resize(
                &img,
                new_width,
                new_height,
                image::imageops::FilterType::Lanczos3,
            );
        } else {
            println!(
                "Image size {}x{} is already optimal for message size",
                original_width, original_height
            );
        }
    }

    encode_bytes_v0(&mut img, &message_bytes)?;

    if let Some(parent) = Path::new(output_image).parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create output directory: {}", e))?;
    }

    save_rgba_with_metadata(&img, output_image, input_image)?;
    Ok(())
}

/// Encode message from file into multiple images.
pub fn encode_from_multiple_files(
    variant: &str,
    input_images: &[String],
    message_file: &str,
    output_dir: &str,
    auto_resize: bool,
) -> Result<(), String> {
    use std::io::Read;

    if variant != "default" {
        return Err("invalid variant".to_string());
    }

    if input_images.is_empty() {
        return Err("No input images provided".to_string());
    }

    // Read message
    let mut message_file_handle = File::open(message_file)
        .map_err(|e| format!("Failed to open message file '{}': {}", message_file, e))?;
    let mut message_bytes = Vec::new();
    message_file_handle
        .read_to_end(&mut message_bytes)
        .map_err(|e| format!("Failed to read message file: {}", e))?;

    // Load all images
    let mut images: Vec<(String, RgbaImage)> = Vec::new();
    for img_path in input_images {
        let img = image::open(img_path)
            .map_err(|e| format!("Failed to open image '{}': {}", img_path, e))?
            .to_rgba8();
        images.push((img_path.clone(), img));
    }

    if auto_resize {
        return Err("--auto-resize is not supported with multiple images".to_string());
    }

    // Calculate total capacity across all images
    let total_capacity: u32 = images
        .iter()
        .map(|(_, img)| {
            let (width, height) = img.dimensions();
            (width * height * 4) / 8
        })
        .sum();

    let message_len = message_bytes.len() as u32;
    if message_len + 4 > total_capacity {
        return Err(format!(
            "Message is too long for all images. Total capacity: {} bytes, message size: {} bytes",
            total_capacity - 4,
            message_len
        ));
    }

    // Prepare data to encode: length prefix + message
    let mut data_to_hide = Vec::new();
    data_to_hide.extend_from_slice(&message_len.to_be_bytes());
    data_to_hide.extend_from_slice(&message_bytes);

    // Encode across multiple images
    let mut bit_index = 0usize;
    let mut current_image_idx = 0usize;

    for &byte in &data_to_hide {
        for bit_pos in 0..8 {
            let bit = (byte >> bit_pos) & 1;

            // Find which image and pixel this bit belongs to
            while current_image_idx < images.len() {
                let (_, img) = &mut images[current_image_idx];
                let (width, height) = img.dimensions();
                let image_capacity_bits = (width * height * 4) as usize;

                if bit_index < image_capacity_bits {
                    let pixel_index = bit_index / 4;
                    let channel_index = bit_index % 4;

                    let x = (pixel_index % width as usize) as u32;
                    let y = (pixel_index / width as usize) as u32;

                    let pixel = img.get_pixel_mut(x, y);
                    pixel[channel_index] = (pixel[channel_index] & 0xFE) | bit;

                    bit_index += 1;
                    break;
                } else {
                    // Move to next image
                    current_image_idx += 1;
                    bit_index = 0;
                }
            }
        }
    }

    // Create output directory
    fs::create_dir_all(output_dir)
        .map_err(|e| format!("Failed to create output directory: {}", e))?;

    // Save all images
    for (i, (input_path, img)) in images.iter().enumerate() {
        let filename = Path::new(input_path)
            .file_name()
            .ok_or_else(|| format!("Invalid input path: {}", input_path))?;

        // Ensure output is PNG
        let mut output_filename = filename.to_string_lossy().to_string();
        if output_filename.to_lowercase().ends_with(".jpg") || output_filename.to_lowercase().ends_with(".jpeg") {
            // Replace extension with .png
            if let Some(pos) = output_filename.rfind('.') {
                output_filename = format!("{}.png", &output_filename[..pos]);
            }
        }

        let output_path = Path::new(output_dir).join(&output_filename);
        let output_path_str = output_path.to_string_lossy().to_string();

        save_rgba_with_metadata(img, &output_path_str, input_path)?;
        println!("Saved encoded image {}/{}: {}", i + 1, images.len(), output_path_str);
    }

    Ok(())
}

pub fn decode_to_file(variant: &str, image_path: &str, output_file: &str) -> Result<(), String> {
    use std::io::Write;

    if variant != "default" {
        return Err("invalid variant".to_string());
    }

    // TODO check if it is a PNG file?

    let img = image::open(image_path)
        .map_err(|e| e.to_string())?
        .to_rgba8();

    let message_bytes = decode_bytes_v0(&img)?;

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

/// Decode message from multiple images to file.
pub fn decode_from_multiple_files(
    variant: &str,
    image_paths: &[String],
    output_file: &str,
) -> Result<(), String> {
    use std::io::Write;

    if variant != "default" {
        return Err("invalid variant".to_string());
    }

    if image_paths.is_empty() {
        return Err("No input images provided".to_string());
    }

    // Load all images
    let mut images: Vec<RgbaImage> = Vec::new();
    for img_path in image_paths {
        let img = image::open(img_path)
            .map_err(|e| format!("Failed to open image '{}': {}", img_path, e))?
            .to_rgba8();
        images.push(img);
    }

    // Extract bits from all images
    let mut bits = Vec::new();
    for img in &images {
        let (width, height) = img.dimensions();
        for y in 0..height {
            for x in 0..width {
                let pixel = img.get_pixel(x, y);
                for i in 0..4 {
                    bits.push(pixel[i] & 1);
                }
            }
        }
    }

    // Decode message length from first 32 bits
    if bits.len() < 32 {
        return Err("Not enough data in images to decode message length".to_string());
    }

    let mut len_bytes = [0u8; 4];
    for i in 0..4 {
        let mut byte = 0u8;
        for j in 0..8 {
            byte |= bits[i * 8 + j] << j;
        }
        len_bytes[i] = byte;
    }
    let byte_count = u32::from_be_bytes(len_bytes) as usize;

    // Check if we have enough bits for the message
    let required_bits = 32 + (byte_count * 8);
    if bits.len() < required_bits {
        return Err(format!(
            "Not enough data in images. Required: {} bits, available: {} bits",
            required_bits,
            bits.len()
        ));
    }

    // Decode message bytes
    let mut message_bytes = Vec::new();
    let message_bits = &bits[32..];
    for chunk in message_bits.chunks(8) {
        let mut byte = 0u8;
        for (i, &bit) in chunk.iter().enumerate() {
            byte |= bit << i;
        }
        message_bytes.push(byte);
        if message_bytes.len() >= byte_count {
            break;
        }
    }

    // Create output directory if needed
    if let Some(parent) = Path::new(output_file).parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create output directory: {}", e))?;
    }

    // Write to file
    let mut file = File::create(output_file)
        .map_err(|e| format!("Failed to create output file '{}': {}", output_file, e))?;
    file.write_all(&message_bytes)
        .map_err(|e| format!("Failed to write to output file: {}", e))?;

    Ok(())
}

fn encode_bytes_v0(img: &mut RgbaImage, message_bytes: &[u8]) -> Result<(), String> {
    let (width, height) = img.dimensions();
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

fn decode_bytes_v0(img: &RgbaImage) -> Result<Vec<u8>, String> {
    let (width, height) = img.dimensions();

    let mut bits = Vec::new();
    let mut byte_count = 0;
    let mut finished = false;

    for y in 0..height {
        for x in 0..width {
            let pixel = img.get_pixel(x, y);
            for i in 0..4 {
                bits.push(pixel[i] & 1);
                // TODO ==
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

                // TODO ==
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
                    return Ok(message_bytes);
                }
            }
        }
    }

    Err("Could not decode message from image.".to_string())
}

/// Collect all image files from a directory, sorted by filename.
fn collect_images_from_dir(dir: &str) -> Result<Vec<String>, String> {
    let path = Path::new(dir);
    if !path.is_dir() {
        return Err(format!("'{}' is not a directory", dir));
    }

    let entries = fs::read_dir(path)
        .map_err(|e| format!("Failed to read directory '{}': {}", dir, e))?;

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

    // Sort by filename to ensure consistent ordering
    image_files.sort();
    Ok(image_files)
}

/// Calculate the optimal dimensions for an image to fit a message of given size.
///
/// # Arguments
/// * `message_bytes` - The size of the message in bytes (including 4-byte length prefix)
/// * `original_width` - Original image width
/// * `original_height` - Original image height
/// * `min_dimension` - Minimum dimension constraint (default 600)
///
/// # Returns
/// A tuple of (width, height) that:
/// - Can fit the message
/// - Maintains the original aspect ratio
/// - Is at least min_dimension x min_dimension (unless original is smaller)
/// - Is no larger than the original image
fn calculate_optimal_dimensions(
    message_bytes: usize,
    original_width: u32,
    original_height: u32,
    min_dimension: u32,
) -> (u32, u32) {
    let total_bytes = message_bytes + 4; // Include 4-byte length prefix
    let min_pixels_needed = ((total_bytes * 8) as f64 / 4.0).ceil() as u32;

    let aspect_ratio = original_width as f64 / original_height as f64;

    let mut new_height = (min_pixels_needed as f64 / aspect_ratio).sqrt().ceil() as u32;
    let mut new_width = (new_height as f64 * aspect_ratio).ceil() as u32;

    let effective_min = std::cmp::min(
        min_dimension,
        std::cmp::min(original_width, original_height),
    );

    if new_width < effective_min || new_height < effective_min {
        if aspect_ratio >= 1.0 {
            new_width = effective_min;
            new_height = (effective_min as f64 / aspect_ratio).ceil() as u32;
        } else {
            new_height = effective_min;
            new_width = (effective_min as f64 * aspect_ratio).ceil() as u32;
        }
    }

    new_width = std::cmp::min(new_width, original_width);
    new_height = std::cmp::min(new_height, original_height);

    let capacity = (new_width * new_height * 4) / 8;
    if capacity < total_bytes as u32 {
        return (original_width, original_height);
    }

    (new_width, new_height)
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::RgbaImage;

    #[test]
    fn test_encode_decode() {
        let mut img = RgbaImage::new(100, 100);
        let message = "This is a secret message.";

        encode_bytes_v0(&mut img, message.as_bytes()).unwrap();
        let decoded_bytes = decode_bytes_v0(&img).unwrap();
        let decoded_message = String::from_utf8(decoded_bytes).unwrap();

        assert_eq!(message, decoded_message);
    }

    #[test]
    fn test_message_too_long() {
        let mut img = RgbaImage::new(1, 1);
        let message = "This message is definitely too long.";
        let result = encode_bytes_v0(&mut img, message.as_bytes());
        assert!(result.is_err());
    }

    #[test]
    fn test_empty_message() {
        let mut img = RgbaImage::new(100, 100);
        let message = "";
        encode_bytes_v0(&mut img, message.as_bytes()).unwrap();
        let decoded_bytes = decode_bytes_v0(&img).unwrap();
        let decoded_message = String::from_utf8(decoded_bytes).unwrap();
        assert_eq!(message, decoded_message);
    }
}
