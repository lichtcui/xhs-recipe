const DEEPSEEK_BASE = "https://api.deepseek.com";

const SYSTEM_PROMPT = `你是专业厨师和食谱分析师。请根据以下从小红书视频/图文提取的文字内容，整理成结构化菜谱。
必须严格按照 JSON 格式返回，Key 包含:
- name: 菜名
- total_time: 预估烹饪时间 (如 "45分钟")
- tags: 标签数组 (如 ["家常菜", "硬菜", "快手菜"])
- ingredients: [{name, amount?, prep?}] 食材清单
- seasonings: [{name, amount?}] 调料清单
- equipment: [string] 所需器具
- steps: [{title, time?, content}] 烹饪步骤
- tips: [string] 小贴士
- is_food: 是否为美食内容 (boolean)
- reason: 非美食时说明原因
如果缺少某个食材用量，请根据经验合理推算。`;

interface StreamCallbacks {
  onToken: (token: string) => void;
  onDone: (fullText: string) => void;
  onError: (error: Error) => void;
}

export async function streamAnalyze(
  rawText: string,
  apiKey: string,
  model: string,
  callbacks: StreamCallbacks
): Promise<void> {
  const body = JSON.stringify({
    model,
    max_tokens: 8192,
    stream: true,
    messages: [
      { role: "system", content: SYSTEM_PROMPT },
      {
        role: "user",
        content: `请根据以下内容提取菜谱:\n\n${rawText}`,
      },
    ],
    response_format: { type: "json_object" },
  });

  try {
    const response = await fetch(`${DEEPSEEK_BASE}/chat/completions`, {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
        Authorization: `Bearer ${apiKey}`,
      },
      body,
    });

    if (!response.ok) {
      const errText = await response.text().catch(() => "");
      throw new Error(`HTTP ${response.status}: ${errText.slice(0, 200)}`);
    }

    const reader = response.body?.getReader();
    if (!reader) throw new Error("No response body");

    const decoder = new TextDecoder();
    let fullText = "";

    while (true) {
      const { done, value } = await reader.read();
      if (done) break;

      const chunk = decoder.decode(value, { stream: true });
      const lines = chunk.split("\n");

      for (const line of lines) {
        const trimmed = line.trim();
        if (!trimmed.startsWith("data: ")) continue;

        const data = trimmed.slice(6);
        if (data === "[DONE]") {
          callbacks.onDone(fullText);
          return;
        }

        try {
          const parsed = JSON.parse(data);
          const delta = parsed.choices?.[0]?.delta?.content;
          if (delta) {
            fullText += delta;
            callbacks.onToken(delta);
          }
        } catch {
          // Skip malformed SSE lines
        }
      }
    }

    callbacks.onDone(fullText);
  } catch (err) {
    callbacks.onError(err instanceof Error ? err : new Error(String(err)));
  }
}
