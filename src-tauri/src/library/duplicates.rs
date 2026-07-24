use crate::error::AppResult;
use crate::library::hashing::{parse_phash, phash_distance};
use crate::models::{DuplicateGroup, Scene};
use std::collections::HashMap;

pub fn cluster_phash_ids(entries: &[(String, String)], threshold: u8) -> Vec<Vec<String>> {
    let n = entries.len();
    if n < 2 {
        return Vec::new();
    }

    let mut parent: Vec<usize> = (0..n).collect();

    fn find(parent: &mut [usize], i: usize) -> usize {
        let mut root = i;
        while parent[root] != root {
            root = parent[root];
        }
        let mut node = i;
        while parent[node] != node {
            let next = parent[node];
            parent[node] = root;
            node = next;
        }
        root
    }

    fn union(parent: &mut [usize], a: usize, b: usize) {
        let ra = find(parent, a);
        let rb = find(parent, b);
        if ra != rb {
            parent[rb] = ra;
        }
    }

    for (i, (_, hash_i)) in entries.iter().enumerate() {
        let Ok(hi) = parse_phash(hash_i) else {
            continue;
        };
        for (j, (_, hash_j)) in entries.iter().enumerate().skip(i + 1) {
            let Ok(hj) = parse_phash(hash_j) else {
                continue;
            };
            if hi.dist(&hj) <= threshold as u32 {
                union(&mut parent, i, j);
            }
        }
    }

    let mut buckets: HashMap<usize, Vec<String>> = HashMap::new();
    for (idx, (id, _)) in entries.iter().enumerate() {
        let root = find(&mut parent, idx);
        buckets.entry(root).or_default().push(id.clone());
    }

    buckets.into_values().filter(|ids| ids.len() > 1).collect()
}

pub fn max_phash_distance_in_group(entries: &[(String, String)], ids: &[String]) -> AppResult<u8> {
    let lookup = |id: &str| {
        entries
            .iter()
            .find(|(sid, _)| sid == id)
            .map(|(_, hash)| hash.as_str())
    };

    let mut max = 0u32;
    for (i, left_id) in ids.iter().enumerate() {
        let Some(left) = lookup(left_id) else {
            continue;
        };
        for right_id in ids.iter().skip(i + 1) {
            let Some(right) = lookup(right_id) else {
                continue;
            };
            max = max.max(phash_distance(left, right)?);
        }
    }
    Ok(max.min(255) as u8)
}

pub fn build_phash_group(
    entries: &[(String, String)],
    ids: &[String],
    scenes: Vec<Scene>,
) -> AppResult<DuplicateGroup> {
    let representative = entries
        .iter()
        .find(|(id, _)| ids.first() == Some(id))
        .map(|(_, phash)| phash.clone())
        .unwrap_or_else(|| "unknown".to_string());
    let max_distance = max_phash_distance_in_group(entries, ids).ok();
    Ok(DuplicateGroup {
        match_type: "phash".to_string(),
        hash: representative,
        scenes,
        max_distance,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{ImageBuffer, Rgb};
    use std::path::Path;

    fn write_test_png(path: &Path, shade: u8) {
        let img: ImageBuffer<Rgb<u8>, Vec<u8>> =
            ImageBuffer::from_fn(16, 16, |_, _| Rgb([shade, shade, shade]));
        img.save(path).unwrap();
    }

    fn sample_phash(shade: u8) -> String {
        let dir = std::env::temp_dir().join("scrawler-dup-test");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join(format!("sample-{shade}.png"));
        write_test_png(&path, shade);
        crate::library::hashing::compute_phash_from_image(&path).unwrap()
    }

    #[test]
    fn identical_phash_clusters() {
        let phash = sample_phash(100);
        let entries = vec![("a".to_string(), phash.clone()), ("b".to_string(), phash)];
        let clusters = cluster_phash_ids(&entries, 0);
        assert_eq!(clusters.len(), 1);
        assert_eq!(clusters[0].len(), 2);
    }
}
