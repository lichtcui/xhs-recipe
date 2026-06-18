import { useState, useCallback, useEffect } from "react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Progress } from "@/components/ui/progress";
import { Badge } from "@/components/ui/badge";
import { Alert, AlertDescription } from "@/components/ui/alert";
import { AlertCircle } from "lucide-react";
import { useSettings } from "@/hooks/useSettings";
import { extractRecipe, onExtractProgress } from "@/lib/tauri";
import {
  classifyError,
  STAGE_ORDER,
  STAGE_PERCENT,
  STAGE_LABELS,
} from "@/lib/helpers";
import type { Recipe } from "@/types/recipe";

interface ExtractSectionProps {
  onExtracted: (recipes: Recipe[]) => void;
}

export default function ExtractSection({ onExtracted }: ExtractSectionProps) {
  const { getExtractPayload } = useSettings();
  const [url, setUrl] = useState("");
  const [extracting, setExtracting] = useState(false);
  const [stage, setStage] = useState("");
  const [detail, setDetail] = useState("");
  const [percent, setPercent] = useState(0);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    onExtractProgress((event) => {
      setStage(event.stage);
      setDetail(event.detail);
      setPercent(STAGE_PERCENT[event.stage] || 0);
    }).then((fn) => {
      unlisten = fn;
    });
    return () => {
      unlisten?.();
    };
  }, []);

  const handleExtract = useCallback(async () => {
    const trimmed = url.trim();
    if (!trimmed) return;

    setExtracting(true);
    setError(null);
    setStage("fetching");
    setDetail("正在抓取页面...");
    setPercent(5);

    try {
      const recipes = await extractRecipe(trimmed, getExtractPayload());
      setStage("done");
      setDetail("提取完成!");
      setPercent(100);
      onExtracted(recipes);
    } catch (e) {
      setError(classifyError(String(e)));
      setStage("");
      setPercent(0);
    } finally {
      setExtracting(false);
    }
  }, [url, getExtractPayload, onExtracted]);

  const stageIdx = STAGE_ORDER.indexOf(stage);

  return (
    <Card>
      <CardHeader className="pb-3">
        <CardTitle className="text-lg">提取菜谱</CardTitle>
      </CardHeader>
      <CardContent className="space-y-4 pt-0">
        <div className="flex gap-2">
          <Input
            placeholder="粘贴小红书分享链接..."
            value={url}
            onChange={(e) => setUrl(e.target.value)}
            disabled={extracting}
            className="flex-1"
            autoComplete="off"
          />
          <Button
            onClick={handleExtract}
            disabled={extracting || !url.trim()}
            className="bg-xhs hover:bg-xhs-hover"
          >
            提取
          </Button>
        </div>

        {extracting && stage && (
          <div className="space-y-3">
            <div className="flex items-center gap-1 flex-wrap">
              {STAGE_ORDER.map((s, i) => (
                <div key={s} className="flex items-center gap-1">
                  {i > 0 && (
                    <span className="text-xs text-muted-foreground mx-0.5">
                      →
                    </span>
                  )}
                  <Badge
                    variant={
                      i < stageIdx
                        ? "default"
                        : i === stageIdx
                          ? "destructive"
                          : "secondary"
                    }
                    className={
                      i < stageIdx ? "bg-green-500 hover:bg-green-500" : ""
                    }
                  >
                    {STAGE_LABELS[s] || s}
                  </Badge>
                </div>
              ))}
            </div>
            <Progress value={percent} className="h-1.5" />
            <p className="text-[13px] text-muted-foreground">
              {detail || "请稍候..."}
            </p>
          </div>
        )}

        {error && (
          <Alert variant="destructive">
            <AlertCircle className="h-4 w-4" />
            <AlertDescription>{error}</AlertDescription>
          </Alert>
        )}
      </CardContent>
    </Card>
  );
}
