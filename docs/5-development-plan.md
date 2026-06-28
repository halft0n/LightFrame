# CatchLight（拾光）详细开发计划

> **文档版本**：v1.1  
> **更新日期**：2026-06-28  
> **产品名称**：拾光 / CatchLight  
> **技术栈**：Tauri 2.x + Rust + React 19 + TypeScript + Python AI 扩展（可选）  
> **开发模式**：单人全职  
> **总工期估算**：24 周（Phase 0–4）  
> **关联文档**：[需求规格](./2-requirements.md) · [架构设计](./3-architecture.md) · [调研报告](./0-research-report.md) · [技术路线决策](./1-tech-stack-decision.md)

---

## 文档说明

本文档是 CatchLight 项目的**执行级开发计划**，在需求规格与架构设计基础上，给出开源方案评估、分阶段路线图、里程碑验收、风险应对、任务依赖与质量保证策略。工作量估算基于**单人全职开发**（每周约 35–40 有效工时），已计入联调、自测与文档同步时间。

---

## 1. 开源方案对比分析与二次开发评估

### 1.1 综合对比表

以下对比基于 CatchLight 需求规格（P0–P2 功能全集）与调研报告结论，评分采用 1–5 分制（5 为最优）。

| 维度 | iPhotron | Lap | FlyPhotos |
|------|----------|-----|-----------|
| **功能覆盖度** | ★★★★☆ **~70%** | ★★★☆☆ **~45%** | ★★☆☆☆ **~25%** |
| **技术栈契合度** | ★★☆☆☆ Python/Qt6，与 Tauri+Rust 差异大 | ★★★★★ Tauri+Rust，栈几乎一致（前端 Vue vs React） | ★★☆☆☆ .NET/WinUI3，仅 Windows |
| **许可证兼容性** | ★★★★★ MIT，可自由参考 | ★☆☆☆☆ **GPL-3.0**，代码不可直接复用 | ★★★☆☆ 需确认具体仓库许可（多为 MIT/Apache） |
| **代码质量与可维护性** | ★★★★☆ MVVM+DDD 分层清晰，Python 类型提示完善 | ★★★☆☆ 功能完整但存在 7300 行单文件模块 | ★★★★☆ 性能优化代码精炼，但 UI 层与 WinUI 强绑定 |
| **社区活跃度** | ★★★☆☆ ~184 Stars，更新中等 | ★★☆☆☆ 社区较小，Issues 响应慢 | ★★★☆☆ Windows 生态活跃，跨平台无 |
| **二次开发可行性** | ★★★☆☆ 功能多但需换栈重写 | ★★☆☆☆ GPL 限制 + 缺核心差异化功能 | ★★☆☆☆ 仅适合借鉴性能模式，无法 fork |

#### 功能覆盖度明细（相对 CatchLight 需求）

| 需求类别 | iPhotron | Lap | FlyPhotos |
|----------|----------|-----|-----------|
| macOS 照片风格 UI | ✅ | ⚠️ 实用主义 | ⚠️ 查看器导向 |
| 文件夹即图库 / 不复制 | ✅ | ✅ | ✅ |
| 极速索引 MFT/USN | ❌ | ❌ | ❌ |
| 时间线 / 地点 / 相簿 | ✅ | ⚠️ 部分 | ❌ |
| 精确/感知去重 | ❌ | ❌ | ❌ |
| 截图识别 / 分类 | ❌ | ❌ | ❌ |
| 相似照片 / CLIP | ❌ | ⚠️ 有 CLIP 搜索 | ❌ |
| 人脸识别 | ✅ | ❌ | ❌ |
| 视频支持 | ✅ | ✅ | ⚠️ 有限 |
| 跨平台 Win+Linux | ✅ | ✅ | ❌ 仅 Windows |

---

### 1.2 二次开发 vs 自研决策

#### 1.2.1 iPhotron

| 评估项 | 分析 |
|--------|------|
| **Fork 可行性** | 技术上可 fork（MIT），但技术栈为 Python + PySide6，与目标 Tauri + Rust + React **完全不兼容**，实质等于「参考产品设计 + 重写后端」 |
| **工作量估算** | 若坚持 Python 栈二次开发：补齐 MFT/USN、去重、截图识别约 **16–20 周**；若换栈则与自研相当 **22–24 周**，且无 Rust 性能收益 |
| **风险** | Python GIL 限制大规模并行哈希/缩略图；PyInstaller 打包体积大；10 万+ 库网格滚动需额外优化；现有扫描为 walkdir，非差异化能力 |
| **结论** | **不推荐 Fork**。作为 UX/功能设计的**参考蓝本**价值最高 |

#### 1.2.2 Lap

