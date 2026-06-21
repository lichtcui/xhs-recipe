use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::sync::atomic::Ordering;
use xhs_recipe::storage::{local::LocalStorage, Storage};

#[derive(Parser)]
#[command(name = "xhs-recipe", version = env!("CARGO_PKG_VERSION"), about = "从社交媒体链接提取菜谱的 CLI 工具")]
struct Cli {
    #[command(subcommand)]
    command: Command,
    #[arg(short, long, global = true, help = "Show verbose output")]
    verbose: bool,
}

#[derive(Subcommand)]
enum Command {
    /// 从社交媒体链接提取菜谱
    Extract {
        url: String,
        #[arg(short, long)]
        output: Option<PathBuf>,
        #[arg(short, long, default_value = "deepseek-chat")]
        model: String,
        #[arg(long, default_value = "qwen3-asr-1.7b")]
        asr_model: String,
        #[arg(long = "no-images", action = clap::ArgAction::SetFalse)]
        images: bool,
        #[arg(short, long, default_value_t = 300, help = "Extraction timeout in seconds")]
        timeout: u64,
    },
    /// 初始化项目环境
    Setup,
    /// 列出本地已保存的菜谱
    List {
        #[arg(short, long, help = "Show all fields including IDs")]
        verbose: bool,
    },
    /// 查看本地已保存的菜谱详情
    Show {
        id: String,
    },
}

fn main() {
    dotenvy::dotenv().ok();
    let cli = Cli::parse();
    xhs_recipe::VERBOSE.store(cli.verbose, Ordering::Relaxed);

    match cli.command {
        Command::Extract { url, output, model, asr_model, images, timeout } => {
            run_extract(&url, output.as_deref(), &model, &asr_model, images, timeout);
        }
        Command::Setup => run_setup(),
        Command::List { verbose } => run_list(verbose),
        Command::Show { id } => run_show(&id),
    }
}

fn run_extract(url: &str, output: Option<&std::path::Path>, model: &str, asr_model: &str, images: bool, timeout: u64) {
    let rt = tokio::runtime::Runtime::new().expect("tokio runtime init");

    // Check cache first
    let store = LocalStorage::default();
    match rt.block_on(store.get_by_source_url(url)) {
        Ok(recipes) if !recipes.is_empty() => {
            xhs_recipe::vprintln!("✓ 命中缓存，直接显示已保存的菜谱\n");
            xhs_recipe::presentation::render::render_terminal_multi(&recipes);
            return;
        }
        Ok(_) => { /* cache miss, continue to pipeline */ }
        Err(e) => {
            xhs_recipe::vprintln!("⚠ 读取本地缓存失败: {}（将继续提取）", e);
        }
    }

    xhs_recipe::vprintln!("\n🔍 正在处理: {}", url);

    let opts = xhs_recipe::pipeline::ExtractOptions {
        url,
        asr_model,
        llm_model: model,
        send_images: images,
        api_key: None,
        timeout_secs: timeout,
    };

    match rt.block_on(xhs_recipe::pipeline::extract(opts)) {
        Ok(mut recipes) => {
            xhs_recipe::presentation::render::render_terminal_multi(&recipes);

            // Auto-save only substantial food recipes to local storage
            let mut saved_ids: Vec<String> = Vec::new();
            for recipe in &mut recipes {
                if !recipe.is_food || !recipe.is_substantial() {
                    continue;
                }
                match rt.block_on(store.save(&*recipe)) {
                    Ok(id) => {
                        recipe.id = Some(id.clone());
                        saved_ids.push(id);
                    }
                    Err(e) => {
                        eprintln!("⚠ 本地保存失败: {}（已跳过）", e);
                    }
                }
            }

            if let Some(first_id) = saved_ids.first() {
                let short = &first_id[..12];
                println!("\n✓ 已保存 {} 个菜谱到本地 ({}...)。运行 `xhs-recipe list` 查看", saved_ids.len(), short);
            }

            if let Some(path) = output {
                if let Err(e) = xhs_recipe::presentation::save::save_to_file(&recipes, path) {
                    eprintln!("保存失败: {}", e);
                } else {
                    println!("\n✓ 已保存到 {}", path.display());
                }
            }
        }
        Err(e) => {
            let msg = e.to_string();
            if msg.contains("需要登录") {
                eprintln!("\n{}", msg);
                eprintln!("\n提示: 小红书页面需要登录才能访问，请确保 Cookie 有效");
                eprintln!("  或尝试使用 --no-images 跳过图片 OCR 以降低被限流概率");
            } else if msg.contains("API key") {
                eprintln!("{}", msg);
            } else {
                eprintln!("处理失败: {}", msg);
            }
            std::process::exit(1);
        }
    }
}

