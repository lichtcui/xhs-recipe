import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

// ── State ─────────────────────────────────────────────

let currentRecipes = [];

// ── Settings ──────────────────────────────────────────

const SETTINGS_KEY = "xhs-recipe-settings";

function loadSettings() {
  const defaults = {
    asrModel: "qwen3-asr-0.6b",
    llmModel: "deepseek-chat",
    ocrImages: true,
    apiKey: "",
    timeout: 300,
  };
  try {
    const saved = localStorage.getItem(SETTINGS_KEY);
    if (saved) return { ...defaults, ...JSON.parse(saved) };
  } catch (e) {
    // ignore
  }
  return defaults;
}

function saveSettings(settings) {
  localStorage.setItem(SETTINGS_KEY, JSON.stringify(settings));
}

function getSettingsFromForm() {
  return {
    asrModel: document.getElementById("setting-asr-model").value,
    llmModel: document.getElementById("setting-llm-model").value,
    ocrImages: document.getElementById("setting-ocr").checked,
    apiKey: document.getElementById("setting-api-key").value || null,
    timeout: parseInt(document.getElementById("setting-timeout").value) || 300,
  };
}

function applySettingsToForm(settings) {
  document.getElementById("setting-asr-model").value = settings.asrModel;
  document.getElementById("setting-llm-model").value = settings.llmModel;
  document.getElementById("setting-ocr").checked = settings.ocrImages;
  document.getElementById("setting-api-key").value = settings.apiKey || "";
  document.getElementById("setting-timeout").value = settings.timeout;
}

// Auto-save on change
function bindSettingsAutoSave() {
  const ids = ["setting-asr-model", "setting-llm-model", "setting-ocr", "setting-api-key", "setting-timeout"];
  ids.forEach((id) => {
    const el = document.getElementById(id);
    if (!el) return;
    el.addEventListener("change", () => {
      saveSettings(getSettingsFromForm());
    });
    if (el.tagName === "INPUT" && el.type !== "checkbox") {
      el.addEventListener("input", () => {
        saveSettings(getSettingsFromForm());
      });
    }
  });
}

// ── Navigation ────────────────────────────────────────

function showPage(name) {
  document.querySelectorAll(".page").forEach((p) => p.classList.remove("active"));
  document.getElementById(`page-${name}`).classList.add("active");

  document.querySelectorAll(".nav-btn").forEach((b) => b.classList.remove("active"));
  const navBtn = document.querySelector(`[data-page="${name}"]`);
  if (navBtn) navBtn.classList.add("active");

  if (name === "home") loadRecipeList();
  if (name === "settings") refreshSettings();
}

document.querySelectorAll(".nav-btn").forEach((btn) => {
  btn.addEventListener("click", () => showPage(btn.dataset.page));
});

document.getElementById("back-btn").addEventListener("click", () => showPage("home"));

// ── Extract Flow ──────────────────────────────────────

const progressContainer = document.getElementById("progress-container");
const progressFill = document.getElementById("progress-fill");
const statusText = document.getElementById("status-text");
const errorContainer = document.getElementById("error-container");
const extractBtn = document.getElementById("extract-btn");
const urlInput = document.getElementById("url-input");

const STAGE_ORDER = ["fetching", "downloading", "ocr", "asr", "analyzing", "done"];
const STAGE_PERCENT = { fetching: 5, downloading: 20, ocr: 40, asr: 60, analyzing: 80, done: 100 };

function setProgressStage(stage) {
  const idx = STAGE_ORDER.indexOf(stage);
  const pct = STAGE_PERCENT[stage] || 0;
  progressFill.style.width = `${pct}%`;

  document.querySelectorAll(".stage").forEach((el) => {
    const s = el.dataset.stage;
    el.classList.remove("active", "done");
    const si = STAGE_ORDER.indexOf(s);
    if (si < idx) el.classList.add("done");
    if (si === idx) el.classList.add("active");
  });
}

function showError(message) {
  errorContainer.textContent = classifyError(message);
  errorContainer.classList.remove("hidden");
}

