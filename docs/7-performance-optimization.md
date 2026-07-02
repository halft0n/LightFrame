# 性能优化方案

## 问题现状

当前扫描和缩略图生成流程在大文件库（尤其 Windows）上表现慢、CPU 利用率低。
虽然 Phase 1 和 Phase 2 已使用 `buffer_unordered(concurrency)` 做文件级并发，
但存在以下瓶颈使得并行效果大打折扣。

---

## 瓶颈清单

### 瓶颈 A：Enrichment 单文件内部多段串行 blocking（高）

**文件**: `src-tauri/src/scan.rs` 行 597–641

当前对每个文件分两次 `spawn_blocking`：
1. BLAKE3 全文件哈希 → await
2. 解码 + dhash/phash + micro/small 缩略图 → await

两次 await 之间有上下文切换开销，blocking 线程池利用率不足。

### 瓶颈 B：Phase 1 同步 DB 操作阻塞 async worker（高）

**文件**: `src-tauri/src/scan.rs` 行 513, 558, 572

`get_media_by_path`、`upsert_media`、`update_media_location` 在 async 任务中同步执行 SQLite，
高并发 `buffer_unordered` 时多个 task 争抢 DB 连接。

### 瓶颈 C：缩略图全量重建完全串行（高）

**文件**: `src-tauri/src/thumb_regen.rs` 行 217–225

`regenerate_all_thumbnails` 用 `for` 循环逐个处理，`std::thread::spawn` + `.join()` 同步等待，
无并行，CPU 利用率极低。

### 瓶颈 D：AI 截图检测在 async 线程同步执行（高）

**文件**: `src-tauri/src/scan.rs` 行 698–714

`detect_screenshot` 和 `classify_screenshot` 未用 `spawn_blocking`，
可能阻塞 tokio worker 线程，降低 Phase 2 有效并发。

### 瓶颈 E：无 Semaphore 分级预算（高）

**文件**: `src-tauri/src/state.rs` 行 216–220

当前 Phase 1 和 Phase 2 共用同一个 `concurrency` 值作为 `buffer_unordered` 上限，
无法区分轻量任务（metadata）和重量任务（RAW 解码、视频帧提取）。

### 瓶颈 F：视频每文件启动 ffmpeg 子进程串行（中）

**文件**: `src-tauri/src/scan.rs` 行 652–679

ffmpeg 进程启动开销大，每个文件内 `extract_frame` → `spawn_blocking` 串行执行，
与图片 decode 共享同一 `concurrency` 槽位。

### 瓶颈 G：BLAKE3 单线程、无 mmap（中）

**文件**: `crates/lightframe-dedup/src/exact.rs` 行 6–19

大文件哈希无法利用多核。blake3 crate 本身支持多线程和 mmap，但未启用。

### 瓶颈 H：`DecodedImage::to_dynamic_image` 全量 clone（中）

**文件**: `crates/lightframe-core/src/media.rs`（`to_dynamic_image` 函数）

解码后 RGBA 数据 clone 一次转为 `DynamicImage`，大图时内存带宽和 CPU 双倍开销。

### 瓶颈 I：Phase 1 逆地理编码 per-file（中）

**文件**: `src-tauri/src/scan.rs` 行 562–575

带 GPS 的大库在 Phase 1 产生大量 `spawn_blocking` 调用，与索引竞争线程池。

### 瓶颈 J：thumb_protocol 同步磁盘读 + DB 查询（中）

**文件**: `src-tauri/src/thumb_protocol.rs`

Tauri 自定义协议 handler 中同步 I/O，高滚动速度时可能卡顿（虽有 LRU 缓解）。

### 瓶颈 K：文件夹扫描队列串行（低）

**文件**: `src-tauri/src/scan.rs` 行 169–185

`ScanQueue` 一次只处理一个文件夹，多文件夹同时添加时无法并行扫描。

### 瓶颈 L：Windows MFT 扫描未实现（低）

**文件**: `crates/lightframe-indexer/src/mft.rs`

当前 placeholder 返回空，walkdir 在大 NTFS 卷上较慢。

### 瓶颈 M：HEIC 缩略图不支持（低）

**文件**: `crates/lightframe-thumbnail/src/lib.rs`

HEIC 文件可索引但缩略图生成时跳过。

---

## 优化方案

### 优化 1：合并 Enrichment blocking 任务（高优先级）✅ 已完成

**目标**：将 BLAKE3 + 图片解码 + dhash/phash + 缩略图生成合并为一次 `spawn_blocking`。

