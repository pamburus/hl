//! Source-URL fetcher with SSRF mitigations.
//!
//! This is the only module that's allowed to make outbound network requests on behalf
//! of the caller. Everything routes through [`SourceClient`], which validates the URL
//! against the configured policy before any I/O happens:
//!
//! 1. Scheme must be in {`http`, `https`} — or `file` if `--allow-file` was passed.
//! 2. Host must match the optional allow-list (when empty, hosts aren't restricted).
//! 3. Hostname must resolve, and *every* resolved address must not be in the blocked
//!    ranges (loopback / private / link-local / CGNAT / multicast / ULA / etc.) unless
//!    `--allow-private` was passed.
//!
//! Known v1 limitation: validation resolves the hostname once, then hands the URL to
//! `reqwest`, which resolves again at connect time. A DNS-rebinding attacker could in
//! principle return a public IP to the validator and a private IP to the connector. We
//! document this rather than ship a custom DNS resolver right now; closing the window
//! is a follow-up via `reqwest::dns::Resolve`.

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::time::Duration;

use anyhow::Context;
use reqwest::{Client, Url, header};
use serde::Serialize;
use thiserror::Error;

/// Runtime policy for [`SourceClient`]. Mirrors the CLI flags one-to-one.
#[derive(Debug, Clone)]
pub struct Config {
    /// Allow URLs whose hostname resolves to a private/loopback/link-local/etc. address.
    /// Default false; set true only for local dev or test rigs.
    pub allow_private: bool,
    /// Allow `file://` URLs. Default false; testing only — opens the server's local FS
    /// to whoever can reach the API.
    pub allow_file_scheme: bool,
    /// Glob patterns of allowed hostnames. Empty = no restriction.
    pub allow_hosts: Vec<HostPattern>,
    /// Hard cap on the source's reported `Content-Length`. Requests that announce a
    /// larger size are refused before any range fetch happens.
    pub max_size: u64,
    pub connect_timeout: Duration,
    pub request_timeout: Duration,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            allow_private: false,
            allow_file_scheme: false,
            allow_hosts: Vec::new(),
            max_size: 10 * 1024 * 1024 * 1024, // 10 GiB
            connect_timeout: Duration::from_secs(10),
            request_timeout: Duration::from_secs(120),
        }
    }
}

/// Simple glob pattern with `*` (any run of chars) and `?` (single char). Case-
/// insensitive on hostnames. Good enough for "*.example.com" style allow-lists; we can
/// swap in a real matcher later if anyone wants per-label semantics.
#[derive(Debug, Clone)]
pub struct HostPattern(String);

impl HostPattern {
    pub fn new(p: impl Into<String>) -> Self {
        Self(p.into().to_ascii_lowercase())
    }

    pub fn matches(&self, host: &str) -> bool {
        glob_match(self.0.as_bytes(), host.to_ascii_lowercase().as_bytes())
    }
}

fn glob_match(pat: &[u8], txt: &[u8]) -> bool {
    fn go(p: &[u8], pi: usize, t: &[u8], ti: usize) -> bool {
        if pi == p.len() {
            return ti == t.len();
        }
        match p[pi] {
            b'*' => {
                // Skip consecutive stars.
                let mut np = pi + 1;
                while np < p.len() && p[np] == b'*' {
                    np += 1;
                }
                for k in ti..=t.len() {
                    if go(p, np, t, k) {
                        return true;
                    }
                }
                false
            }
            b'?' => ti < t.len() && go(p, pi + 1, t, ti + 1),
            c => ti < t.len() && t[ti] == c && go(p, pi + 1, t, ti + 1),
        }
    }
    go(pat, 0, txt, 0)
}

