# LightFrame 用户指南 / User Guide

## 安装 / Installation

### Windows

1. 下载 `.msi` 或 `.exe` 安装程序
2. 运行安装程序，按提示完成安装
3. 首次运行可能触发 Windows SmartScreen 警告（因程序暂未签名），点击「仍要运行」即可

### Linux

- **Debian/Ubuntu**: `sudo dpkg -i lightframe_*.deb`
- **Fedora/RHEL**: `sudo rpm -i lightframe-*.rpm`
- **通用**: 下载 `.AppImage`，赋予执行权限后运行

### macOS

1. 下载 `.dmg` 文件
2. 打开后将 LightFrame 拖入 Applications 文件夹

## 快速开始 / Getting Started

### 添加照片文件夹

1. 点击左下角 **设置**
2. 在「文件夹管理」中点击 **添加文件夹**
3. 选择包含照片/视频的目录
4. LightFrame 会自动扫描并生成缩略图

### 浏览照片

- **所有照片**: 以时间网格展示所有媒体
- **时间线**: 按日期分组查看
- **视频**: 仅显示视频文件
- **文件夹**: 按文件夹浏览

### 搜索

在顶部搜索栏输入关键词，支持：

- 文件名搜索
- 日期搜索（如 `2024-01`）
- 搜索历史（最近 10 条）

## 键盘快捷键 / Keyboard Shortcuts

### 照片网格

| 快捷键 | 功能 |
|--------|------|
| `Delete` / `Backspace` | 删除选中照片 |
| `F` | 切换收藏 |
| `Ctrl+A` / `Cmd+A` | 全选 |
| `Escape` | 取消选择 |

### 照片查看器

| 快捷键 | 功能 |
|--------|------|
| `←` / `→` | 上/下一张 |
| `R` | 顺时针旋转 |
| `Shift+R` | 逆时针旋转 |
| `F` | 切换收藏 |
| `I` | 显示/隐藏信息面板 |
| `E` | 打开编辑器 |
| `Delete` | 删除当前照片 |
| `Escape` | 关闭查看器 |
| 滚轮 | 缩放 |

### 视频播放器

| 快捷键 | 功能 |
|--------|------|
| `Space` | 播放/暂停 |
| `←` / `→` | 快退/快进 5 秒 |

### 图片编辑器

| 快捷键 | 功能 |
|--------|------|
| `Ctrl+Z` / `Cmd+Z` | 撤销 |
| `Ctrl+Shift+Z` / `Cmd+Shift+Z` | 重做 |

## 功能说明 / Features

### 相册管理

- 创建、重命名、删除相册
- 拖拽照片到侧边栏相册
- 设置相册封面

### 智能相册

- 基于规则自动筛选（如文件类型、日期范围）

### 重复检测

- BLAKE3 精确去重
- DHash/PHash 感知去重（相似图片检测）

### 照片编辑

- 曲线、色阶、选区颜色调整
- 非破坏性编辑（保存为 JSON sidecar）

### AI 功能（可选）

LightFrame 的 AI 功能在本地运行，不上传云端。需要 ONNX 模型文件：

| 模型 | 用途 | 约大小 |
|------|------|--------|
| `clip-vit-b32-visual.onnx` | 相似照片搜索 | ~350 MB |
| `scrfd_500m_bnkps.onnx` | 人脸检测 | ~5 MB |
| `w600k_r50.onnx` | 人脸识别嵌入 | ~170 MB |

#### 模型安装步骤

1. 打开 **设置 → AI 功能**
2. 点击 **下载模型**（若可用），或点击 **打开模型目录** 手动放置文件
3. 也可从 [GitHub Releases](https://github.com/halft0n/LightFrame/releases) 下载模型包
4. 点击 **刷新状态** 确认模型已识别

#### 模型目录

| 平台 | 路径 |
|------|------|
| Linux | `~/.local/share/lightframe/models/` |
| Windows | `%LOCALAPPDATA%\lightframe\models\` |
| macOS | `~/Library/Application Support/lightframe/models/` |

#### 可选：Python AI 扩展

若未安装 ONNX 模型，可安装 Python 3.10+ 并使用内置 sidecar（设置页会显示状态）。

## 性能调优 / Performance Tuning

- **缩略图尺寸**: 在照片网格工具栏选择 Small / Medium / Large，较小尺寸加载更快
- **扫描并发**: 默认并发处理；大量照片首次扫描需耐心等待完成
- **文件夹数量**: 仅添加需要的目录，减少 file watcher 开销
- **AI 功能**: CLIP 嵌入计算较耗时，建议在空闲时批量处理；无 GPU 时 ONNX 使用 CPU 推理
- **数据库**: 应用使用 SQLite WAL 模式；请勿在外部直接修改 `library.db`

## 故障排除 / Troubleshooting

### Windows SmartScreen 警告

LightFrame 暂未购买代码签名证书。首次运行时点击「更多信息」→「仍要运行」。详见 [SIGNING.md](SIGNING.md)。

### 缩略图不显示

1. 确认已添加文件夹并完成扫描
2. 检查照片格式是否支持（JPG, PNG, WebP, GIF, BMP, TIFF, HEIC）
3. 在设置中点击「重新扫描全部」
4. 尝试重启应用

### AI 模型无法加载

1. 确认模型文件完整下载（检查文件大小）
2. 确认文件名与上表一致
3. 点击「刷新状态」重新检测
4. 查看模型目录权限是否可读

### 搜索无结果

1. 确认扫描已完成（文件夹状态为 idle）
2. 尝试文件名的一部分而非完整路径
3. 中文文件名需等待 FTS 索引完成

### 360/杀毒软件拦截

WebView2 组件可能触发安全软件。建议将 LightFrame 添加到信任列表。

### 数据库锁定错误

若出现「database locked」，关闭其他可能访问数据库的进程，重启 LightFrame。不要在外部工具中打开 `library.db`。

### Linux 依赖缺失

若无法启动，安装 WebKit/GTK 依赖：

```bash
# Debian/Ubuntu
sudo apt install libwebkit2gtk-4.1-dev libgtk-3-dev libayatana-appindicator3-dev librsvg2-dev
```

## 相关文档

- [Beta 路线图](BETA_ROADMAP.md)
- [隐私政策](PRIVACY.md)
- [代码签名说明](SIGNING.md)
