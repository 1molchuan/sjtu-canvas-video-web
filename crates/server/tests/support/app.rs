use std::{net::SocketAddr, sync::Arc, time::Duration};

use async_trait::async_trait;
use axum::Router;
use canvas_core::{
    ProtocolError,
    canvas::{CanvasCourse, IdentitySource, UserIdentity},
    client::{DnsOverride, ProtocolConfig, ProtocolContext},
    jaccount::{QrLoginOptions, QrLoginProgress},
    lti::CourseVideoAuth,
    video::{CanvasVideo, VideoInfo, VideoTrack, VideoTrackInput, VideoTrackKind},
};
use secrecy::SecretString;
use server::{
    app_router,
    auth::login::{AuthenticatedLogin, LoginProvider},
    config::AppConfig,
    gateway::{ProtocolGateway, VideoDetailRequest, VideoDetailSession},
    state::{AppServices, AppState},
};
use tokio::sync::mpsc;
use url::Url;

use super::{CONTENT_HOST, GatewayMode, HarnessOptions, LoginMode, PUBLIC_ORIGIN};

struct Login {
    mode: LoginMode,
}

struct Gateway {
    upstream_url: String,
    mode: GatewayMode,
}

#[async_trait]
impl LoginProvider for Login {
    async fn authenticate(
        &self,
        context: ProtocolContext,
        _options: QrLoginOptions,
        progress: mpsc::UnboundedSender<QrLoginProgress>,
    ) -> Result<AuthenticatedLogin, ProtocolError> {
        progress
            .send(QrLoginProgress::QrReady {
                url: SecretString::from("https://qr.example.test/synthetic".to_owned()),
            })
            .map_err(|_| ProtocolError::JAccountLoginCancelled)?;
        progress
            .send(QrLoginProgress::Scanned)
            .map_err(|_| ProtocolError::JAccountLoginCancelled)?;
        if matches!(self.mode, LoginMode::DuplicateScanned) {
            progress
                .send(QrLoginProgress::Scanned)
                .map_err(|_| ProtocolError::JAccountLoginCancelled)?;
        }
        if matches!(self.mode, LoginMode::IdentityUnavailable) {
            return Err(ProtocolError::IdentityUnavailable);
        }
        let stable_id = match self.mode {
            LoginMode::Rejected => "denied-user",
            _ => "allowed-user",
        };
        Ok(AuthenticatedLogin {
            context,
            identity: UserIdentity {
                stable_id: SecretString::from(stable_id.to_owned()),
                account: None,
                display_name: None,
                source: IdentitySource::MySjtuAccount,
            },
        })
    }
}

#[async_trait]
impl ProtocolGateway for Gateway {
    async fn courses(
        &self,
        _context: &ProtocolContext,
    ) -> Result<Vec<CanvasCourse>, ProtocolError> {
        if matches!(self.mode, GatewayMode::CoursesFail) {
            return Err(ProtocolError::CanvasCourseDiscoveryRejected);
        }
        Ok(vec![CanvasCourse {
            id: 4242,
            name: "Synthetic Course".to_owned(),
            course_code: "TEST100".to_owned(),
            term: None,
        }])
    }

    async fn videos(
        &self,
        _context: &ProtocolContext,
        canvas_course_id: i64,
    ) -> Result<(CourseVideoAuth, Vec<CanvasVideo>), ProtocolError> {
        if matches!(self.mode, GatewayMode::VideosFail) {
            return Err(ProtocolError::VideoListFailed);
        }
        Ok((
            course_auth(canvas_course_id),
            vec![CanvasVideo {
                id: "synthetic-video-id".to_owned(),
                name: "Synthetic Recording".to_owned(),
                started_at: None,
                ended_at: None,
            }],
        ))
    }

    async fn video_detail(
        &self,
        _context: &ProtocolContext,
        request: VideoDetailRequest<'_>,
    ) -> Result<VideoDetailSession, ProtocolError> {
        if matches!(self.mode, GatewayMode::DetailFail) {
            return Err(ProtocolError::VideoDetailFailed);
        }
        Ok(VideoDetailSession {
            auth: Arc::new(course_auth(request.canvas_course_id)),
            info: VideoInfo {
                id: request.video_id.to_owned(),
                name: "Synthetic Recording".to_owned(),
                tracks: vec![VideoTrack::new(VideoTrackInput {
                    id: "synthetic-track-id".to_owned(),
                    kind: VideoTrackKind::Screen,
                    suggested_filename: "../unsafe\r\nname.mp4".to_owned(),
                    upstream_url: SecretString::from(self.upstream_url.clone()),
                })],
            },
        })
    }
}

pub(super) fn build(
    origin: Url,
    address: SocketAddr,
    options: HarnessOptions,
) -> (Router, AppState) {
    let upstream_url = origin
        .join("video.mp4?credential=synthetic")
        .expect("video URL");
    let protocol = ProtocolConfig::mock(
        origin,
        Duration::from_millis(options.protocol_timeout_millis),
    )
    .with_dns_overrides(vec![DnsOverride::new(CONTENT_HOST, address)]);
    let services = AppServices {
        login_provider: Arc::new(Login {
            mode: options.login_mode,
        }),
        protocol_gateway: Arc::new(Gateway {
            upstream_url: upstream_url.to_string(),
            mode: options.gateway_mode,
        }),
    };
    let state =
        AppState::with_services(parse_config(options), protocol, services).expect("app state");
    (app_router(state.clone()), state)
}

fn course_auth(canvas_course_id: i64) -> CourseVideoAuth {
    CourseVideoAuth {
        canvas_course_id,
        video_course_id: "synthetic-video-course".to_owned(),
        token: SecretString::from("synthetic-token".to_owned()),
    }
}

fn parse_config(options: HarnessOptions) -> AppConfig {
    let source = format!(
        r#"
[server]
host = "127.0.0.1"
port = 3000
public_origin = "{PUBLIC_ORIGIN}"
session_ttl_hours = 8
pending_login_ttl_minutes = 5
download_ticket_ttl_seconds = {}
download_delivery = "{}"
max_global_downloads = {}
max_downloads_per_user = {}
max_pending_logins = 20
api_timeout_seconds = 30
upstream_connect_timeout_seconds = 10
shutdown_grace_seconds = 15
[auth]
allowed_stable_ids = ["allowed-user"]
allowed_stable_id_hashes = []
[cookie]
name = "__Host-sjtu_canvas_video_session"
secure = true
same_site = "Lax"
[security]
csrf_header = "X-CSRF-Token"
trust_proxy_headers = false
"#,
        options.ticket_ttl_seconds,
        if options.direct_downloads {
            "redirect_experimental"
        } else {
            "proxy"
        },
        options.max_global_downloads,
        options.max_downloads_per_user,
    );
    toml::from_str(&source).expect("test config")
}
