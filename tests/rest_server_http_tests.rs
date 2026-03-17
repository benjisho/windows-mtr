use std::net::SocketAddr;
use std::path::PathBuf;
use std::time::Duration;

use tokio::net::TcpListener;
use tokio::sync::oneshot;
use tokio::time::{Instant, sleep};
use windows_mtr::service::rest_api::{AuthStrategy, RestApiConfig};
use windows_mtr::service::rest_server::{RestServerState, build_router};

fn probe_runner_path() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_mtr"))
}

fn build_http_client() -> reqwest::Client {
    reqwest::Client::builder()
        .no_proxy()
        .build()
        .expect("http client should build")
}

async fn spawn_server_with_config(mut config: RestApiConfig) -> (SocketAddr, oneshot::Sender<()>) {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("listener should bind");
    let addr = listener.local_addr().expect("local addr should resolve");

    config.bind_addr = addr;
    let state = RestServerState::new_with_probe_runner(config, probe_runner_path())
        .expect("state should initialize");
    let app = build_router(state);

    let (tx, rx) = oneshot::channel();
    tokio::spawn(async move {
        let server = axum::serve(
            listener,
            app.into_make_service_with_connect_info::<SocketAddr>(),
        )
        .with_graceful_shutdown(async {
            let _ = rx.await;
        });
        server.await.expect("server should run");
    });

    (addr, tx)
}

async fn spawn_server() -> (SocketAddr, oneshot::Sender<()>) {
    spawn_server_with_config(RestApiConfig::default()).await
}

fn assert_meta(body: &serde_json::Value) {
    assert_eq!(body["meta"]["schema_version"], "v1");
    assert!(body["meta"]["request_id"].is_null());
}

fn assert_error_shape(body: &serde_json::Value, status: u16, code: &str) {
    assert_meta(body);
    assert_eq!(body["error"]["status"], status);
    assert_eq!(body["error"]["code"], code);
    assert!(body["error"]["type"].is_string());
    assert!(body["error"]["title"].is_string());
    assert!(body["error"]["detail"].is_string());
}

async fn create_probe_payload(
    client: &reqwest::Client,
    addr: SocketAddr,
    payload: serde_json::Value,
) -> serde_json::Value {
    let create_res = client
        .post(format!("http://{addr}/api/v1/probes"))
        .json(&payload)
        .send()
        .await
        .expect("create probe request should succeed");

    assert_eq!(create_res.status(), reqwest::StatusCode::ACCEPTED);
    create_res.json().await.expect("json body expected")
}

async fn create_probe(
    client: &reqwest::Client,
    addr: SocketAddr,
    target: &str,
) -> serde_json::Value {
    create_probe_payload(
        client,
        addr,
        serde_json::json!({
            "targets": [target],
            "protocol": "icmp",
            "count": 1
        }),
    )
    .await
}

async fn fetch_probe(client: &reqwest::Client, addr: SocketAddr, id: &str) -> serde_json::Value {
    let get_res = client
        .get(format!("http://{addr}/api/v1/probes/{id}"))
        .send()
        .await
        .expect("get probe request should succeed");

    assert_eq!(get_res.status(), reqwest::StatusCode::OK);
    get_res.json().await.expect("json body expected")
}

async fn wait_for_probe_status(
    client: &reqwest::Client,
    addr: SocketAddr,
    id: &str,
    expected: &str,
) -> serde_json::Value {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        let probe = fetch_probe(client, addr, id).await;
        if probe["data"]["status"] == expected {
            return probe;
        }

        assert!(
            Instant::now() < deadline,
            "probe never reached status {expected}, last={probe}"
        );
        sleep(Duration::from_millis(15)).await;
    }
}

async fn wait_for_terminal_status_with_running_seen(
    client: &reqwest::Client,
    addr: SocketAddr,
    id: &str,
    expected_terminal: &str,
) -> serde_json::Value {
    let deadline = Instant::now() + Duration::from_secs(10);
    let mut saw_running = false;

    loop {
        let probe = fetch_probe(client, addr, id).await;
        match probe["data"]["status"].as_str() {
            Some("running") => saw_running = true,
            Some(status) if status == expected_terminal => {
                assert!(
                    saw_running,
                    "probe reached {expected_terminal} without observing running, last={probe}"
                );
                return probe;
            }
            _ => {}
        }

        assert!(
            Instant::now() < deadline,
            "probe never reached status {expected_terminal}, last={probe}"
        );
        sleep(Duration::from_millis(15)).await;
    }
}

