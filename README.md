# xhs-recipe

从小红书（Xiaohongshu / RedNote）帖子链接中提取结构化中文食谱的 CLI 工具 + HTTP 服务器 + Tauri 桌面应用。

## 功能

- 抓取小红书食谱帖子的文字和图片
- 使用 Qwen3-ASR 将视频音频转写为文字
- 视频画面文字识别（ffmpeg 抽帧 + tesseract OCR）
- 图文笔记图片文字识别（ffmpeg + tesseract OCR）
- 通过 DeepSeek LLM 提取结构化食谱数据（食材、步骤、小贴士）
- 合集多菜谱自动分批提取
- 输出到终端（彩色）、Markdown 或 JSON
- 内置 HTTP server（axum），支持 SSE 流式返回
- Tauri 桌面应用前端

## 前置依赖

- ffmpeg (`brew install ffmpeg` / `apt install ffmpeg`)
- tesseract + chi_sim 语言包 (`brew install tesseract`)
- [Qwen3-ASR](https://github.com/Qwen/Qwen3-ASR)（用于视频转写）：
  ```bash
  cargo install qwen-asr-cli
  qwen-asr download qwen3-asr-0.6b
  ```

### 国内用户（ModelScope 下载 Qwen3-ASR）

```bash
brew install git-lfs && git lfs install
git clone https://www.modelscope.cn/Qwen/Qwen3-ASR-0.6B.git \
  ~/.cache/qwen-asr/qwen3-asr-0.6b
rm -rf ~/.cache/qwen-asr/qwen3-asr-0.6b/.git
```

## 安装

```bash
git clone <repo-url>
cd xhs-recipe
cargo build
```

## 配置

需要 DeepSeek API key。按以下 **任意一种方式** 配置 `DEEPSEEK_API_KEY`：

### 方式 1：环境变量

```bash
export DEEPSEEK_API_KEY=sk-your-key
```

或写入 `.env` 文件（工具会自动加载）：

```bash
echo "DEEPSEEK_API_KEY=sk-your-key" > .env
```

### 方式 2：macOS 钥匙串（推荐）

存入钥匙串后无需每次设置环境变量：

```bash
# 添加
security add-generic-password -a "$USER" -s DEEPSEEK_API_KEY -w "sk-your-key"

# 更新（-U 覆盖已有条目）
security add-generic-password -a "$USER" -s DEEPSEEK_API_KEY -w "sk-new-key" -U

# 查看已保存的 key（仅输出最后 4 位）
security find-generic-password -s DEEPSEEK_API_KEY -w | sed 's/.*\(.\{4\}\)$/****\1/'

# 删除
security delete-generic-password -s DEEPSEEK_API_KEY
```

## 使用方法

```bash
# 提取食谱（自动保存到本地，重复提取同一链接会直接显示缓存）
cargo run -- extract <xhs-url>

# 保存为 Markdown 文件
cargo run -- extract <xhs-url> -o recipe.md

# 跳过图片 OCR
cargo run -- extract <xhs-url> --no-images

# 使用更高精度的 ASR 模型
cargo run -- extract <xhs-url> --asr-model qwen3-asr-1.7b

# 指定 LLM 模型
cargo run -- extract <xhs-url> --model deepseek-chat

# 设置超时时间（秒）
cargo run -- extract <xhs-url> --timeout 600
```

### 本地存储管理

每次提取的食谱自动保存在 `~/.xhs-recipe/recipes/`。同一 URL 重复提取时直接显示缓存内容，跳过抓取和 LLM 调用。

```bash
# 列出所有已保存的食谱
cargo run -- list

# 查看某个食谱详情
cargo run -- show <id>
```

### HTTP Server

```bash
# 启动 server（默认 3000 端口）
cargo run -p xhs-recipe-server

# 自定义端口
PORT=8080 cargo run -p xhs-recipe-server
```

Server 端点：
- `POST /process` — SSE 流式提取（`fetching → downloading → ocr → asr → result`）
- `GET /health` — 健康检查

### 其他命令

```bash
# 检查依赖
cargo run -- setup

# 安全审计
cargo audit
```

## 工作原理

```
URL → Source Adapter → Textifier → Analyzer → Presentation
```

1. **Source Adapter** — HTTP 抓取小红书页面，提取文字和图片
2. **Textifier** — 视频 → reqwest 下载 + symphonia 音频提取 + Qwen3-ASR 转写 + ffmpeg 抽帧 + tesseract OCR；图文笔记 → ffmpeg + tesseract OCR
3. **Analyzer** — OCR 文字 → DeepSeek API function calling → `Recipe` 模型
4. **Storage** — 自动保存到 `~/.xhs-recipe/recipes/`，同一 URL 重复提取自动去重
5. **Presentation** — 渲染到终端、Markdown 或 JSON

## 项目结构

```
xhs-recipe/
├── Cargo.toml              # workspace 根
├── src/                    # CLI/lib crate
│   ├── main.rs             # CLI 入口（clap）
│   ├── lib.rs              # 库根模块
│   ├── models.rs           # 数据模型（serde）
│   ├── pipeline.rs         # 编排：fetch → textify → analyze
│   ├── textifier.rs        # reqwest + symphonia + Qwen3-ASR + ffmpeg/tesseract OCR
│   ├── analyzer.rs         # LLM function calling (DeepSeek)
│   ├── sources/
│   │   ├── mod.rs          # Source 路由
│   │   ├── base.rs         # URL 路由 & 域检查
│   │   └── xiaohongshu/    # 小红书适配器
│   │       ├── auth.rs     # Cookie 管理
│   │       ├── scraper.rs  # 抓取 (reqwest HTTP)
│   │       └── url.rs      # URL 解析
│   ├── storage/
│   │   ├── mod.rs          # Storage trait
│   │   └── local.rs        # 本地文件存储 ~/.xhs-recipe/recipes/
│   └── presentation/
│       ├── render.rs       # 终端输出 (彩色)
│       └── save.rs         # .md / .json 保存
├── server/                 # HTTP server crate (xhs-recipe-server)
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs         # axum 启动
│       ├── routes.rs       # POST /process (SSE) + GET /health
│       ├── splitter.rs     # 内容拆分
│       └── error.rs        # 错误码
├── tauri-app/              # Tauri 桌面应用
│   ├── src-tauri/          # Rust 后端
│   │   ├── Cargo.toml
│   │   └── tauri.conf.json
│   ├── src/                # 前端 (JS)
│   │   ├── main.js
│   │   └── styles.css
│   └── index.html
└── tests/
    ├── integration.rs
    └── testdata/
        ├── recipe_test.json
        └── recipe_test.md
```

## 测试

```bash
cargo test                   # 全部测试（lib + bin + server）
cargo test -p xhs-recipe     # 仅 CLI/lib 测试
cargo test -p xhs-recipe-server  # 仅 server 测试
cargo audit                  # 安全审计（安装: cargo install cargo-audit）
```

## 许可证

MIT