function classifyError(message) {
  if (message.includes("ffmpeg not found") || message.includes("ffmpeg")) {
    return `未找到 ffmpeg。请运行: brew install ffmpeg`;
  }
  if (message.includes("tesseract not found") || message.includes("tesseract")) {
    return `未找到 tesseract。请运行: brew install tesseract`;
  }
  if (message.includes("qwen-asr not found") || message.includes("qwen-asr")) {
    return `未找到 qwen-asr。请运行: cargo install qwen-asr-cli && qwen-asr download qwen3-asr-0.6b`;
  }
  if (message.includes("API key") || message.includes("DEEPSEEK_API_KEY") || message.includes("MissingApiKey")) {
    return `未配置 API Key。请在设置页面填写，或设置环境变量 DEEPSEEK_API_KEY。`;
  }
  if (message.includes("需要登录") || message.includes("cookie") || message.includes("Cookie")) {
    return `Cookie 可能已过期。请在浏览器重新登录小红书，导出 Cookie JSON 后在设置页面导入。`;
  }
  return `提取失败: ${message}`;
}

extractBtn.addEventListener("click", async () => {
  const url = urlInput.value.trim();
  if (!url) return;

  const settings = loadSettings();
  errorContainer.classList.add("hidden");
  progressContainer.classList.remove("hidden");
  extractBtn.disabled = true;
  setProgressStage("fetching");
  statusText.textContent = "正在抓取页面...";

  try {
    const recipes = await invoke("extract", {
      url,
      settings: {
        asr_model: settings.asrModel,
        ocr_images: settings.ocrImages,
        llm_model: settings.llmModel,
        api_key: settings.apiKey || null,
        timeout_secs: settings.timeout,
      },
    });

    currentRecipes = recipes;
    if (recipes.length === 1) {
      showRecipeDetail(recipes[0]);
    } else if (recipes.length > 1) {
      showRecipeDetail(recipes[0]);
    }
  } catch (e) {
    showError(String(e));
    setProgressStage("fetching");
    progressFill.style.width = "0%";
  } finally {
    extractBtn.disabled = false;
    setTimeout(() => {
      if (errorContainer.classList.contains("hidden")) {
        progressContainer.classList.add("hidden");
      }
    }, 2000);
  }
});

// Listen for progress events
listen("extract:progress", (event) => {
  const { stage, detail } = event.payload;
  setProgressStage(stage);
  if (detail) statusText.textContent = detail;
  else if (stage === "done") statusText.textContent = "提取完成!";
});

// ── Recipe List ───────────────────────────────────────

async function loadRecipeList() {
  const listEl = document.getElementById("recipe-list");
  try {
    const recipes = await invoke("list_recipes");
    if (!recipes || recipes.length === 0) {
      listEl.innerHTML = '<p class="empty-hint">暂无保存的菜谱</p>';
      return;
    }
    listEl.innerHTML = recipes
      .map(
        (r) => `
      <div class="recipe-card" data-id="${r.id}">
        <div class="recipe-card-main">
          <span class="recipe-name">${escHtml(r.name)}</span>
          <span class="recipe-url">${escHtml(truncateUrl(r.source_url))}</span>
        </div>
        <div class="recipe-card-actions">
          <button class="btn-sm view-btn" data-id="${r.id}">查看</button>
          <button class="btn-sm delete-btn" data-id="${r.id}">删除</button>
        </div>
      </div>`
      )
      .join("");

    // Bind click handlers
    listEl.querySelectorAll(".view-btn").forEach((btn) => {
      btn.addEventListener("click", async (e) => {
        e.stopPropagation();
        const id = btn.dataset.id;
        try {
          const recipe = await invoke("get_recipe", { id });
          showRecipeDetail(recipe);
        } catch (err) {
          alert("加载菜谱失败: " + err);
        }
      });
    });

    listEl.querySelectorAll(".delete-btn").forEach((btn) => {
      btn.addEventListener("click", async (e) => {
        e.stopPropagation();
        const id = btn.dataset.id;
        try {
          await invoke("delete_recipe", { id });
          loadRecipeList();
        } catch (err) {
          alert("删除失败: " + err);
        }
      });
    });

    listEl.querySelectorAll(".recipe-card").forEach((card) => {
      card.addEventListener("click", async () => {
        const id = card.dataset.id;
        try {
          const recipe = await invoke("get_recipe", { id });
          showRecipeDetail(recipe);
        } catch (err) {
          alert("加载菜谱失败: " + err);
        }
      });
    });
  } catch (e) {
    listEl.innerHTML = `<p class="empty-hint">加载失败: ${escHtml(String(e))}</p>`;
  }
}

// ── Recipe Detail ─────────────────────────────────────

