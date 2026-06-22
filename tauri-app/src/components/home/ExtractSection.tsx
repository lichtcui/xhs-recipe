import { useEffect } from "react";
import { useExtractionState } from "@/hooks/useExtractionState";
import { Alert, AlertDescription } from "@/components/ui/alert";
import { AlertCircle } from "lucide-react";
import { classifyError } from "@/lib/helpers";
import UrlInputBar from "@/components/inspire/UrlInputBar";
import ProgressIndicator from "@/components/inspire/ProgressIndicator";
import type { Recipe } from "@/types/recipe";

interface ExtractSectionProps {
  onExtracted: (recipes: Recipe[]) => void;
  onBusyChange?: (busy: boolean) => void;
}

export default function ExtractSection({
  onExtracted,
  onBusyChange,
}: ExtractSectionProps) {
  const { state, startExtraction } = useExtractionState();
  const { status, progress, error } = state;

  // Report busy state to parent
  useEffect(() => {
    onBusyChange?.(status !== "IDLE");
  }, [status, onBusyChange]);

  const handleExtract = async (url: string) => {
    const recipes = await startExtraction(url);
    if (recipes.length > 0) {
      onExtracted(recipes);
    }
  };

  return (
    <div className="space-y-4">
      {/* Show URL input when idle or after extraction (hide during PARSING) */}
      {status !== "PARSING" && (
        <UrlInputBar onExtract={handleExtract} disabled={false} />
      )}

      {/* PARSING: progress indicator */}
      {status === "PARSING" && progress && (
        <ProgressIndicator
          stage={progress.stage}
          detail=""
          percent={progress.percent}
        />
      )}

      {/* ERROR state with retry */}
      {status === "IDLE" && error && (
        <Alert variant="destructive">
          <AlertCircle className="h-4 w-4" />
          <AlertDescription>{classifyError(error)}</AlertDescription>
        </Alert>
      )}
    </div>
  );
}
