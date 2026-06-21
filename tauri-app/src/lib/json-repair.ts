// Lightweight JSON repair for LLM streaming output.
// Handles the 3 most common issues:
// 1. Markdown code fences (```json ... ```)
// 2. Trailing commas (,] → ], ,} → })
// 3. Unclosed brackets/braces (truncation)

export function repairJson(raw: string): string {
  let text = raw.trim();

  // 1. Remove markdown code fences
  text = text.replace(/^```(?:json)?\s*\n?/i, "");
  text = text.replace(/\n?```\s*$/i, "");

  // 2. Remove trailing commas before closing brackets/braces
  text = text.replace(/,(\s*[}\]])/g, "$1");

  // 3. Count brackets to close unclosed ones
  const braces = (text.match(/\{/g) || []).length - (text.match(/\}/g) || []).length;
  const brackets = (text.match(/\[/g) || []).length - (text.match(/\]/g) || []).length;

  // 4. Close any unclosed string (trailing quote without matching close)
  const inString =
    (text.match(/(?<!\\)"/g) || []).length % 2 === 1;
  if (inString) {
    text += '"';
  }

  // 5. Close unclosed brackets
  for (let i = 0; i < brackets; i++) {
    text += "]";
  }
  for (let i = 0; i < braces; i++) {
    text += "}";
  }

  return text;
}

export function tryParseJson(text: string): unknown {
  try {
    return JSON.parse(text);
  } catch {
    const repaired = repairJson(text);
    return JSON.parse(repaired);
  }
}
