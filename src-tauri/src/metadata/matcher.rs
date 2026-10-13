//! Metadata matching for StashDB when fingerprints don't match — the usual
//! case for tube clips, which are re-encoded, cut, or carry a site intro.
//!
//! A few targeted text searches (cleaned title; performers + key title
//! words; studio / channel + key words) collect candidates, and each one is
//! scored against the library scene on title words, performers, studio and
//! duration.

use crate::models::{Scene, StashSceneMatch};
use std::collections::HashSet;

/// Words that say nothing about which scene it is.
const STOP_WORDS: &[&str] = &[
    "a", "an", "and", "the", "with", "her", "his", "him", "she", "he", "gets", "get", "got", "in",
    "on", "of", "to", "for", "by", "is", "my", "me", "i", "you", "your", "at", "from", "it",
    "this", "that", "who", "while", "then", "into", "up", "out", "so", "but", "or", "as", "be",
    "are", "was", "has", "have", "all", "very", "too", "s", "t", "de", "la", "el", "le", "und",
];

/// Tube-site noise in titles.
const NOISE_WORDS: &[&str] = &[
    "hd",
    "4k",
    "uhd",
    "1080p",
    "720p",
    "480p",
    "2160p",
    "60fps",
    "full",
    "video",
    "videos",
    "porn",
    "porno",
    "xxx",
    "sex",
    "new",
    "premium",
    "exclusive",
    "free",
    "official",
    "clip",
    "trailer",
    "preview",
    "pornhub",
    "xvideos",
    "xhamster",
    "xnxx",
    "spankbang",
    "eporner",
    "youporn",
    "redtube",
    "tube8",
    "com",
    "net",
    "www",
    "red",
    "onlyfans",
    "of",
    "fansly",
    "watch",
    "more",
    "part",
    "pt",
    "vol",
];

fn normalize(text: &str) -> String {
    text.to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { ' ' })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn squash(text: &str) -> String {
    normalize(text).replace(' ', "")
}

/// Meaningful words of a title, in order.
fn title_words(text: &str) -> Vec<String> {
    let mut seen = HashSet::new();
    normalize(text)
        .split(' ')
        .filter(|w| !w.is_empty() && !STOP_WORDS.contains(w) && !NOISE_WORDS.contains(w))
        .filter(|w| seen.insert(w.to_string()))
        .map(str::to_string)
        .collect()
}

/// Title with tube noise removed, for searching.
pub fn clean_title(title: &str) -> String {
    // "Title - Pornhub.com", "Title | SiteName": keep the first part when
    // it still has a few words.
    let main = title
        .split([' ', '\t'])
        .collect::<Vec<_>>()
        .join(" ")
        .split(" | ")
        .next()
        .unwrap_or(title)
        .to_string();
    let main = match main.rsplit_once(" - ") {
        Some((head, tail)) if title_words(head).len() >= 3 && title_words(tail).len() <= 2 => {
            head.to_string()
        }
        _ => main,
    };
    normalize(&main)
        .split(' ')
        .filter(|w| !w.is_empty() && !NOISE_WORDS.contains(w))
        .take(12)
        .collect::<Vec<_>>()
        .join(" ")
}

/// Most distinctive title words (longest first), skipping performer names.
fn key_words(scene: &Scene, n: usize) -> Vec<String> {
    let names: HashSet<String> = scene
        .performers
        .iter()
        .flat_map(|p| title_words(p))
        .collect();
    let mut words: Vec<String> = title_words(&scene.title)
        .into_iter()
        .filter(|w| !names.contains(w) && w.len() >= 4 && !w.chars().all(|c| c.is_numeric()))
        .collect();
    words.sort_by_key(|w| std::cmp::Reverse(w.len()));
    words.truncate(n);
    words
}

