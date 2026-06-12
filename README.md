# xhs-recipe

从小红书（Xiaohongshu / RedNote）帖子链接中提取结构化中文食谱的 CLI 工具。

## 功能

- 抓取小红书食谱帖子的文字和图片
- 使用 Qwen3-ASR 将视频音频转写为文字
- 通过 DeepSeek LLM 提取结构化食谱数据（食材、步骤、小贴士）
- 输出到终端（彩色）、Markdown 或 JSON

## 前置依赖

- [ffmpeg](https://ffmpeg.org/) — `brew install ffmpeg`
- [yt-dlp](https://github.com/yt-dlp/yt-dlp) — `brew install yt-dlp`
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

在 `.env` 文件或环境变量中设置 `DEEPSEEK_API_KEY`：

```bash
echo "DEEPSEEK_API_KEY=sk-your-key" > .env
```

## 使用方法

```bash
# 提取食谱（自动保存到本地，重复提取同一链接会直接显示缓存）
cargo run -- extract <xhs-url>

# 保存为 Markdown 文件
cargo run -- extract <xhs-url> -o recipe.md

# 不发送图片给 LLM
cargo run -- extract <xhs-url> --no-images

# 使用更高精度的 ASR 模型
cargo run -- extract <xhs-url> --asr-model qwen3-asr-1.7b
```

### 本地存储管理

每次提取的食谱自动保存在 `~/.xhs-recipe/recipes/`。同一 URL 重复提取时直接显示缓存内容，跳过抓取和 LLM 调用。

```bash
# 列出所有已保存的食谱
cargo run -- list

# 查看某个食谱详情
cargo run -- show <id>
```

### 其他命令

```bash
# 手动登录（如果自动 cookie 失败）
cargo run -- login [--headless]

# 清除已保存的 cookie
cargo run -- logout

# 检查依赖
cargo run -- setup
```

## 工作原理

```
URL → Source Adapter → Textifier → Analyzer → Presentation
```

1. **Source Adapter** — 浏览器自动化抓取小红书页面，提取文字和图片
2. **Textifier** — 通过 yt-dlp 下载视频，提取音频，使用 Qwen3-ASR 转写
3. **Analyzer** — 将文字（和可选图片）发送给 DeepSeek API，通过 function calling 返回结构化 `Recipe`
4. **Presentation** — 渲染到终端、Markdown 或 JSON

## 项目结构

```
src/
├── main.rs               # CLI 入口（clap）
├── lib.rs                # 库根模块
├── models.rs             # 数据模型（serde）
├── pipeline.rs           # 编排：fetch → textify → analyze
├── textifier.rs          # yt-dlp + ffmpeg + ASR
├── analyzer.rs           # LLM function calling
├── sources/
│   ├── xiaohongshu/      # 小红书适配器
│   │   ├── auth.rs       # Cookie / 登录
│   │   ├── scraper.rs    # 抓取降级策略
│   │   └── url.rs        # URL 解析
│   └── ...
└── presentation/
    ├── render.rs         # 终端输出
    └── save.rs           # .md / .json 保存
```

## 测试

```bash
cargo test
cargo test --lib             # 仅库测试
cargo test --bin xhs-recipe  # 仅 CLI 测试
```

## 许可证

MIT