**当前代码**（`scan.rs` 行 597–641）：
```rust
// 第一次 blocking: BLAKE3
let blake3_hash = tokio::task::spawn_blocking({ ... file_hash ... }).await??;
// 第二次 blocking: decode + thumb
let (dhash, phash, micro_blob) = tokio::task::spawn_blocking({ ... decode_image + thumb ... }).await?;
```

**优化方案**：
```rust
let result = tokio::task::spawn_blocking({
    let path = path.clone();
    move || -> EnrichBlockingResult {
        // 1. BLAKE3 哈希
        let blake3_hash = lightframe_dedup::file_hash(&path)?;

        // 2. 图片解码 + dhash/phash + 缩略图（仅图片）
        let (dhash, phash, micro_blob) = if is_image {
            let decoded = lightframe_core::decode::decode_image(&path)?;
            let dhash = lightframe_dedup::dhash_from_decoded(&decoded).ok();
            let phash = lightframe_dedup::phash_from_decoded(&decoded).ok();
            let _ = lightframe_thumbnail::generate_from_decoded(&decoded, &blake3_hash, Micro);
            let _ = lightframe_thumbnail::generate_from_decoded(&decoded, &blake3_hash, Small);
            let micro = lightframe_thumbnail::micro_blob_from_decoded(&decoded).ok();
            (dhash, phash, micro)
        } else {
            (None, None, None)
        };

        Ok(EnrichBlockingResult { blake3_hash, dhash, phash, micro_blob })
    }
}).await??;
```

**收益**：减少一次 async/blocking 上下文切换，blocking 线程池持续被利用。
预计图片 enrichment 单文件耗时降低 10-20%。

---

### 优化 2：Phase 1 DB 操作移入批量写队列（高优先级）✅ 已完成

**目标**：将 `upsert_media` 和 `update_media_location` 改为通过 channel 发送到专用写线程。

**当前代码**（`scan.rs` 行 558, 572）：
```rust
let media_id = db.upsert_media(folder_id, &media)?;  // 同步 DB 写
let _ = db.update_media_location(media_id, city, country);  // 同步 DB 写
```

**优化方案**：
```rust
// 启动前创建写队列
let (p1_write_tx, mut p1_write_rx) = tokio::sync::mpsc::channel::<Phase1WriteOp>(256);

// 专用写线程
let p1_writer = tokio::spawn(async move {
    let mut batch: Vec<Phase1WriteOp> = Vec::with_capacity(50);
    while let Some(op) = p1_write_rx.recv().await {
        batch.push(op);
        // 攒批或 channel 清空时写入
        while let Ok(more) = p1_write_rx.try_recv() {
            batch.push(more);
            if batch.len() >= 50 { break; }
        }
        db_batch.batch_upsert_media(&batch)?;
        batch.clear();
    }
});

// Phase 1 async 任务中：
p1_write_tx.send(Phase1WriteOp { folder_id, media, geo }).await;
```

**注意**：需要重构 `quick_index_inner` 的返回值，因为 `media_id` 不再同步返回。
可考虑使用 `oneshot` channel 回传 ID，或在 Phase 1 结束后重新从 DB 查询。

**收益**：消除 async worker 中的 DB 锁争抢，Phase 1 吞吐量可提升 2-3 倍。

---

### 优化 3：缩略图全量重建并行化（高优先级）✅ 已完成

**目标**：将串行 `for` 循环改为 `buffer_unordered` + `spawn_blocking`。

**当前代码**（`thumb_regen.rs` 行 207–229）：
```rust
while offset < total {
    let batch = state.db.get_all_media(PAGE_SIZE, offset)?;
    for media in batch {
        match regenerate_thumbnails_for_media(state, media.id) { ... }
        emit(processed, regenerated, "running");
    }
    offset += PAGE_SIZE;
}
```

**优化方案**：
```rust
let concurrency = state.scan_concurrency;
let all_ids: Vec<i64> = collect_all_media_ids(&state.db)?;
let total = all_ids.len() as i64;

stream::iter(all_ids.into_iter().map(|media_id| {
    let db = Arc::clone(&state.db);
    let thumb_cache = Arc::clone(&state.thumb_cache);
    async move {
        tokio::task::spawn_blocking(move || {
            regenerate_thumbnails_for_media_db(&db, media_id)
        }).await
    }
}))
.buffer_unordered(concurrency)
.for_each(|result| {
    // 更新 processed/regenerated 计数器，throttle emit
    async { ... }
})
.await;
```

