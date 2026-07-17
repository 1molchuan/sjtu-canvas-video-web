mod support;

use std::{
    io::{self, Write},
    sync::{Arc, Mutex},
};

use axum::body::to_bytes;
use support::{HarnessOptions, RequestSpec, harness, ready_download, request};

#[derive(Clone)]
struct LogWriter(Arc<Mutex<Vec<u8>>>);

impl Write for LogWriter {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        self.0.lock().expect("log buffer").extend_from_slice(buffer);
        Ok(buffer.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[tokio::test]
async fn request_logs_use_route_templates_without_browser_or_upstream_secrets() {
    let logs = Arc::new(Mutex::new(Vec::new()));
    install_subscriber(logs.clone());
    let harness = harness(HarnessOptions::default()).await;
    let ready = ready_download(&harness.app).await;
    let response = request(
        &harness.app,
        RequestSpec::get(&ready.download_url).cookie(&ready.auth.cookie),
    )
    .await;
    to_bytes(response.into_body(), 64)
        .await
        .expect("download body");

    let output = String::from_utf8(logs.lock().expect("logs").clone()).expect("UTF-8 logs");
    assert!(output.contains("/api/download/{ticket}"));
    assert!(output.contains("download stream closed"));
    assert!(output.contains("bytes_streamed=15"));
    assert!(!output.contains(&ready.download_url));
    assert!(!output.contains(&ready.auth.cookie));
    assert!(!output.contains("credential=synthetic"));
    assert!(!output.contains("synthetic-token"));
}

fn install_subscriber(logs: Arc<Mutex<Vec<u8>>>) {
    let writer = move || LogWriter(logs.clone());
    let subscriber = tracing_subscriber::fmt()
        .with_ansi(false)
        .without_time()
        .with_writer(writer)
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("test subscriber");
}
