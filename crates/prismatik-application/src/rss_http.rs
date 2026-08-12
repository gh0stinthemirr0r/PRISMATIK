//! Hardened conditional HTTP acquisition for reviewed RSS/Atom URLs.

use reqwest::{header, redirect::Policy, Url};
use std::{
    net::{IpAddr, SocketAddr},
    time::Duration,
};

/// Conditional request state retained by the feed scheduler.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FeedConditionalHeaders {
    /// Prior ETag value.
    pub etag: Option<String>,
    /// Prior Last-Modified value.
    pub last_modified: Option<String>,
}

/// Bounded response from one reviewed feed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FeedHttpResponse {
    /// HTTP status, normally 200 or 304.
    pub status: u16,
    /// Response media type.
    pub media_type: Option<String>,
    /// Updated ETag.
    pub etag: Option<String>,
    /// Updated Last-Modified.
    pub last_modified: Option<String>,
    /// Raw bounded response body; empty for 304.
    pub bytes: Vec<u8>,
}

/// Validate and fetch one HTTPS feed without redirects or private-network access.
pub async fn fetch_reviewed_feed(
    raw_url: &str,
    conditional: &FeedConditionalHeaders,
    max_bytes: usize,
) -> Result<FeedHttpResponse, String> {
    if !(1..=8_000_000).contains(&max_bytes) {
        return Err("feed response limit must be between 1 byte and 8 MB".into());
    }
    let url = validate_feed_url(raw_url)?;
    let host = url.host_str().ok_or("feed URL host is missing")?.to_owned();
    let addresses: Vec<IpAddr> = tokio::net::lookup_host((host.as_str(), 443))
        .await
        .map_err(|error| format!("feed DNS resolution failed: {error}"))?
        .map(|address| address.ip())
        .collect();
    if addresses.is_empty() || addresses.iter().any(|address| !is_public_ip(*address)) {
        return Err("feed DNS resolution included a non-public address".into());
    }
    // Pin the validated public addresses so the request cannot re-resolve to a
    // private target between validation and connection.
    let pinned = addresses
        .iter()
        .copied()
        .map(|ip| SocketAddr::new(ip, 443))
        .collect::<Vec<_>>();
    let client = reqwest::Client::builder()
        .redirect(Policy::none())
        .timeout(Duration::from_secs(30))
        .connect_timeout(Duration::from_secs(10))
        .user_agent("PRISMATIK-feed-ingest/0.1")
        .resolve_to_addrs(&host, &pinned)
        .build()
        .map_err(|error| error.to_string())?;
    let mut request = client
        .get(url)
        .header(
            header::ACCEPT,
            "application/atom+xml, application/rss+xml, application/xml, text/xml;q=0.9",
        )
        .header(header::ACCEPT_ENCODING, "identity");
    if let Some(etag) = conditional
        .etag
        .as_deref()
        .filter(|value| !value.trim().is_empty())
    {
        request = request.header(header::IF_NONE_MATCH, etag);
    }
    if let Some(modified) = conditional
        .last_modified
        .as_deref()
        .filter(|value| !value.trim().is_empty())
    {
        request = request.header(header::IF_MODIFIED_SINCE, modified);
    }
    let mut response = request.send().await.map_err(|error| error.to_string())?;
    let status = response.status();
    if status.as_u16() == 304 {
        return Ok(FeedHttpResponse {
            status: 304,
            media_type: None,
            etag: header_text(response.headers(), header::ETAG),
            last_modified: header_text(response.headers(), header::LAST_MODIFIED),
            bytes: Vec::new(),
        });
    }
    if !status.is_success() {
        return Err(format!(
            "feed returned HTTP {status}; redirects are not followed automatically"
        ));
    }
    if response
        .content_length()
        .is_some_and(|length| length > max_bytes as u64)
    {
        return Err(format!("feed Content-Length exceeds {max_bytes} bytes"));
    }
    let media_type = header_text(response.headers(), header::CONTENT_TYPE);
    if !media_type.as_deref().is_some_and(is_feed_media_type) {
        return Err(format!(
            "feed returned unsupported Content-Type {}",
            media_type.as_deref().unwrap_or("<missing>")
        ));
    }
    let etag = header_text(response.headers(), header::ETAG);
    let last_modified = header_text(response.headers(), header::LAST_MODIFIED);
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(|error| error.to_string())? {
        if bytes.len().saturating_add(chunk.len()) > max_bytes {
            return Err(format!("feed body exceeds {max_bytes} bytes"));
        }
        bytes.extend_from_slice(&chunk);
    }
    Ok(FeedHttpResponse {
        status: status.as_u16(),
        media_type,
        etag,
        last_modified,
        bytes,
    })
}

