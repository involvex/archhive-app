use regex::Regex;

pub fn apply_filename_rules(filename: &str, rules: &[String]) -> (Vec<String>, Vec<String>) {
    let mut performers = Vec::new();
    let mut tags = Vec::new();

    for rule in rules {
        if let Ok(re) = Regex::new(rule) {
            if let Some(caps) = re.captures(filename) {
                if let Some(p) = caps.name("performer") {
                    performers.push(p.as_str().to_string());
                }
                if let Some(t) = caps.name("tag") {
                    tags.push(t.as_str().to_string());
                }
            }
        }
    }

    if performers.is_empty() {
        if let Some((performer, _)) = filename.split_once('-') {
            let p = performer.trim().to_string();
            if !p.is_empty() && p.len() < 40 {
                performers.push(p);
            }
        }
    }

    (performers, tags)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_performer_from_rule() {
        let rules = vec![r"(?<performer>[a-z]+)-\d+".to_string()];
        let (p, _) = apply_filename_rules("leynainu-12345", &rules);
        assert_eq!(p, vec!["leynainu"]);
    }
}