**收益**：CPU 利用率从单核提升至多核并行，大库重建速度提升 `N` 倍（`N` = CPU 核数 × 0.7）。

---

### 优化 4：AI 截图检测移入 spawn_blocking（高优先级）✅ 已完成

**目标**：将 `detect_screenshot` 和 `classify_screenshot` 包裹在 `spawn_blocking` 中。

**当前代码**（`scan.rs` 行 698–714）：
```rust
if lightframe_ai::detect_screenshot(&path, w, h)
    .map(|s| s.is_likely_screenshot())
    .unwrap_or(false)
{
    let _ = db.set_media_type(media_id, "Screenshot");
    if let Ok(st) = lightframe_ai::classify_screenshot(&path) { ... }
}
```

**优化方案**：
```rust
let (is_screenshot, screenshot_type) = tokio::task::spawn_blocking({
    let path = path.clone();
    move || {
        let is_ss = if matches!(mt, MediaType::Photo) {
            lightframe_ai::detect_screenshot(&path, w, h)
                .map(|s| s.is_likely_screenshot())
                .unwrap_or(false)
        } else {
            true // MediaType::Screenshot 已确认
        };
        let st = if is_ss || matches!(mt, MediaType::Screenshot) {
            lightframe_ai::classify_screenshot(&path).ok()
        } else {
            None
        };
        (is_ss, st)
    }
}).await.map_err(|e| lightframe_core::Error::Other(e.to_string()))?;

if is_screenshot {
    let _ = db.set_media_type(media_id, "Screenshot");
}
if let Some(st) = screenshot_type {
    let _ = db.set_screenshot_type(media_id, st.label());
}
```

**收益**：避免阻塞 tokio async worker，Phase 2 有效并发不再被 AI 推理拖累。

---

### 优化 5：实现 Semaphore 分级预算（高优先级）✅ 已完成

**目标**：用 `tokio::sync::Semaphore` 区分轻量/重量任务，避免资源争抢。

**当前代码**（`state.rs` 行 216–220）：
```rust
let concurrency = ((cpus as f64) * 0.7).ceil() as usize;
let concurrency = concurrency.clamp(2, 16);
```

**优化方案**：

在 `AppState` 中添加：
```rust
pub struct ProcessingBudget {
    pub light: Arc<Semaphore>,   // metadata, EXIF, DB 操作
    pub heavy: Arc<Semaphore>,   // 图片解码, 缩略图, dhash/phash
    pub video: Arc<Semaphore>,   // ffmpeg 帧提取（进程数受限）
}

impl ProcessingBudget {
    pub fn new(cpus: usize) -> Self {
        let light_permits = cpus.clamp(4, 32);       // 轻量任务多放
        let heavy_permits = (cpus * 3 / 4).clamp(2, 12);  // CPU 密集
        let video_permits = (cpus / 2).clamp(1, 4);  // ffmpeg 数量受限
        Self {
            light: Arc::new(Semaphore::new(light_permits)),
            heavy: Arc::new(Semaphore::new(heavy_permits)),
            video: Arc::new(Semaphore::new(video_permits)),
        }
    }
}
```

**使用方式**：
```rust
// Phase 2 enrichment 中
let _permit = budget.heavy.acquire().await.unwrap();
let result = tokio::task::spawn_blocking(|| {
    // BLAKE3 + decode + thumb ...
}).await;
drop(_permit);

// 视频帧提取
let _permit = budget.video.acquire().await.unwrap();
lightframe_video::extract_frame(&path, &frame, 1.0).await;
drop(_permit);
```

**收益**：轻重任务不再互相阻塞，视频 ffmpeg 进程数可控，整体吞吐更均衡。

---

### 优化 6：视频缩略图 ffmpeg 进程控制（中优先级）

**目标**：限制同时运行的 ffmpeg 进程数量，考虑进程复用。

**方案**：通过优化 5 的 `video` Semaphore 自然限流。
进一步可考虑批量 `concat demuxer` 减少进程启动次数。

---

### 优化 7：BLAKE3 启用多线程/mmap（中优先级）✅ 已完成

**目标**：利用 blake3 crate 内置的多线程和 mmap 能力。

**当前代码**（`exact.rs`）：
```rust
let mut hasher = blake3::Hasher::new();
let mut buf = vec![0u8; 128 * 1024];
loop {
    let n = file.read(&mut buf)?;
    if n == 0 { break; }
    hasher.update(&buf[..n]);
}
```

