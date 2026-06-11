use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::sync::atomic::Ordering;

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
        #[arg(long, default_value = "qwen3-asr-0.6b")]
        asr_model: String,
        #[arg(long = "no-images", action = clap::ArgAction::SetFalse)]
        images: bool,
        #[arg(short, long, default_value_t = 300, help = "Extraction timeout in seconds")]
        timeout: u64,
    },
    /// 初始化项目环境
    Setup,
    /// 扫码登录小红书
    Login {
        #[arg(long)]
        headless: bool,
        #[arg(short, long, default_value_t = 120)]
        timeout: u32,
    },
    /// 清除已保存的 Cookie
    Logout,
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
        Command::Login { headless, timeout } => run_login(headless, timeout),
        Command::Logout => run_logout(),
    }
}

fn run_extract(url: &str, output: Option<&std::path::Path>, model: &str, asr_model: &str, images: bool, timeout: u64) {
    xhs_recipe::vprintln!("\n🔍 正在处理: {}", url);

    let rt = tokio::runtime::Runtime::new().expect("tokio runtime init");
    let opts = xhs_recipe::pipeline::ExtractOptions {
        url,
        asr_model,
        llm_model: model,
        send_images: images,
        api_key: None,
        timeout_secs: timeout,
    };

    match rt.block_on(xhs_recipe::pipeline::extract(opts)) {
        Ok(recipe) => {
            xhs_recipe::presentation::render::render_terminal(&recipe);
            if let Some(path) = output {
                if let Err(e) = xhs_recipe::presentation::save::save_to_file(&recipe, path) {
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
                eprintln!("\n提示: 小红书自动获取 Cookie 失败");
                eprintln!("  或者尝试手动扫码登录获取新的 Cookie：");
                eprintln!("  xhs-recipe login");
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

    if which("ffmpeg").is_some() {
        println!("  ✓ ffmpeg 已安装");
    } else {
        println!("  ✗ ffmpeg 未安装");
        println!("    macOS: brew install ffmpeg");
        missing.push("ffmpeg");
    }

    if which("yt-dlp").is_some() {
        println!("  ✓ yt-dlp 已安装");
    } else {
        println!("  ✗ yt-dlp 未安装（pip install yt-dlp）");
        missing.push("yt-dlp");
    }

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

    println!();
    println!("📦 安装 Playwright 浏览器...");
    println!("  运行: playwright install chromium");
    println!();
    println!("🔑 配置 API Key");
    if std::env::var("DEEPSEEK_API_KEY").ok().filter(|k| !k.is_empty()).is_some() {
        println!("  ✓ DEEPSEEK_API_KEY 已设置");
    } else {
        println!("  DEEPSEEK_API_KEY 未设置。将密钥添加到 .env 文件：");
        println!("    DEEPSEEK_API_KEY=sk-...");
        println!("  或使用 macOS 钥匙串：");
        println!("    security add-generic-password -a \"$USER\" -s deepseek-api -w \"sk-...\"");
    }
    println!();

    if missing.is_empty() {
        println!("✅ 全部就绪！运行 xhs-recipe extract <链接> 开始使用");
    } else {
        println!("⚠ 缺少: {}。按上述指引安装后重试", missing.join(", "));
    }
}

fn run_login(headless: bool, timeout: u32) {
    println!("📱 小红书登录");

    let rt = tokio::runtime::Runtime::new().expect("tokio runtime init");
    match rt.block_on(xhs_recipe::sources::xiaohongshu::auth::login(headless, timeout)) {
        Ok(true) => {
            println!("\n现在可以运行 xhs-recipe extract 来提取菜谱了！");
        }
        Ok(false) => {
            std::process::exit(1);
        }
        Err(e) => {
            eprintln!("登录失败: {}", e);
            std::process::exit(1);
        }
    }
}

fn run_logout() {
    let rt = tokio::runtime::Runtime::new().expect("tokio runtime init");
    rt.block_on(xhs_recipe::sources::xiaohongshu::auth::logout());
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
    fn test_cli_login() {
        let cli = Cli::try_parse_from(["xhs-recipe", "login"]).unwrap();
        match cli.command {
            Command::Login { headless, timeout } => {
                assert!(!headless);
                assert_eq!(timeout, 120);
            }
            _ => panic!("expected Login"),
        }
    }

    #[test]
    fn test_cli_login_with_options() {
        let cli = Cli::try_parse_from(["xhs-recipe", "login", "--headless", "--timeout", "60"]).unwrap();
        match cli.command {
            Command::Login { headless, timeout } => {
                assert!(headless);
                assert_eq!(timeout, 60);
            }
            _ => panic!("expected Login"),
        }
    }

    #[test]
    fn test_cli_logout() {
        let cli = Cli::try_parse_from(["xhs-recipe", "logout"]).unwrap();
        assert!(matches!(cli.command, Command::Logout));
    }
}
