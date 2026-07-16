//! Per-run protocol clients and isolated in-memory Cookie Stores.

mod policy;

use std::{sync::Arc, time::Duration};

use reqwest::redirect::Policy;
use reqwest_cookie_store::CookieStoreMutex;
use secrecy::SecretString;
use url::Url;

use crate::error::ProtocolError;

pub use policy::{UpstreamPolicy, UpstreamPurpose, validate_upstream_url};

use policy::follow_redirects;

const USER_AGENT: &str = "SJTU-Canvas-Video-Web-Protocol-Validation/0.1";

#[derive(Debug, Clone)]
pub struct ProtocolEndpoints {
    pub my_info: Url,
    pub express_login: Url,
    pub websocket_base: Url,
    pub qr_confirm: Url,
}

#[derive(Debug, Clone)]
pub struct ProtocolOrigins {
    pub jaccount: Url,
    pub my_sjtu: Url,
    pub canvas: Url,
    pub video_api: Url,
    pub video_content: Url,
}

#[derive(Debug, Clone)]
pub struct ProtocolConfig {
    pub endpoints: ProtocolEndpoints,
    pub policy: UpstreamPolicy,
    pub request_timeout: Duration,
    pub connect_timeout: Duration,
}

#[derive(Clone)]
pub struct ProtocolContext {
    pub client: reqwest::Client,
    pub no_redirect_client: reqwest::Client,
    pub cookie_store: Arc<CookieStoreMutex>,
    pub endpoints: ProtocolEndpoints,
    pub policy: UpstreamPolicy,
}

impl ProtocolEndpoints {
    pub fn production() -> Self {
        Self {
            my_info: static_url("https://my.sjtu.edu.cn/ui/appmyinfo"),
            express_login: static_url("https://jaccount.sjtu.edu.cn/jaccount/expresslogin"),
            websocket_base: static_url("wss://jaccount.sjtu.edu.cn/jaccount/sub/"),
            qr_confirm: static_url("https://jaccount.sjtu.edu.cn/jaccount/confirmscancode"),
        }
    }

    fn mock(origins: &ProtocolOrigins) -> Self {
        let mut websocket_base = origins
            .jaccount
            .join("ja/ws/")
            .expect("mock WebSocket path is valid");
        websocket_base
            .set_scheme("ws")
            .expect("HTTP mock origin can become WebSocket origin");
        Self {
            my_info: origins.my_sjtu.join("my/info").expect("mock path is valid"),
            express_login: origins
                .jaccount
                .join("ja/express")
                .expect("mock path is valid"),
            websocket_base,
            qr_confirm: origins.jaccount.join("ja/qr").expect("mock path is valid"),
        }
    }

    pub fn jaccount_origin(&self) -> Url {
        origin_of(&self.express_login)
    }
}

impl ProtocolOrigins {
    pub fn for_mock(jaccount: Url, my_sjtu: Url) -> Self {
        Self {
            canvas: jaccount.clone(),
            video_api: jaccount.clone(),
            video_content: jaccount.clone(),
            jaccount,
            my_sjtu,
        }
    }

    fn single(origin: Url) -> Self {
        Self::for_mock(origin.clone(), origin)
    }

    fn policy_entries(&self, websocket: &Url) -> Vec<(UpstreamPurpose, Url)> {
        vec![
            (UpstreamPurpose::JAccount, self.jaccount.clone()),
            (UpstreamPurpose::JAccount, websocket.clone()),
            (UpstreamPurpose::MySjtu, self.my_sjtu.clone()),
            (UpstreamPurpose::Canvas, self.canvas.clone()),
            (UpstreamPurpose::VideoApi, self.video_api.clone()),
            (UpstreamPurpose::VideoContent, self.video_content.clone()),
        ]
    }
}

impl ProtocolConfig {
    pub fn production(timeout: Duration) -> Self {
        Self {
            endpoints: ProtocolEndpoints::production(),
            policy: UpstreamPolicy::production(),
            request_timeout: timeout,
            connect_timeout: timeout,
        }
    }

    pub fn mock(origin: Url, timeout: Duration) -> Self {
        Self::mock_with_origins(ProtocolOrigins::single(origin), timeout)
    }

    pub fn mock_with_origins(origins: ProtocolOrigins, timeout: Duration) -> Self {
        let endpoints = ProtocolEndpoints::mock(&origins);
        Self {
            policy: UpstreamPolicy::from_urls(
                origins.policy_entries(&endpoints.websocket_base),
                false,
            ),
            endpoints,
            request_timeout: timeout,
            connect_timeout: timeout,
        }
    }
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
}

fn build_client(
    config: &ProtocolConfig,
    cookie_store: Arc<CookieStoreMutex>,
    redirect_policy: Policy,
) -> Result<reqwest::Client, ProtocolError> {
    reqwest::Client::builder()
        .user_agent(USER_AGENT)
        .connect_timeout(config.connect_timeout)
        .timeout(config.request_timeout)
        .cookie_provider(cookie_store)
        .redirect(redirect_policy)
        .build()
        .map_err(|_| ProtocolError::HttpClientBuildFailed)
}

fn static_url(value: &str) -> Url {
    Url::parse(value).expect("static protocol URL must be valid")
}

fn origin_of(url: &Url) -> Url {
    let mut origin = url.clone();
    origin.set_path("/");
    origin.set_query(None);
    origin.set_fragment(None);
    origin
}
