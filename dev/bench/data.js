window.BENCHMARK_DATA = {
  "lastUpdate": 1782953055805,
  "repoUrl": "https://github.com/halft0n/LightFrame",
  "entries": {
    "LightFrame Benchmarks": [
      {
        "commit": {
          "author": {
            "email": "w_sunshine163@163.com",
            "name": "halft0n",
            "username": "halft0n"
          },
          "committer": {
            "email": "w_sunshine163@163.com",
            "name": "halft0n",
            "username": "halft0n"
          },
          "distinct": true,
          "id": "6f9767a4be59be9146383df7a33e067b02fc6849",
          "message": "fix: harden bench CI — add permissions, guard empty results\n\nAdd explicit contents:write permission for gh-pages auto-push.\nSkip benchmark-action step when no results are extracted.\nWarn on empty Criterion output to aid debugging.",
          "timestamp": "2026-07-01T19:20:11+08:00",
          "tree_id": "a28e27ea070507f0f1c360d99ae50f06d86b1e73",
          "url": "https://github.com/halft0n/LightFrame/commit/6f9767a4be59be9146383df7a33e067b02fc6849"
        },
        "date": 1782905640473,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "upsert_media_1000",
            "value": 125680000,
            "unit": "ns"
          },
          {
            "name": "search_media_fts_10k",
            "value": 8339499.999999999,
            "unit": "ns"
          },
          {
            "name": "compute_dhash",
            "value": 1244500,
            "unit": "ns"
          },
          {
            "name": "compute_phash",
            "value": 1604500,
            "unit": "ns"
          },
          {
            "name": "scan_walkdir/1000",
            "value": 725730,
            "unit": "ns"
          },
          {
            "name": "scan_walkdir/5000",
            "value": 3645300,
            "unit": "ns"
          },
          {
            "name": "scan_walkdir/10000",
            "value": 7282100,
            "unit": "ns"
          },
          {
            "name": "is_media_file_batch_8",
            "value": 773.81,
            "unit": "ns"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "w_sunshine163@163.com",
            "name": "halft0n",
            "username": "halft0n"
          },
          "committer": {
            "email": "w_sunshine163@163.com",
            "name": "halft0n",
            "username": "halft0n"
          },
          "distinct": true,
          "id": "59f0a9788fc0d797133617186fed30bcd3b31f08",
          "message": "chore: bump version to v0.0.21\n\nFix Windows startup crash (WebView2 incompatible flags),\nharden bench CI, add gh-pages branch for benchmark data.",
          "timestamp": "2026-07-01T19:42:40+08:00",
          "tree_id": "fda6af483c78e7d72b5b0d211c439affe4657130",
          "url": "https://github.com/halft0n/LightFrame/commit/59f0a9788fc0d797133617186fed30bcd3b31f08"
        },
        "date": 1782906787422,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "upsert_media_1000",
            "value": 149430000,
            "unit": "ns"
          },
          {
            "name": "search_media_fts_10k",
            "value": 9201300,
            "unit": "ns"
          },
          {
            "name": "compute_dhash",
            "value": 1256200,
            "unit": "ns"
          },
          {
            "name": "compute_phash",
            "value": 1546000,
            "unit": "ns"
          },
          {
            "name": "scan_walkdir/1000",
            "value": 763490,
            "unit": "ns"
          },
          {
            "name": "scan_walkdir/5000",
            "value": 3641200,
            "unit": "ns"
          },
          {
            "name": "scan_walkdir/10000",
            "value": 7295900,
            "unit": "ns"
          },
          {
            "name": "is_media_file_batch_8",
            "value": 763.2,
            "unit": "ns"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "w_sunshine163@163.com",
            "name": "halft0n",
            "username": "halft0n"
          },
          "committer": {
            "email": "w_sunshine163@163.com",
            "name": "halft0n",
            "username": "halft0n"
          },
          "distinct": true,
          "id": "198dadb1431e304e985b9de77794a6cd6b13bdf4",
          "message": "fix: use tauri::async_runtime::spawn in setup to avoid tokio panic\n\nThe setup closure runs before Tauri's tokio runtime context is\navailable on the current thread. Replace direct tokio::spawn calls\nwith tauri::async_runtime::spawn which guarantees a valid runtime.",
          "timestamp": "2026-07-02T08:34:31+08:00",
          "tree_id": "d33c784daca293f418b45e28f5dc42fd00d8e1a0",
          "url": "https://github.com/halft0n/LightFrame/commit/198dadb1431e304e985b9de77794a6cd6b13bdf4"
        },
        "date": 1782953055540,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "upsert_media_1000",
            "value": 148170000,
            "unit": "ns"
          },
          {
            "name": "search_media_fts_10k",
            "value": 9020400,
            "unit": "ns"
          },
          {
            "name": "compute_dhash",
            "value": 1256300,
            "unit": "ns"
          },
          {
            "name": "compute_phash",
            "value": 1541700,
            "unit": "ns"
          },
          {
            "name": "scan_walkdir/1000",
            "value": 731310,
            "unit": "ns"
          },
          {
            "name": "scan_walkdir/5000",
            "value": 3509200,
            "unit": "ns"
          },
          {
            "name": "scan_walkdir/10000",
            "value": 7209500,
            "unit": "ns"
          },
          {
            "name": "is_media_file_batch_8",
            "value": 745.63,
            "unit": "ns"
          }
        ]
      }
    ]
  }
}