/// Up to three searches for one scene, most specific first.
pub fn search_terms(scene: &Scene) -> Vec<String> {
    let mut terms = Vec::new();
    let title = clean_title(&scene.title);
    if !title.is_empty() {
        terms.push(title);
    }
    let words = key_words(scene, 3).join(" ");
    if !scene.performers.is_empty() {
        let names = scene
            .performers
            .iter()
            .take(2)
            .map(String::as_str)
            .collect::<Vec<_>>()
            .join(" ");
        terms.push(format!("{names} {words}").trim().to_string());
    }
    let studio = scene
        .studio_name
        .as_deref()
        .or(scene.channel.as_deref())
        .map(str::trim)
        .filter(|s| !s.is_empty());
    if let Some(studio) = studio {
        if !words.is_empty() {
            terms.push(format!("{studio} {words}"));
        }
    }
    let mut seen = HashSet::new();
    terms.retain(|t| !t.is_empty() && seen.insert(normalize(t)));
    terms.truncate(3);
    terms
}

/// How well `candidate` fits `scene`: 0.0–1.0 plus the reasons shown in the UI.
pub fn score(scene: &Scene, candidate: &StashSceneMatch) -> (f32, Vec<String>) {
    let mut total = 0.0f32;
    let mut weight = 0.0f32;
    let mut reasons = Vec::new();
    let local_text = normalize(&format!(
        "{} {} {}",
        scene.title,
        scene.performers.join(" "),
        scene.channel.as_deref().unwrap_or("")
    ));
    let local_squashed = local_text.replace(' ', "");

    // Title words: how much of the official title the local one contains.
    let cand: HashSet<String> = title_words(&candidate.title).into_iter().collect();
    let local: HashSet<String> = title_words(&local_text).into_iter().collect();
    if !cand.is_empty() {
        let common = cand.intersection(&local).count() as f32;
        let s = 0.75 * common / cand.len() as f32 + 0.25 * common / local.len().max(1) as f32;
        total += 0.45 * s;
        weight += 0.45;
        if s >= 0.3 {
            reasons.push(format!("title {}%", (s * 100.0).round()));
        }
    }

    // Performers: named in the local performers or in the title itself.
    let local_names: HashSet<String> = scene.performers.iter().map(|p| squash(p)).collect();
    if !candidate.performers.is_empty() {
        let matched: Vec<&str> = candidate
            .performers
            .iter()
            .filter(|p| {
                let n = squash(&p.name);
                n.len() >= 4 && (local_names.contains(&n) || local_squashed.contains(&n))
            })
            .map(|p| p.name.as_str())
            .collect();
        let s = matched.len() as f32 / candidate.performers.len().min(3) as f32;
        total += 0.30 * s.min(1.0);
        weight += 0.30;
        if !matched.is_empty() {
            reasons.push(matched.join(", "));
        }
    }

    // Studio / channel: only when the library scene has one to compare
    // (a studio named in the title still counts as a hit).
    if let Some(studio) = candidate.studio.as_ref().map(|s| squash(&s.name)) {
        let local_studio = squash(
            scene
                .studio_name
                .as_deref()
                .or(scene.channel.as_deref())
                .unwrap_or(""),
        );
        let hit = studio.len() >= 3 && (local_studio == studio || local_squashed.contains(&studio));
        if hit || !local_studio.is_empty() {
            total += if hit { 0.10 } else { 0.0 };
            weight += 0.10;
        }
        if hit {
            reasons.push(
                candidate
                    .studio
                    .as_ref()
                    .map(|s| s.name.clone())
                    .unwrap_or_default(),
            );
        }
    }

    // Duration: tube uploads add intros / outros or are cut down.
    if let (Some(local), Some(theirs)) = (scene.duration, candidate.duration) {
        if local > 0 && theirs > 0 {
            let diff = local.abs_diff(theirs);
            let s = if diff <= 20 {
                reasons.push("same length".into());
                1.0
            } else if diff <= 90 {
                reasons.push(format!("length ±{diff}s"));
                0.7
            } else if local < theirs {
                // A clip of a longer scene.
                0.3
            } else {
                0.0
            };
            total += 0.15 * s;
            weight += 0.15;
        }
    }

    if weight == 0.0 {
        return (0.0, reasons);
    }
    ((total / weight).clamp(0.0, 1.0), reasons)
}

