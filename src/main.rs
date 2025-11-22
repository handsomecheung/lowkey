use std::env;
use image::RgbaImage;

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

pub fn encode_from_file(input_image: &str, message: &str, output_image: &str) -> Result<(), String> {
    let output_lower = output_image.to_lowercase();
    if output_lower.ends_with(".jpg") || output_lower.ends_with(".jpeg") {
        return Err("JPEG format is not supported for output. JPEG's lossy compression will destroy the hidden data. Please use PNG format instead.".to_string());
    }

    let mut img = image::open(input_image).map_err(|e| e.to_string())?.to_rgba8();
    encode(&mut img, message)?;
    img.save(output_image).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn decode_from_file(image_path: &str) -> Result<String, String> {
    let img = image::open(image_path).map_err(|e| e.to_string())?.to_rgba8();
    decode(&img)
}

fn encode(img: &mut RgbaImage, message: &str) -> Result<(), String> {
    let (width, height) = img.dimensions();
    let message_bytes = message.as_bytes();
    let message_len = message_bytes.len() as u32;
    let message_len_bytes = message_len.to_be_bytes();

    let total_capacity = (width * height * 4) / 8;
    if (message_len + 4) as u32 > total_capacity {
        return Err(format!("Message is too long for the image. Capacity: {} bytes", total_capacity));
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
                           byte |= bits[i*8 + j] << j;
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
