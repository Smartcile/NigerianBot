//! Small helpers shared by services that handle media URLs.

/// Best-effort provider name from a URL's host, used for display/filtering.
pub fn provider_of(url: &str) -> &'static str {
    let lower = url.to_lowercase();
    if lower.contains("vimeo.com") {
        "vimeo"
    } else if lower.contains("youtube.com") || lower.contains("youtu.be") {
        "youtube"
    } else {
        "url"
    }
}
