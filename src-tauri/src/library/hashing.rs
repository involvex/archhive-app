use crate::error::AppResult;
use image::ImageReader;
use image_hasher::{HashAlg, HasherConfig, ImageHash};
use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

const OSHASH_CHUNK: u64 = 64 * 1024;

pub fn compute_oshash(path: &Path) -> AppResult<String> {
    let mut file = File::open(path)?;
    let size = file.metadata()?.len();
    let mut hasher = Sha256::new();

    let mut buf = vec![0u8; OSHASH_CHUNK as usize];
    let read_head = read_chunk(&mut file, &mut buf)?;
    hasher.update(&buf[..read_head]);

    if size > OSHASH_CHUNK {
        let tail_start = size.saturating_sub(OSHASH_CHUNK);
        file.seek(SeekFrom::Start(tail_start))?;
        let read_tail = read_chunk(&mut file, &mut buf)?;
        hasher.update(&buf[..read_tail]);
    }

    hasher.update(size.to_le_bytes());
    Ok(format!("{:x}", hasher.finalize()))
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

fn read_chunk(file: &mut File, buf: &mut [u8]) -> AppResult<usize> {
    Ok(file.read(buf)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{ImageBuffer, Rgb};

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
    fn oshash_is_stable() {
        let path = std::env::temp_dir().join("scrawler-oshash-test.bin");
        std::fs::write(&path, b"hello scrawler").unwrap();
        let h1 = compute_oshash(&path).unwrap();
        let h2 = compute_oshash(&path).unwrap();
        assert_eq!(h1, h2);
    }
}
