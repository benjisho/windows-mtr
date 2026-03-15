use std::net::SocketAddr;
use std::time::Duration;

use tokio::net::TcpListener;
use tokio::sync::oneshot;
use tokio::time::{Instant, sleep};
use windows_mtr::service::rest_api::RestApiConfig;
use windows_mtr::service::rest_server::{RestServerState, build_router};

fn build_http_client() -> reqwest::Client {
    reqwest::Client::builder()
        .no_proxy()
        .build()
        .expect("http client should build")
}

async fn spawn_server() -> (SocketAddr, oneshot::Sender<()>) {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("listener should bind");
    let addr = listener.local_addr().expect("local addr should resolve");

    let config = RestApiConfig {
        bind_addr: addr,
        ..RestApiConfig::default()
    };
    let state = RestServerState::new(config).expect("state should initialize");
    let app = build_router(state);

    let (tx, rx) = oneshot::channel();
    tokio::spawn(async move {
        let server = axum::serve(listener, app).with_graceful_shutdown(async {
            let _ = rx.await;
        });
        server.await.expect("server should run");
    });

    (addr, tx)
}

async fn create_probe(
    client: &reqwest::Client,
    addr: SocketAddr,
    target: &str,
) -> serde_json::Value {
    let create_res = client
        .post(format!("http://{addr}/api/v1/probes"))
        .json(&serde_json::json!({
            "targets": [target],
            "protocol": "icmp"
        }))
        .send()
        .await
        .expect("create probe request should succeed");

    assert_eq!(create_res.status(), reqwest::StatusCode::ACCEPTED);
    create_res.json().await.expect("json body expected")
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
    let deadline = Instant::now() + Duration::from_secs(2);
    loop {
        let probe = fetch_probe(client, addr, id).await;
        if probe["status"] == expected {
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
    let deadline = Instant::now() + Duration::from_secs(2);
    let mut saw_running = false;

    loop {
        let probe = fetch_probe(client, addr, id).await;
        match probe["status"].as_str() {
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
    assert_eq!(body["status"], "ok");

    let _ = shutdown.send(());
}

#[tokio::test]
async fn create_probe_transitions_through_queued_running_and_completed() {
    let (addr, shutdown) = spawn_server().await;
    let client = build_http_client();

    let created = create_probe(&client, addr, "1.1.1.1").await;
    let id = created["id"].as_str().expect("id should be a string");
    assert_eq!(created["status"], "queued");

    let queued_or_running = fetch_probe(&client, addr, id).await;
    assert!(
        queued_or_running["status"] == "queued" || queued_or_running["status"] == "running",
        "expected queued or running status, got {queued_or_running}"
    );

    let completed =
        wait_for_terminal_status_with_running_seen(&client, addr, id, "completed").await;

    assert_eq!(completed["id"], id);
    assert_eq!(completed["result"]["targets"][0], "1.1.1.1");
    assert_eq!(completed["result"]["protocol"], "icmp");
    assert_eq!(completed["result"]["completed"], true);
    assert_eq!(completed["error"], serde_json::Value::Null);

    let _ = shutdown.send(());
}

#[tokio::test]
async fn create_probe_failed_transition_persists_error_details() {
    let (addr, shutdown) = spawn_server().await;
    let client = build_http_client();

    let created = create_probe(&client, addr, "simulate-failure").await;
    let id = created["id"].as_str().expect("id should be a string");

    let failed = wait_for_probe_status(&client, addr, id, "failed").await;
    assert_eq!(failed["result"], serde_json::Value::Null);
    assert!(
        failed["error"]
            .as_str()
            .expect("error text should exist")
            .contains("simulate-failure")
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

    let _ = shutdown.send(());
}
