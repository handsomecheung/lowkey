# lowkey

A secure command-line LSB (Least Significant Bit) steganography tool for hiding encrypted messages in PNG images.

## Features

- **Strong Encryption**: All messages are encrypted using ChaCha20-Poly1305 AEAD before embedding
- **Customizable Keys**: Use any password of any length (SHA256-hashed to derive 32-byte keys). Keys can include emoji and international characters (e.g., "我的密钥🔐")
- **Multi-Image Support**: Automatically split large messages across multiple images
- **Auto-Resize**: Automatically resize images to accommodate message size
- **RGBA Encoding**: Utilizes all four color channels (including alpha) for maximum capacity

## Installation

### Prerequisites

- Rust 1.70 or later
- Cargo

### Build from Source

```bash
git clone https://github.com/handsomecheung/lowkey.git
cd lowkey
cargo build --release
```

The compiled binary will be available at `target/release/lowkey`.

## Usage

### Basic Operations

#### Encode a message (with default key)

```bash
lowkey encode --image input.jpg --message message.txt --output output.png
```

#### Decode a message (with default key)

```bash
lowkey decode --image output.png --output recovered.txt
```

### Custom Encryption Keys

#### Encode with custom key

```bash
lowkey encode --image input.jpg --message secret.txt --output output.png --key "my-secret-password"
```

#### Decode with custom key

```bash
lowkey decode --image output.png --output recovered.txt --key "my-secret-password"
```

**Note**: The same key must be used for both encoding and decoding.

### Multi-Image Operations

#### Encode across multiple images

```bash
# Using image list
lowkey encode --image-list img1.jpg img2.png img3.jpg --message large.txt --output-dir ./encoded

# Using directory
lowkey encode --image-dir ./images --message secret.txt --output-dir ./encoded --key "password"
```

#### Decode from multiple images

```bash
# Images are automatically ordered using sequence metadata
lowkey decode --image-list encoded/img1.png encoded/img2.png encoded/img3.png --output recovered.txt

# Using directory
lowkey decode --image-dir ./encoded --output recovered.txt --key "password"
```

### Auto-Resize

Automatically resize images when the message is too large:

```bash
lowkey encode --image small.jpg --message big.txt --output output.png --auto-resize
```

### Unicode Keys

Full Unicode support for encryption keys:

```bash
lowkey encode --image input.jpg --message msg.txt --output output.png --key "MyKey🔐"
lowkey decode --image output.png --output msg.txt --key "MyKey🔐"
```

## How It Works

### LSB Steganography

lowkey uses Least Significant Bit (LSB) steganography to hide data in images:

1. Each pixel in a PNG image has 4 color channels: Red, Green, Blue, and Alpha (transparency)
2. Each channel is stored as an 8-bit value (0-255)
3. The tool modifies the least significant bit of each channel to store hidden data
4. This creates imperceptible changes to the image while embedding information

**Storage Capacity**: `(width × height × 4) / 8` bytes per image

### Encryption

Before embedding, all messages are encrypted using:

- **Algorithm**: ChaCha20-Poly1305 AEAD (Authenticated Encryption with Associated Data)
- **Key Derivation**: SHA256 hashing of user-provided password (any length → 32 bytes)
- **Nonce**: 12 bytes, randomly generated per encryption
- **Authentication**: 16-byte Poly1305 MAC tag for integrity verification

### Message Format

Each encoded message contains:

1. **Version byte** (1 byte): Protocol version for future compatibility
2. **Length field** (4 bytes): Size of encrypted data
3. **Encrypted payload**: `[12-byte nonce][ciphertext][16-byte auth tag]`

### Multi-Image Sequence

When using multiple images, lowkey adds custom PNG metadata (lKsq chunk) to track:
- Current image index
- Total image count

This allows automatic ordering during decoding, regardless of input file order.

## Technical Details

### Why PNG Output Only?

lowkey **requires PNG output** because:
- **Lossless compression**: PNG preserves every bit exactly as written
- **RGBA support**: Full access to all four color channels including alpha
- **JPEG is lossy**: Would destroy LSB-encoded data during compression
- **Metadata support**: PNG allows custom chunks for sequence information

Input images can be any format (JPEG, PNG, BMP, etc.), but they are converted to PNG for output.

## Testing

### Run unit tests

```bash
cargo test
```

### Run integration tests

```bash
./test.sh
```

## Security Considerations

- The security of your hidden message depends on the strength of your encryption key
- Use long, random passwords for maximum security
- The default key is publicly known - always use `--key` for sensitive data
- LSB steganography can be detected by statistical analysis if someone suspects hidden data
- This tool is designed for legitimate privacy uses, not for evading lawful surveillance

## License

Apache-2.0 license

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## Acknowledgments

Built with:
- [image-rs](https://github.com/image-rs/image) - Image processing
- [png](https://github.com/image-rs/image-png) - PNG encoding/decoding
- [RustCrypto](https://github.com/RustCrypto) - ChaCha20-Poly1305 and SHA2 implementations
- [clap](https://github.com/clap-rs/clap) - Command-line argument parsing
- [bitvec](https://github.com/bitvecto-rs/bitvec) - Bit manipulation and vector operations
