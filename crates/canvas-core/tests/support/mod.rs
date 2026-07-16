use std::sync::Arc;

use axum::Router;
use tokio::{net::TcpListener, task::JoinHandle};
use url::Url;

pub mod topology;

pub struct MockServer {
    origin: Url,
    task: JoinHandle<()>,
}

impl MockServer {
    pub async fn spawn(router: Router) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("mock listener should bind");
        let address = listener.local_addr().expect("listener has local address");
        let origin = Url::parse(&format!("http://{address}/")).expect("mock origin is valid");
        let task = tokio::spawn(async move {
            axum::serve(listener, router)
                .await
                .expect("mock server should run");
        });
        Self { origin, task }
    }

    pub fn origin(&self) -> Url {
        self.origin.clone()
    }
}

impl Drop for MockServer {
    fn drop(&mut self) {
        self.task.abort();
    }
}

pub type Shared<T> = Arc<T>;