| 评估项 | 分析 |
|--------|------|
| **Fork 可行性** | 技术栈最接近（Tauri + Rust），但 **GPL-3.0** 要求衍生作品开源且传染性强，与商业/闭源分发目标冲突；直接 copy 代码存在法律风险 |
| **工作量估算** | 在 Lap 基础上补齐去重/截图/MFT/时间线/地图：约 **18–22 周**（含 GPL 合规重构若仅「参考思路」则 **+0 周**） |
| **风险** | 单文件巨石模块难维护；缺少 CatchLight 核心差异化；Vue → React 迁移成本高；社区支持弱，上游 merge 困难 |
| **结论** | **不推荐 Fork**。借鉴 IPC 分层、ProcessingBudget、thumb:// 协议、CLIP 管线等**设计模式**（洁净室实现） |

#### 1.2.3 FlyPhotos

| 评估项 | 分析 |
|--------|------|
| **Fork 可行性** | C# + WinUI 3，**无法跨平台**；架构与 Tauri 差异极大，仅 Windows 性能优化思路可移植 |
| **工作量估算** | 移植 Preview/HQ 双轨缓存、Burst 预取、多解码器 fallback 到 Rust：约 **3–4 周**（作为 Phase 3 性能优化子任务） |
| **风险** | Win2D/GPU 解码方案无法直接用于 Linux；需用 Rust `image` + 可选 GPU 解码重新实现 |
| **结论** | **不推荐 Fork**。作为**性能优化与查看器体验**的参考 |

#### 1.2.4 总体决策

```mermaid
flowchart LR
    A[CatchLight 目标] --> B{技术栈}
    B -->|Tauri+Rust+React| C[自研为主]
    C --> D[iPhotron: UX/功能参考]
    C --> E[Lap: 架构模式参考]
    C --> F[FlyPhotos: 性能模式参考]
    G[Fork 任一项目] --> H[❌ 不推荐]
```

**推荐路径：自研 + 洁净室借鉴**

- 自研 Tauri 2.x + Rust + React 19 代码库
- 从 iPhotron 借鉴产品功能与交互设计
- 从 Lap 借鉴 Tauri 后端架构模式（不复制 GPL 代码）
- 从 FlyPhotos 借鉴查看器性能优化策略

---

### 1.3 可复用模块清单（参考，非 Fork）

> 以下均为**设计思路与算法模式**的洁净室实现，不直接复制 GPL 或其他不兼容许可的源码。

#### 1.3.1 iPhotron

| 模块 | 可借鉴内容 | CatchLight 对应实现 |
|------|-----------|-------------------|
| **Folder-native 设计** | 监控文件夹 = 图库入口，JSON manifest 管理虚拟相簿 | `watched_folders` 表 + 侧边栏文件夹树 |
| **SQL keyset 分页** | 大库游标分页，避免 OFFSET 性能退化 | `catchlight-db` 中 `keyset_paginate(taken_at, id)` |
| **扫描-缩略图契约** | 扫描完成后触发缩略图队列，状态字段 `thumb_cached` | indexer → thumbnail 事件总线 |
| **MVVM 分层** | View / ViewModel / Service 清晰分离 | React 组件 + `appStore`（useSyncExternalStore）+ Tauri commands |

#### 1.3.2 Lap

| 模块 | 可借鉴内容 | CatchLight 对应实现 |
|------|-----------|-------------------|
| **Tauri IPC 分层** | commands 按域拆分，state 注入 | `src-tauri/src/commands/{index,media,album}.rs` |
| **ProcessingBudget** | 并发任务预算，防止 CPU/IO 过载 | `catchlight-core::task_scheduler` |
| **thumb:// 协议** | 自定义 URI 加载缩略图，前端 `<img src="thumb://...">` | `protocol.rs` + 三级缓存 |
| **CLIP 搜索管线** | ONNX 嵌入 + 向量索引 | `catchlight-ai` + `usearch` |
| **FFmpeg sidecar** | 外部 FFmpeg 进程生成视频缩略图/预览 | `catchlight-video` |

#### 1.3.3 FlyPhotos

| 模块 | 可借鉴内容 | CatchLight 对应实现 |
|------|-----------|-------------------|
| **Preview/HQ 双轨缓存** | 低分辨率即时显示，高分辨率后台升级 | micro/small 即时 + large/HQ 异步 |
| **Burst 预取** | 查看器切换时预加载相邻 N 张 | viewer 组件 ±3 张预取 |
| **多解码器 fallback** | JPEG → WebP → PNG → TIFF 解码链 | `image` crate + kamadak-exif + ExifTool |

---

## 2. 开发路线图

### 总览

| Phase | 名称 | 工期 | 累计 |
|-------|------|------|------|
| **Phase 0** | 项目初始化 | 1 周 | W1 |
| **Phase 1** | 核心基础 MVP | 7 周 | W2–W8 |
| **Phase 2** | 照片组织与智能功能 | 6 周 | W9–W14 |
| **Phase 3** | 增强体验 | 6 周 | W15–W20 |
| **Phase 4** | 打磨发布 | 4 周 | W21–W24 |

