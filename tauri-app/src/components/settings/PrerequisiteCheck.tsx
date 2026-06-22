import { useState, useEffect, useCallback } from "react";
import { checkPrerequisites } from "@/lib/tauri";
import type { PrerequisiteStatus } from "@/types/recipe";

export default function PrerequisiteCheck() {
  const [status, setStatus] = useState<PrerequisiteStatus | null>(null);
  const [error, setError] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    try {
      const s = await checkPrerequisites();
      setStatus(s);
    } catch (e) {
      setError(String(e));
    }
  }, []);

  useEffect(() => {
    refresh();
  }, [refresh]);

  return (
    <div>
      <h3 className="text-base font-semibold text-gray-600 mb-3">
        前置依赖
      </h3>
      {error ? (
        <p className="text-sm text-red-500">检测失败: {error}</p>
      ) : status ? (
        <div className="space-y-1 text-sm">
          <p>ffmpeg: {status.ffmpeg ? "✅" : "❌ 未安装"}</p>
          <p>tesseract: {status.tesseract ? "✅" : "❌ 未安装"}</p>
          <p>qwen-asr: {status.qwen_asr ? "✅" : "❌ 未安装"}</p>
        </div>
      ) : (
        <p className="text-sm text-muted-foreground">检测中...</p>
      )}
    </div>
  );
}
