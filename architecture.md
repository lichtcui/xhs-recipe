# xhs-recipe App — 前后端分离设计方案

## 概述

将现有 CLI 工具拆分为 **前端（多平台）** + **后端（macOS/Linux）**：

- **前端**：Flutter 跨平台 App（桌面 + 移动），负责发送 URL、调用 LLM、UI 展示
- **后端**：Rust 独立服务，运行在 macOS/Linux，负责页面抓取、视频/音频处理等平台相关的重活

前端只需向后端发送 URL，拿到提取到的全部文字后自行调用 LLM，LLM API key 留存在用户设备上。

---

## 架构总览

```
┌───────────────────────────────────┐     ┌──────────────────────────┐
│          Frontend (Flutter)        │     │  Backend (Rust Server)   │
│                                   │     │  macOS / Linux           │
│  Step 1: 用户输入 URL              │     │                          │
│         │                         │     │                          │
│         ▼                         │     │                          │
│  发送 URL ──────────────────────url──→│  Step 2:                   │
│                                   │     │  sources::fetch()        │
│                                   │     │  → 爬取页面              │
│                                   │     │  textifier::process()    │
│                                   │     │  → 视频下载 + ASR        │
│                                   │     │  → 帧提取 + OCR          │
│                                   │     │  → splitter::split()     │
│                                   │     │  → 拆分为独立 items      │
│                                   │     │                          │
│  Step 3: 收到文字 ←───SSE/text─────│                          │
│         │                         │     │                          │
│         ▼                         │     │                          │
│  调 LLM (DeepSeek API)            │     │                          │
│  → 结构化 Recipe                  │     │                          │
│         │                         │     │                          │
│         ▼                         │     │                          │
│  Step 4: 展示 / 本地存储           │     │                          │
└───────────────────────────────────┘     └──────────────────────────┘
```

### 数据流

```
前端 → POST /process { url } → 后端 → SSE 流式返回 { title, items } → 前端 → LLM → 展示
```

1. **前端**：用户输入小红书（或其他平台）URL，发送给后端
2. **后端**：爬取页面 → 视频下载 + ASR 转写 → 帧提取 + OCR → 拆分为独立项 → SSE 流式返回
3. **前端**：收到 items 后逐项调用 LLM（DeepSeek API，function calling）→ 合并结果 → 展示
4. **前端**：展示 + 本地 SQLite 存储

前后端职责清晰：**后端只管提取原始文字并拆项，LLM 调用全部交给前端**。

---

## Cargo Workspace 结构

当前 `Cargo.toml` 为单 package，需要改为 workspace，让 server crate 能依赖 lib crate：

```toml
# Cargo.toml（workspace root）
[workspace]
members = [".", "server"]
resolver = "2"

[workspace.package]
version = "0.1.0"
edition = "2021"
```

```toml
# server/Cargo.toml
[package]
name = "xhs-recipe-server"
version.workspace = true
edition.workspace = true

[dependencies]
xhs-recipe = { path = ".." }         # 复用 sources + textifier + models
axum = "0.7"
tokio = { version = "1", features = ["rt-multi-thread", "macros", "sync"] }
tokio-stream = "0.1"                 # SSE 事件流
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tower-http = { version = "0.5", features = ["cors"] }
```

server 只依赖 lib crate 的 `sources` + `textifier` + `models` 模块，不依赖 `analyzer`、`pipeline`、`presentation`、`storage`。CLI 代码保持不变，且因 textifier 统一为 ffmpeg + tesseract，CLI 也不再依赖 swiftc / Xcode，`cargo run -- setup` 的依赖检查从 swiftc 改为 ffmpeg + tesseract。

---

## 后端 API 设计

Rust 后端（`axum`），在现有 CLI 的 `sources::fetch()` + `textifier::process()` 基础上封装 HTTP 接口，**不调 analyzer**。新增 `splitter` 模块负责将 TextContent 拆分为 items。

### Endpoints

```
POST /process
  Body: {
    url: String,
    asr_model?: String,    // 默认 "qwen3-asr-0.6b"
    ocr_images?: bool,     // 默认 true，false 则跳过图片/视频帧 OCR
  }
  Response (SSE): text/event-stream
    event: progress  → { stage: "fetching" | "downloading" | "ocr" | "asr" }
    event: result    → { title: String, items: Vec<String> }
    event: error     → { code: String, message: String }

GET /health
  Response: {
    status: "ok",
    deps: {
      ffmpeg: bool,
      tesseract: bool,
      tesseract_chi_sim: bool,
      qwen_asr: bool,
      qwen_asr_model: bool,
    }
  }
```

