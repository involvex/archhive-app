/// Convert ArcHive naming templates to yt-dlp `-o` templates.
///
/// Settings use placeholders like `{performer}/{title}.{ext}`. yt-dlp expects
/// `%(uploader)s/%(title)s.%(ext)s`. Without conversion, files are written to a
/// literal path named `{performer}/{title}.{ext}`.
/// yt-dlp field for `{performer}` / `{uploader}` / `{channel}`: the uploader,
/// else the channel, else the site's domain. Many tube sites report no
/// uploader, and plain `%(uploader)s` then files them under a literal `NA/`.
const UPLOADER: &str = "%(uploader,channel,webpage_url_domain|Unknown)s";

pub fn to_ytdlp_output_template(template: &str) -> String {
    let trimmed = template.trim();
    if trimmed.is_empty() {
        return format!("{UPLOADER}/%(title)s.%(ext)s");
    }
    // Already a yt-dlp template.
    if trimmed.contains("%(") {
        return trimmed.to_string();
    }

    let mut out = trimmed.to_string();
    let replacements = [
        ("{performer}", UPLOADER),
        ("{channel}", UPLOADER),
        ("{uploader}", UPLOADER),
        ("{title}", "%(title)s"),
        ("{ext}", "%(ext)s"),
        ("{id}", "%(id)s"),
        ("{site}", "%(extractor)s"),
        ("{extractor}", "%(extractor)s"),
        ("{date}", "%(upload_date)s"),
    ];
    for (from, to) in replacements {
        out = out.replace(from, to);
    }
    // Common brace variants users might type.
    out = out.replace("{Performer}", UPLOADER);
    out = out.replace("{Title}", "%(title)s");
    out = out.replace("{Ext}", "%(ext)s");

    if !out.contains("%(") {
        // Unknown brace template — fall back to a safe default rather than
        // writing a literal `{foo}` path again.
        return format!("{UPLOADER}/%(title)s.%(ext)s");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_default_archhive_template() {
        assert_eq!(
            to_ytdlp_output_template("{performer}/{title}.{ext}"),
            "%(uploader,channel,webpage_url_domain|Unknown)s/%(title)s.%(ext)s"
        );
    }

    #[test]
    fn leaves_ytdlp_template_alone() {
        let t = "%(uploader)s/%(title)s [%(id)s].%(ext)s";
        assert_eq!(to_ytdlp_output_template(t), t);
    }

    #[test]
    fn empty_falls_back() {
        assert_eq!(
            to_ytdlp_output_template("  "),
            "%(uploader,channel,webpage_url_domain|Unknown)s/%(title)s.%(ext)s"
        );
    }
}
