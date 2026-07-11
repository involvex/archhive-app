use crate::error::AppResult;
use image::ImageReader;
use image_hasher::{HashAlg, HasherConfig};
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

pub fn compute_phash_from_image(path: &Path) -> AppResult<String> {
    let image = ImageReader::open(path)?.decode()?;
    let hasher = HasherConfig::new().hash_alg(HashAlg::DoubleGradient).to_hasher();
    let hash = hasher.hash_image(&image);
    Ok(hash.to_base64())
}

fn read_chunk(file: &mut File, buf: &mut [u8]) -> AppResult<usize> {
    Ok(file.read(buf)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn oshash_is_stable() {
        let mut tmp = NamedTempFile::new().unwrap();
        tmp.write_all(b"hello scrawler").unwrap();
        let h1 = compute_oshash(tmp.path()).unwrap();
        let h2 = compute_oshash(tmp.path()).unwrap();
        assert_eq!(h1, h2);
    }
}