/// Score, sort best first and drop duplicates.
pub fn rank(scene: &Scene, candidates: Vec<StashSceneMatch>) -> Vec<StashSceneMatch> {
    let mut seen = HashSet::new();
    let mut out: Vec<StashSceneMatch> = candidates
        .into_iter()
        .filter(|c| seen.insert(c.stash_id.clone()))
        .map(|mut c| {
            let (s, reasons) = score(scene, &c);
            c.score = Some(s);
            c.match_reasons = reasons;
            c
        })
        .collect();
    out.sort_by(|a, b| {
        b.score
            .unwrap_or(0.0)
            .partial_cmp(&a.score.unwrap_or(0.0))
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{StashPerformerRef, StashStudioRef};

    fn scene(title: &str, performers: &[&str], channel: Option<&str>, dur: u32) -> Scene {
        Scene {
            title: title.into(),
            performers: performers.iter().map(|p| p.to_string()).collect(),
            channel: channel.map(str::to_string),
            duration: Some(dur),
            ..Default::default()
        }
    }

    fn cand(id: &str, title: &str, performers: &[&str], studio: &str, dur: u32) -> StashSceneMatch {
        StashSceneMatch {
            stash_id: id.into(),
            title: title.into(),
            performers: performers
                .iter()
                .map(|p| StashPerformerRef {
                    name: p.to_string(),
                    ..Default::default()
                })
                .collect(),
            studio: Some(StashStudioRef {
                name: studio.into(),
                ..Default::default()
            }),
            duration: Some(dur),
            ..Default::default()
        }
    }

    #[test]
    fn cleans_tube_titles() {
        assert_eq!(
            clean_title("Riley Reid Gets Her Massage (FULL VIDEO) HD 1080p - Pornhub.com"),
            "riley reid gets her massage"
        );
        assert_eq!(clean_title("A Night In | Brazzers"), "a night in");
    }

    #[test]
    fn search_terms_use_performers_and_studio() {
        let s = scene(
            "Stepsister helps with homework after school 😈",
            &["Riley Reid"],
            Some("Brazzers"),
            1500,
        );
        let terms = search_terms(&s);
        assert_eq!(terms[0], "stepsister helps with homework after school");
        assert!(terms[1].starts_with("Riley Reid "), "{terms:?}");
        assert!(terms[1].contains("stepsister"), "{terms:?}");
        assert!(terms[2].starts_with("Brazzers "), "{terms:?}");
    }

    #[test]
    fn ranks_the_right_scene_first() {
        let s = scene(
            "Riley Reid massage turns into more - Brazzers full video",
            &[],
            Some("Brazzers Official"),
            1830,
        );
        let ranked = rank(
            &s,
            vec![
                cand(
                    "x",
                    "Office Party",
                    &["Someone Else"],
                    "Reality Kings",
                    2400,
                ),
                cand(
                    "ok",
                    "Massage Turns Into More",
                    &["Riley Reid"],
                    "Brazzers",
                    1801,
                ),
                cand(
                    "ok",
                    "Massage Turns Into More",
                    &["Riley Reid"],
                    "Brazzers",
                    1801,
                ),
                cand("y", "Massage Therapy", &["Other Girl"], "Brazzers", 900),
            ],
        );
        assert_eq!(ranked.len(), 3);
        assert_eq!(ranked[0].stash_id, "ok");
        let top = ranked[0].score.unwrap();
        assert!(top > 0.85, "{top} {:?}", ranked[0].match_reasons);
        assert!(ranked[0].match_reasons.iter().any(|r| r == "Riley Reid"));
        assert!(ranked[1].score.unwrap() < 0.6);
        assert!(ranked[2].score.unwrap() < 0.2);
    }

    #[test]
    fn clip_of_longer_scene_still_scores() {
        let s = scene(
            "Riley Reid massage turns into more",
            &["Riley Reid"],
            None,
            600,
        );
        let (score, _) = score(
            &s,
            &cand(
                "ok",
                "Massage Turns Into More",
                &["Riley Reid"],
                "Brazzers",
                1800,
            ),
        );
        assert!(score > 0.75, "{score}");
    }
}
