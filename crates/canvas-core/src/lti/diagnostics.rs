use reqwest::{
    Response,
    header::{CONTENT_TYPE, LOCATION},
};
use url::Url;

#[derive(Clone, Copy)]
pub(super) struct LtiProbe {
    operation: &'static str,
    method: &'static str,
    path_template: &'static str,
}

impl LtiProbe {
    pub(super) const fn external_tool() -> Self {
        Self::new(
            "lti_external_tool",
            "GET",
            "/courses/:course_id/external_tools/:tool_id",
        )
    }

    pub(super) const fn oidc_submission() -> Self {
        Self::new(
            "lti_oidc_submission",
            "POST",
            "/jy-application-canvas-sjtu/oidc/login_initiations",
        )
    }

    pub(super) const fn auth_submission() -> Self {
        Self::new(
            "lti_auth_submission",
            "POST",
            "/jy-application-canvas-sjtu/lti3/lti3Auth/ivs",
        )
    }

    pub(super) const fn oidc_redirect() -> Self {
        Self::new(
            "lti_oidc_redirect",
            "GET",
            "<validated-canvas-lti-redirect>",
        )
    }

    pub(super) const fn token_exchange() -> Self {
        Self::new(
            "lti_token_exchange",
            "GET",
            "/jy-application-canvas-sjtu/lti3/getAccessTokenByTokenId",
        )
    }

    const fn new(
        operation: &'static str,
        method: &'static str,
        path_template: &'static str,
    ) -> Self {
        Self {
            operation,
            method,
            path_template,
        }
    }

    pub(super) fn record(self, response: &Response) {
        let redirect_host = response
            .headers()
            .get(LOCATION)
            .and_then(|value| value.to_str().ok())
            .and_then(|location| redirect_host_from_location(response.url(), location));
        let content_type = response
            .headers()
            .get(CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .unwrap_or("unknown");
        tracing::info!(
            operation = self.operation,
            method = self.method,
            host = response.url().host_str().unwrap_or("unknown"),
            path_template = self.path_template,
            status = response.status().as_u16(),
            content_type,
            redirected = response.status().is_redirection(),
            response_length = ?response.content_length(),
            redirect_host = ?redirect_host,
            "LTI response classified"
        );
    }
}

fn redirect_host_from_location(base: &Url, location: &str) -> Option<String> {
    base.join(location)
        .ok()
        .and_then(|url| url.host_str().map(str::to_owned))
}

#[cfg(test)]
mod tests {
    use super::redirect_host_from_location;
    use url::Url;

    #[test]
    fn redirect_summary_exposes_only_host() {
        let base = Url::parse("https://video.example.test/oidc/start").expect("valid base URL");
        let location = "/ui?tokenId=private-token&account=private-account";

        let summary = redirect_host_from_location(&base, location);

        assert_eq!(summary.as_deref(), Some("video.example.test"));
        assert!(!format!("{summary:?}").contains("private-token"));
        assert!(!format!("{summary:?}").contains("private-account"));
    }
}
