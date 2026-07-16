#![allow(
    dead_code,
    reason = "each integration-test binary uses a different subset of the shared topology"
)]

use std::{net::SocketAddr, time::Duration};

use canvas_core::client::{DnsOverride, ProtocolConfig, ProtocolOrigins};
use url::Url;

use super::MockServer;

const JACCOUNT_HOST: &str = "jaccount.sjtu.mock.test";
const MY_SJTU_HOST: &str = "my.sjtu.mock.test";
const CANVAS_HOST: &str = "canvas.sjtu.mock.test";
const VIDEO_API_HOST: &str = "video.sjtu.mock.test";
const VIDEO_CONTENT_HOST: &str = "content.sjtu.mock.test";

pub struct MockTopology {
    pub config: ProtocolConfig,
    pub canvas_host: &'static str,
    pub canvas_origin: Url,
    pub video_api_origin: Url,
    pub video_content_origin: Url,
}

impl MockTopology {
    pub fn for_server(server: &MockServer) -> Self {
        let address = mock_address(server);
        Self::build(server, origin(JACCOUNT_HOST, address.port()))
    }

    pub fn with_socket_jaccount(server: &MockServer) -> Self {
        Self::build(server, server.origin())
    }

    fn build(server: &MockServer, jaccount: Url) -> Self {
        let address = mock_address(server);
        let origins = ProtocolOrigins {
            jaccount,
            my_sjtu: origin(MY_SJTU_HOST, address.port()),
            canvas: origin(CANVAS_HOST, address.port()),
            video_api: origin(VIDEO_API_HOST, address.port()),
            video_content: origin(VIDEO_CONTENT_HOST, address.port()),
        };
        let overrides = [
            JACCOUNT_HOST,
            MY_SJTU_HOST,
            CANVAS_HOST,
            VIDEO_API_HOST,
            VIDEO_CONTENT_HOST,
        ]
        .into_iter()
        .map(|host| DnsOverride::new(host, address))
        .collect();
        let canvas_origin = origins.canvas.clone();
        let video_api_origin = origins.video_api.clone();
        let video_content_origin = origins.video_content.clone();
        let config = ProtocolConfig::mock_with_origins(origins, Duration::from_secs(2))
            .with_dns_overrides(overrides);
        Self {
            config,
            canvas_host: CANVAS_HOST,
            canvas_origin,
            video_api_origin,
            video_content_origin,
        }
    }
}

pub fn redirect_url(headers: &axum::http::HeaderMap, host: &str, path: &str) -> String {
    let port = headers
        .get("host")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.rsplit_once(':'))
        .map_or("80", |(_, port)| port);
    format!("http://{host}:{port}{path}")
}

fn mock_address(server: &MockServer) -> SocketAddr {
    let port = server.origin().port().expect("mock origin contains a port");
    SocketAddr::from(([127, 0, 0, 1], port))
}

fn origin(host: &str, port: u16) -> Url {
    Url::parse(&format!("http://{host}:{port}/")).expect("mock logical origin is valid")
}