```mermaid
gantt
    title CatchLight 开发路线图（24 周）
    dateFormat YYYY-MM-DD
    axisFormat W%W

    section Phase 0
    脚手架与 CI           :p0, 2026-07-01, 7d

    section Phase 1
    索引引擎与 DB         :p1a, after p0, 14d
    元数据与缩略图        :p1b, after p1a, 14d
    网格与时间线 UI       :p1c, after p1b, 14d
    查看器与文件夹 UI     :p1d, after p1c, 7d

    section Phase 2
    去重系统              :p2a, after p1d, 14d
    截图识别与地点        :p2b, after p2a, 14d
    相簿搜索与地图        :p2c, after p2b, 14d

    section Phase 3
    AI 相似与人脸         :p3a, after p2c, 14d
    智能相簿与回忆        :p3b, after p3a, 14d
    批量操作与主题视频    :p3c, after p3b, 14d

    section Phase 4
    性能压测与打包        :p4, after p3c, 28d
```

---

### Phase 0：项目初始化（1 周）

**目标**：可编译、可 CI、可协作的工程骨架。

| 天 | 任务 | 交付物 |
|----|------|--------|
| D1–D2 | Tauri 2.x + React 19 + TypeScript 脚手架；Vite 配置；TailwindCSS v4 + shadcn/ui 初始化 | 空壳应用可启动 |
| D2–D3 | Cargo workspace 多 crate 结构：`catchlight-core`、`catchlight-db`、`catchlight-indexer` 等占位 crate | `cargo build` 通过 |
| D3–D4 | GitHub Actions：Windows + Ubuntu 双平台 `cargo test` + `pnpm test` + `cargo clippy` | CI badge 可用 |
| D4–D5 | 自定义 i18n（`src/i18n/`）；`locales/zh-CN.json`、`locales/en.json`；localStorage + 浏览器语言检测 | 语言切换可用 |
| D5 | eslint + prettier + husky；rustfmt + clippy；CONTRIBUTING.md 代码规范摘要 | lint 钩子生效 |

**目录结构（目标）**：

```
CatchLight/
├── src/                    # React 前端
├── src-tauri/              # Tauri 主 crate
├── crates/
│   ├── catchlight-core/
│   ├── catchlight-db/
│   ├── catchlight-indexer/
│   ├── catchlight-metadata/
│   └── catchlight-thumbnail/
├── locales/
├── .github/workflows/
└── docs/
```

---

### Phase 1：核心基础 MVP（7 周）— ✅ 已实现

**目标**：用户可添加监控文件夹，极速/常规索引照片，网格与时间线浏览，全屏查看，缩略图流畅加载。

**实现摘要（2026-06）：**
- Linux walkdir 扫描 + `futures::stream` / `buffer_unordered` 有界并发流水线
- SQLite schema v1 + v2（`watched_folders.scan_status`）
- 三级缩略图：micro 64 JPEG BLOB / small 256 WebP / large 1024 WebP
- FFmpeg 视频帧抽取（best-effort）、规则层截图检测集成于扫描
- React 网格 + 时间线虚拟滚动、全屏查看器、文件夹管理 UI
- 自定义 i18n + `useSyncExternalStore` 全局 store

#### 第 1 周（W2）：文件索引引擎 — 抽象与 Linux 实现

| 任务 | 详情 |
|------|------|
| `FileIndexer` trait 定义 | `full_scan`、`watch_changes`、`FileChange` 事件 |
| Linux 实现 | `walkdir` 递归扫描 + `notify` inotify 监听 |
| 扩展名过滤 | 图片/视频扩展名白名单（对齐 FMT-xxx 需求） |
| 单元测试 | 临时目录 fixture，增删改事件验证 |

#### 第 2 周（W3）：Windows 索引 + SQLite Schema

| 任务 | 详情 |
|------|------|
| Windows NTFS | `ntfs-reader` MFT 全量扫描 + USN Journal 增量 |
| 降级路径 | 非 NTFS / 无管理员权限 → `walkdir` + `ReadDirectoryChangesW` |
| SQLite schema v1 | `media_files`、`watched_folders` 表；`refinery`/`sqlx migrate` |
| 索引写入 | 批量 INSERT OR REPLACE，事务批量提交 |

#### 第 3 周（W4）：EXIF 元数据提取

| 任务 | 详情 |
|------|------|
| kamadak-exif | JPEG/HEIF/PNG 原生解析 |
| ExifTool fallback | sidecar 进程，超时与并发限制 |
| 字段映射 | `taken_at`、`camera`、`GPS`、`width/height` 写入 DB |
| 后台队列 | 扫描完成后异步 EXIF 提取，emit 进度事件 |

#### 第 4 周（W5）：缩略图系统 + thumb:// 协议

| 任务 | 详情 |
|------|------|
| 三级缩略图 | micro 64×64、small 256×256、large 1024×1024，WebP 80% |
| 生成策略 | 优先 EXIF 嵌入缩略图 → 异步精确生成 |
| `thumb://` 协议 | Tauri custom protocol handler |
| LRU 内存缓存 | 热点 micro 缩略图内存缓存 |
| ProcessingBudget | 并发上限可配置（借鉴 Lap） |

#### 第 5 周（W6）：照片网格视图

