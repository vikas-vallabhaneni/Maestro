mod common;

use reqwest::Client;
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

#[tokio::test]
async fn scan_then_list_returns_tracks() {
    let fixtures = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let (addr, pool) = common::spawn_test_server(fixtures.clone()).await;

    let scan_id = maestro::db::scans::create(&pool)
        .await
        .expect("create scan");
    let pool_clone = pool.clone();
    tokio::spawn(async move {
        let result = maestro::scan::scan_dir(&fixtures, &pool_clone, scan_id).await;
        if let Err(err) = &result {
            eprintln!("scan error: {err:#}");
        }
        result
    });

    // Poll until the scan finishes
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

    // Verify the id is a valid hex string (32 hex chars = 16 bytes)
    let id_str = track["id"].as_str().expect("id is a string");
    assert_eq!(id_str.len(), 32, "id should be 32 hex chars (16 bytes)");
    assert!(
        id_str.chars().all(|c| c.is_ascii_hexdigit()),
        "id should be hex"
    );

    // Verify expected fields exist
    assert!(track.get("title").is_some());
    assert!(track.get("artist").is_some());
    assert!(track.get("album").is_some());
    assert!(track.get("track_no").is_some());
    assert!(track.get("duration_ms").is_some());

    // Verify health endpoint reflects populated library
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
async fn stream_404s_for_unknown_id() {
    let tmp = tempfile::tempdir().expect("failed to create temp dir");
    let (addr, _pool) = common::spawn_test_server(tmp.path().to_path_buf()).await;

    let resp = reqwest::get(format!(
        "http://{addr}/api/tracks/00000000000000000000000000000000/stream"
    ))
    .await
    .expect("request failed");

    assert_eq!(resp.status(), 404);
}

#[tokio::test]
async fn stream_supports_range() {
    let fixtures = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let (addr, pool) = common::spawn_test_server(fixtures.clone()).await;

    let scan_id = maestro::db::scans::create(&pool)
        .await
        .expect("create scan");
    let pool_clone = pool.clone();
    let fixtures_clone = fixtures.clone();
    tokio::spawn(async move {
        maestro::scan::scan_dir(&fixtures_clone, &pool_clone, scan_id)
            .await
            .ok();
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

    let tracks: Vec<Value> = reqwest::get(format!("http://{addr}/api/tracks"))
        .await
        .expect("request failed")
        .json()
        .await
        .expect("invalid json");
    assert!(!tracks.is_empty(), "expected at least one track");

    let track_id = tracks[0]["id"].as_str().expect("id is a string");

    let client = Client::new();
    let resp = client
        .get(format!("http://{addr}/api/tracks/{track_id}/stream"))
        .header("Range", "bytes=0-99")
        .send()
        .await
        .expect("request failed");

    assert_eq!(resp.status(), 206, "expected 206 Partial Content");

    let content_range = resp
        .headers()
        .get("content-range")
        .expect("missing content-range header")
        .to_str()
        .expect("invalid content-range header");
    assert!(
        content_range.starts_with("bytes 0-99/"),
        "content-range should start with 'bytes 0-99/', got: {content_range}"
    );

    let body = resp.bytes().await.expect("failed to read body");
    assert_eq!(body.len(), 100, "expected 100 bytes");

    // Full request without Range should return 200
    let full_resp = client
        .get(format!("http://{addr}/api/tracks/{track_id}/stream"))
        .send()
        .await
        .expect("request failed");
    assert_eq!(full_resp.status(), 200, "expected 200 OK for full request");
    assert_eq!(
        full_resp
            .headers()
            .get("accept-ranges")
            .map(|v| v.to_str().unwrap()),
        Some("bytes"),
        "expected accept-ranges: bytes"
    );
}

#[tokio::test]
async fn path_traversal_guard() {
    let library_dir = tempfile::tempdir().expect("failed to create library dir");
    let outside_dir = tempfile::tempdir().expect("failed to create outside dir");

    // Create a file outside the library root
    let outside_file = outside_dir.path().join("secret.wav");
    std::fs::write(&outside_file, b"secret audio data").expect("write outside file");

    let (addr, pool) = common::spawn_test_server(library_dir.path().to_path_buf()).await;

    // Directly insert a track row whose path lies outside the library root
    let fake_id: Vec<u8> = vec![0xDE, 0xAD, 0xBE, 0xEF, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
    sqlx::query("INSERT INTO tracks (id, path, size, mtime) VALUES (?1, ?2, ?3, ?4)")
        .bind(&fake_id)
        .bind(outside_file.to_str().unwrap())
        .bind(17_i64)
        .bind(0_i64)
        .execute(&pool)
        .await
        .expect("insert fake track");

    let hex_id = fake_id.iter().fold(String::new(), |mut acc, b| {
        use std::fmt::Write;
        let _ = write!(acc, "{b:02x}");
        acc
    });
    let resp = reqwest::get(format!("http://{addr}/api/tracks/{hex_id}/stream"))
        .await
        .expect("request failed");

    assert_eq!(resp.status(), 404, "path outside library root must be 404");

    // Verify no bytes from the outside file were returned
    let body = resp.bytes().await.expect("read body");
    assert!(
        !body.windows(6).any(|w| w == b"secret"),
        "must not leak content from outside library root"
    );
}
