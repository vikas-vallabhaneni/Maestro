mod common;

use serde_json::Value;

#[tokio::test]
async fn boots_and_health_responds() {
    let tmp = tempfile::tempdir().expect("failed to create temp dir");
    let (addr, _pool) = common::spawn_test_server(tmp.path().to_path_buf()).await;

    let resp = reqwest::get(format!("http://{addr}/api/health"))
        .await
        .expect("request failed");

    assert_eq!(resp.status(), 200);

    let body: Value = resp.json().await.expect("invalid json");
    assert_eq!(body["ok"], true);
    assert_eq!(body["track_count"], 0);
    assert!(body["library_path"].is_string());
}