| 任务 | 详情 |
|------|------|
| 虚拟滚动 | `@tanstack/react-virtual`，keyset 分页加载 |
| 懒加载 | 可视区 ±2 屏缓冲，`thumb://` 加载 |
| 网格项 UI | 占位符、收藏角标、视频时长、RAW 标签 |
| 多选基础 | 单击/Ctrl/Shift 选择（Phase 3 扩展批量操作） |

#### 第 6 周（W7）：时间线 + 全屏查看器

| 任务 | 详情 |
|------|------|
| 时间线分组 | 年→月→日 SQL 分组，sticky 标题 |
| 时间源优先级 | DateTimeOriginal > CreateDate > mtime |
| 全屏查看器 | 缩放/拖拽/旋转、左右切换、胶片条 |
| Preview/HQ 双轨 | 先 large 缩略图，后台加载原图（借鉴 FlyPhotos） |

#### 第 7 周（W8）：文件夹管理 UI + 联调

| 任务 | 详情 |
|------|------|
| 监控文件夹 CRUD | 设置页 + 拖拽添加 |
| 文件夹树浏览 | 侧边栏文件系统层级 |
| 状态栏 | 照片/视频计数、索引进度 |
| Phase 1 联调 | 5 万张照片实测；修复 P0 bug |
| i18n 补充 | 核心浏览流程中英翻译 |

---

### Phase 2：照片组织与智能功能（6 周）— ✅ 已实现

**目标**：去重、截图识别、地点分组、相簿、搜索、地图——形成差异化竞争力。

**实现摘要（2026-06）：**
- BLAKE3 精确去重 + DHash 感知去重（汉明距离阈值聚类），去重工具页 UI
- 截图识别（规则层）集成扫描流水线，截图专属视图
- rrgeo 离线反向地理编码（reverse_geocoder），地点按国家/城市分组视图
- 相簿 CRUD、收藏夹、软删除（30天自动清理），4 个新视图组件
- SQLite FTS5 全文搜索，同步触发器，顶部搜索栏 + 结果视图
- DB schema 升级到 v5（is_favorite, is_deleted, FTS5, 部分索引）

#### 第 8 周（W9）：精确去重（BLAKE3）

| 任务 | 详情 |
|------|------|
| 文件大小预筛 | 同 size 候选分组 |
| BLAKE3 并行哈希 | `rayon` 并行，结果写入 `file_hash` |
| 重复组 UI | 重复照片列表，保留/删除建议 |
| 进度与暂停 | 后台任务可取消 |

#### 第 9 周（W10）：感知去重（DHash/PHash）

| 任务 | 详情 |
|------|------|
| DHash/PHash 计算 | `img_hash` 或自实现 |
| 汉明距离聚类 | 阈值可配置（默认 DHash distance=0） |
| `duplicate_group_id` | 精确+感知统一分组展示 |
| 去重工具页 | 侧边栏「重复照片」入口 |

#### 第 10 周（W11）：截图自动识别

| 任务 | 详情 |
|------|------|
| 规则快筛 | 无 EXIF 相机、屏幕分辨率、宽高比 |
| 视觉特征 | 边缘方差、状态栏检测 |
| CLIP 可选层 | ONNX Runtime CPU，`media_type='screenshot'` |
| 截图相册 | 自动归入「截图」智能集合 |

#### 第 11 周（W12）：地点分组 + 地图视图

| 任务 | 详情 |
|------|------|
| rrgeo 反向编码 | GPS → 国家/城市 |
| DBSCAN 聚类 | 合并近距离坐标 |
| Leaflet 地图 | 照片钉点、聚合、点击跳转 |
| 地点侧边栏 | 国家→城市→地点三级 |

#### 第 12 周（W13）：相簿 + 收藏 + 最近删除

| 任务 | 详情 |
|------|------|
| 用户相簿 CRUD | `albums` + `album_files` 虚拟关联 |
| 收藏夹 | `is_favorite` 标记与筛选 |
| 最近删除 | `is_deleted` 软删除，30 天自动清理 |
| 拖拽加入相簿 | 网格多选 → 加入相簿 |

#### 第 13 周（W14）：全文搜索（FTS5）+ 联调

| 任务 | 详情 |
|------|------|
| FTS5 虚拟表 | `file_name`、`camera`、`country`、`city` |
| 搜索 UI | 顶部搜索栏，<50ms 响应 |
| 智能相簿基础 | SQL 动态查询（全部照片/视频/收藏/截图） |
| Phase 2 联调 | 10 万库去重/截图压测 |

---

### Phase 3：增强体验（6 周）

**目标**：AI 能力（含 Python 扩展层）、视频、人脸、回忆、批量操作、主题——对标 macOS 照片完整体验。

#### 第 14 周（W15）：Python AI 扩展框架 + 相似照片

| 任务 | 详情 |
|------|------|
| Python sidecar 框架 | Rust JSON-RPC 客户端（tokio 子进程管理、超时、重连） |
| catchlight-ai-py 骨架 | Python 包入口、模型管理器、JSON-RPC server |
| AI 安装引导 UI | Python 环境检测 + 一键安装提示（Settings → AI 扩展） |
| CLIP ViT-B/32 | Rust ONNX 基础搜索 + Python FAISS 高级语义搜索 |
| 向量索引 | `usearch` / HNSW 近似最近邻 |
| 相似分组 UI | 「相似照片」工具页 |