### SSE 事件流

```
POST /process
  → SSE: event: progress, data: {"stage":"fetching"}          // 抓取页面（sources::fetch）
  → SSE: event: progress, data: {"stage":"downloading"}       // 下载视频/图片
  → SSE: event: progress, data: {"stage":"ocr"}               // 帧 OCR（视频）或图片 OCR（图文）
  → SSE: event: progress, data: {"stage":"asr"}               // 音频转写（仅视频笔记）
  → SSE: event: result,   data: {"title":"...","items":[...]} // 最终结果
  → 连接关闭

出错时:
  → SSE: event: error,    data: {"code":"INVALID_URL","message":"..."}
  → 连接关闭
```

图文笔记无 `asr` 阶段；视频笔记中 `ocr` 与 `asr` 并行执行，各自完成后发出 progress 事件，两者全部完成后发 result。

### items 拆分规则（splitter 模块）

后端根据内容类型决定 `items` 拆分方式。该逻辑放在新增的 `server/src/splitter.rs` 中：

| 内容类型 | items 规则 |
|---------|-----------|
| 视频笔记 | `[描述 + ASR 转写]` 单条 |
| 图文笔记（单篇） | `[描述 + 全部 OCR]` 单条 |
| 合集帖（如 "11道菜"） | `[描述 + 图片1 OCR, 描述 + 图片2 OCR, ...]` 逐条 |

笔记类型由 `RawContent.content_type` 字段提供（scraper 在抓取阶段已判断）。splitter 同时接收 `&RawContent` 和 `&TextContent`，从 RawContent 读取类型、从 TextContent 读取各图片 OCR 文本，按规则拆分为 items。当 `content_type` 不可靠时，回退到启发式检测：图片数量 > 1 且描述中含合集关键词（如"合集"、"第X道"等）。

### 错误码

| code | 含义 |
|------|------|
| `INVALID_URL` | URL 格式不正确或非支持平台 |
| `FETCH_FAILED` | 页面抓取失败（网络错误、404、反爬） |
| `UNSUPPORTED_CONTENT` | 页面不含有效内容（非笔记页面、纯文字无视频无图） |
| `DOWNLOAD_FAILED` | 视频/图片下载失败 |
| `PROCESSING_FAILED` | ASR / OCR 处理异常 |
| `TIMEOUT` | 处理超时（超过 5 分钟） |
| `INTERNAL_ERROR` | 服务器内部未知错误 |

### 并发与资源管理

- **单请求串行**：每个请求内 sources → textifier → splitter 顺序执行
- **多请求并发**：tokio 多任务并发，通过 `Semaphore` 限制并发数（如 3）
- **临时文件**：每个请求创建独立 `tempdir`，请求结束后（无论成功失败）自动清理
- **超时**：单请求整体超时 5 分钟，超时后发送 `error` 事件（code: `TIMEOUT`）并关闭连接
- **客户端断开**：SSE 连接断开时，自动取消对应请求的后台任务（tokio `CancellationToken` 或 `AbortHandle`），释放临时文件和网络资源

---

## 后端鉴权

| 部署模式 | 方案 |
|---------|------|
| **本地同机** | `axum` bind `127.0.0.1` 仅监听本地回环，无鉴权 |
| **局域网** | 首次启动生成随机 token，前端通过设置页手动输入 |
| **自托管云** | `X-API-Key` header + HTTPS（推荐 nginx 反向代理 + Let's Encrypt） |

MVP 阶段仅支持本地同机，鉴权通过 localhost-only bind 保证。

---

## Flutter 前端

### 页面 / 路由

```
/                     → HomePage（历史列表 + 本地存储）
/extract              → ExtractPage（输入 URL → 发送到后端 → 调 LLM → 展示）
/recipe/:id           → RecipeDetailPage
/recipe/:id/edit      → RecipeEditPage
/settings             → SettingsPage（API Key, 后端地址, ASR 模型, 主题）
```

### 前端依赖

