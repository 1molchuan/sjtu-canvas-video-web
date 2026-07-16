//! Per-run protocol clients and isolated in-memory Cookie Stores.

mod body;
mod config;
mod policy;

use std::sync::Arc;

use reqwest::redirect::Policy;
use reqwest_cookie_store::CookieStoreMutex;
use secrecy::{ExposeSecret, SecretString};
use url::Url;

use crate::error::ProtocolError;

pub use body::read_limited_body;
pub use config::{DnsOverride, ProtocolConfig, ProtocolEndpoints, ProtocolOrigins};
pub use policy::{UpstreamPolicy, UpstreamPurpose, validate_upstream_url};

use policy::follow_redirects;

const USER_AGENT: &str = "SJTU-Canvas-Video-Web-Protocol-Validation/0.1";

#[derive(Clone)]
pub struct ProtocolContext {
    pub client: reqwest::Client,
    pub no_redirect_client: reqwest::Client,
    pub cookie_store: Arc<CookieStoreMutex>,
    pub endpoints: ProtocolEndpoints,
    pub policy: UpstreamPolicy,
}

pub(crate) struct HostCookie<'a> {
    pub name: &'static str,
    pub value: &'a SecretString,
    pub target: &'a Url,
}

impl ProtocolContext {
    pub fn new(config: ProtocolConfig) -> Result<Self, ProtocolError> {
        let cookie_store = Arc::new(CookieStoreMutex::default());
        let client = build_client(
            &config,
            cookie_store.clone(),
            follow_redirects(&config.policy),
        )?;
        let no_redirect_client = build_client(&config, cookie_store.clone(), Policy::none())?;
        Ok(Self {
            client,
            no_redirect_client,
            cookie_store,
            endpoints: config.endpoints,
            policy: config.policy,
        })
    }

    pub fn cookie_names(&self, url: &Url) -> Result<Vec<String>, ProtocolError> {
        let store = self
            .cookie_store
            .lock()
            .map_err(|_| ProtocolError::CookieStoreUnavailable)?;
        Ok(store
            .get_request_values(url)
            .map(|(name, _)| name.to_owned())
            .collect())
    }

    pub(crate) fn cookie_value(
        &self,
        url: &Url,
        expected_name: &str,
    ) -> Result<Option<SecretString>, ProtocolError> {
        let store = self
            .cookie_store
            .lock()
            .map_err(|_| ProtocolError::CookieStoreUnavailable)?;
        Ok(store
            .get_request_values(url)
            .find(|(name, _)| *name == expected_name)
            .map(|(_, value)| SecretString::from(value.to_owned())))
    }

    pub(crate) fn insert_host_cookie(&self, cookie: HostCookie<'_>) -> Result<(), ProtocolError> {
        let secure = if cookie.target.scheme() == "https" {
            "; Secure"
        } else {
            ""
        };
        let raw = format!(
            "{}={}; Path=/; HttpOnly{secure}",
            cookie.name,
            cookie.value.expose_secret()
        );
        let mut store = self
            .cookie_store
            .lock()
            .map_err(|_| ProtocolError::CookieStoreUnavailable)?;
        store
            .parse(&raw, cookie.target)
            .map(|_| ())
            .map_err(|_| ProtocolError::CookieInsertFailed)
    }

    pub(crate) fn remove_cookies_named(&self, name: &str) -> Result<(), ProtocolError> {
        let mut store = self
            .cookie_store
            .lock()
            .map_err(|_| ProtocolError::CookieStoreUnavailable)?;
        let keys = store
            .iter_any()
            .filter(|cookie| cookie.name() == name)
            .filter_map(|cookie| {
                let domain = cookie.domain.as_cow()?.into_owned();
                Some((domain, cookie.path.to_string()))
            })
            .collect::<Vec<_>>();
        for (domain, path) in keys {
            store.remove(&domain, &path, name);
        }
        Ok(())
    }
}

fn build_client(
    config: &ProtocolConfig,
    cookie_store: Arc<CookieStoreMutex>,
    redirect_policy: Policy,
) -> Result<reqwest::Client, ProtocolError> {
    let mut builder = reqwest::Client::builder()
        .user_agent(USER_AGENT)
        .connect_timeout(config.connect_timeout)
        .timeout(config.request_timeout)
        .cookie_provider(cookie_store)
        .redirect(redirect_policy)
        .no_proxy();
    for dns in &config.dns_overrides {
        builder = builder.resolve(&dns.host, dns.address);
    }
    builder
        .build()
        .map_err(|_| ProtocolError::HttpClientBuildFailed)
}