#### 第 15 周（W16）：截图分类 + 人脸检测

| 任务 | 详情 |
|------|------|
| 截图细分类 | Python CLIP 零样本分类（代码/聊天/文档/游戏） |
| InsightFace ONNX | Rust 基础人脸检测 + Python 高级聚类 |
| 人脸聚类 | Chinese Whispers（Python sidecar 实现） |
| 人物侧边栏 | 人物相册入口 |

#### 第 16 周（W17）：智能相簿 + 回忆功能

| 任务 | 详情 |
|------|------|
| 规则引擎 | 日期范围、地点、镜头、类型组合 |
| 回忆生成 | 按「日期+地点」聚类自动生成 |
| 回忆卡片 UI | 标题、封面、照片数 |
| 自动刷新 | 索引更新后增量生成 |

#### 第 17 周（W18）：批量操作

| 任务 | 详情 |
|------|------|
| 批量删除/移动 | 操作原始文件，确认对话框 |
| 批量加入相簿 | 多选 → 相簿 |
| 批量导出路径 | 复制到指定文件夹（非导入/导出图库） |
| 撤销提示 | 文件系统操作不可逆警告 |

#### 第 18 周（W19）：主题 + 视频播放器

| 任务 | 详情 |
|------|------|
| 深色/浅色主题 | Tailwind dark mode + 系统跟随 |
| FFmpeg sidecar | 视频缩略图、元数据 |
| 视频播放器 | 内嵌播放，基本控制条 |
| 视频网格角标 | 时长显示 |

#### 第 19 周（W20）：Phase 3 联调与 UX 打磨

| 任务 | 详情 |
|------|------|
| AI 功能集成测试 | CLIP/人脸模型加载失败降级 |
| 动画与过渡 | Framer Motion 页面切换 |
| 键盘快捷键 | 查看器/网格通用快捷键 |
| 无障碍 | Radix UI 焦点管理 |

---

### Phase 4：打磨发布（4 周）

**目标**：10 万+ 性能达标，双平台安装包，自动更新，Beta 发布。

#### 第 20 周（W21）：性能优化

| 任务 | 详情 |
|------|------|
| 10 万+ 压测 | 索引、滚动、搜索、去重基准 |
| 瓶颈分析 | `tracing` + 火焰图 |
| 优化项 | SQL 索引、缩略图队列优先级、Burst 预取 |
| 内存占用 | 空闲 < 150MB 目标 |

#### 第 21 周（W22）：打包与分发

| 任务 | 详情 |
|------|------|
| Windows | NSIS/MSI 安装包，文件关联 |
| Linux | AppImage + deb + rpm |
| 代码签名 | Windows Authenticode（可选） |
| 依赖文档 | FFmpeg、WebKitGTK 安装说明 |

#### 第 22 周（W23）：自动更新 + 文档

| 任务 | 详情 |
|------|------|
| Tauri updater | 更新通道、签名验证 |
| 用户文档 | 中/英双语 README + 使用指南 |
| 隐私说明 | 本地处理、无上传承诺 |
| CHANGELOG | 版本发布说明 |

#### 第 23 周（W24）：Beta 测试与发布

| 任务 | 详情 |
|------|------|
| Beta 招募 | GitHub Discussions / Issues 模板 |
| Bug 修复 | P0/P1 问题清零 |
| v0.1.0-beta 发布 | GitHub Release 自动化 |
| 反馈迭代计划 | v0.2 路线图 |

---

## 3. 里程碑定义

### Phase 0 里程碑：工程就绪

| 交付物 | 验收标准 |
|--------|----------|
| 可编译应用 | Windows + Linux CI 绿色通过 |
| 开发规范 | eslint/clippy 零 warning（可配置例外） |
| i18n 骨架 | 切换中/英 UI 无硬编码中文 |

### Phase 1 里程碑：可浏览 MVP — ✅ 已完成

| 交付物 | 验收标准 | 状态 |
|--------|----------|------|
| 索引引擎 | Linux walkdir 扫描；stream + buffer_unordered 并发处理 | ✅ |
| 数据库 | schema v1/v2 迁移；EXIF 提取；micro_thumb BLOB | ✅ |
| 缩略图 | 三级缓存（64 BLOB / 256 / 1024 WebP）；thumb:// 协议 | ✅ |
| 网格+时间线 | 虚拟滚动；时间线分页合并同日期组 | ✅ |
| 查看器 | 缩放/切换；相邻预加载 | ✅ |
| 文件夹管理 | 监控文件夹 CRUD；scan_status 进度 | ✅ |

**Phase 1 完成标志**：用户添加本地照片文件夹，无需导入即可网格/时间线浏览并全屏查看。 — **已达成**

### Phase 2 里程碑：智能整理 — ✅ 已完成

