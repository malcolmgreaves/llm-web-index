use std::io::Cursor;

use crate::Error;

/// Compresses a string using Brotli algorithm.
pub fn compress_string(input: &str) -> Result<Vec<u8>, Error> {
    compress(input.as_bytes())
}

/// Compresses a byte slice using Brotli algorithm.
pub fn compress(input: &[u8]) -> Result<Vec<u8>, Error> {
    let mut input_cursor = Cursor::new(input);
    let mut compressed = Vec::new();

    // Parameters: input, output, {buffer_size, quality (0-11), lg_window_size}
    brotli::BrotliCompress(
        &mut input_cursor,
        &mut compressed,
        &brotli::enc::BrotliEncoderParams::default(),
    )?;

    Ok(compressed)
}

/// Decompress Brotli-compressed data as a string.
pub fn decompress_to_string(compressed: &[u8]) -> Result<String, Error> {
    let decompressed = decompress(compressed)?;
    let result = String::from_utf8(decompressed)?;
    Ok(result)
}

/// Decompress Brotli-compressed data.
pub fn decompress(compressed: &[u8]) -> Result<Vec<u8>, Error> {
    let mut input_cursor = Cursor::new(compressed);
    let mut decompressed = Vec::new();
    brotli::BrotliDecompress(&mut input_cursor, &mut decompressed)?;
    Ok(decompressed)
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_compress_roundtrip() {
        let input = "Hello world! How are you doing today?";
        let compressed = compress_string(&input).unwrap();
        let decompressed = decompress_to_string(&compressed).unwrap();
        assert_eq!(input, decompressed);
    }
}
