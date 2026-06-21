import { useEffect, useCallback } from "react";
import { useExtractionState } from "@/hooks/useExtractionState";
import { useLlmStream } from "@/hooks/useLlmStream";
import { useSettings } from "@/hooks/useSettings";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Alert, AlertDescription } from "@/components/ui/alert";
import { AlertCircle } from "lucide-react";
import { toast } from "sonner";
import { classifyError } from "@/lib/helpers";
import UrlInputBar from "@/components/inspire/UrlInputBar";
import ProgressIndicator from "@/components/inspire/ProgressIndicator";
import TheBridgeCard from "@/components/inspire/TheBridgeCard";
import RecipeEditor from "@/components/inspire/RecipeEditor";
import GeneratingView from "@/components/inspire/GeneratingView";
import type { Recipe } from "@/types/recipe";

interface ExtractSectionProps {
  onExtracted: (recipes: Recipe[]) => void;
  onRefineRecipe: (recipe: Recipe) => void;
}

export default function ExtractSection({
  onExtracted,
  onRefineRecipe,
}: ExtractSectionProps) {
  const { state, startExtraction, refineRecipe, saveEditedRecipe, reset, dispatch } =
    useExtractionState();
  const { streamText, tokens } = useLlmStream();
  const { settings } = useSettings();
  const { status, progress, error, recipe, rawText } = state;

  // When SAVED, clean up state after a brief delay
  useEffect(() => {
    if (status === "SAVED") {
      const t = setTimeout(() => reset(), 500);
      return () => clearTimeout(t);
    }
  }, [status, reset]);

  const handleExtract = async (url: string) => {
    const recipes = await startExtraction(url);
    if (recipes.length > 0) {
      onExtracted(recipes);
    }
  };

  const handleRefine = (r: Recipe) => {
    refineRecipe(r);
  };

  const handleSave = async (editedRecipe: Recipe) => {
    await saveEditedRecipe(editedRecipe);
    toast.success("菜谱已保存", {
      description: editedRecipe.name,
      action: { label: "查看", onClick: () => onRefineRecipe(editedRecipe) },
    });
    onExtracted([editedRecipe]);
    onRefineRecipe(editedRecipe);
  };

  const handleRegenerate = useCallback(async () => {
    if (!rawText) {
      toast.error("无法重新生成，请重新提取链接");
      return;
    }

    // Path B: streaming if API key is available
    if (settings.apiKey) {
      dispatch({ type: "START_GENERATING" });
      try {
        const recipes = await streamText(rawText);
        if (recipes.length > 0) {
          dispatch({ type: "GENERATED", recipe: recipes[0] });
        } else {
          dispatch({ type: "ERROR", message: "未生成任何菜谱" });
        }
      } catch (err) {
        dispatch({ type: "ERROR", message: String(err) });
      }
      return;
    }

    // Path A fallback: re-extract via Rust
    if (state.url) {
      const recipes = await startExtraction(state.url);
      if (recipes.length > 0) {
        dispatch({ type: "GENERATED", recipe: recipes[0] });
      }
    } else {
      toast.error("无法重新生成，请重新提取链接");
    }
  }, [rawText, settings.apiKey, state.url, streamText, startExtraction, dispatch]);

  const title =
    status === "GENERATED" || status === "SAVED"
      ? "编辑菜谱"
      : status === "GENERATING"
        ? "AI 生成中"
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

        {/* ERROR state with retry */}
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

        {/* GENERATING: streaming typewriter */}
        {status === "GENERATING" && (
          <GeneratingView tokens={tokens} />
        )}

        {/* GENERATED / SAVED: RecipeEditor */}
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