| 交付物 | 验收标准 | 状态 |
|--------|----------|------|
| 精确去重 | BLAKE3 哈希 + 重复组 UI；按 file_size 预筛 | ✅ |
| 感知去重 | DHash 汉明距离聚类；精确/感知统一展示 | ✅ |
| 截图识别 | 规则层集成扫描流水线；截图专属视图 | ✅ |
| 地点分组 | rrgeo 反向编码；国家/城市分组视图 | ✅ |
| 相簿/收藏/删除 | 相簿 CRUD、收藏夹、软删除 30 天自动清理 | ✅ |
| FTS5 搜索 | FTS5 虚拟表 + 同步触发器；顶部搜索栏 | ✅ |

**Phase 2 完成标志**：差异化功能（去重、截图、搜索）可用，形成与 iPhotron/Lap 的差异。 — **已达成**

### Phase 3 里程碑：完整体验

| 交付物 | 验收标准 |
|--------|----------|
| 相似照片 | CLIP 分组可用；无 GPU 可 CPU 运行（慢但可用） |
| 人脸/人物 | 检测率可接受；同一人基本聚类 |
| 智能相簿/回忆 | 规则创建相簿；回忆自动生成 ≥ 1 条（有 GPS+日期数据时） |
| 批量操作 | 多选批量删除/移动/加相簿 |
| 主题/视频 | 深/浅色切换；常见 MP4 可播放 |

**Phase 3 完成标志**：功能对标 macOS 照片核心能力（除编辑外）。

### Phase 4 里程碑：可发布 Beta

| 交付物 | 验收标准 |
|--------|----------|
| 性能 | 10 万+ 库各场景达到调研报告性能目标 |
| 安装包 | Win 安装成功；Linux 三种格式可运行 |
| 自动更新 | 测试通道可检测到新版本 |
| 文档 | 中英用户文档完整 |
| Beta | v0.1.0-beta 发布，已知 P0 bug = 0 |

---

## 4. 技术风险与应对

### Phase 0 风险

| 风险 | 影响 | 概率 | 缓解措施 |
|------|------|------|----------|
| Tauri 2.x API 变更 | 脚手架返工 | 中 | 锁定版本；关注 changelog |
| Linux CI WebKitGTK 依赖 | CI 失败 | 中 | 文档化 apt/dnf 依赖；Docker 镜像 |

### Phase 1 风险

| 风险 | 影响 | 概率 | 缓解措施 |
|------|------|------|----------|
| MFT 需管理员权限 | Windows 首次索引慢 | 高 | 常规扫描降级 + UI 提示提权收益 |
| USN Journal 缓冲区溢出 | 长时间未运行漏变更 | 中 | 启动时校验 + 定期全量补偿 |
| Linux inotify 上限 | 大库漏监听 | 中 | 文档提示 `max_user_watches`；定时轮询 |
| HEIF/RAW 解码失败 | 缩略图缺失 | 中 | ExifTool/libheif fallback；占位图 |
| 虚拟滚动 + thumb:// 内存泄漏 | 长时间卡顿 | 中 | LRU 限制；CI 内存 profiling |

### Phase 2 风险

| 风险 | 影响 | 概率 | 缓解措施 |
|------|------|------|----------|
| BLAKE3 大文件 IO 瓶颈 | 去重耗时超预期 | 中 |  size 预筛 + 并行度限制 |
| CLIP 模型体积 350MB | 用户拒绝下载 | 高 | AI 功能可选；规则层独立可用 |
| rrgeo 精度不足 | 地点分组粗糙 | 低 | 城市级足够；后续可换 open-geocode |
| FTS5 中文分词 | 中文搜索弱 | 中 | unicode61 tokenizer；后续 jieba 扩展 |

### Phase 3 风险

| 风险 | 影响 | 概率 | 缓解措施 |
|------|------|------|----------|
| CLIP CPU 推理慢 | 10 万库相似分组小时级 | 高 | 后台低优先级；仅对小库或抽样 |
| InsightFace 误检 | 人物分组混乱 | 中 | 置信度阈值；手动合并/拆分（v0.2） |
| FFmpeg 未安装 | 视频功能不可用 | 中 | 启动检测 + 安装引导 |
| Linux WebKit 渲染差异 | UI 布局问题 | 中 | Linux 专项 CI screenshot 对比 |

### Phase 4 风险

| 风险 | 影响 | 概率 | 缓解措施 |
|------|------|------|----------|
| 10 万库性能不达标 | 发布延期 | 中 | Phase 1 末提前 5 万库压测；预留 W21 优化 |
| Linux 发行版碎片化 | 支持成本高 | 高 | 优先 AppImage；deb/rpm 次之 |
| 代码签名成本 | Windows 安装警告 | 低 | 文档说明；后续申请证书 |

---

## 5. 任务依赖关系图

### 5.1 关键路径（Mermaid）