#[tokio::test]
async fn health_endpoint_returns_ok() {
    let (addr, shutdown) = spawn_server().await;
    let client = build_http_client();

    let res = client
        .get(format!("http://{addr}/api/v1/health"))
        .send()
        .await
        .expect("request should succeed");

    assert_eq!(res.status(), reqwest::StatusCode::OK);
    let body: serde_json::Value = res.json().await.expect("json body expected");
    assert_meta(&body);
    assert_eq!(body["data"]["status"], "ok");
    assert_eq!(body["data"]["service"], "windows-mtr");

    let _ = shutdown.send(());
}

#[tokio::test]
async fn create_probe_transitions_through_queued_running_and_completed() {
    let (addr, shutdown) = spawn_server().await;
    let client = build_http_client();

    let created = create_probe(&client, addr, "127.0.0.1").await;
    assert_meta(&created);
    let id = created["data"]["id"]
        .as_str()
        .expect("id should be a string");
    assert_eq!(created["data"]["status"], "queued");

    let queued_or_running = fetch_probe(&client, addr, id).await;
    assert!(
        queued_or_running["data"]["status"] == "queued"
            || queued_or_running["data"]["status"] == "running",
        "expected queued or running status, got {queued_or_running}"
    );

    let completed =
        wait_for_terminal_status_with_running_seen(&client, addr, id, "completed").await;

    assert_meta(&completed);
    assert_eq!(completed["data"]["id"], id);
    assert_eq!(completed["data"]["result"]["targets"][0], "127.0.0.1");
    assert_eq!(completed["data"]["result"]["protocol"], "icmp");
    assert_eq!(completed["data"]["result"]["completed"], true);
    assert_eq!(completed["data"]["error"], serde_json::Value::Null);

    let _ = shutdown.send(());
}

#[tokio::test]
async fn create_probe_failed_transition_persists_error_details() {
    let (addr, shutdown) = spawn_server().await;
    let client = build_http_client();

    let created = create_probe(&client, addr, "definitely-not-a-real-host.invalid").await;
    assert_meta(&created);
    let id = created["data"]["id"]
        .as_str()
        .expect("id should be a string");

    let failed = wait_for_probe_status(&client, addr, id, "failed").await;
    assert_meta(&failed);
    assert_eq!(failed["data"]["result"], serde_json::Value::Null);
    assert!(
        failed["data"]["error"]
            .as_str()
            .expect("error text should exist")
            .contains("definitely-not-a-real-host.invalid")
    );

    let _ = shutdown.send(());
}

#[tokio::test]
async fn create_probe_rejects_icmp_with_port_as_bad_request() {
    let (addr, shutdown) = spawn_server().await;
    let client = build_http_client();

    let create_res = client
        .post(format!("http://{addr}/api/v1/probes"))
        .json(&serde_json::json!({
            "targets": ["1.1.1.1"],
            "protocol": "icmp",
            "port": 443
        }))
        .send()
        .await
        .expect("create probe request should succeed");

    assert_eq!(create_res.status(), reqwest::StatusCode::BAD_REQUEST);
    let body: serde_json::Value = create_res.json().await.expect("json body expected");
    assert_error_shape(&body, 400, "invalid_request");

    let _ = shutdown.send(());
}

#[tokio::test]
async fn create_probe_rejects_missing_tcp_port_as_unprocessable_entity() {
    let (addr, shutdown) = spawn_server().await;
    let client = build_http_client();

    let create_res = client
        .post(format!("http://{addr}/api/v1/probes"))
        .json(&serde_json::json!({
            "targets": ["1.1.1.1"],
            "protocol": "tcp"
        }))
        .send()
        .await
        .expect("create probe request should succeed");

    assert_eq!(
        create_res.status(),
        reqwest::StatusCode::UNPROCESSABLE_ENTITY
    );
    let body: serde_json::Value = create_res.json().await.expect("json body expected");
    assert_error_shape(&body, 422, "invalid_request");

    let _ = shutdown.send(());
}

#[tokio::test]
async fn create_probe_rejects_oversized_payload_with_413() {
    let config = RestApiConfig {
        max_payload_bytes: 32,
        ..RestApiConfig::default()
    };
    let (addr, shutdown) = spawn_server_with_config(config).await;
    let client = build_http_client();

    let oversized_targets = ["a".repeat(64)];
    let create_res = client
        .post(format!("http://{addr}/api/v1/probes"))
        .json(&serde_json::json!({
            "targets": oversized_targets,
            "protocol": "icmp"
        }))
        .send()
        .await
        .expect("create probe request should succeed");

    assert_eq!(create_res.status(), reqwest::StatusCode::PAYLOAD_TOO_LARGE);

    let body: serde_json::Value = create_res.json().await.expect("json body expected");
    assert_error_shape(&body, 413, "payload_too_large");

    let _ = shutdown.send(());
}

