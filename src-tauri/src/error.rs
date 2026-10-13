use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("database error: {0}")]
    Db(#[from] rusqlite::Error),
    #[error("http error: {}", reqwest_chain(.0))]
    Http(#[from] reqwest::Error),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("image error: {0}")]
    Image(#[from] image::ImageError),
    #[error("not found: {0}")]
    NotFound(String),
    #[error("invalid input: {0}")]
    InvalidInput(String),
    #[error("download error: {0}")]
    Download(String),
    #[error("site error: {0}")]
    Site(String),
    #[error("{0}")]
    Other(String),
}

pub type AppResult<T> = Result<T, AppError>;

/// reqwest's own message ("error sending request for url (…)") hides the
/// cause; append the source chain (DNS / connect / TLS / timeout detail) so
/// field reports say what actually failed.
fn reqwest_chain(e: &reqwest::Error) -> String {
    let mut out = e.to_string();
    let mut source = std::error::Error::source(e);
    while let Some(cause) = source {
        let text = cause.to_string();
        if !out.contains(&text) {
            out.push_str(": ");
            out.push_str(&text);
        }
        source = cause.source();
    }
    out
}

#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn http_errors_include_the_cause() {
        // Port 9 on localhost refuses connections: the message must say so.
        let err = reqwest::Client::new()
            .get("http://127.0.0.1:9/")
            .send()
            .await
            .unwrap_err();
        let msg = super::AppError::from(err).to_string();
        assert!(
            msg.starts_with("http error: error sending request"),
            "{msg}"
        );
        assert!(
            msg.len() > "http error: error sending request for url (http://127.0.0.1:9/)".len(),
            "{msg}"
        );
    }
}
