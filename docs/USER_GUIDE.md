# CatchLight 用户指南 / User Guide

## 安装 / Installation

### Windows
1. 下载 `.msi` 或 `.exe` 安装程序
2. 运行安装程序，按提示完成安装
3. 首次运行可能触发 Windows SmartScreen 警告（因程序暂未签名），点击"仍要运行"即可

### Linux
- **Debian/Ubuntu**: `sudo dpkg -i catchlight_*.deb`
- **Fedora/RHEL**: `sudo rpm -i catchlight-*.rpm`
- **通用**: 下载 `.AppImage`，赋予执行权限后运行

### macOS
1. 下载 `.dmg` 文件
2. 打开后将 CatchLight 拖入 Applications 文件夹

## 快速开始 / Getting Started

### 添加照片文件夹
1. 点击左下角 **设置**
2. 在"文件夹管理"中点击 **添加文件夹**
3. 选择包含照片/视频的目录
4. CatchLight 会自动扫描并生成缩略图

### 浏览照片
- **所有照片**: 以时间网格展示所有媒体
- **时间线**: 按日期分组查看
- **视频**: 仅显示视频文件
- **文件夹**: 按文件夹浏览

### 搜索
在顶部搜索栏输入关键词，支持：
- 文件名搜索
- 日期搜索（如 "2024-01"）
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
需要下载 ONNX 模型文件：
1. CLIP 模型 → 相似照片搜索
2. 人脸检测模型 → 人物识别与分组

模型放置路径：`~/.local/share/catchlight/models/`（Linux）或 `%APPDATA%/catchlight/models/`（Windows）

## 故障排除 / Troubleshooting

### Windows SmartScreen 警告
CatchLight 暂未购买代码签名证书。首次运行时点击"更多信息" → "仍要运行"。

### 缩略图不显示
1. 确认已添加文件夹并完成扫描
2. 检查照片格式是否支持（JPG, PNG, WebP, GIF, BMP, TIFF, HEIC）
3. 尝试重启应用

### 360/杀毒软件拦截
WebView2 组件可能触发安全软件。建议将 CatchLight 添加到信任列表。