#[tokio::test]
async fn create_probe_rejects_burst_traffic_with_429() {
    let config = RestApiConfig {
        request_timeout: Duration::from_secs(1),
        max_concurrent_probes: 2,
        ..RestApiConfig::default()
    };
    let (addr, shutdown) = spawn_server_with_config(config).await;
    let client = build_http_client();

    let url = format!("http://{addr}/api/v1/probes");
    let payload = serde_json::json!({
        "targets": ["1.1.1.1"],
        "protocol": "icmp"
    });

    let first = client
        .post(&url)
        .json(&payload)
        .send()
        .await
        .expect("first create probe request should succeed");
    assert_eq!(first.status(), reqwest::StatusCode::ACCEPTED);

    let second = client
        .post(&url)
        .json(&payload)
        .send()
        .await
        .expect("second create probe request should succeed");
    assert_eq!(second.status(), reqwest::StatusCode::ACCEPTED);

    let third = client
        .post(&url)
        .json(&payload)
        .send()
        .await
        .expect("third create probe request should succeed");
    assert_eq!(third.status(), reqwest::StatusCode::TOO_MANY_REQUESTS);

    let body: serde_json::Value = third.json().await.expect("json body expected");
    assert_error_shape(&body, 429, "rate_limited");

    let _ = shutdown.send(());
}

#[tokio::test]
async fn api_key_auth_rejects_missing_or_invalid_key_and_accepts_valid_key() {
    let config = RestApiConfig {
        auth_strategy: AuthStrategy::ApiKey,
        api_key: Some("secret-key".to_string()),
        ..RestApiConfig::default()
    };
    let (addr, shutdown) = spawn_server_with_config(config).await;
    let client = build_http_client();

    let missing = client
        .get(format!("http://{addr}/api/v1/health"))
        .send()
        .await
        .expect("request should succeed");
    assert_eq!(missing.status(), reqwest::StatusCode::UNAUTHORIZED);

    let missing_body: serde_json::Value = missing.json().await.expect("json body expected");
    assert_error_shape(&missing_body, 401, "missing_api_key");

    let invalid = client
        .get(format!("http://{addr}/api/v1/health"))
        .header("X-API-Key", "wrong-key")
        .send()
        .await
        .expect("request should succeed");
    assert_eq!(invalid.status(), reqwest::StatusCode::FORBIDDEN);

    let invalid_body: serde_json::Value = invalid.json().await.expect("json body expected");
    assert_error_shape(&invalid_body, 403, "invalid_api_key");

    let ok = client
        .get(format!("http://{addr}/api/v1/health"))
        .header("X-API-Key", "secret-key")
        .send()
        .await
        .expect("request should succeed");
    assert_eq!(ok.status(), reqwest::StatusCode::OK);

    let _ = shutdown.send(());
}

#[tokio::test]
async fn mtls_auth_requires_upstream_client_identity_header() {
    let config = RestApiConfig {
        auth_strategy: AuthStrategy::Mtls,
        ..RestApiConfig::default()
    };
    let (addr, shutdown) = spawn_server_with_config(config).await;
    let client = build_http_client();

    let unauthorized = client
        .get(format!("http://{addr}/api/v1/health"))
        .send()
        .await
        .expect("request should succeed");
    assert_eq!(unauthorized.status(), reqwest::StatusCode::UNAUTHORIZED);

    let body: serde_json::Value = unauthorized.json().await.expect("json body expected");
    assert_error_shape(&body, 401, "missing_mtls_identity");

    let authorized = client
        .get(format!("http://{addr}/api/v1/health"))
        .header("X-Client-Cert", "present")
        .send()
        .await
        .expect("request should succeed");
    assert_eq!(authorized.status(), reqwest::StatusCode::OK);

    let _ = shutdown.send(());
}

#[tokio::test]
async fn get_probe_rejects_whitespace_id_as_bad_request() {
    let (addr, shutdown) = spawn_server().await;
    let client = build_http_client();

    let res = client
        .get(format!("http://{addr}/api/v1/probes/%20"))
        .send()
        .await
        .expect("get request should succeed");

    assert_eq!(res.status(), reqwest::StatusCode::BAD_REQUEST);
    let body: serde_json::Value = res.json().await.expect("json body expected");
    assert_error_shape(&body, 400, "invalid_probe_id");

    let _ = shutdown.send(());
}

#[tokio::test]
async fn get_probe_returns_not_found_for_unknown_id() {
    let (addr, shutdown) = spawn_server().await;
    let client = build_http_client();

    let res = client
        .get(format!("http://{addr}/api/v1/probes/does-not-exist"))
        .send()
        .await
        .expect("get request should succeed");

    assert_eq!(res.status(), reqwest::StatusCode::NOT_FOUND);
    let body: serde_json::Value = res.json().await.expect("json body expected");
    assert_error_shape(&body, 404, "probe_not_found");

    let _ = shutdown.send(());
}
