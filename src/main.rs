use clap::{Parser, Subcommand};

mod crypto;
mod img;
use img::codec::{decode_from_files, encode_from_file, encode_from_files};
use img::io::collect_images_from_dir;

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

    let result = match cli.command {
        Commands::Encode {
            image,
            image_list,
            image_dir,
            message,
            output,
            output_dir,
            auto_resize,
        } => encode(
            image,
            image_list,
            image_dir,
            message,
            output,
            output_dir,
            auto_resize,
        ),
        Commands::Decode {
            image,
            image_list,
            image_dir,
            output,
        } => decode(image, image_list, image_dir, output),
    };

    match result {
        Ok(m) => {
            println!("OK: {}", m);
        }
        Err(m) => {
            eprintln!("{}", m);
            std::process::exit(1);
        }
    }
}

fn encode(
    image: Option<String>,
    image_list: Option<Vec<String>>,
    image_dir: Option<String>,
    message: String,
    output: Option<String>,
    output_dir: Option<String>,
    auto_resize: bool,
) -> Result<String, String> {
    let image_param_count = [image.is_some(), image_list.is_some(), image_dir.is_some()]
        .iter()
        .filter(|&&x| x)
        .count();

    if image_param_count == 0 {
        return Err("Must specify one of --image, --image-list, or --image-dir".into());
    }

    if image_param_count > 1 {
        return Err("Only one of --image, --image-list, or --image-dir can be specified".into());
    }

    if image.is_some() {
        if output.is_none() {
            return Err("--output is required when using --image".into());
        }
        if output_dir.is_some() {
            return Err("--output-dir cannot be used with --image (use --output instead)".into());
        }
    } else {
        if output_dir.is_none() {
            return Err("--output-dir is required when using --image-list or --image-dir".into());
        }
        if output.is_some() {
            return Err("--output cannot be used with --image-list or --image-dir (use --output-dir instead)".into());
        }

        if auto_resize {
            return Err("--auto-resize is not supported with multiple images yet".to_string());
        }
    }

    let result = if let Some(single_image) = &image {
        encode_from_file(
            single_image,
            &message,
            output.as_ref().unwrap(),
            auto_resize,
        )
    } else if let Some(images) = &image_list {
        encode_from_files(images, &message, output_dir.as_ref().unwrap())
    } else if let Some(dir) = &image_dir {
        match collect_images_from_dir(dir) {
            Ok(images) => encode_from_files(&images, &message, output_dir.as_ref().unwrap()),
            Err(e) => Err(e),
        }
    } else {
        unreachable!()
    };

    match result {
        Ok(_) => {
            if let Some(out) = &output {
                Ok(format!("Encoded message into {}", out))
            } else if let Some(out_dir) = &output_dir {
                Ok(format!("Encoded message into output directory {}", out_dir))
            } else {
                unreachable!()
            }
        }
        Err(e) => Err(format!("Failed to encode message: {}", e)),
    }
}

fn decode(
    image: Option<String>,
    image_list: Option<Vec<String>>,
    image_dir: Option<String>,
    output: String,
) -> Result<String, String> {
    let image_param_count = [image.is_some(), image_list.is_some(), image_dir.is_some()]
        .iter()
        .filter(|&&x| x)
        .count();

    if image_param_count == 0 {
        return Err("Must specify one of --image, --image-list, or --image-dir".into());
    }

    if image_param_count > 1 {
        return Err("Only one of --image, --image-list, or --image-dir can be specified".into());
    }

    let images = if let Some(single_image) = image {
        vec![single_image]
    } else if let Some(images) = image_list {
        images
    } else if let Some(dir) = image_dir {
        collect_images_from_dir(&dir)
            .map_err(|e| format!("Failed to read image directory: {}", e))?
    } else {
        unreachable!()
    };

    decode_from_files(&images, &output)?;
    Ok(format!("Successfully decoded message to {}", output))
}
