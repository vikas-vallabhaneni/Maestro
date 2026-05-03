mod common;

use serde_json::Value;
use tokio_util::sync::CancellationToken;

#[tokio::test]
async fn boots_and_health_responds() {
    let tmp = tempfile::tempdir().expect("failed to create temp dir");
    let (addr, _pool, _token) = common::spawn_test_server(tmp.path().to_path_buf()).await;

    let resp = reqwest::get(format!("http://{addr}/api/health"))
        .await
        .expect("request failed");

    assert_eq!(resp.status(), 200);

    let body: Value = resp.json().await.expect("invalid json");
    assert_eq!(body["ok"], true);
    assert_eq!(body["track_count"], 0);
    assert!(body["library_path"].is_string());
}

#[tokio::test]
async fn scan_then_list_returns_tracks() {
    let fixtures = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let (addr, pool, _token) = common::spawn_test_server(fixtures.clone()).await;

    let cancel = CancellationToken::new();
    let scan_id = maestro::db::scans::create(&pool)
        .await
        .expect("create scan");
    let pool_clone = pool.clone();
    let fixtures_clone = fixtures.clone();
    let cancel_clone = cancel.clone();
    tokio::spawn(async move {
        let result =
            maestro::scan::scan_dir(&fixtures_clone, &pool_clone, scan_id, cancel_clone).await;
        if let Err(err) = &result {
            eprintln!("scan error: {err:#}");
        }
        result
    });

    for _ in 0..50 {
        if maestro::db::scans::is_finished(&pool, scan_id)
            .await
            .unwrap_or(false)
        {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
    assert!(
        maestro::db::scans::is_finished(&pool, scan_id)
            .await
            .expect("check scan finished"),
        "scan did not finish in time"
    );

    let resp = reqwest::get(format!("http://{addr}/api/tracks"))
        .await
        .expect("request failed");
    assert_eq!(resp.status(), 200);

    let tracks: Vec<Value> = resp.json().await.expect("invalid json");
    assert!(!tracks.is_empty(), "expected at least one track");

    let track = &tracks[0];
    assert!(track["id"].is_string(), "id must be a hex string");
    assert!(track.get("path").is_none(), "path must not be exposed");
    assert!(track.get("size").is_none(), "size must not be exposed");
    assert!(track.get("mtime").is_none(), "mtime must not be exposed");
    assert!(
        track.get("added_at").is_none(),
        "added_at must not be exposed"
    );

    let id_str = track["id"].as_str().expect("id is a string");
    assert_eq!(id_str.len(), 32, "id should be 32 hex chars (16 bytes)");
    assert!(
        id_str.chars().all(|c| c.is_ascii_hexdigit()),
        "id should be hex"
    );

    assert!(track.get("title").is_some());
    assert!(track.get("artist").is_some());
    assert!(track.get("album").is_some());
    assert!(track.get("track_no").is_some());
    assert!(track.get("duration_ms").is_some());

    let health_resp = reqwest::get(format!("http://{addr}/api/health"))
        .await
        .expect("health request failed");
    let health: Value = health_resp.json().await.expect("invalid json");
    assert!(
        health["track_count"].as_i64().expect("track_count") > 0,
        "health track_count should reflect scanned tracks"
    );
}

#[tokio::test]
async fn scan_failure_records_error_message() {
    let nonexistent = std::path::PathBuf::from("/tmp/maestro_nonexistent_library_root_42");
    let (addr, pool, _token) = common::spawn_test_server(nonexistent).await;

    let client = reqwest::Client::new();
    let resp = client
        .post(format!("http://{addr}/api/scans"))
        .send()
        .await
        .expect("POST failed");

    let status = resp.status().as_u16();
    assert!(
        status == 200 || status == 202,
        "expected 200 or 202, got {status}"
    );

    let body: Value = resp.json().await.expect("invalid json");
    let scan_id = body["id"].as_i64().expect("id");

    for _ in 0..50 {
        if maestro::db::scans::is_finished(&pool, scan_id)
            .await
            .unwrap_or(false)
        {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }

    let scan_resp = reqwest::get(format!("http://{addr}/api/scans/{scan_id}"))
        .await
        .expect("GET scan failed");
    assert_eq!(scan_resp.status(), 200);

    let scan: Value = scan_resp.json().await.expect("invalid json");
    assert!(
        scan["finished_at"].is_string(),
        "scan should have finished_at"
    );
    assert!(
        scan["error_message"].is_string(),
        "scan should have error_message"
    );
}

#[tokio::test]
async fn concurrent_scan_requests_return_same_run_id() {
    let fixtures = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let (addr, _pool, _token) = common::spawn_test_server(fixtures).await;

    let client = reqwest::Client::new();

    let (resp1, resp2) = tokio::join!(
        client.post(format!("http://{addr}/api/scans")).send(),
        client.post(format!("http://{addr}/api/scans")).send(),
    );

    let body1: Value = resp1
        .expect("req1 failed")
        .json()
        .await
        .expect("invalid json");
    let body2: Value = resp2
        .expect("req2 failed")
        .json()
        .await
        .expect("invalid json");

    let id1 = body1["id"].as_i64().expect("id1");
    let id2 = body2["id"].as_i64().expect("id2");
    assert_eq!(
        id1, id2,
        "concurrent POST /api/scans should return the same scan run id"
    );
}
