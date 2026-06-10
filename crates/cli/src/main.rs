use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::process::Command as StdCmd;

#[derive(Parser)]
#[command(name = "xhs-recipe", about = "从社交媒体链接提取菜谱的 CLI 工具")]
struct Cli {
    #[command(subcommand)]
    command: Command,
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
        #[arg(long, default_value = "medium")]
        whisper_model: String,
        #[arg(long = "images", default_value_t = true)]
        images: bool,
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

    match cli.command {
        Command::Extract { url, output, model, whisper_model, images } => {
            run_extract(&url, output.as_deref(), &model, &whisper_model, images);
        }
        Command::Setup => run_setup(),
        Command::Login { headless, timeout } => run_login(headless, timeout),
        Command::Logout => run_logout(),
    }
}

fn run_extract(url: &str, output: Option<&std::path::Path>, model: &str, whisper_model: &str, images: bool) {
    println!("\n🔍 正在处理: {}", url);

    let rt = tokio::runtime::Runtime::new().unwrap();
    let opts = pipeline::ExtractOptions {
        url,
        whisper_model,
        llm_model: model,
        send_images: images,
        api_key: None,
    };

    match rt.block_on(pipeline::extract(opts)) {
        Ok(recipe) => {
            presentation::render::render_terminal(&recipe);
            if let Some(path) = output {
                if let Err(e) = presentation::save::save_to_file(&recipe, path) {
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
    println!("📦 检查系统依赖...");

    if which("ffmpeg").is_some() {
        println!("  ✓ ffmpeg 已安装");
    } else {
        println!("  ✗ ffmpeg 未安装");
        println!("    macOS: brew install ffmpeg");
    }

    if which("yt-dlp").is_some() {
        println!("  ✓ yt-dlp 已安装");
    } else {
        println!("  ✗ yt-dlp 未安装（pip install yt-dlp）");
    }

    println!();
    println!("📦 安装 Playwright 浏览器...");
    println!("  运行: playwright install chromium");
    println!();
    println!("🔑 配置 API Key");
    println!("  将 DEEPSEEK_API_KEY 添加到 .env 文件");
    println!("  或存入 macOS 钥匙串: security add-generic-password -a \"$USER\" -s deepseek-api -w \"sk-...\"");
    println!();
    println!("完成！运行 xhs-recipe extract <链接> 开始使用");
}

fn run_login(headless: bool, timeout: u32) {
    println!("📱 小红书登录");

    let script = find_script("login.py");
    let mut cmd = StdCmd::new("python3");
    cmd.arg(&script);
    if headless {
        cmd.arg("--headless");
    }
    cmd.arg("--timeout");
    cmd.arg(timeout.to_string());

    let status = cmd.status().unwrap_or_else(|e| {
        eprintln!("启动登录失败: {}", e);
        std::process::exit(1);
    });

    if status.success() {
        println!("\n现在可以运行 xhs-recipe extract 来提取菜谱了！");
    } else {
        std::process::exit(1);
    }
}

fn run_logout() {
    let script = find_script("logout.py");
    let status = StdCmd::new("python3")
        .arg(&script)
        .status()
        .unwrap_or_else(|e| {
            eprintln!("退出登录失败: {}", e);
            std::process::exit(1);
        });
    std::process::exit(if status.success() { 0 } else { 1 });
}

// ── Helpers ─────────────────────────────────────────────────────────

fn which(name: &str) -> Option<String> {
    let path = std::env::var_os("PATH").unwrap_or_default();
    for dir in std::env::split_paths(&path) {
        let candidate = dir.join(name);
        if candidate.exists() {
            return Some(candidate.to_string_lossy().to_string());
        }
    }
    None
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
            Command::Extract { url, model, output, whisper_model, images } => {
                assert_eq!(url, "http://xhslink.com/test");
                assert_eq!(model, "deepseek-chat");
                assert!(images);
                assert_eq!(whisper_model, "medium");
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
            "--whisper-model", "large-v3",
        ]).unwrap();
        match cli.command {
            Command::Extract { url, output, model, whisper_model, .. } => {
                assert_eq!(url, "http://xhslink.com/test");
                assert_eq!(output.unwrap().to_str().unwrap(), "recipe.md");
                assert_eq!(model, "claude-3-5-sonnet-20241022");
                assert_eq!(whisper_model, "large-v3");
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

fn find_script(name: &str) -> String {
    let candidates: Vec<String> = std::iter::once(format!("scripts/{}", name))
        .chain(std::iter::once(format!("../scripts/{}", name)))
        .chain(std::env::current_exe().ok().and_then(|exe| {
            let mut probe = exe.clone();
            probe.pop();
            for _ in 0..4 {
                let candidate = probe.join("scripts").join(name);
                if candidate.exists() {
                    return Some(candidate.to_string_lossy().to_string());
                }
                probe.pop();
            }
            None
        }))
        .collect();
    for c in &candidates {
        if std::path::Path::new(c).exists() {
            return c.clone();
        }
    }
    format!("scripts/{}", name)
}
