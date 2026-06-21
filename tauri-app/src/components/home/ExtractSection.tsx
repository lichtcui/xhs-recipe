import { useExtractionState } from "@/hooks/useExtractionState";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Alert, AlertDescription } from "@/components/ui/alert";
import { AlertCircle } from "lucide-react";
import { classifyError } from "@/lib/helpers";
import UrlInputBar from "@/components/inspire/UrlInputBar";
import ProgressIndicator from "@/components/inspire/ProgressIndicator";
import TheBridgeCard from "@/components/inspire/TheBridgeCard";
import type { Recipe } from "@/types/recipe";

interface ExtractSectionProps {
  onExtracted: (recipes: Recipe[]) => void;
  onRefineRecipe: (recipe: Recipe) => void;
}

export default function ExtractSection({
  onExtracted,
  onRefineRecipe,
}: ExtractSectionProps) {
  const { state, startExtraction } = useExtractionState();
  const { status, progress, error, recipe } = state;

  const handleExtract = async (url: string) => {
    const recipes = await startExtraction(url);
    if (recipes.length > 0) {
      onExtracted(recipes);
    }
  };

  const isIdle = status === "IDLE";
  const isParsing = status === "PARSING";
  const isParsed = status === "PARSED";
  const hasError = !!error;

  const handleRefine = (r: Recipe) => {
    onRefineRecipe(r);
    // Also notify parent for list refresh
    onExtracted([r]);
  };

  return (
    <Card>
      <CardHeader className="pb-3">
        <CardTitle className="text-lg">提取菜谱</CardTitle>
      </CardHeader>
      <CardContent className="space-y-4 pt-0">
        {/* IDLE: Show URL input */}
        {(isIdle || hasError) && (
          <UrlInputBar onExtract={handleExtract} disabled={isParsing} />
        )}

        {/* PARSING: Show progress */}
        {isParsing && progress && (
          <ProgressIndicator
            stage={progress.stage}
            detail=""
            percent={progress.percent}
          />
        )}

        {/* PARSED: Show The Bridge */}
        {isParsed && recipe && (
          <TheBridgeCard recipe={recipe} onRefine={handleRefine} />
        )}

        {/* ERROR */}
        {hasError && (
          <Alert variant="destructive">
            <AlertCircle className="h-4 w-4" />
            <AlertDescription>{classifyError(error)}</AlertDescription>
          </Alert>
        )}
      </CardContent>
    </Card>
  );
}
