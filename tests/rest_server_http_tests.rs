use std::net::SocketAddr;

use tokio::net::TcpListener;
use tokio::sync::oneshot;
use windows_mtr::service::rest_api::RestApiConfig;
use windows_mtr::service::rest_server::{RestServerState, build_router};

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

#[tokio::test]
async fn health_endpoint_returns_ok() {
    let (addr, shutdown) = spawn_server().await;
    let client = reqwest::Client::new();

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
async fn create_probe_then_get_probe_by_id() {
    let (addr, shutdown) = spawn_server().await;
    let client = reqwest::Client::new();

    let create_res = client
        .post(format!("http://{addr}/api/v1/probes"))
        .json(&serde_json::json!({
            "targets": ["1.1.1.1"],
            "protocol": "icmp"
        }))
        .send()
        .await
        .expect("create probe request should succeed");

    assert_eq!(create_res.status(), reqwest::StatusCode::ACCEPTED);
    let created: serde_json::Value = create_res.json().await.expect("json body expected");
    let id = created["id"].as_str().expect("id should be a string");

    let get_res = client
        .get(format!("http://{addr}/api/v1/probes/{id}"))
        .send()
        .await
        .expect("get probe request should succeed");

    assert_eq!(get_res.status(), reqwest::StatusCode::OK);
    let fetched: serde_json::Value = get_res.json().await.expect("json body expected");
    assert_eq!(fetched["id"], id);
    assert_eq!(fetched["status"], "completed");
    assert_eq!(fetched["result"]["targets"][0], "1.1.1.1");

    let _ = shutdown.send(());
}

#[tokio::test]
async fn create_probe_rejects_icmp_with_port_as_bad_request() {
    let (addr, shutdown) = spawn_server().await;
    let client = reqwest::Client::new();

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
    let client = reqwest::Client::new();

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
    let client = reqwest::Client::new();

    let res = client
        .get(format!("http://{addr}/api/v1/probes/does-not-exist"))
        .send()
        .await
        .expect("get request should succeed");

    assert_eq!(res.status(), reqwest::StatusCode::NOT_FOUND);

    let _ = shutdown.send(());
}
