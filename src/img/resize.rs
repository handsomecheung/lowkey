use image::RgbaImage;

pub fn resize_image(
    img: &mut RgbaImage,
    message_bytes_len: usize,
    min_size: u32,
) -> Result<RgbaImage, String> {
    let (original_width, original_height) = img.dimensions();
    let (new_width, new_height) =
        calculate_optimal_dimensions(message_bytes_len, original_width, original_height, min_size);

    if new_width < original_width || new_height < original_height {
        println!(
            "Resizing image from {}x{} to {}x{} to optimize for message size",
            original_width, original_height, new_width, new_height
        );
        *img = image::imageops::resize(
            img,
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

    Ok(img.clone())
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