```mermaid
flowchart TB
    P0[Phase 0 脚手架] --> IDX[索引引擎]
    P0 --> DB[(SQLite Schema)]
    P0 --> I18N[i18n 框架]

    IDX --> EXIF[EXIF 提取]
    DB --> EXIF
    EXIF --> THUMB[缩略图 + thumb://]
    THUMB --> GRID[网格虚拟滚动]
    EXIF --> TIMELINE[时间线分组]

    GRID --> VIEWER[全屏查看器]
    TIMELINE --> VIEWER
    IDX --> FOLDER[文件夹管理 UI]

    VIEWER --> P1M[Phase 1 MVP ✓ 已完成]
    FOLDER --> P1M

    P1M --> DEDUP1[BLAKE3 精确去重]
    DEDUP1 --> DEDUP2[DHash/PHash 感知去重]
    P1M --> SCREEN[截图识别]
    P1M --> GEO[GPS + rrgeo]
    GEO --> MAP[Leaflet 地图]

    P1M --> ALBUM[相簿/收藏/删除]
    ALBUM --> FTS[FTS5 搜索]
    DEDUP2 --> P2M[Phase 2 ✓]
    SCREEN --> P2M
    MAP --> P2M
    FTS --> P2M

    P2M --> CLIP[CLIP 相似分组]
    P2M --> FACE[人脸检测聚类]
    CLIP --> SMART[智能相簿/回忆]
    FACE --> SMART
    P2M --> BATCH[批量操作]
    P2M --> VIDEO[视频播放]
    SMART --> P3M[Phase 3 ✓]
    BATCH --> P3M
    VIDEO --> P3M

    P3M --> PERF[性能压测]
    PERF --> PKG[打包分发]
    PKG --> UPD[自动更新]
    UPD --> BETA[Beta 发布]
```

### 5.2 并行任务说明

| 可并行组合 | 前提 | 说明 |
|-----------|------|------|
| Linux 索引 ∥ Windows MFT | Phase 0 完成 | 不同平台实现，同 trait |
| EXIF 提取 ∥ 缩略图生成 | DB schema 就绪 | 共享队列，不同 worker |
| 网格 UI ∥ 时间线 UI | thumb:// 可用 | 共享 VirtualGrid 组件 |
| BLAKE3 去重 ∥ 截图规则层 | Phase 1 完成 | 无数据依赖 |
| 地图视图 ∥ 相簿 CRUD | GPS 数据入库 | Phase 2 中后段可并行 |
| 主题 ∥ 视频播放器 | Phase 2 完成 | Phase 3 独立模块 |

### 5.3 详细 Gantt（按周）

```mermaid
gantt
    title CatchLight 任务依赖（单人开发）
    dateFormat YYYY-MM-DD

    section Phase0
    脚手架 CI i18n lint     :done, ph0, 2026-07-01, 7d

    section Phase1-W2-W3
    FileIndexer trait        :p1t, after ph0, 3d
    Linux walkdir inotify    :p1l, after p1t, 4d
    SQLite schema migrate    :p1db, after p1t, 4d
    Windows MFT USN          :p1w, after p1l, 7d

    section Phase1-W4-W5
    EXIF kamadak ExifTool    :p1e, after p1w, 7d
    三级缩略图 thumb协议     :p1th, after p1e, 7d

    section Phase1-W6-W8
    虚拟滚动网格             :p1g, after p1th, 7d
    时间线分组               :p1tl, after p1g, 4d
    全屏查看器               :p1v, after p1g, 4d
    文件夹管理 UI            :p1f, after p1tl, 5d
    Phase1 联调              :p1i, after p1v, 3d

    section Phase2
    BLAKE3 精确去重          :p2d1, after p1i, 7d
    DHash PHash 感知去重     :p2d2, after p2d1, 7d
    截图识别                 :p2s, after p2d1, 7d
    GPS rrgeo 地图           :p2m, after p2d2, 7d
    相簿收藏删除             :p2a, after p2s, 7d
    FTS5 搜索联调            :p2f, after p2m, 7d

    section Phase3
    CLIP 相似分组            :p3c, after p2f, 7d
    人脸检测聚类             :p3f, after p2f, 7d
    智能相簿回忆             :p3m, after p3c, 7d
    批量操作                 :p3b, after p3f, 7d
    主题与视频               :p3v, after p3m, 7d
    Phase3 联调              :p3i, after p3b, 7d

    section Phase4
    性能压测优化             :p4p, after p3i, 7d
    Win Linux 打包           :p4k, after p4p, 7d
    自动更新与文档           :p4d, after p4k, 7d
    Beta 测试发布            :p4b, after p4d, 7d
```

---

## 6. 质量保证计划

### 6.1 测试金字塔

```mermaid
flowchart TB
    E2E[E2E 测试 · 少量关键路径]
    INT[集成测试 · IPC + DB + 索引]
    UNIT[单元测试 · Rust + React 主体]

    UNIT --> INT --> E2E
```

### 6.2 单元测试策略

#### Rust（`cargo test`）