#[derive(Debug, Error)]
pub enum SourceError {
    #[error("invalid URL: {0}")]
    InvalidUrl(String),
    #[error("scheme not allowed: {0}")]
    SchemeNotAllowed(String),
    #[error("host not in allow-list: {0}")]
    HostNotAllowed(String),
    #[error("address blocked by SSRF guard: {host} resolves to {addr}")]
    AddressBlocked { host: String, addr: IpAddr },
    #[error("DNS resolution failed for {host}: {source}")]
    DnsFailed {
        host: String,
        #[source]
        source: std::io::Error,
    },
    #[error("upstream returned {status}")]
    UpstreamStatus { status: u16 },
    #[error("source too large: {bytes} bytes exceeds max {max}")]
    TooLarge { bytes: u64, max: u64 },
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

/// Metadata returned from a HEAD probe. We don't pass through unknown headers — only the
/// fields the client actually needs to drive range requests.
#[derive(Debug, Clone, Serialize)]
pub struct Metadata {
    pub content_length: Option<u64>,
    pub accept_ranges: bool,
    pub etag: Option<String>,
    pub last_modified: Option<String>,
    pub content_type: Option<String>,
}

pub struct SourceClient {
    config: Config,
    http: Client,
}

impl SourceClient {
    pub fn new(config: Config) -> anyhow::Result<Self> {
        let http = Client::builder()
            .connect_timeout(config.connect_timeout)
            .timeout(config.request_timeout)
            .user_agent(concat!("hl-server/", env!("CARGO_PKG_VERSION")))
            .build()
            .context("failed to build reqwest client")?;
        Ok(Self { config, http })
    }

    /// HEAD-equivalent probe. For HTTP(S), issues a HEAD request; for `file://`, stats
    /// the local file.
    pub async fn probe(&self, url: &str) -> Result<Metadata, SourceError> {
        let parsed = self.validate(url).await?;
        if parsed.scheme() == "file" {
            return self.probe_file(&parsed).await;
        }
        let resp = self.http.head(parsed).send().await?;
        let status = resp.status();
        if !status.is_success() {
            return Err(SourceError::UpstreamStatus {
                status: status.as_u16(),
            });
        }
        let h = resp.headers();
        // `Response::content_length()` reads from the body's size hint, which is 0 on a
        // HEAD response even when the server sent a Content-Length header. Parse the
        // header directly so we don't lie to the client.
        let content_length = h
            .get(header::CONTENT_LENGTH)
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.parse::<u64>().ok());
        if let Some(n) = content_length {
            if n > self.config.max_size {
                return Err(SourceError::TooLarge {
                    bytes: n,
                    max: self.config.max_size,
                });
            }
        }
        Ok(Metadata {
            content_length,
            accept_ranges: h
                .get(header::ACCEPT_RANGES)
                .and_then(|v| v.to_str().ok())
                .map(|s| s.contains("bytes"))
                .unwrap_or(false),
            etag: header_str(h, header::ETAG),
            last_modified: header_str(h, header::LAST_MODIFIED),
            content_type: header_str(h, header::CONTENT_TYPE),
        })
    }

    async fn probe_file(&self, url: &Url) -> Result<Metadata, SourceError> {
        let path = url
            .to_file_path()
            .map_err(|_| SourceError::InvalidUrl("file URL is not a valid path".into()))?;
        let meta = tokio::fs::metadata(&path).await?;
        if meta.len() > self.config.max_size {
            return Err(SourceError::TooLarge {
                bytes: meta.len(),
                max: self.config.max_size,
            });
        }
        Ok(Metadata {
            content_length: Some(meta.len()),
            accept_ranges: true,
            etag: None,
            last_modified: None,
            content_type: None,
        })
    }

    async fn validate(&self, url: &str) -> Result<Url, SourceError> {
        let parsed = Url::parse(url).map_err(|e| SourceError::InvalidUrl(e.to_string()))?;
        match parsed.scheme() {
            "http" | "https" => {}
            "file" if self.config.allow_file_scheme => return Ok(parsed),
            s => return Err(SourceError::SchemeNotAllowed(s.into())),
        }
        let host = parsed
            .host_str()
            .ok_or_else(|| SourceError::InvalidUrl("URL has no host".into()))?
            .to_string();
        if !self.config.allow_hosts.is_empty()
            && !self.config.allow_hosts.iter().any(|p| p.matches(&host))
        {
            return Err(SourceError::HostNotAllowed(host));
        }
        // Port doesn't matter for DNS lookup; just feed lookup_host any non-zero port.
        let port = parsed.port_or_known_default().unwrap_or(80);
        let addrs: Vec<_> = tokio::net::lookup_host((host.as_str(), port))
            .await
            .map_err(|source| SourceError::DnsFailed {
                host: host.clone(),
                source,
            })?
            .collect();
        if addrs.is_empty() {
            return Err(SourceError::DnsFailed {
                host: host.clone(),
                source: std::io::Error::other("no addresses returned"),
            });
        }
        if !self.config.allow_private {
            for sa in &addrs {
                if is_blocked_ip(&sa.ip()) {
                    return Err(SourceError::AddressBlocked {
                        host,
                        addr: sa.ip(),
                    });
                }
            }
        }
        Ok(parsed)
    }
}

