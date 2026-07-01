window.BENCHMARK_DATA = {
  "lastUpdate": 1782905640828,
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
      }
    ]
  }
}