| Crate | 测试重点 | 目标覆盖率 |
|-------|----------|-----------|
| `catchlight-indexer` | 扫描结果、事件解析、扩展名过滤 | 核心路径 80%+ |
| `catchlight-db` | 迁移、keyset 分页、FTS 查询 | 90%+ |
| `catchlight-metadata` | EXIF 解析 fixture（JPEG/HEIF/无 EXIF） | 85%+ |
| `catchlight-thumbnail` | 尺寸、格式、缓存 key | 80%+ |
| `catchlight-dedup` | BLAKE3、汉明距离聚类 | 90%+ |
| `catchlight-ai` | 模型 mock、嵌入维度 | 70%+ |

**Fixture 管理**：`crates/*/tests/fixtures/` 存放小样例图片；大样本（1 万+）仅 CI nightly 使用。

#### React（Vitest + Testing Library）

| 模块 | 测试重点 |
|------|----------|
| 虚拟滚动 | 渲染项数量、滚动位置恢复 |
| 时间线 | 分组标题格式化（中/英） |
| 查看器 | 键盘事件、缩放边界 |
| i18n | 关键 UI 字符串无缺失 key |
| Store | 选择状态、相簿操作 |

**运行命令**：

```bash
# Rust
cargo test --workspace
cargo test -p catchlight-indexer -- --nocapture

# 前端
pnpm test
pnpm test:coverage
```

### 6.3 集成测试

| 场景 | 方法 | 频率 |
|------|------|------|
| Tauri IPC | `tauri::test` mock app，invoke commands | 每 PR |
| 索引 → DB → 缩略图 | 临时目录端到端 pipeline | 每 PR |
| thumb:// 协议 | 请求 micro/small/large 返回有效 WebP | 每 PR |
| EXIF → 时间线 | 样本集 taken_at 正确分组 | 每周 |
| 去重 pipeline | 已知重复集识别率 | Phase 2 起每 PR |

### 6.4 性能测试基准

| 基准 | 目标 | 工具 | 频率 |
|------|------|------|------|
| MFT 扫描 10 万文件 | < 5s（Win NTFS） | criterion bench | Phase 1 起 |
| walkdir 10 万文件 | < 15s（Linux） | criterion bench | Phase 1 起 |
| 网格滚动 FPS | ≥ 55fps @ 1080p | 自定义 telemetry + Playwright | Phase 1 末 |
| BLAKE3 10 万张 | < 5min | criterion | Phase 2 |
| FTS5 查询 | p95 < 50ms | criterion | Phase 2 |
| 内存 idle | < 150MB | CI 采样 | Phase 4 |

**基准数据存储**：`benches/results/` JSON，CI 对比 regression > 10% 告警。

### 6.5 CI 自动化

```yaml
# .github/workflows/ci.yml 目标结构
jobs:
  rust:
    strategy:
      matrix:
        os: [ubuntu-latest, windows-latest]
    steps:
      - cargo fmt --check
      - cargo clippy -- -D warnings
      - cargo test --workspace
      - cargo bench --no-run  # 编译 bench

  frontend:
    steps:
      - pnpm lint
      - pnpm test
      - pnpm build

  e2e:  # Phase 1 末启用
    runs-on: ubuntu-latest
    steps:
      - xvfb-run pnpm test:e2e  # Tauri WebDriver
```

| CI 阶段 | 触发 | 阻断合并 |
|---------|------|----------|
| lint + unit test | PR / push main | 是 |
| 集成测试 | PR | 是 |
| bench regression | nightly | 告警 |
| Linux + Windows 构建 | PR | 是 |
| E2E | release 分支 | 是 |

### 6.6 人工测试清单（每 Phase 末）

- [ ] 添加/删除监控文件夹，索引正确
- [ ] 10 万库网格滚动无明显卡顿
- [ ] 中英语言切换完整
- [ ] 查看器 EXIF 信息面板正确
- [ ] 去重/截图/搜索结果符合预期
- [ ] 删除照片进入「最近删除」，30 天逻辑可模拟
- [ ] Windows 无管理员 / 有管理员两种索引路径
- [ ] Ubuntu 22.04 + Fedora 39 实机验证

---

## 7. 附录

### 7.1 技术依赖与许可证

详见调研报告 §7.3 与架构文档。核心依赖均为 MIT/Apache/BSD 兼容许可；FFmpeg 以 sidecar 形式 LGPL 隔离；AI 模型权重单独分发。

### 7.2 版本规划

| 版本 | 范围 | 说明 |
|------|------|------|
| v0.1.0-beta | Phase 0–4 完成 | 首次公开测试 |
| v0.2.0 | Beta 反馈 + 人脸合并/拆分 | 体验修复 |
| v0.3.0 | RAW 预览增强、GPU 加速 | 性能增强 |
| v1.0.0 | 稳定版 | 功能完整、性能达标 |

### 7.3 文档维护

| 文档 | 更新时机 |
|------|----------|
| `requirements.md` | 需求变更时 |
| `architecture.md` | 架构决策变更时 |
| `development-plan.md` | 每 Phase 结束复盘更新 |

---

> **拾光 / CatchLight** — 拾一束光，留一段时光。  
> Catch the light, keep the moment.