fn run_setup() {
    let mut missing = Vec::new();

    println!("📦 检查系统依赖...");

    if which("qwen-asr").is_some() {
        println!("  ✓ qwen-asr 已安装");

        let model_dir = home_dir().join(".cache").join("qwen-asr").join("qwen3-asr-0.6b");
        if model_dir.exists() {
            println!("  ✓ Qwen3-ASR 模型 (0.6B) 已下载");
        } else {
            println!("  ↓ Qwen3-ASR 模型 (0.6B) 未下载");
            println!("    从 HuggingFace 下载:");
            println!("      qwen-asr download qwen3-asr-0.6b");
            println!("    从国内源（ModelScope）下载（推荐）:");
            println!("      brew install git-lfs && git lfs install");
            println!("      git clone https://www.modelscope.cn/Qwen/Qwen3-ASR-0.6B.git \\");
            println!("        ~/.cache/qwen-asr/qwen3-asr-0.6b");
            println!("      rm -rf ~/.cache/qwen-asr/qwen3-asr-0.6b/.git");
        }
    } else {
        println!("  ✗ qwen-asr 未安装");
        println!("    运行: cargo install qwen-asr-cli");
        println!("    然后: qwen-asr download qwen3-asr-0.6b");
        missing.push("qwen-asr");
    }

    if which("ffmpeg").is_some() {
        println!("  ✓ ffmpeg 已安装");
    } else {
        println!("  ✗ ffmpeg 未安装");
        println!("    运行: brew install ffmpeg");
        missing.push("ffmpeg");
    }

    if which("swiftc").is_some() {
        println!("  ✓ swiftc 已安装 (用于 Vision OCR)");
    } else {
        println!("  ✗ swiftc 未安装");
        println!("    macOS 系统应自带 swiftc，请安装 Xcode Command Line Tools:");
        println!("      xcode-select --install");
        missing.push("swiftc");
    }

    println!();
    println!("🔒 安全审计...");
    match which("cargo-audit") {
        Some(_) => println!("  ✓ cargo-audit 已安装（运行: cargo audit）"),
        None => {
            println!("  ✗ cargo-audit 未安装（cargo install cargo-audit）");
            missing.push("cargo-audit");
        }
    }

    println!();
    println!("🔑 配置 API Key");
    let env_key = std::env::var("DEEPSEEK_API_KEY").ok().filter(|k| !k.is_empty());
    let keychain_key = if cfg!(target_os = "macos") {
        std::process::Command::new("security")
            .args(["find-generic-password", "-a", &std::env::var("USER").unwrap_or_default(), "-s", "DEEPSEEK_API_KEY", "-w"])
            .output()
            .ok()
            .filter(|o| o.status.success())
            .and_then(|o| {
                let k = String::from_utf8_lossy(&o.stdout).trim().to_string();
                if k.is_empty() { None } else { Some(k) }
            })
    } else {
        None
    };
    if env_key.is_some() {
        println!("  ✓ DEEPSEEK_API_KEY（环境变量）");
    } else if keychain_key.is_some() {
        println!("  ✓ DEEPSEEK_API_KEY（macOS 钥匙串）");
    } else {
        println!("  DEEPSEEK_API_KEY 未设置。将密钥添加到 .env 文件：");
        println!("    DEEPSEEK_API_KEY=sk-...");
        println!("  或使用 macOS 钥匙串：");
        println!("    security add-generic-password -a \"$USER\" -s DEEPSEEK_API_KEY -w \"sk-...\"");
    }
    println!();

    if missing.is_empty() {
        println!("✅ 全部就绪！运行 xhs-recipe extract <链接> 开始使用");
    } else {
        println!("⚠ 缺少: {}。按上述指引安装后重试", missing.join(", "));
    }
}

