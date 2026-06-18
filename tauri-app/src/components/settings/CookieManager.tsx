import { useState, useEffect, useCallback } from "react";
import { Button } from "@/components/ui/button";
import { Textarea } from "@/components/ui/textarea";
import { Label } from "@/components/ui/label";
import { checkCookies, importCookies } from "@/lib/tauri";

export default function CookieManager() {
  const [cookieJson, setCookieJson] = useState("");
  const [cookieStatus, setCookieStatus] = useState<string>("检测中...");
  const [result, setResult] = useState<{
    text: string;
    ok: boolean;
  } | null>(null);

  const refresh = useCallback(async () => {
    try {
      const has = await checkCookies();
      setCookieStatus(has ? "已配置 ✅" : "未配置 ⚠️");
    } catch {
      setCookieStatus("检测失败");
    }
  }, []);

  useEffect(() => {
    refresh();
  }, [refresh]);

  const handleImport = async () => {
    const json = cookieJson.trim();
    if (!json) return;
    try {
      const msg = await importCookies(json);
      setResult({ text: msg, ok: true });
      setCookieJson("");
      refresh();
    } catch (e) {
      setResult({ text: `导入失败: ${e}`, ok: false });
    }
  };

  return (
    <div>
      <h3 className="text-base font-semibold text-gray-600 mb-3">
        Cookie 管理
      </h3>
      <p className="text-sm mb-3">
        状态：<span>{cookieStatus}</span>
      </p>
      <div className="space-y-2">
        <Label htmlFor="cookie-input" className="text-sm font-medium">
          导入 Cookie JSON
        </Label>
        <Textarea
          id="cookie-input"
          placeholder="粘贴浏览器导出的 Cookie JSON 数组..."
          rows={4}
          value={cookieJson}
          onChange={(e) => setCookieJson(e.target.value)}
        />
        <Button
          onClick={handleImport}
          className="bg-xhs hover:bg-xhs-hover"
          size="sm"
        >
          导入 Cookie
        </Button>
      </div>
      {result && (
        <p
          className={`text-[13px] mt-2 ${
            result.ok ? "text-green-600" : "text-red-600"
          }`}
        >
          {result.text}
        </p>
      )}
    </div>
  );
}
