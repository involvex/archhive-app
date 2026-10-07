use crate::error::AppResult;
use image::ImageReader;
use image_hasher::{HashAlg, HasherConfig, ImageHash};
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

/// Chunk size used by the OpenSubtitles hash (first + last 64 KiB).
const OSHASH_CHUNK: u64 = 64 * 1024;

/// True OpenSubtitlesHash: `filesize + sum(u64-LE words of first 64 KiB)
/// + sum(u64-LE words of last 64 KiB)` (head/tail may overlap on small
/// files), rendered as 16 lowercase hex chars.
///
/// This is the value StashDB stores as `OSHASH`. The previous implementation
/// wrote a SHA-256 digest here instead, which could never match anything —
/// see MIGRATION_019, which clears those bogus 64-char rows.
pub fn compute_oshash(path: &Path) -> AppResult<String> {
    let mut file = File::open(path)?;
    let size = file.metadata()?.len();
    let mut buf = vec![0u8; OSHASH_CHUNK as usize];

    let mut hash = size;
    let head_n = read_at(&mut file, 0, &mut buf)?;
    hash = hash.wrapping_add(sum_words(&buf[..head_n]));

    if size > 0 {
        let tail_start = size.saturating_sub(OSHASH_CHUNK);
        let tail_n = read_at(&mut file, tail_start, &mut buf)?;
        hash = hash.wrapping_add(sum_words(&buf[..tail_n]));
    }

    Ok(format!("{hash:016x}"))
}

fn sum_words(bytes: &[u8]) -> u64 {
    let mut acc = 0u64;
    for chunk in bytes.chunks(8) {
        let mut word = [0u8; 8];
        word[..chunk.len()].copy_from_slice(chunk);
        acc = acc.wrapping_add(u64::from_le_bytes(word));
    }
    acc
}

fn read_at(file: &mut File, offset: u64, buf: &mut [u8]) -> AppResult<usize> {
    file.seek(SeekFrom::Start(offset))?;
    let mut total = 0;
    while total < buf.len() {
        match file.read(&mut buf[total..])? {
            0 => break,
            n => total += n,
        }
    }
    Ok(total)
}

/// Full-file MD5 hex digest — the most common StashDB fingerprint type.
/// Streamed in 128 KiB chunks so multi-GB files don't blow up memory.
pub fn compute_md5(path: &Path) -> AppResult<String> {
    let mut file = File::open(path)?;
    let mut ctx = md5::Context::new();
    let mut buf = [0u8; 128 * 1024];
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        ctx.consume(&buf[..n]);
    }
    Ok(format!("{:x}", ctx.compute()))
}

pub fn parse_phash(encoded: &str) -> AppResult<ImageHash> {
    ImageHash::from_base64(encoded)
        .map_err(|e| crate::error::AppError::Other(format!("phash decode: {e:?}")))
}

pub fn phash_distance(a: &str, b: &str) -> AppResult<u32> {
    Ok(parse_phash(a)?.dist(&parse_phash(b)?))
}

pub fn compute_phash_from_image(path: &Path) -> AppResult<String> {
    let image = ImageReader::open(path)?.decode()?;
    let hasher = HasherConfig::new()
        .hash_alg(HashAlg::DoubleGradient)
        .to_hasher();
    let hash = hasher.hash_image(&image);
    Ok(hash.to_base64())
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{ImageBuffer, Rgb};
    use std::io::Write;

    fn write_test_png(path: &Path, shade: u8) {
        let img: ImageBuffer<Rgb<u8>, Vec<u8>> =
            ImageBuffer::from_fn(16, 16, |_, _| Rgb([shade, shade, shade]));
        img.save(path).unwrap();
    }

    #[test]
    fn phash_distance_is_zero_for_identical() {
        let dir = std::env::temp_dir().join("scrawler-phash-test");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("a.png");
        write_test_png(&path, 42);
        let h = compute_phash_from_image(&path).unwrap();
        assert_eq!(phash_distance(&h, &h).unwrap(), 0);
    }

    #[test]
    fn oshash_is_16_hex_chars_and_stable() {
        let path = std::env::temp_dir().join("scrawler-oshash-test.bin");
        std::fs::write(&path, b"hello scrawler").unwrap();
        let h1 = compute_oshash(&path).unwrap();
        let h2 = compute_oshash(&path).unwrap();
        assert_eq!(h1, h2);
        assert_eq!(h1.len(), 16);
        assert!(h1.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn oshash_matches_reference_vector() {
        // 8 zero bytes: head word 0 + tail word 0 + size 8.
        let path = std::env::temp_dir().join("scrawler-oshash-zero.bin");
        std::fs::write(&path, [0u8; 8]).unwrap();
        assert_eq!(compute_oshash(&path).unwrap(), "0000000000000008");

        // "abcdefgh" (8 bytes, one LE word 0x6867666564636261) counted twice
        // (head == tail on small files) plus size 8.
        let path = std::env::temp_dir().join("scrawler-oshash-word.bin");
        {
            let mut f = File::create(&path).unwrap();
            f.write_all(b"abcdefgh").unwrap();
        }
        let word = u64::from_le_bytes(*b"abcdefgh");
        let expected = 8u64.wrapping_add(word).wrapping_add(word);
        assert_eq!(compute_oshash(&path).unwrap(), format!("{expected:016x}"));
    }

    #[test]
    fn md5_matches_known_vector() {
        let path = std::env::temp_dir().join("scrawler-md5-test.bin");
        std::fs::write(&path, b"abc").unwrap();
        assert_eq!(
            compute_md5(&path).unwrap(),
            "900150983cd24fb0d6963f7d28e17f72"
        );
    }
}