fn validate_feed_url(raw: &str) -> Result<Url, String> {
    let url = Url::parse(raw).map_err(|error| format!("invalid feed URL: {error}"))?;
    if url.scheme() != "https"
        || url.username() != ""
        || url.password().is_some()
        || url.fragment().is_some()
    {
        return Err("feed URL must be canonical HTTPS without userinfo or fragment".into());
    }
    if url.port_or_known_default() != Some(443) {
        return Err("feed URL must use the standard HTTPS port".into());
    }
    let host = url.host_str().ok_or("feed URL host is missing")?;
    if host.eq_ignore_ascii_case("localhost") || host.ends_with(".localhost") {
        return Err("feed URL cannot target localhost".into());
    }
    if let Ok(ip) = host.parse::<IpAddr>() {
        if !is_public_ip(ip) {
            return Err("feed URL cannot target a non-public address".into());
        }
    }
    Ok(url)
}

/// Validate a reviewed feed URL and return its normalized host partition.
/// Registries use this before persistence and schedulers use it to ensure one
/// due request per publisher host in each cycle.
pub fn reviewed_feed_host(raw: &str) -> Result<String, String> {
    validate_feed_url(raw).and_then(|url| {
        url.host_str()
            .map(str::to_ascii_lowercase)
            .ok_or_else(|| "feed URL host is missing".to_owned())
    })
}

fn is_public_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(ip) => {
            !(ip.is_private()
                || ip.is_loopback()
                || ip.is_link_local()
                || ip.is_broadcast()
                || ip.is_documentation()
                || ip.is_unspecified()
                || ip.is_multicast()
                || ip.octets()[0] == 0)
        },
        IpAddr::V6(ip) => {
            !(ip.is_loopback()
                || ip.is_unspecified()
                || ip.is_multicast()
                || (ip.segments()[0] & 0xfe00) == 0xfc00
                || (ip.segments()[0] & 0xffc0) == 0xfe80)
        },
    }
}

fn header_text(headers: &header::HeaderMap, name: header::HeaderName) -> Option<String> {
    headers
        .get(name)
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned)
}

fn is_feed_media_type(value: &str) -> bool {
    matches!(
        value
            .split(';')
            .next()
            .unwrap_or_default()
            .trim()
            .to_ascii_lowercase()
            .as_str(),
        "application/atom+xml" | "application/rss+xml" | "application/xml" | "text/xml"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn url_gate_rejects_ssrf_and_noncanonical_targets() {
        assert!(validate_feed_url("http://example.com/feed").is_err());
        assert!(validate_feed_url("https://127.0.0.1/feed").is_err());
        assert!(validate_feed_url("https://user@example.com/feed").is_err());
        assert!(validate_feed_url("https://example.com:8443/feed").is_err());
        assert!(validate_feed_url("https://example.com/feed#item").is_err());
        assert!(validate_feed_url("https://example.com/feed").is_ok());
        assert_eq!(
            reviewed_feed_host("https://EXAMPLE.com/feed").unwrap(),
            "example.com"
        );
    }
    #[test]
    fn content_types_are_explicitly_bounded() {
        assert!(is_feed_media_type("application/rss+xml; charset=utf-8"));
        assert!(!is_feed_media_type("text/html"));
    }
}
