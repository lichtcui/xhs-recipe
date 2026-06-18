export const STEP_NUMS = [
  "①",
  "②",
  "③",
  "④",
  "⑤",
  "⑥",
  "⑦",
  "⑧",
  "⑨",
  "⑩",
];

export const GENERIC_AMOUNTS = new Set([
  "适量",
  "少许",
  "适量即可",
  "少量",
  "若干",
  "一点",
]);

export function fmtAmount(amt?: string | null): string {
  if (!amt) return "";
  return GENERIC_AMOUNTS.has(amt.trim()) ? "" : ` ${amt}`;
}

export function truncateUrl(url: string, maxLen = 50): string {
  if (!url) return "";
  return url.length > maxLen ? url.slice(0, maxLen) + "..." : url;
}

export function classifyError(message: string): string {
  if (message.includes("ffmpeg not found") || message.includes("ffmpeg")) {
    return "未找到 ffmpeg。请运行: brew install ffmpeg";
  }
  if (message.includes("tesseract not found") || message.includes("tesseract")) {
    return "未找到 tesseract。请运行: brew install tesseract";
  }
  if (
    message.includes("qwen-asr not found") ||
    message.includes("qwen-asr")
  ) {
    return "未找到 qwen-asr。请运行: cargo install qwen-asr-cli && qwen-asr download qwen3-asr-0.6b";
  }
  if (
    message.includes("API key") ||
    message.includes("DEEPSEEK_API_KEY") ||
    message.includes("MissingApiKey")
  ) {
    return "未配置 API Key。请在设置页面填写，或设置环境变量 DEEPSEEK_API_KEY。";
  }
  if (
    message.includes("需要登录") ||
    message.includes("cookie") ||
    message.includes("Cookie")
  ) {
    return "Cookie 可能已过期。请在浏览器重新登录小红书，导出 Cookie JSON 后在设置页面导入。";
  }
  return `提取失败: ${message}`;
}

export const STAGE_ORDER = [
  "fetching",
  "downloading",
  "ocr",
  "asr",
  "analyzing",
  "done",
];

export const STAGE_PERCENT: Record<string, number> = {
  fetching: 5,
  downloading: 20,
  ocr: 40,
  asr: 60,
  analyzing: 80,
  done: 100,
};

export const STAGE_LABELS: Record<string, string> = {
  fetching: "抓取",
  downloading: "下载",
  ocr: "OCR",
  asr: "ASR",
  analyzing: "分析",
  done: "完成",
};
