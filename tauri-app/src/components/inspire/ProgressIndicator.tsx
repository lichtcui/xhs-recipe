import { Progress } from "@/components/ui/progress";
import { Badge } from "@/components/ui/badge";
import { STAGE_ORDER, STAGE_LABELS } from "@/lib/helpers";

interface ProgressIndicatorProps {
  stage: string;
  detail: string;
  percent: number;
}

export default function ProgressIndicator({
  stage,
  detail,
  percent,
}: ProgressIndicatorProps) {
  const stageIdx = STAGE_ORDER.indexOf(stage);

  return (
    <div className="space-y-3">
      <div className="flex items-center gap-1 flex-wrap">
        {STAGE_ORDER.map((s, i) => (
          <div key={s} className="flex items-center gap-1">
            {i > 0 && (
              <span className="text-xs text-muted-foreground mx-0.5">→</span>
            )}
            <Badge
              variant={
                i < stageIdx
                  ? "default"
                  : i === stageIdx
                    ? "destructive"
                    : "secondary"
              }
              className={i < stageIdx ? "bg-green-500 hover:bg-green-500" : ""}
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
  );
}
