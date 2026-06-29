# 照片查看软件调研与技术方案报告

> **项目名称：影迹 / LightFrame**
> **调研日期：2026-06-28**
> **状态：技术方案探索阶段**

---

## 一、项目概述

### 1.1 需求摘要

开发一款**跨平台（Windows + Linux）**的照片查看与管理软件，核心定位：

| 维度 | 描述 |
|------|------|
| **界面风格** | 对标 macOS 照片应用，支持时间线、地点、相簿等多维度浏览 |
| **核心差异** | 不做导入/导出（不复制文件），采用类似 Everything/WizTree 的快速索引方式，就地查看 |
| **智能功能** | 相同照片去重、相似照片识别、截图照片自动识别与分类 |
| **用户体验** | 自定义相簿、收藏夹、智能相簿、回忆等核心照片管理能力 |
| **多语言** | 至少支持简体中文和英文，架构上预留多语言扩展能力 |
| **跨平台** | Windows 10/11 + Linux (Ubuntu/Debian/Fedora 等主流发行版) |

### 1.2 应用命名

#### 应用名称（已确定）

| 属性 | 说明 |
|------|------|
| **中文名** | **影迹** |
| **英文名** | **LightFrame** |
| **含义** | 中文"影迹"指光影在时间中留下的痕迹；英文 Light + Frame = 光影 + 帧，强调每张照片都是时间的印迹 |
| **特点** | 诗意独特，有中国文化底蕴；LightFrame 体现每帧光影与时间印迹的产品理念 |
| **中文 Slogan** | "每一帧光影，都是时间的印迹" |
| **英文 Slogan** | "Every frame of light is a trace of time." |

---

## 二、现有开源方案调研

### 2.1 全面对标方案

#### 2.1.1 iPhotron — 最接近需求的现有方案