| 功能 | 依赖 | 说明 |
|------|------|------|
| HTTP 请求 | `dio` | 调用后端（含 SSE 流）+ 调 LLM |
| 本地存储 | `drift` (SQLite) | 独立维护前端本地菜谱数据 |
| 安全存储 | `flutter_secure_storage` | 存 LLM API key |
| 状态管理 | `riverpod` | 与异步请求天然契合 |

注意：**Flutter 端不编译任何 Rust 代码**，不依赖 flutter_rust_bridge。前端不做页面爬取，只发 URL 给后端。
前端 SQLite 存储独立于后端 `~/.xhs-recipe/recipes/`，两者不共享。

### LLM 调用

现有 `analyzer.rs` 的 function calling 逻辑需移植为 Dart 代码。这部分工作量较大（JSON schema 定义、function calling 调用、结果校验与重试），MVP 阶段提供两种方式：

- **方式 A（推荐）**：移植到 Dart，前端直接调 DeepSeek API。优势：后端简单，API key 留在用户设备
- **方式 B（可选）**：server 增加 `POST /analyze` endpoint，由前端选择是否走服务端 LLM。前端传 API key（每次请求传 or 后端不存），后端调 LLM 后返回结构化 Recipe

MVP 优先实现方式 A，方式 B 按需添加。

---

## 各平台依赖状态

```
功能                  macOS    Linux    Windows    iOS    Android
──────────────────────────────────────────────────────────────
页面爬虫（后端）         ✓        ✓        —         —      —
后端通信（前端）         ✓        ✓        ✓         ✓      ✓
LLM 调用（前端）         ✓        ✓        ✓         ✓      ✓
──────────────────────────────────────────────────────────────
后端 (macOS/Linux 运行):
  视频帧提取            ✓        ✓         —
  OCR                   ✓        ✓         —
  音频转写(Qwen3-ASR)   ✓        ✓         —
```

移动端（iOS/Android）不参与任何页面爬取或媒体处理，只负责 UI 和 LLM 调用。

### OCR 策略（统一 ffmpeg + tesseract）

不区分平台，macOS 和 Linux 走同一套 ffmpeg + tesseract 路径，无需条件编译：

```
ocr_video_frames():
  1. ffmpeg -i <video> -vf fps=1/3 -q:v 2 -y <dir>/frame_%04d.png   // 每 3 秒抽一帧
  2. tesseract <frame.png> stdout -l chi_sim+eng                      // 逐帧 OCR
  3. 去重合并多帧结果
```

图片 OCR（图文笔记）同理：下载图片 → tesseract 识别。

**历史验证情况**：

- 第 1 步 ffmpeg 抽帧在 commit `14d2a87` 中验证过（当时配合 Vision OCR 使用，后被 `5b965e7` 替换为 AVAssetImageGenerator）
- 第 2 步 tesseract OCR 是全新方案，替代了原有的 macOS Vision framework（删除 `ensure_ocr_helper`、Swift 内嵌源码编译等逻辑，不再依赖 swiftc）
- 更早的 Python 版本（`72af130`）无 OCR，图片直接以 base64 送给支持视觉的 LLM

依赖检查：server 启动时检测以下工具，缺失则打印安装提示：

- `ffmpeg`：抽帧
- `tesseract` + `chi_sim` 语言包：OCR 中文识别
- `qwen-asr` + 已下载的 ASR 模型：音频转写

安装方式：