/// 从 URL 提取菜谱并显示/保存
fn run_list(verbose: bool) {
    let rt = tokio::runtime::Runtime::new().expect("tokio runtime init");
    let store = LocalStorage::default();

    match rt.block_on(store.list()) {
        Ok(recipes) => {
            if recipes.is_empty() {
                println!("暂无已保存的菜谱。运行 `xhs-recipe extract <url>` 提取第一个菜谱。");
                return;
            }

            println!("已保存的菜谱 ({}):\n", recipes.len());
            let mut table = Vec::new();
            for r in &recipes {
                let short = &r.id[..12];
                let time = xhs_recipe::storage::local::relative_time(r.saved_at);
                let name = &r.name;
                if verbose {
                    table.push(format!("  {}...  {}  {}  {}", short, name, time, r.source_url));
                } else {
                    table.push(format!("  {}...  {}  {}", short, name, time));
                }
            }
            println!("{}", table.join("\n"));
            println!("\n运行 `xhs-recipe show <id>` 查看详情");
        }
        Err(e) => {
            eprintln!("读取本地存储失败: {}", e);
            std::process::exit(1);
        }
    }
}

fn run_show(id: &str) {
    let rt = tokio::runtime::Runtime::new().expect("tokio runtime init");
    let store = LocalStorage::default();

    match rt.block_on(store.get(id)) {
        Ok(recipe) => {
            xhs_recipe::presentation::render::render_terminal(&recipe);
        }
        Err(xhs_recipe::storage::StorageError::NotFound { .. }) => {
            eprintln!("未找到菜谱: {}\n运行 `xhs-recipe list` 查看所有已保存的菜谱。", id);
            std::process::exit(1);
        }
        Err(e) => {
            eprintln!("读取失败: {}", e);
            std::process::exit(1);
        }
    }
}

// ── Helpers ─────────────────────────────────────────────────────────

fn which(name: &str) -> Option<String> {
    xhs_recipe::which(name)
}

fn home_dir() -> std::path::PathBuf {
    xhs_recipe::home_dir()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cli_no_args() {
        let result = Cli::try_parse_from(["xhs-recipe"]);
        assert!(result.is_err()); // needs a subcommand
    }

    #[test]
    fn test_cli_extract_url() {
        let cli = Cli::try_parse_from(["xhs-recipe", "extract", "http://xhslink.com/test"]).unwrap();
        match cli.command {
            Command::Extract { url, model, output, asr_model, images, .. } => {
                assert_eq!(url, "http://xhslink.com/test");
                assert_eq!(model, "deepseek-chat");
                assert!(images);
                assert_eq!(asr_model, "qwen3-asr-0.6b");
                assert!(output.is_none());
            }
            _ => panic!("expected Extract"),
        }
    }

    #[test]
    fn test_cli_extract_all_options() {
        let cli = Cli::try_parse_from([
            "xhs-recipe", "extract",
            "http://xhslink.com/test",
            "--output", "recipe.md",
            "--model", "claude-3-5-sonnet-20241022",
            "--asr-model", "qwen3-asr-1.7b",
        ]).unwrap();
        match cli.command {
            Command::Extract { url, output, model, asr_model, .. } => {
                assert_eq!(url, "http://xhslink.com/test");
                assert_eq!(output.unwrap().to_str().unwrap(), "recipe.md");
                assert_eq!(model, "claude-3-5-sonnet-20241022");
                assert_eq!(asr_model, "qwen3-asr-1.7b");
            }
            _ => panic!("expected Extract"),
        }
    }

    #[test]
    fn test_cli_extract_no_images() {
        let cli = Cli::try_parse_from([
            "xhs-recipe", "extract",
            "http://xhslink.com/test",
            "--no-images",
        ]).unwrap();
        match cli.command {
            Command::Extract { images, .. } => {
                assert!(!images);
            }
            _ => panic!("expected Extract"),
        }
    }

    #[test]
    fn test_cli_setup() {
        let cli = Cli::try_parse_from(["xhs-recipe", "setup"]).unwrap();
        assert!(matches!(cli.command, Command::Setup));
    }

    #[test]
    fn test_cli_list() {
        let cli = Cli::try_parse_from(["xhs-recipe", "list"]).unwrap();
        assert!(matches!(cli.command, Command::List { .. }));
    }

    #[test]
    fn test_cli_list_verbose() {
        let cli = Cli::try_parse_from(["xhs-recipe", "list", "--verbose"]).unwrap();
        match cli.command {
            Command::List { verbose } => assert!(verbose),
            _ => panic!("expected List"),
        }
    }

    #[test]
    fn test_cli_show() {
        let cli = Cli::try_parse_from(["xhs-recipe", "show", "abc123"]).unwrap();
        match cli.command {
            Command::Show { id } => assert_eq!(id, "abc123"),
            _ => panic!("expected Show"),
        }
    }
}