**优化方案**：
```rust
pub fn blake3_hash(path: &Path) -> Result<String> {
    let file = File::open(path)?;
    let metadata = file.metadata()?;

    if metadata.len() > 4 * 1024 * 1024 {
        // 大文件：使用 mmap + rayon 多线程
        let mmap = unsafe { memmap2::Mmap::map(&file)? };
        let hash = blake3::Hasher::new().update_rayon(&mmap).finalize();
        Ok(hash.to_hex().to_string())
    } else {
        // 小文件：传统流式读取
        let mut hasher = blake3::Hasher::new();
        let mut buf = vec![0u8; 128 * 1024];
        let mut reader = std::io::BufReader::new(file);
        loop {
            let n = reader.read(&mut buf)?;
            if n == 0 { break; }
            hasher.update(&buf[..n]);
        }
        Ok(hasher.finalize().to_hex().to_string())
    }
}
```

**依赖**：需添加 `memmap2` 和 `blake3` 的 `rayon` feature。

**收益**：大文件（RAW/视频）哈希速度可提升 2-4 倍。

---

### 优化 8：避免 DecodedImage clone（中优先级）✅ 已完成

**目标**：使用 `into_raw()` 转移所有权，避免 RGBA 数据 clone。

**方案**：修改 `to_dynamic_image` 接收 `self`（move）而非 `&self`：
```rust
pub fn into_dynamic_image(self) -> Result<image::DynamicImage> {
    let img = image::RgbaImage::from_raw(self.width, self.height, self.rgba)
        .ok_or(Error::InvalidDimensions)?;
    Ok(image::DynamicImage::ImageRgba8(img))
}
```

**收益**：大图解码后避免一次全量内存拷贝。

---

### 优化 9：逆地理编码延后到 Phase 2（中优先级）✅ 已完成

**目标**：从 Phase 1 移出逆地理编码，放到 Phase 2 或独立批处理。

**方案**：Phase 1 仅存储 `latitude`/`longitude`，Phase 2 批量反查地理位置。

**收益**：Phase 1 更快完成首屏展示，减少 blocking 线程池争抢。

---

### 优化 10：thumb_protocol 异步化（中优先级）✅ 已完成

**目标**：将 DB 查询和磁盘读改为非阻塞。

**实际方案**：Tauri `register_uri_scheme_protocol` 已在线程池运行 handler，
真正的瓶颈是每次请求都要查 `list_watched_folders`。新增 `WatchedFoldersCache`
（5 秒 TTL），三个协议 handler（thumb/face/original）共享缓存，
在 `add_watched_folder`/`remove_watched_folder` 时失效。

---

### 优化 11：文件夹并行扫描（低优先级）

**目标**：`ScanQueue` 支持有限并行（如 2 个 folder 同时扫描）。

**方案**：改为 `Arc<Semaphore>` 控制并发，共享全局 `ProcessingBudget`。

---

### 优化 12：Windows MFT 扫描（低优先级）

**目标**：在 Windows 上用 MFT 加速文件发现。

**方案**：实现 `mft.rs` 中的 MFT 读取逻辑，回退到 walkdir 作为 fallback。

---

### 优化 13：HEIC 缩略图支持（低优先级）

**目标**：使用 `libheif` 解码 HEIC 文件。

**方案**：添加 `libheif-rs` 依赖，在 `decode_image` 中增加 HEIC 分支。

---

## 优先级实施顺序

| 阶段 | 优化项 | 预期收益 | 工作量 |
|------|--------|---------|--------|
| **Phase A** | 1. 合并 blocking | 减少上下文切换 10-20% | 小 |
| **Phase A** | 4. AI 检测 spawn_blocking | 释放 async worker | 小 |
| **Phase A** | 3. 缩略图重建并行化 | N 倍提速 | 中 |
| **Phase B** | 5. Semaphore 分级 | 资源隔离、吞吐均衡 | 中 |
| **Phase B** | 2. Phase 1 DB 批量写 | Phase 1 吞吐 2-3x | 大 |
| **Phase C** | 7. BLAKE3 mmap+rayon | 大文件哈希 2-4x | 小 |
| **Phase C** | 8. 避免 clone | 减少内存拷贝 | 小 |
| **Phase C** | 9. 逆地理延后 | Phase 1 首屏更快 | 中 |
| **Phase D** | 6. ffmpeg 限流 | 通过 Semaphore 实现 | 已含 |
| **Phase D** | 10-13. 其余优化 | 各类改善 | 大 |
