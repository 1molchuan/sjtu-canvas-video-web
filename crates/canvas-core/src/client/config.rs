use std::{net::SocketAddr, time::Duration};

use url::Url;

use super::policy::{UpstreamPolicy, UpstreamPurpose};

#[derive(Debug, Clone)]
pub struct ProtocolEndpoints {
    pub my_info: Url,
    pub my_account: Url,
    pub express_login: Url,
    pub websocket_base: Url,
    pub qr_confirm: Url,
    pub canvas_login: Url,
    pub canvas_protected: Url,
    pub canvas_self: Url,
    pub canvas_courses: Url,
    pub canvas_dashboard: Url,
    pub oidc_login: Url,
    pub lti_auth: Url,
    pub token_exchange: Url,
    pub video_list: Url,
    pub video_detail: Url,
    pub video_ui_referer: Url,
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
pub struct DnsOverride {
    pub(crate) host: String,
    pub(crate) address: SocketAddr,
}

#[derive(Debug, Clone)]
pub struct ProtocolConfig {
    pub endpoints: ProtocolEndpoints,
    pub policy: UpstreamPolicy,
    pub request_timeout: Duration,
    pub connect_timeout: Duration,
    pub(crate) dns_overrides: Vec<DnsOverride>,
}

impl DnsOverride {
    pub fn new(host: impl Into<String>, address: SocketAddr) -> Self {
        Self {
            host: host.into(),
            address,
        }
    }
}

impl ProtocolEndpoints {
    pub fn production() -> Self {
        Self {
            my_info: static_url("https://my.sjtu.edu.cn/ui/appmyinfo"),
            my_account: static_url("https://my.sjtu.edu.cn/api/account"),
            express_login: static_url("https://jaccount.sjtu.edu.cn/jaccount/expresslogin"),
            websocket_base: static_url("wss://jaccount.sjtu.edu.cn/jaccount/sub/"),
            qr_confirm: static_url("https://jaccount.sjtu.edu.cn/jaccount/confirmscancode"),
            canvas_login: static_url("https://oc.sjtu.edu.cn/login/openid_connect"),
            canvas_protected: static_url("https://oc.sjtu.edu.cn/"),
            canvas_self: static_url("https://oc.sjtu.edu.cn/api/v1/users/self"),
            canvas_courses: static_url("https://oc.sjtu.edu.cn/api/v1/courses"),
            canvas_dashboard: static_url("https://oc.sjtu.edu.cn/"),
            oidc_login: static_url(
                "https://v.sjtu.edu.cn/jy-application-canvas-sjtu/oidc/login_initiations",
            ),
            lti_auth: static_url(
                "https://v.sjtu.edu.cn/jy-application-canvas-sjtu/lti3/lti3Auth/ivs",
            ),
            token_exchange: static_url(
                "https://v.sjtu.edu.cn/jy-application-canvas-sjtu/lti3/getAccessTokenByTokenId",
            ),
            video_list: static_url(
                "https://v.sjtu.edu.cn/jy-application-canvas-sjtu/directOnDemandPlay/findVodVideoList",
            ),
            video_detail: static_url(
                "https://v.sjtu.edu.cn/jy-application-canvas-sjtu/directOnDemandPlay/getVodVideoInfos",
            ),
            video_ui_referer: static_url("https://v.sjtu.edu.cn/jy-application-canvas-sjtu-ui/"),
        }
    }

    pub fn jaccount_origin(&self) -> Url {
        origin_of(&self.express_login)
    }

    pub fn my_sjtu_origin(&self) -> Url {
        origin_of(&self.my_info)
    }

    pub fn canvas_origin(&self) -> Url {
        origin_of(&self.canvas_login)
    }

    fn mock(origins: &ProtocolOrigins) -> Self {
        let websocket_base = websocket_url(&origins.jaccount);
        Self {
            my_info: join(&origins.my_sjtu, "my/info"),
            my_account: join(&origins.my_sjtu, "my/account"),
            express_login: join(&origins.jaccount, "ja/express"),
            websocket_base,
            qr_confirm: join(&origins.jaccount, "ja/qr"),
            canvas_login: join(&origins.canvas, "canvas/login"),
            canvas_protected: join(&origins.canvas, "canvas/protected"),
            canvas_self: join(&origins.canvas, "canvas/api/self"),
            canvas_courses: join(&origins.canvas, "canvas/api/courses"),
            canvas_dashboard: join(&origins.canvas, "canvas/dashboard"),
            oidc_login: join(&origins.video_api, "video/oidc/login_initiations"),
            lti_auth: join(&origins.video_api, "video/lti3/lti3Auth/ivs"),
            token_exchange: join(&origins.video_api, "video/lti3/getAccessTokenByTokenId"),
            video_list: join(
                &origins.video_api,
                "video/directOnDemandPlay/findVodVideoList",
            ),
            video_detail: join(
                &origins.video_api,
                "video/directOnDemandPlay/getVodVideoInfos",
            ),
            video_ui_referer: join(&origins.video_api, "video-ui/"),
        }
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
            dns_overrides: Vec::new(),
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
            dns_overrides: Vec::new(),
        }
    }

    pub fn with_dns_overrides(mut self, overrides: Vec<DnsOverride>) -> Self {
        self.dns_overrides = overrides;
        self
    }
}

fn websocket_url(origin: &Url) -> Url {
    let mut websocket = join(origin, "ja/ws/");
    websocket
        .set_scheme("ws")
        .expect("HTTP mock origin can become WebSocket origin");
    websocket
}

fn join(origin: &Url, path: &str) -> Url {
    origin.join(path).expect("static mock path is valid")
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
