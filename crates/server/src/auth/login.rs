use async_trait::async_trait;
use canvas_core::{
    ProtocolError,
    canvas::{UserIdentity, establish_canvas_session, probe_identity},
    client::ProtocolContext,
    jaccount::{QrLoginOptions, QrLoginProgress, login_with_qr},
};
use tokio::sync::mpsc;

pub struct AuthenticatedLogin {
    pub context: ProtocolContext,
    pub identity: UserIdentity,
}

#[async_trait]
pub trait LoginProvider: Send + Sync {
    async fn authenticate(
        &self,
        context: ProtocolContext,
        options: QrLoginOptions,
        progress: mpsc::UnboundedSender<QrLoginProgress>,
    ) -> Result<AuthenticatedLogin, ProtocolError>;
}

pub struct ProductionLoginProvider;

#[async_trait]
impl LoginProvider for ProductionLoginProvider {
    async fn authenticate(
        &self,
        context: ProtocolContext,
        options: QrLoginOptions,
        progress: mpsc::UnboundedSender<QrLoginProgress>,
    ) -> Result<AuthenticatedLogin, ProtocolError> {
        let jaccount = login_with_qr(&context, options, progress).await?;
        establish_canvas_session(&context, &jaccount.ja_auth_cookie).await?;
        let identity = probe_identity(&context).await?;
        Ok(AuthenticatedLogin { context, identity })
    }
}
