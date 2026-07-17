use std::{
    convert::Infallible,
    net::{IpAddr, Ipv4Addr},
};

use axum::{
    extract::{ConnectInfo, FromRequestParts},
    http::request::Parts,
};

use crate::state::AppState;

pub struct PeerAddress(pub IpAddr);

impl FromRequestParts<AppState> for PeerAddress {
    type Rejection = Infallible;

    async fn from_request_parts(
        parts: &mut Parts,
        _state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let address = parts
            .extensions
            .get::<ConnectInfo<std::net::SocketAddr>>()
            .map(|ConnectInfo(address)| address.ip())
            .unwrap_or(IpAddr::V4(Ipv4Addr::LOCALHOST));
        Ok(Self(address))
    }
}
