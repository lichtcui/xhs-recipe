import { useEffect } from "react";
import { useExtractionState } from "@/hooks/useExtractionState";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Alert, AlertDescription } from "@/components/ui/alert";
import { AlertCircle } from "lucide-react";
import { toast } from "sonner";
import { classifyError } from "@/lib/helpers";
import UrlInputBar from "@/components/inspire/UrlInputBar";
import ProgressIndicator from "@/components/inspire/ProgressIndicator";
import TheBridgeCard from "@/components/inspire/TheBridgeCard";
import RecipeEditor from "@/components/inspire/RecipeEditor";
import type { Recipe } from "@/types/recipe";

interface ExtractSectionProps {
  onExtracted: (recipes: Recipe[]) => void;
  onRefineRecipe: (recipe: Recipe) => void;
}

export default function ExtractSection({
  onExtracted,
  onRefineRecipe,
}: ExtractSectionProps) {
  const { state, startExtraction, refineRecipe, saveEditedRecipe, reset } =
    useExtractionState();
  const { status, progress, error, recipe } = state;

  // When SAVED, notify parent and reset after a moment
  useEffect(() => {
    if (status === "SAVED" && recipe) {
      toast.success("菜谱已保存", {
        description: recipe.name,
        action: {
          label: "查看",
          onClick: () => onRefineRecipe(recipe),
        },
      });
      // Notify parent for list refresh, then navigate to cooking
      onExtracted([recipe]);
      onRefineRecipe(recipe);
      // Reset after a brief delay so user sees the toast
      const t = setTimeout(() => reset(), 500);
      return () => clearTimeout(t);
    }
  }, [status, recipe, onExtracted, onRefineRecipe, reset]);

  const handleExtract = async (url: string) => {
    const recipes = await startExtraction(url);
    if (recipes.length > 0) {
      onExtracted(recipes);
    }
  };

  const handleRefine = (r: Recipe) => {
    // Transition to GENERATED → show RecipeEditor inline (Phase 3)
    // Navigation to CookingPage happens after save (in SAVED useEffect)
    refineRecipe(r);
  };

  const handleSave = async (editedRecipe: Recipe) => {
    await saveEditedRecipe(editedRecipe);
    // SAVED state triggers useEffect above
  };

  const handleRegenerate = () => {
    // Re-extract with the same URL (path A: calls full extract)
    if (state.url) {
      startExtraction(state.url).then((recipes) => {
        if (recipes.length > 0) {
          refineRecipe(recipes[0]);
        }
      });
    } else {
      toast.error("无法重新生成，请重新提取链接");
    }
  };

  const title =
    status === "GENERATED" || status === "SAVED"
      ? "编辑菜谱"
      : "提取菜谱";

  return (
    <Card>
      <CardHeader className="pb-3">
        <CardTitle className="text-lg">{title}</CardTitle>
      </CardHeader>
      <CardContent className="space-y-4 pt-0">
        {/* IDLE: URL input */}
        {status === "IDLE" && (
          <UrlInputBar onExtract={handleExtract} disabled={false} />
        )}

        {/* ERROR: show error + URL input */}
        {status === "IDLE" && error && (
          <Alert variant="destructive">
            <AlertCircle className="h-4 w-4" />
            <AlertDescription>{classifyError(error)}</AlertDescription>
          </Alert>
        )}

        {/* PARSING: progress */}
        {status === "PARSING" && progress && (
          <ProgressIndicator
            stage={progress.stage}
            detail=""
            percent={progress.percent}
          />
        )}

        {/* PARSED: The Bridge */}
        {status === "PARSED" && recipe && (
          <TheBridgeCard recipe={recipe} onRefine={handleRefine} />
        )}

        {/* GENERATED: RecipeEditor */}
        {(status === "GENERATED" || status === "SAVED") && recipe && (
          <RecipeEditor
            recipe={recipe}
            onSave={handleSave}
            onRegenerate={handleRegenerate}
            onCancel={reset}
          />
        )}
      </CardContent>
    </Card>
  );
}
