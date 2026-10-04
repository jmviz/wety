//! Rate-limiting key for a server that may sit behind a trusted proxy.

use std::net::{IpAddr, SocketAddr};

use axum::{
    extract::ConnectInfo,
    http::{HeaderName, Request},
};
use tower_governor::{GovernorError, key_extractor::KeyExtractor};

/// Keys requests by client IP. When `header` is set (behind Cloudflare Tunnel,
/// `cf-connecting-ip`), the IP comes from that header; otherwise, or when the
/// header is absent (local health checks), from the peer address.
///
/// Only set `header` when the server is reachable solely through a proxy that
/// overwrites it: any direct client could otherwise choose its own key.
#[derive(Clone, Debug)]
pub struct ClientIpKeyExtractor {
    header: Option<HeaderName>,
}

impl ClientIpKeyExtractor {
    #[must_use]
    pub fn new(header: Option<HeaderName>) -> Self {
        Self { header }
    }
}

impl KeyExtractor for ClientIpKeyExtractor {
    type Key = IpAddr;
    type KeyExtractionError = GovernorError;

    fn name(&self) -> &'static str {
        "client IP"
    }

    fn extract<T>(&self, req: &Request<T>) -> Result<Self::Key, GovernorError> {
        self.header
            .as_ref()
            .and_then(|header| req.headers().get(header))
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.trim().parse().ok())
            .or_else(|| {
                req.extensions()
                    .get::<ConnectInfo<SocketAddr>>()
                    .map(|ConnectInfo(addr)| addr.ip())
            })
            .ok_or(GovernorError::UnableToExtractKey)
    }

    fn key_name(&self, key: &Self::Key) -> Option<String> {
        Some(key.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(header: Option<&str>) -> Request<()> {
        let mut builder = Request::builder();
        if let Some(value) = header {
            builder = builder.header("cf-connecting-ip", value);
        }
        let mut req = builder.body(()).unwrap();
        req.extensions_mut()
            .insert(ConnectInfo(SocketAddr::from(([127, 0, 0, 1], 51000))));
        req
    }

    #[test]
    fn uses_header_when_configured() {
        let extractor = ClientIpKeyExtractor::new(Some(HeaderName::from_static("cf-connecting-ip")));
        let key = extractor.extract(&request(Some("203.0.113.7"))).unwrap();
        assert_eq!(key, "203.0.113.7".parse::<IpAddr>().unwrap());
    }

    #[test]
    fn falls_back_to_peer_without_header() {
        let extractor = ClientIpKeyExtractor::new(Some(HeaderName::from_static("cf-connecting-ip")));
        let key = extractor.extract(&request(None)).unwrap();
        assert_eq!(key, "127.0.0.1".parse::<IpAddr>().unwrap());
    }

    #[test]
    fn ignores_header_when_not_configured() {
        let extractor = ClientIpKeyExtractor::new(None);
        let key = extractor.extract(&request(Some("203.0.113.7"))).unwrap();
        assert_eq!(key, "127.0.0.1".parse::<IpAddr>().unwrap());
    }
}
