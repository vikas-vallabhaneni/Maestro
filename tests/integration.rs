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