| 属性 | 详情 |
|------|------|
| **GitHub** | [OliverZhaohaibin/iPhotron](https://github.com/oliverzhaohaibin/iphotron-localphotoalbummanager) |
| **Stars** | 184 ⭐ |
| **许可证** | MIT |
| **技术栈** | Python + PySide6 (Qt6) + SQLite + ExifTool + FFmpeg |
| **架构** | MVVM + DDD 分层架构 |

**与需求对照：**

| 需求项 | iPhotron 支持情况 | 差距 |
|--------|------------------|------|
| macOS 照片风格 UI | ✅ 高度还原 | — |
| 文件夹即相簿 | ✅ Folder-native | — |
| 不复制文件 | ✅ 就地索引 | — |
| 时间线浏览 | ✅ 支持 | — |
| 地图/地点浏览 | ✅ GPS + 离线地图 | — |
| 自定义相簿 | ✅ JSON manifest | — |
| 智能相簿 | ✅ 全部照片/视频/收藏 | — |
| Live Photo | ✅ HEIC/MOV 配对 | — |
| 人脸识别 | ✅ InsightFace/ONNX | — |
| **照片去重** | ❌ 不支持 | **需新增** |
| **相似照片识别** | ❌ 不支持 | **需新增** |
| **截图识别** | ❌ 不支持 | **需新增** |
| **快速文件索引** | ⚠️ 常规扫描 | **需用 MFT/USN 加速** |
| EXIF 元数据 | ✅ 完整支持 | — |
| 视频支持 | ✅ FFmpeg | — |

**评估结论：** iPhotron 是目前最接近需求的开源方案，覆盖了约 70% 的功能需求。但在核心差异化功能（去重、截图识别、极速索引）上有明显空缺。其 Python + Qt6 技术栈在性能上也存在瓶颈。

#### 2.1.2 Lap — 大规模库优化

| 属性 | 详情 |
|------|------|
| **GitHub** | [julyx10/lap](https://github.com/julyx10/lap) |
| **定位** | 面向大型本地库的私有照片管理器 |
| **技术特点** | 本地优先、100K+ 文件优化、文件夹优先工作流 |
| **差距** | 缺少去重/截图识别；社区较小 |

#### 2.1.3 digiKam — 专业级全功能

| 属性 | 详情 |
|------|------|
| **官网** | [digikam.org](https://www.digikam.org/) |
| **成熟度** | 极高（持续开发 20+ 年）|
| **技术栈** | C++ / Qt / KDE |
| **功能** | 人脸识别、RAW 处理、版本追踪、标签、地理定位 |
| **差距** | 界面复杂，非 macOS 照片风格；需导入到数据库 |

#### 2.1.4 Recasa — 自托管照片管理

| 属性 | 详情 |
|------|------|
| **GitHub** | [yonie/recasa](https://github.com/yonie/recasa) |
| **亮点** | 时间线、人物、事件、地图、AI 标签、Live Photo |
| **去重** | ✅ 感知哈希去重 |
| **差距** | 自托管 Web 应用，非桌面原生；需 Docker 部署 |

### 2.2 专项功能方案

#### 2.2.1 高性能照片查看器

| 项目 | 技术栈 | 特点 | Stars |
|------|--------|------|-------|
| **HyperBrowse** | C++20 / Win32 / Direct2D | 原生 Windows，异步加载，GPU JPEG 解码 | 新项目 |
| **SimpleImageViewer** | Rust / egui / GPU | 递归扫描，背景预加载，跨平台 | 新项目 |
| **FlyPhotos** | C# / WinUI 3 / Win2D | 原生 AOT，极速启动，Windows 10/11 | 活跃 |
| **μPhotos** | C# / .NET 9 / Skia | 100K 照片 60fps 滚动，GPU 加速 | 开源 |

#### 2.2.2 照片去重方案

| 方案 | 语言 | 算法 | 性能 | 适合场景 |
|------|------|------|------|----------|
| **imagededup** | Python | PHash/DHash/AHash/WHash + CNN | 中等 | 精确去重 + 近似去重 |
| **imgfprint-rs** | Rust | AHash/DHash/PHash + BLAKE3 | 极高 | 大规模高速处理 |
| **Perception** | Python | 多种哈希 + Facebook PDQ | 中等 | 研究和生产环境 |

**去重算法性能对比（基于 imagededup 基准测试）：**

| 算法 | 精确去重准确率 | 近似去重准确率 | 速度 | 推荐用途 |
|------|--------------|--------------|------|---------|
| DHash (distance=0) | 99.9% | 低 | **最快** | 精确去重首选 |
| PHash (distance=10) | 99.9% | 中等 | 快 | 平衡方案 |
| CNN (threshold=0.9) | 99.9% | **95%+** | 慢(需GPU) | 近似去重首选 |

#### 2.2.3 截图识别方案

| 方案 | 技术 | 准确率 | 特点 |
|------|------|--------|------|
| **CONAN** | Markov图像 + CNN | **99.8%** | 最先进，抗压缩/缩放 |
| **启发式规则** | EXIF分析+边缘检测+莫尔纹检测 | ~90% | 轻量，无需深度学习 |
| **CLIP分类** | CLIP (ViT-B/32) + ONNX | ~95% | 可扩展多种分类 |
| **Snapy** | CLIP + OCR (Tesseract) | ~95% | 分类 + 文字提取 |

**推荐方案：** 多层检测策略
1. **第一层（快速筛选）**：启发式规则 — 检查 EXIF 中的设备信息（截图通常无相机信息）、图片分辨率是否匹配常见屏幕分辨率、宽高比等
2. **第二层（精确分类）**：CLIP 模型 — 对第一层疑似截图进行视觉内容分类
3. **第三层（可选增强）**：CONAN 方法 — 对极端情况进行字节级特征分析

---

## 三、核心技术方案

### 3.1 技术栈选型

#### 3.1.1 应用框架对比

| 框架 | 语言 | 安装包 | 内存占用(空闲) | 启动时间 | 渲染一致性 | 生态 |
|------|------|--------|--------------|---------|-----------|------|
| **Tauri 2.x** | Rust + Web | 5-15 MB | 30-80 MB | 0.2-0.8s | 中(依赖OS) | 快速增长 |
| **Electron** | Node.js + Web | 80-200 MB | 200-400 MB | 1-3s | 高(Chromium) | 庞大 |
| **Qt6 (C++)** | C++ | 20-40 MB | 50-100 MB | 0.3-0.5s | 高 | 成熟 |
| **PySide6 (Qt6)** | Python | 30-60 MB | 80-150 MB | 0.5-1s | 高 | 中等 |
| **WinUI 3** | C#/.NET | 15-30 MB | 60-120 MB | 0.3-0.8s | 高(仅Windows) | Windows原生 |
| **.NET + Avalonia** | C# | 15-25 MB | 50-100 MB | 0.3-0.6s | 高 | 成长中 |

#### 3.1.2 推荐技术栈

**方案 A（推荐）：Tauri 2.x + Rust + React/Vue**

```
┌─────────────────────────────────────────┐
│              前端 (WebView2)              │
│   React/Vue + TailwindCSS + Framer      │
│   虚拟滚动 + 图片懒加载 + 动画          │
├─────────────────────────────────────────┤
│              Tauri IPC                    │
├─────────────────────────────────────────┤
│              后端 (Rust)                  │
│  ┌──────────┐ ┌──────────┐ ┌──────────┐│
│  │文件索引引擎│ │元数据提取 │ │图像处理  ││
│  │MFT/USN   │ │EXIF/GPS  │ │缩略图生成││
│  └──────────┘ └──────────┘ └──────────┘│
│  ┌──────────┐ ┌──────────┐ ┌──────────┐│
│  │去重引擎   │ │截图检测  │ │反向地理  ││
│  │Hash+CNN  │ │CLIP/规则 │ │编码      ││
│  └──────────┘ └──────────┘ └──────────┘│
├─────────────────────────────────────────┤
│           SQLite + FTS5                  │
│       全局索引 + 全文搜索                │
└─────────────────────────────────────────┘
```

**方案 A 的优势：**
- Rust 后端提供极致的文件系统操作性能和内存安全
- Tauri 安装包小（< 15MB），内存占用低
- Web 前端可快速迭代 macOS 照片风格的精美 UI
- **原生跨平台**：Windows (WebView2) + Linux (WebKitGTK) 一套代码编译
- Rust 生态有 `ntfs-reader`、`rrgeo` 等直接可用的高质量库
- **i18n 成熟方案**：react-i18next + @tauri-apps/plugin-os 自动检测系统语言

**方案 B（备选）：.NET 9 + Avalonia UI**

- 优势：跨平台 UI 框架成熟，C# 开发效率高，ONNX Runtime 集成良好
- 劣势：文件索引性能不如 Rust，社区比 Tauri 小
- 参考：FlyPhotos、μPhotos、albummaker 均采用 .NET 体系

**方案 C（备选）：基于 iPhotron 二次开发**

- 优势：最快实现路径，已有 70% 功能
- 劣势：Python 性能瓶颈，大规模库卡顿风险，代码架构需大幅重构
- 适合：MVP 验证阶段

### 3.2 快速文件索引引擎

#### 3.2.1 技术原理

类似 Everything 的极速索引依赖两个 Windows NTFS 底层机制：

| 机制 | 用途 | 性能 |
|------|------|------|
| **MFT (Master File Table)** | 首次全盘扫描，读取所有文件记录 | 百万级文件 < 5秒 |
| **USN Journal** | 增量变更监听，实时感知文件增删改 | 实时（毫秒级）|

#### 3.2.2 实现方案

```
首次启动:
  MFT 全量扫描 → 过滤图片/视频扩展名 → 写入 SQLite 索引

后续使用:
  USN Journal 监听 → 增量更新 SQLite 索引 → 按需提取 EXIF 元数据

索引数据结构:
  ┌─────────────────────────────────────┐
  │ file_id | path | name | ext | size  │
  │ created | modified | parent_dir    │
  │ exif_extracted | thumbnail_cached  │
  └─────────────────────────────────────┘
```

**可用的 Rust 库：**

| 库 | 功能 | 成熟度 |
|----|------|--------|
| `ntfs-reader` (0.4.5) | MFT 读取 + USN Journal 监听 | 生产可用 |
| `usn-journal-rs` | USN Journal 专项 | 稳定 |
| `omerbenamram/mft` | MFT 解析（跨平台分析用） | 成熟 |

**跨平台索引策略：**

| 平台 | 首次扫描 | 增量监听 | 性能 |
|------|---------|---------|------|
| **Windows (NTFS)** | MFT 全量扫描 | USN Journal | 百万文件 < 5秒 |
| **Windows (非NTFS)** | 递归目录扫描 | ReadDirectoryChangesW | 百万文件 ~30秒 |
| **Linux (ext4/btrfs)** | 递归目录扫描 + `walkdir` | **inotify** / **fanotify** | 百万文件 ~15秒 |

```rust
// 跨平台索引引擎抽象
trait FileIndexer {
    async fn full_scan(&self, root: &Path) -> Vec<FileEntry>;
    async fn watch_changes(&self, root: &Path) -> Receiver<FileChange>;
}

// Windows 实现
struct NtfsIndexer; // MFT + USN Journal

// Linux 实现
struct LinuxIndexer; // walkdir + inotify

// 通用降级实现
struct GenericIndexer; // std::fs + notify crate
```

**注意事项：**
- Windows MFT/USN 访问需要管理员权限
- Linux inotify 有文件监听数量上限（默认 8192），大型库需调整 `fs.inotify.max_user_watches`
- Linux fanotify 是 inotify 的增强版，支持文件系统级监听，但需 root 权限
- 需对非 NTFS 分区和 Linux 提供通用递归扫描降级方案
- USN Journal 是循环缓冲区，长期不运行需全量重扫
- Rust `notify` crate 可作为跨平台文件监听的统一抽象层

### 3.3 照片元数据提取与分组

#### 3.3.1 EXIF 元数据提取

| 方案 | 语言 | 格式支持 | 性能 |
|------|------|---------|------|
| `kamadak-exif` (Rust) | Rust | JPEG/TIFF/HEIF | 极高，原生解析 |
| `rexiv2` (Rust) | Rust | 全格式(基于libexiv2) | 高 |
| ExifTool (外部) | Perl | **最全格式** | 中等(进程调用) |

**推荐策略：** Rust 原生解析常见格式 (JPEG/HEIF/PNG) + ExifTool 兜底处理特殊格式

#### 3.3.2 时间分组

```
照片按时间分组逻辑:
  1. 优先使用 EXIF DateTimeOriginal
  2. 回退到 EXIF CreateDate
  3. 最后使用文件系统修改时间
  4. 按 年 → 月 → 日 三级分组
  5. 同一天内按时间排序
```

#### 3.3.3 地点分组（离线反向地理编码）

| 方案 | 语言 | 数据源 | 精度 | 大小 |
|------|------|--------|------|------|
| `rrgeo` | **Rust** | GeoNames | 城市级 | ~10MB |
| `reverse-geocoder` | Python | GeoNames | 城市级 | ~20MB |
| `offline-geocode-city` | JS/TS | GeoNames | 城市级 | 217KB |
| `open-geocode` | Rust | OpenStreetMap | 地址级 | 可变 |

**推荐：** 使用 `rrgeo`（Rust K-D Tree 实现），0 毫秒级查询延迟，嵌入 GeoNames 数据。

```
地点分组逻辑:
  1. 提取 EXIF GPS 经纬度
  2. rrgeo 反向编码 → 国家/城市
  3. DBSCAN 聚类 → 合并近距离地点
  4. 按 国家 → 城市 → 地点 三级分组
  5. 无 GPS 照片归入"未知地点"
```

### 3.4 照片去重系统

#### 3.4.1 三级去重架构

```
Level 1: 精确去重 (文件级)
  ├─ 文件大小快速筛选 (O(1))
  └─ BLAKE3 哈希对比 (极快，Rust 原生)
      → 结果：完全相同的文件

Level 2: 感知去重 (视觉级)
  ├─ DHash 差值哈希 (8x8 灰度矩阵)
  ├─ PHash 感知哈希 (DCT 变换)
  └─ 汉明距离 < 阈值 → 判定相似
      → 结果：视觉相同但编码不同的照片
      → 如：不同压缩率、不同分辨率

Level 3: 语义去重 (内容级)
  ├─ CNN 特征提取 (MobileNetV3 / ONNX)
  └─ 余弦相似度 > 0.9 → 判定相似
      → 结果：构图/内容相似的照片
      → 如：连拍、微调、截图
```

#### 3.4.2 Rust 实现方案

| 功能 | Rust 库 | 说明 |
|------|---------|------|
| 文件哈希 | `blake3` | 最快的通用哈希算法 |
| 感知哈希 | `img_hash` / 自实现 | DHash/PHash/AHash |
| 图像处理 | `image` | 缩放、灰度转换 |
| CNN 推理 | `ort` (ONNX Runtime) | 加载预训练模型 |
| 相似度搜索 | `hnsw` / `usearch` | 高维向量近似最近邻 |

### 3.5 截图照片识别

#### 3.5.1 混合识别策略

```
┌────────────────────────────────────────┐
│           截图识别 Pipeline             │
├────────────────────────────────────────┤
│                                        │
│  Step 1: 元数据快筛 (< 1ms/张)          │
│  ├─ EXIF 无相机型号 → 疑似              │
│  ├─ EXIF 无光圈/快门/ISO → 疑似         │
│  ├─ 分辨率匹配屏幕分辨率 → 疑似          │
│  └─ 宽高比匹配 16:9/16:10 → 加分        │
│                                        │
│  Step 2: 视觉特征分析 (< 10ms/张)       │
│  ├─ 边缘方差极低（纯色背景）→ 加分       │
│  ├─ 顶部/底部状态栏检测 → 加分          │
│  ├─ 颜色直方图分析 → UI 色彩模式        │
│  └─ 综合评分 > 阈值 → 标记为截图         │
│                                        │
│  Step 3: CLIP 深度分类 (可选, ~100ms)    │
│  ├─ 类别: 截图/文档/代码/聊天/游戏/其他  │
│  └─ ONNX Runtime CPU 推理               │
│                                        │
└────────────────────────────────────────┘
```

#### 3.5.2 CLIP 分类实现

```
预定义类别嵌入:
  "a screenshot of a mobile phone"
  "a screenshot of a computer screen"
  "a photo taken by a camera"
  "a document or text image"
  "a screenshot of code or terminal"
  "a screenshot of a chat conversation"

推理流程:
  1. 加载 CLIP ViT-B/32 ONNX 模型 (~350MB)
  2. 图像预处理 → 224x224 → 归一化
  3. 计算图像嵌入向量
  4. 与预定义类别嵌入计算余弦相似度
  5. 取最高相似度类别作为分类结果
```

### 3.6 多语言 (i18n) 架构

#### 3.6.1 技术方案

基于 **react-i18next** + **@tauri-apps/plugin-os** 实现，这是 Tauri 生态中最成熟的 i18n 方案。

```
/locales/
├── zh-CN.json    # 简体中文（默认）
├── en.json       # English
├── zh-TW.json    # 繁體中文（预留）
├── ja.json       # 日本語（预留）
└── ko.json       # 한국어（预留）

/src/i18n/
├── config.ts         # i18next 初始化配置
├── language-init.ts  # 系统语言自动检测
└── index.ts          # 导出
```

#### 3.6.2 语言检测与切换流程

```
应用启动:
  1. 读取用户保存的语言偏好 (SQLite settings)
  2. 若为 "auto" 或未设置 → 调用 @tauri-apps/plugin-os locale()
  3. 匹配支持的语言 → 加载对应 JSON
  4. 无匹配 → 回退到 en

手动切换:
  1. 设置页面选择语言
  2. i18n.changeLanguage(newLang)
  3. 保存偏好到 SQLite
  4. 多窗口场景: Tauri emit('language-changed') 同步
```

#### 3.6.3 翻译文件结构示例

```json
// zh-CN.json
{
  "sidebar": {
    "library": "图库",
    "timeline": "时间",
    "places": "地点",
    "people": "人物",
    "albums": "相簿",
    "tools": "工具",
    "duplicates": "重复照片",
    "screenshots": "截图",
    "similar": "相似照片",
    "folders": "文件夹"
  },
  "viewer": {
    "info": "信息",
    "favorite": "收藏",
    "delete": "删除",
    "zoom_in": "放大",
    "zoom_out": "缩小"
  },
  "status": {
    "photos_count": "{{count}} 张照片",
    "videos_count": "{{count}} 个视频",
    "scanning": "正在扫描...",
    "indexing": "正在索引 {{current}}/{{total}}"
  }
}
```

#### 3.6.4 Rust 后端多语言

Rust 侧使用 `rust-i18n` 或 `fluent` 处理后端消息（错误提示、通知、日志等），前端通过 IPC 传递当前语言设置。

### 3.7 数据存储架构

#### 3.6.1 SQLite 数据库设计

```sql
-- 核心文件索引表
CREATE TABLE media_files (
    id          INTEGER PRIMARY KEY,
    file_path   TEXT NOT NULL UNIQUE,
    file_name   TEXT NOT NULL,
    file_ext    TEXT NOT NULL,
    file_size   INTEGER NOT NULL,
    file_hash   BLOB,              -- BLAKE3 哈希
    created_at  INTEGER,           -- 文件创建时间
    modified_at INTEGER,           -- 文件修改时间

    -- EXIF 元数据
    taken_at    INTEGER,           -- 拍摄时间 (DateTimeOriginal)
    camera      TEXT,              -- 相机型号
    lens        TEXT,              -- 镜头型号
    focal_len   REAL,              -- 焦距
    aperture    REAL,              -- 光圈
    shutter     TEXT,              -- 快门速度
    iso         INTEGER,           -- ISO
    width       INTEGER,           -- 图片宽度
    height      INTEGER,           -- 图片高度

    -- GPS 信息
    latitude    REAL,
    longitude   REAL,
    country     TEXT,              -- 反向编码国家
    city        TEXT,              -- 反向编码城市

    -- 智能分类
    media_type  TEXT DEFAULT 'photo', -- photo/video/screenshot/document
    is_favorite INTEGER DEFAULT 0,
    is_hidden   INTEGER DEFAULT 0,
    is_deleted  INTEGER DEFAULT 0,
    deleted_at  INTEGER,

    -- 去重信息
    phash       BLOB,              -- 感知哈希
    dhash       BLOB,              -- 差值哈希
    duplicate_group_id INTEGER,    -- 重复组 ID

    -- 索引状态
    exif_extracted  INTEGER DEFAULT 0,
    thumb_cached    INTEGER DEFAULT 0,
    hash_computed   INTEGER DEFAULT 0,
    ai_classified   INTEGER DEFAULT 0
);

-- 性能索引
CREATE INDEX idx_taken_at ON media_files(taken_at);
CREATE INDEX idx_location ON media_files(country, city);
CREATE INDEX idx_media_type ON media_files(media_type);
CREATE INDEX idx_parent_dir ON media_files(file_path);
CREATE INDEX idx_duplicate ON media_files(duplicate_group_id);

-- 相簿表
CREATE TABLE albums (
    id          INTEGER PRIMARY KEY,
    name        TEXT NOT NULL,
    cover_file  INTEGER REFERENCES media_files(id),
    created_at  INTEGER,
    updated_at  INTEGER,
    sort_order  INTEGER DEFAULT 0,
    album_type  TEXT DEFAULT 'user' -- user/smart/auto
);

-- 相簿-文件关联（虚拟关联，不复制文件）
CREATE TABLE album_files (
    album_id    INTEGER REFERENCES albums(id),
    file_id     INTEGER REFERENCES media_files(id),
    sort_order  INTEGER DEFAULT 0,
    added_at    INTEGER,
    PRIMARY KEY (album_id, file_id)
);

-- 监控文件夹表
CREATE TABLE watched_folders (
    id          INTEGER PRIMARY KEY,
    folder_path TEXT NOT NULL UNIQUE,
    recursive   INTEGER DEFAULT 1,
    last_scan   INTEGER,
    file_count  INTEGER DEFAULT 0
);

-- FTS5 全文搜索
CREATE VIRTUAL TABLE media_fts USING fts5(
    file_name,
    camera,
    country,
    city,
    content='media_files',
    content_rowid='id',
    tokenize='unicode61'
);
```

#### 3.6.2 缩略图缓存策略

```
缩略图存储:
  ~/.lightframe/cache/thumbs/
    ├── micro/    (64x64)    用于网格快速滚动
    ├── small/    (256x256)  用于相簿浏览
    └── large/    (1024x1024) 用于预览

命名规则:
  {BLAKE3_hash_前16字符}.webp

生成策略:
  1. 优先使用嵌入的 EXIF 缩略图 (JPEG 自带)
  2. 异步后台生成精确缩略图
  3. WebP 格式压缩（质量 80%）
  4. 内存 LRU 缓存热点缩略图
```

---

## 四、功能模块详细设计

### 4.1 核心功能对标 macOS 照片

| macOS 照片功能 | 影迹实现方案 | 优先级 |
|----------------|-------------|--------|
| **图库** (所有照片时间线) | SQLite 按 `taken_at` 排序 + 虚拟滚动 | P0 |
| **回忆** | 自动按日期+地点聚类生成 | P2 |
| **人物** | InsightFace ONNX 人脸检测+聚类 | P2 |
| **地点** | GPS + rrgeo 反向编码 + 地图视图 | P1 |
| **相簿** (用户创建) | album_files 虚拟关联 | P0 |
| **智能相簿** | SQL 动态查询（按类型/日期/镜头等） | P1 |
| **收藏** | is_favorite 标记 | P0 |
| **最近删除** | is_deleted + 30天自动清理 | P1 |
| **编辑** | 非破坏性编辑，sidecar 文件存储调整参数 | P3 |
| **搜索** | FTS5 全文搜索 + CLIP 语义搜索 | P1 |
| ~~导入/导出~~ | **不实现** — 核心差异点 | — |

### 4.2 差异化功能

| 功能 | 实现方案 | 优先级 |
|------|---------|--------|
| **极速文件索引** | MFT 全量扫描 + USN Journal 增量监听 | P0 |
| **精确去重** | BLAKE3 文件哈希 | P0 |
| **感知去重** | DHash + PHash 感知哈希 | P1 |
| **相似照片分组** | CNN 特征向量 + 余弦相似度 | P1 |
| **截图自动识别** | EXIF 规则 + 视觉特征 + CLIP 分类 | P1 |
| **截图分类** | CLIP 语义分类（代码/聊天/文档等） | P2 |
| **文件夹浏览** | 树形结构，保持原始文件系统层级 | P0 |
| **批量操作** | 多选移动/删除/加入相簿（操作原始文件） | P1 |

### 4.3 UI/UX 设计要点

```
主界面布局（对标 macOS 照片）:

┌──────────────────────────────────────────────┐
│  🔍 搜索栏                          ⚙️ 设置  │
├────────┬─────────────────────────────────────┤
│ 侧边栏  │                                     │
│        │     照片网格区域（虚拟滚动）           │
│ 📸 图库  │                                     │
│ 🗓 时间  │  ┌─────┐ ┌─────┐ ┌─────┐ ┌─────┐  │
│ 📍 地点  │  │     │ │     │ │     │ │     │  │
│ 👤 人物  │  │ 📷  │ │ 📷  │ │ 📷  │ │ 📷  │  │
│         │  │     │ │     │ │     │ │     │  │
│ 📁 相簿  │  └─────┘ └─────┘ └─────┘ └─────┘  │
│  ├ 旅行  │                                     │
│  ├ 家人  │  ┌─────┐ ┌─────┐ ┌─────┐ ┌─────┐  │
│  └ 工作  │  │     │ │     │ │     │ │     │  │
│         │  │ 📷  │ │ 📷  │ │ 📷  │ │ 📷  │  │
│ 🔧 工具  │  │     │ │     │ │     │ │     │  │
│  ├ 重复  │  └─────┘ └─────┘ └─────┘ └─────┘  │
│  ├ 截图  │                                     │
│  └ 相似  │         2024年8月 · 东京              │
│         │                                     │
│ 📂 文件夹│  ┌─────┐ ┌─────┐ ┌─────┐           │
│  ├ D:\   │  │     │ │     │ │     │           │
│  └ E:\   │  │ 📷  │ │ 📷  │ │ 📷  │           │
│         │  └─────┘ └─────┘ └─────┘           │
├────────┴─────────────────────────────────────┤
│  状态栏: 12,345 张照片 · 456 个视频 · 2.3 TB   │
└──────────────────────────────────────────────┘

照片详情视图:

┌──────────────────────────────────────────────┐
│  ← 返回          照片名.jpg        ♥ ⓘ 🗑    │
├──────────────────────────────────────────────┤
│                                              │
│                                              │
│              全尺寸照片预览                    │
│           (支持缩放/拖拽/旋转)                 │
│                                              │
│                                              │
├──────────────────────────────────────────────┤
│ ◀ ●●●○●●●●●●●●●●●●●●●●●●●●●●● ▶            │
│          底部胶片条（相邻照片缩略图）            │
└──────────────────────────────────────────────┘

信息面板 (ⓘ):
  ┌──────────────────┐
  │ 📷 相机信息       │
  │ Canon EOS R5     │
  │ RF 24-70mm f/2.8 │
  │ f/4 · 1/250s     │
  │ ISO 400 · 24mm   │
  ├──────────────────┤
  │ 📍 拍摄地点       │
  │ 日本 · 东京       │
  │ [mini map]       │
  ├──────────────────┤
  │ 📅 2024-08-15    │
  │ 14:32:15         │
  ├──────────────────┤
  │ 📁 D:\Photos\    │
  │ Tokyo2024\       │
  │ IMG_1234.CR3     │
  │ 45.2 MB          │
  └──────────────────┘
```

---

## 五、技术可行性评估

### 5.1 各功能模块可行性

| 功能模块 | 技术成熟度 | 开源支持 | 实现难度 | 可行性 |
|---------|-----------|---------|---------|--------|
| macOS 照片风格 UI | ⭐⭐⭐⭐ | 多个参考实现 | 中 | ✅ 完全可行 |
| MFT/USN 快速索引 | ⭐⭐⭐⭐ | Rust 库成熟 | 中 | ✅ 完全可行 |
| EXIF 元数据提取 | ⭐⭐⭐⭐⭐ | ExifTool 等 | 低 | ✅ 完全可行 |
| 时间线分组 | ⭐⭐⭐⭐⭐ | SQLite 排序 | 低 | ✅ 完全可行 |
| 地点分组 | ⭐⭐⭐⭐ | rrgeo 离线编码 | 低 | ✅ 完全可行 |
| 用户相簿 | ⭐⭐⭐⭐⭐ | 标准数据库操作 | 低 | ✅ 完全可行 |
| 精确去重 | ⭐⭐⭐⭐⭐ | BLAKE3 | 低 | ✅ 完全可行 |
| 感知去重 | ⭐⭐⭐⭐ | imagededup/自实现 | 中 | ✅ 完全可行 |
| 相似照片分组 | ⭐⭐⭐ | CNN + 向量搜索 | 高 | ✅ 可行，需 GPU 加速 |
| 截图识别 | ⭐⭐⭐ | CLIP + 启发式 | 中 | ✅ 可行 |
| 人脸识别 | ⭐⭐⭐ | InsightFace ONNX | 高 | ✅ 可行，后期加入 |
| 地图视图 | ⭐⭐⭐⭐ | Leaflet/MapLibre | 中 | ✅ 完全可行 |
| 视频播放 | ⭐⭐⭐⭐ | FFmpeg + Web | 中 | ✅ 完全可行 |

### 5.2 性能预估

| 场景 | 目标 | 实现路径 |
|------|------|---------|
| 10万张照片首次索引 | < 30秒 | MFT 扫描(~5s) + 异步 EXIF 提取 |
| 增量索引更新 | < 1秒 | USN Journal 实时监听 |
| 照片网格滚动 (10万张) | 60fps | 虚拟滚动 + micro 缩略图 |
| 精确去重 (10万张) | < 5分钟 | BLAKE3 并行哈希 |
| 感知去重 (10万张) | < 10分钟 | DHash 并行计算 |
| 截图识别 (10万张) | < 30分钟 | 规则快筛 + CLIP 精确分类 |
| 搜索响应 | < 50ms | SQLite FTS5 |
| 应用启动 | < 1秒 | Tauri 原生启动 |
| 安装包大小 | < 50MB | Tauri + 嵌入模型 |

### 5.3 跨平台适配评估

| 模块 | Windows | Linux | 备注 |
|------|---------|-------|------|
| Tauri 2.x 运行 | WebView2 (Chromium) | WebKitGTK | Linux 需安装 libwebkit2gtk-4.1 |
| 文件索引 | MFT/USN (NTFS原生) | walkdir + inotify | Linux 稍慢但足够 |
| EXIF 提取 | kamadak-exif (Rust) | 相同 | 跨平台无差异 |
| SQLite | rusqlite | 相同 | 跨平台无差异 |
| 缩略图生成 | image crate | 相同 | 跨平台无差异 |
| 视频处理 | FFmpeg sidecar | 相同 | 两平台均需安装 FFmpeg |
| ONNX Runtime | DirectML/CUDA 可选 | CPU / CUDA | Linux GPU 加速需额外配置 |
| 系统托盘 | 原生支持 | libayatana-appindicator | 需额外依赖 |
| 自动更新 | Tauri updater (NSIS) | AppImage/deb | 不同分发格式 |
| 文件关联 | 注册表 | .desktop 文件 | 需平台特定代码 |

**Linux 额外依赖：**
```bash
# Ubuntu/Debian
sudo apt install libwebkit2gtk-4.1-dev libgtk-3-dev \
  libayatana-appindicator3-dev librsvg2-dev ffmpeg

# Fedora
sudo dnf install webkit2gtk4.1-devel gtk3-devel \
  libappindicator-gtk3-devel librsvg2-devel ffmpeg
```

### 5.4 风险与挑战

| 风险 | 影响 | 缓解措施 |
|------|------|---------|
| MFT 需管理员权限 (Windows) | 首次索引需提权 | 提供常规扫描降级方案 |
| CLIP 模型体积大 (~350MB) | 首次下载慢 | 可选安装 AI 功能包 |
| 非 NTFS 分区/Linux 扫描 | 索引速度降低 | walkdir + inotify + 进度条 |
| 视频缩略图生成慢 | 后台任务积压 | 异步队列 + 优先级调度 |
| 网络驱动器/NAS | 索引效率低 | 标记为"慢速源"，降低扫描频率 |
| Linux WebKitGTK 渲染差异 | 部分 CSS 表现不一致 | 增加 Linux CI 测试 + 降级样式 |
| inotify 监听数量上限 | 大型库可能漏更新 | 定期全量扫描补偿 |
| Linux 发行版碎片化 | 依赖安装复杂 | 优先提供 AppImage 自包含格式 |

---

## 六、开发路线图

### Phase 1: 核心基础 (MVP, ~8周)

- [ ] Tauri 2.x + React 19 + TypeScript 项目脚手架
- [ ] **i18n 框架搭建** (react-i18next + 中文/英文翻译文件)
- [ ] 跨平台文件索引引擎（Windows: MFT/USN, Linux: walkdir + inotify）
- [ ] SQLite 数据库 + EXIF 元数据提取
- [ ] 照片网格视图（虚拟滚动）
- [ ] 时间线分组浏览
- [ ] 全屏照片查看（缩放/拖拽）
- [ ] 缩略图生成与缓存
- [ ] 监控文件夹管理
- [ ] Windows + Linux 双平台 CI/CD 构建

### Phase 2: 智能功能 (~6周)

- [ ] 精确去重（BLAKE3）
- [ ] 感知去重（DHash/PHash）
- [ ] 截图自动识别（规则 + CLIP）
- [ ] 地点分组（GPS + rrgeo）
- [ ] 用户相簿（创建/编辑/删除）
- [ ] 收藏夹 & 最近删除
- [ ] 搜索功能（FTS5）
- [ ] 地图视图（Leaflet/MapLibre）

### Phase 3: 增强体验 (~6周)

- [ ] 相似照片分组（CNN 向量）
- [ ] 智能相簿（自动规则）
- [ ] 截图分类（代码/聊天/文档）
- [ ] 视频播放支持
- [ ] 人脸检测与分组
- [ ] 回忆功能（自动生成）
- [ ] 批量操作（移动/删除/旋转）
- [ ] 深色/浅色主题

### Phase 4: 打磨发布 (~4周)

- [ ] 性能优化 & 压力测试 (10万+ 照片场景)
- [ ] Windows 安装程序 (MSI/NSIS)
- [ ] Linux 分发包 (AppImage + .deb + .rpm)
- [ ] Tauri 自动更新机制
- [ ] 用户文档 (中/英双语)
- [ ] 多语言翻译完善 & 预留扩展接口
- [ ] GitHub Release 自动化发布

---

## 七、结论与建议

### 7.1 方案可行性结论

**技术完全可行。** 所有核心功能模块均有成熟的开源库支持，且 Rust + Tauri 技术栈能在性能和体积上达到优秀水平。最大的工作量在于 UI 开发和各模块的集成调优。

### 7.2 核心建议

1. **不建议直接 fork iPhotron**：虽然功能覆盖度最高，但 Python + Qt6 的技术栈在 Windows 上的性能和分发体验不如 Rust + Tauri
2. **推荐自研 + 借鉴**：以 Tauri 2.x + Rust 为基础自研，大量参考 iPhotron 的功能设计和 UX 交互
3. **优先实现差异化功能**：快速索引和去重是核心竞争力，应优先保证这两项的体验
4. **AI 功能模块化**：CLIP/CNN 等 AI 模型作为可选组件，不影响基础功能的轻量运行
5. **渐进式开发**：先做好"极速索引 + 美观浏览"的基础体验，再逐步添加智能功能

### 7.3 技术依赖汇总

| 类别 | 依赖 | 许可证 | 用途 |
|------|------|--------|------|
| 框架 | Tauri 2.x | MIT/Apache-2.0 | 跨平台桌面应用框架 |
| 前端 | React 19 + TailwindCSS v4 | MIT | UI 界面 |
| UI 组件 | shadcn/ui + Radix UI | MIT | 无障碍组件库 |
| 虚拟滚动 | @tanstack/react-virtual | MIT | 大列表渲染 |
| 多语言 | i18next + react-i18next | MIT | 前端国际化 |
| 系统语言 | @tauri-apps/plugin-os | MIT/Apache-2.0 | 系统 locale 检测 |
| 地图 | Leaflet / MapLibre GL | BSD-2 | 地图展示 |
| 数据库 | SQLite (rusqlite) | 公共领域 | 数据存储 |
| 文件索引(Win) | ntfs-reader | MIT | MFT/USN 读取 |
| 文件监听 | notify (Rust) | MIT/Apache-2.0 | 跨平台文件变更监听 |
| EXIF | kamadak-exif | BSD-2 | 元数据提取 |
| 视频 | FFmpeg (sidecar) | LGPL/GPL | 视频处理 |
| 去重哈希 | blake3 + img_hash | Apache-2.0/MIT | 去重检测 |
| 反向编码 | rrgeo | Apache-2.0/MIT | GPS→地名 |
| AI 推理 | ort (ONNX Runtime) | MIT | CLIP/CNN 推理 |
| 图像处理 | image (Rust) | MIT/Apache-2.0 | 缩略图等 |
| Rust i18n | rust-i18n / fluent | MIT/Apache-2.0 | 后端消息国际化 |

---

> **影迹 / LightFrame** — 每一帧光影，都是时间的印迹。Every frame of light is a trace of time.