function showRecipeDetail(recipe) {
  showPage("detail");
  const el = document.getElementById("detail-content");

  const ingredients = recipe.ingredients || [];
  const seasonings = recipe.seasonings || [];
  const steps = recipe.steps || [];
  const tips = recipe.tips || [];
  const equipment = recipe.equipment || [];

  let html = `
    <h2 class="recipe-title">${escHtml(recipe.name)}</h2>
    ${recipe.total_time ? `<p class="recipe-time">⏱ ${escHtml(recipe.total_time)}</p>` : ""}
    ${!recipe.is_food ? `<p class="not-food">非美食内容${recipe.reason ? `: ${escHtml(recipe.reason)}` : ""}</p>` : ""}
  `;

  if (ingredients.length) {
    html += `<h3>食材</h3><ul class="ingredient-list">`;
    ingredients.forEach((i) => {
      html += `<li><strong>${escHtml(i.name)}</strong>${i.amount ? ` — ${escHtml(i.amount)}` : ""}${i.prep ? `（${escHtml(i.prep)}）` : ""}</li>`;
    });
    html += `</ul>`;
  }

  if (seasonings.length) {
    html += `<h3>调料</h3><ul class="ingredient-list">`;
    seasonings.forEach((s) => {
      html += `<li><strong>${escHtml(s.name)}</strong>${s.amount ? ` — ${escHtml(s.amount)}` : ""}</li>`;
    });
    html += `</ul>`;
  }

  if (equipment.length) {
    html += `<h3>器具</h3><p>${equipment.map((e) => escHtml(e)).join("、")}</p>`;
  }

  if (steps.length) {
    html += `<h3>步骤</h3><ol class="step-list">`;
    steps.forEach((s) => {
      html += `<li><strong>${escHtml(s.title)}</strong>${s.time ? ` (${escHtml(s.time)})` : ""}<p>${escHtml(s.content)}</p></li>`;
    });
    html += `</ol>`;
  }

  if (tips.length) {
    html += `<h3>小贴士</h3><ul>`;
    tips.forEach((t) => {
      html += `<li>${escHtml(t)}</li>`;
    });
    html += `</ul>`;
  }

  el.innerHTML = html;
}

// ── Settings Page ────────────────────────────────────

async function refreshSettings() {
  const settings = loadSettings();
  applySettingsToForm(settings);
  await checkPrerequisites();
  await refreshCookieStatus();
}

async function checkPrerequisites() {
  const el = document.getElementById("prereq-status");
  try {
    const status = await invoke("check_prerequisites");
    el.innerHTML = `
      <p>ffmpeg: ${status.ffmpeg ? "✅" : "❌ 未安装"}</p>
      <p>tesseract: ${status.tesseract ? "✅" : "❌ 未安装"}</p>
      <p>qwen-asr: ${status.qwen_asr ? "✅" : "❌ 未安装"}</p>
      <p>Cookie: ${status.cookies_exist ? "✅ 已配置" : "⚠️ 未配置"}</p>
    `;
  } catch (e) {
    el.innerHTML = `<p>检测失败: ${escHtml(String(e))}</p>`;
  }
}

async function refreshCookieStatus() {
  const el = document.getElementById("cookie-status-text");
  try {
    const has = await invoke("check_cookies");
    el.textContent = has ? "已配置 ✅" : "未配置 ⚠️";
  } catch (e) {
    el.textContent = "检测失败";
  }
}

document.getElementById("import-cookies-btn").addEventListener("click", async () => {
  const json = document.getElementById("cookie-json-input").value.trim();
  if (!json) return;
  const resultEl = document.getElementById("cookie-import-result");
  try {
    const msg = await invoke("import_cookies", { cookieJson: json });
    resultEl.textContent = msg;
    resultEl.className = "cookie-result success";
    await refreshCookieStatus();
    document.getElementById("cookie-json-input").value = "";
  } catch (e) {
    resultEl.textContent = `导入失败: ${e}`;
    resultEl.className = "cookie-result error";
  }
});

// ── Helpers ───────────────────────────────────────────

function escHtml(s) {
  if (!s) return "";
  const div = document.createElement("div");
  div.textContent = s;
  return div.innerHTML;
}

function truncateUrl(url) {
  if (!url) return "";
  return url.length > 50 ? url.slice(0, 50) + "..." : url;
}

// ── Init ──────────────────────────────────────────────

document.addEventListener("DOMContentLoaded", () => {
  bindSettingsAutoSave();
  loadRecipeList();
});
