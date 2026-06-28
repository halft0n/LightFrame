# CatchLight 技术路线决策报告

> **决策日期：2026-06-28**
> **决策结论：混合方案 — Rust + Tauri + React + Python AI 扩展**

---

## 一、决策背景

基于以下三个核心优先级进行技术选型：

| 优先级 | 权重 | 说明 |
|--------|------|------|
| **界面美观度 + 操作流畅度** | 35% | 对标 macOS 照片应用的视觉品质 |
| **AI 功能易用性 + 扩展性** | 30% | 去重、截图识别、未来新模型热插拔 |
| **大规模处理能力** | 35% | 10万-100万照片的索引/查询/滚动稳定性 |

## 二、候选方案评估

### 评分矩阵

| 优先级 | 权重 | Rust+Tauri+React | 混合(Rust+Python) | C#+Avalonia | Python+Qt |
|--------|------|-----------------|------------------|------------|----------|
| UI 美观流畅 | 35% | 9.5 | **9.5** | 7.5 | 6.5 |
| AI 扩展性 | 30% | 6 | **10** | 7.5 | 10 |
| 大规模性能 | 35% | **10** | 9 | 8 | 4 |
| **加权总分** | | 8.55 | **9.45** | 7.65 | 6.65 |

### 各方案详细分析

#### 1. Rust + Tauri + React（纯 Rust）
- 性能最优但 AI 扩展性受限
- 新模型集成需 ONNX 转换 + Rust 预处理/后处理，周期长

#### 2. 混合方案（已选定）
- Rust 核心保证性能，Python 扩展保证 AI 灵活性
- 核心功能不依赖 Python，AI 扩展为可选安装

#### 3. C# + Avalonia
- 开发效率和性能的良好平衡
- UI 自由度不如 React，XAML 做 macOS 风格 UI 工作量大

#### 4. Python + PySide6
- 开发最快，AI 最强
- 100万照片时 GIL 和内存问题无法解决

## 三、最终架构

```
┌─────────────────────────────────────────────────┐
│           CatchLight / 拾光                      │
├─────────────────────────────────────────────────┤
│                                                  │
│  ┌──────────────────────────────────────────┐    │
│  │  前端: React 19 + TypeScript + Vite       │    │
│  │  UI: TailwindCSS v4 + shadcn/ui          │    │
│  │  动画: Framer Motion                      │    │
│  │  虚拟滚动: @tanstack/react-virtual        │    │
│  │  地图: Leaflet / MapLibre GL             │    │
│  │  i18n: react-i18next                     │    │
│  │  状态: Zustand                           │    │
│  └──────────────┬───────────────────────────┘    │
│                 │ Tauri IPC (命令 + 事件)          │
│  ┌──────────────┴───────────────────────────┐    │
│  │  Rust 核心 (Cargo Workspace)              │    │
│  │                                           │    │
│  │  catchlight-core     领域模型              │    │
│  │  catchlight-indexer  MFT/USN/inotify      │    │
│  │  catchlight-db       SQLite + FTS5        │    │
│  │  catchlight-metadata EXIF/GPS             │    │
│  │  catchlight-thumbnail 三级缩略图缓存      │    │
│  │  catchlight-dedup    BLAKE3/PHash/DHash   │    │
│  │  catchlight-ai       ONNX Runtime         │    │
│  │  catchlight-geo      rrgeo 反向地理编码    │    │
│  │  catchlight-video    FFmpeg sidecar       │    │
│  └──────────────┬───────────────────────────┘    │
│                 │ JSON-RPC (stdin/stdout)         │
│  ┌──────────────┴───────────────────────────┐    │
│  │  Python AI 扩展 (可选, 按需启动)           │    │
│  │                                           │    │
│  │  • 高级语义搜索 (CLIP + FAISS)            │    │
│  │  • 截图 OCR + 内容分类                    │    │
│  │  • 场景/活动自动标签                      │    │
│  │  • 相似照片深度聚类                       │    │
│  │  • 新模型热插拔框架                       │    │
│  │  • 自定义分类器训练                       │    │
│  └──────────────────────────────────────────┘    │
│                                                  │
│  分层功能策略:                                    │
│  ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━         │
│  核心层 (无 Python, ~15MB):                       │
│    索引 · 浏览 · 时间线 · 相簿 · 收藏             │
│    BLAKE3/PHash 去重 · EXIF · 地图                │
│    ONNX CLIP 基础搜索 · 人脸检测                  │
│                                                  │
│  AI 增强层 (需 Python, +20-30MB):                 │
│    高级标签 · OCR · 深度聚类 · 新模型              │
│  ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━         │
│                                                  │
└─────────────────────────────────────────────────┘
```

## 四、Rust-Python 通信机制

### 4.1 通信协议

```json
// Rust → Python (请求)
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "classify_screenshot",
  "params": {
    "image_path": "/photos/screenshot.png",
    "categories": ["code", "chat", "browser", "game", "document"]
  }
}

// Python → Rust (响应)
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": {
    "category": "code",
    "confidence": 0.94,
    "subcategory": "terminal",
    "ocr_text": "git status..."
  }
}
```

### 4.2 生命周期管理

```
Rust 核心启动
  │
  ├─ 检测 Python 环境是否可用
  │   ├─ 可用 → 记录路径，标记 AI 扩展就绪
  │   └─ 不可用 → 标记 AI 扩展未安装，核心功能正常
  │
  └─ 用户触发 AI 功能
      ├─ Python 已安装 → 启动 sidecar 进程
      │   ├─ 首次启动加载模型 (~3s)
      │   ├─ 后续复用进程 (JSON-RPC 长连接)
      │   └─ 空闲 5 分钟自动退出 (释放 GPU/内存)
      └─ Python 未安装 → 引导用户安装
          └─ 提供一键安装脚本
```

### 4.3 Python 扩展安装方式

```bash
# 方式 1: 内置安装引导 (推荐)
# CatchLight 设置页面 → AI 扩展 → 一键安装
# 内部执行:
pip install catchlight-ai  # 包含 CLIP/InsightFace/Tesseract 等

# 方式 2: 手动安装
pip install catchlight-ai
# 或
pip install catchlight-ai[full]  # 包含所有可选模型
```

## 五、工作量影响

相比纯 Rust 方案，混合方案增加的工作量：

| 增量工作 | 时间 | 说明 |
|---------|------|------|
| JSON-RPC 协议层 (Rust 侧) | +3 天 | serde + tokio 子进程管理 |
| Python AI 框架骨架 | +3 天 | 入口、模型管理、RPC server |
| 安装引导 UI | +2 天 | 检测/安装/进度 |
| Python 打包与分发 | +2 天 | pip 包 + 模型下载 |
| **总增量** | **+10 天** | 在 Phase 2 中实现 |

## 六、开源项目参考复用策略

| 来源 | 可参考设计 | 使用方式 |
|------|----------|---------|
| **Lap** (Tauri+Rust) | IPC 分层、ProcessingBudget、thumb:// 协议、ONNX 集成 | 洁净室重新实现（GPL 限制） |
| **iPhotron** (Python+Qt) | Folder-native 设计、SQL keyset 分页、扫描契约 | 架构思路参考 |
| **FlyPhotos** (.NET) | Preview/HQ 双轨缓存、Burst 预取 | 性能优化技巧参考 |

**许可证注意**：Lap 为 GPL-3.0，不可直接使用其代码。CatchLight 代码需完全独立编写。

---

> **拾光 / CatchLight** — Catch the light, keep the moment.
> 技术路线：Rust + Tauri + React + Python AI 扩展
