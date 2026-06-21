import { useState, useCallback } from "react";
import { useSettings } from "@/hooks/useSettings";
import { streamAnalyze } from "@/lib/llm";
import { tryParseJson } from "@/lib/json-repair";
import { analyzeRecipe } from "@/lib/tauri";
import type { Recipe } from "@/types/recipe";

interface UseLlmStreamReturn {
  streamText: (rawText: string) => Promise<Recipe[]>;
  isStreaming: boolean;
  tokens: string;
  error: string | null;
}

export function useLlmStream(): UseLlmStreamReturn {
  const { settings } = useSettings();
  const [isStreaming, setIsStreaming] = useState(false);
  const [tokens, setTokens] = useState("");
  const [error, setError] = useState<string | null>(null);

  const streamText = useCallback(
    async (rawText: string): Promise<Recipe[]> => {
      setIsStreaming(true);
      setTokens("");
      setError(null);

      // Try frontend streaming first
      if (settings.apiKey) {
        try {
          const recipe = await new Promise<Recipe[]>((resolve, reject) => {
            streamAnalyze(
              rawText,
              settings.apiKey,
              settings.llmModel,
              {
                onToken: (token) => {
                  setTokens((prev) => prev + token);
                },
                onDone: (fullText) => {
                  try {
                    const parsed = tryParseJson(fullText) as Record<string, unknown>;
                    // Handle both { recipes: [...] } and direct recipe object
                    if (Array.isArray(parsed?.recipes)) {
                      resolve(parsed.recipes as Recipe[]);
                    } else {
                      resolve([parsed as unknown as Recipe]);
                    }
                  } catch (err) {
                    reject(err);
                  }
                },
                onError: (err) => {
                  reject(err);
                },
              }
            );
          });
          setIsStreaming(false);
          return recipe;
        } catch {
          // Fallback to Rust analyze
          setError("流式生成失败，回退到后端分析...");
        }
      }

      // Fallback: use Rust analyze_recipe command
      try {
        const recipes = await analyzeRecipe(
          rawText,
          settings.llmModel,
          settings.apiKey || undefined
        );
        setIsStreaming(false);
        setError(null);
        return recipes;
      } catch (err) {
        setIsStreaming(false);
        const msg = String(err);
        setError(msg);
        throw new Error(msg);
      }
    },
    [settings.apiKey, settings.llmModel]
  );

  return { streamText, isStreaming, tokens, error };
}