fn header_str(h: &header::HeaderMap, name: header::HeaderName) -> Option<String> {
    h.get(name).and_then(|v| v.to_str().ok()).map(String::from)
}

/// Returns true if `ip` lies in any range we never want the server to dial: loopback,
/// RFC1918 private, link-local (incl. the AWS/GCP/Azure metadata net `169.254/16`),
/// CGNAT `100.64/10`, broadcast, multicast, reserved IPv4 (`240/4`), and the IPv6
/// equivalents (loopback `::1`, multicast `ff00::/8`, ULA `fc00::/7`, link-local
/// `fe80::/10`), plus IPv4-mapped IPv6 addresses that reduce to a blocked v4.
fn is_blocked_ip(ip: &IpAddr) -> bool {
    match ip {
        IpAddr::V4(v) => is_blocked_v4(v),
        IpAddr::V6(v) => is_blocked_v6(v),
    }
}

fn is_blocked_v4(ip: &Ipv4Addr) -> bool {
    if ip.is_loopback()
        || ip.is_private()
        || ip.is_link_local()
        || ip.is_broadcast()
        || ip.is_multicast()
        || ip.is_unspecified()
        || ip.is_documentation()
    {
        return true;
    }
    let o = ip.octets();
    // 100.64.0.0/10 — Carrier-grade NAT (RFC 6598).
    if o[0] == 100 && (64..=127).contains(&o[1]) {
        return true;
    }
    // 240.0.0.0/4 — reserved for future use, including 255.255.255.255 (already caught
    // by `is_broadcast`, but cheap to keep).
    if o[0] >= 240 {
        return true;
    }
    false
}

fn is_blocked_v6(ip: &Ipv6Addr) -> bool {
    if ip.is_loopback() || ip.is_multicast() || ip.is_unspecified() {
        return true;
    }
    if let Some(v4) = ip.to_ipv4_mapped() {
        if is_blocked_v4(&v4) {
            return true;
        }
    }
    // Also catch 6to4 / IPv4-compatible style embeddings.
    if let Some(v4) = ip.to_ipv4() {
        if is_blocked_v4(&v4) {
            return true;
        }
    }
    let s = ip.segments();
    // fc00::/7 — Unique Local Addresses.
    if s[0] & 0xFE00 == 0xFC00 {
        return true;
    }
    // fe80::/10 — link-local unicast.
    if s[0] & 0xFFC0 == 0xFE80 {
        return true;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blocks_loopback_and_private() {
        for s in ["127.0.0.1", "10.0.0.5", "192.168.1.1", "172.16.5.5", "169.254.169.254"] {
            let ip: IpAddr = s.parse().unwrap();
            assert!(is_blocked_ip(&ip), "expected {s} blocked");
        }
    }

    #[test]
    fn allows_public_ipv4() {
        for s in ["1.1.1.1", "8.8.8.8", "93.184.216.34"] {
            let ip: IpAddr = s.parse().unwrap();
            assert!(!is_blocked_ip(&ip), "expected {s} allowed");
        }
    }

    #[test]
    fn blocks_cgnat() {
        let ip: IpAddr = "100.64.0.1".parse().unwrap();
        assert!(is_blocked_ip(&ip));
        // boundary cases
        assert!(is_blocked_ip(&"100.127.255.255".parse().unwrap()));
        // just outside CGNAT
        assert!(!is_blocked_ip(&"100.128.0.0".parse().unwrap()));
        assert!(!is_blocked_ip(&"100.63.255.255".parse().unwrap()));
    }

    #[test]
    fn blocks_ipv6() {
        for s in ["::1", "fc00::1", "fe80::1", "ff00::1"] {
            let ip: IpAddr = s.parse().unwrap();
            assert!(is_blocked_ip(&ip), "expected {s} blocked");
        }
    }

    #[test]
    fn blocks_ipv4_mapped_loopback() {
        let ip: IpAddr = "::ffff:127.0.0.1".parse().unwrap();
        assert!(is_blocked_ip(&ip));
    }

    #[test]
    fn host_pattern_matches() {
        let p = HostPattern::new("*.example.com");
        assert!(p.matches("foo.example.com"));
        assert!(p.matches("a.b.example.com"));
        assert!(!p.matches("example.com")); // '*' requires at least the leading dot
        assert!(!p.matches("example.org"));
    }
}