| 工具 | macOS | Linux |
|------|-------|-------|
| ffmpeg | `brew install ffmpeg` | `apt install ffmpeg` |
| tesseract | `brew install tesseract` | `apt install tesseract-ocr` |
| chi_sim | 手动下载 [`chi_sim.traineddata`](https://github.com/tesseract-ocr/tessdata/raw/main/chi_sim.traineddata) → `/opt/homebrew/share/tessdata/` 或 `/usr/local/share/tessdata/` | `apt install tesseract-ocr-chi-sim` |
| qwen-asr | `cargo install qwen-asr-cli` | 同左 |
| ASR 模型 | `qwen-asr download qwen3-asr-0.6b` | 同左 |

上述依赖检测结果通过 `GET /health` 的 `deps` 字段暴露给前端，前端可在设置页展示环境状态。

### textifier 进度回调

当前 textifier 通过 `println!` 输出进度，服务端需要改为回调机制以驱动 SSE：

```rust
// 回调 trait（定义在 lib crate）
pub trait ProgressCallback: Send + Sync {
    fn progress(&self, stage: &str);
}

// textifier::process() 增加回调参数
pub async fn process(
    raw: &RawContent,
    asr_model: &str,
    ocr_images: bool,
    on_progress: Option<&dyn ProgressCallback>,  // None 时退化为 println!
) -> Result<TextContent, TextifierError>
```

- CLI 传 `None`，保持原有 println! 行为
- server 传入 SSE channel sender，将 progress 转发为 SSE 事件
- 回调 trait 极简（只有一个方法），不引入重量级依赖

---

## 后端部署方式

| 方式 | 适用场景 | 说明 |
|------|---------|------|
| **本地同机** | 桌面端 App | 用户在 macOS/Linux 上运行后端，前端连接 localhost |
| **局域网** | 家庭/办公环境 | 后端跑在 NAS 或 Linux 服务器上，手机 App 连接内网地址 |
| **自托管云** | 跨网络使用 | 后端部署在 VPS，前端配置远程地址（需配置 token + HTTPS） |

推荐：**MVP 阶段仅支持本地同机**，后端由 Flutter 桌面端自动启动子进程。

### 后端子进程管理（Flutter 桌面端）

```
Flutter App 启动
  → 检查 localhost:3000 是否已有服务端运行（GET /health）
  → 若无，按以下优先级查找 xhs-recipe-server 二进制：
      1. $PATH 中的 xhs-recipe-server
      2. Flutter App 同目录下的 xhs-recipe-server
  → spawn 子进程: xhs-recipe-server --port 3000
  → 轮询 GET /health 直到 deps 全部就绪或超时（30s）
  → 连接就绪，进入 App 主界面

Flutter App 退出
  → 向子进程发 SIGTERM
  → 等待 5s 超时后 SIGKILL
```

端口：默认 `3000`，可通过 server 的 `--port` 参数或前端设置页修改。

---

## 项目目录结构

```
xhs-recipe/
├── Cargo.toml              # workspace root
├── architecture.md
├── src/                    # 现有 CLI 代码保持不变
│   ├── main.rs             # CLI binary
│   ├── lib.rs
│   ├── models.rs
│   ├── pipeline.rs
│   ├── textifier.rs        # ffmpeg 抽帧 + tesseract OCR（全平台统一）
│   ├── analyzer.rs         # CLI 保留 analyzer，server 不依赖它
│   ├── sources/
│   ├── storage/
│   └── presentation/
├── server/                 # 新增：axum HTTP server
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs         # axum 启动 + 路由注册
│       ├── routes.rs       # POST /process handler
│       ├── splitter.rs     # items 拆分逻辑
│       └── error.rs        # 错误码定义
├── flutter_app/
│   ├── lib/
│   │   ├── main.dart
│   │   ├── app.dart
│   │   ├── router.dart
│   │   └── src/
│   │       ├── api/           # 后端 HTTP 客户端 + DeepSeek LLM 客户端
│   │       ├── providers/     # Riverpod providers
│   │       ├── pages/         # 页面
│   │       ├── widgets/       # 可复用组件
│   │       └── models/        # Dart 数据类
│   ├── android/
│   ├── ios/
│   ├── macos/
│   └── windows/
└── README.md
```

CLI 的 `analyzer.rs` 保持不变（CLI 仍需它），server 只依赖 `sources` + `textifier` + `models`。

---

## 预期工作量

| 阶段 | 内容 | 预估时间 | 产出 |
|------|------|----------|------|
| **MVP** | Cargo workspace 改造 + server（axum + SSE /process API + splitter）+ textifier 统一 ffmpeg+tesseract OCR + Flutter 桌面端（输入 URL → 展示 recipe，SSE 进度）+ Dart 版 analyzer | 2-3 周 | 端到端 macOS + Linux 跑通 |
| **V2** | 详情/编辑页 + SQLite 存储 + 列表页 + 历史记录 | +1 周 | 完整 CRUD |
| **移动端** | iOS/Android Flutter 界面适配 + 后端地址配置 | +1 周 | 全平台覆盖 |
| **V3** | 局域网部署 + token 鉴权 | +1 周 | 局域网可用 |
