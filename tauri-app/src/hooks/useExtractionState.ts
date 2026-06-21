import { useReducer, useCallback, useEffect, useRef } from "react";
import type { UnlistenFn } from "@tauri-apps/api/event";
import {
  extractionReducer,
  initialExtractionState,
  type ExtractionState,
} from "@/state/extractionReducer";
import { extractRecipe, onExtractProgress, saveRecipe, deleteRecipe } from "@/lib/tauri";
import { useSettings } from "@/hooks/useSettings";
import type { Recipe } from "@/types/recipe";
import { STAGE_PERCENT } from "@/lib/helpers";

export function useExtractionState() {
  const [state, dispatch] = useReducer(extractionReducer, initialExtractionState);
  const { getExtractPayload } = useSettings();
  const unlistenRef = useRef<UnlistenFn | null>(null);

  // Cleanup progress listener on unmount
  useEffect(() => {
    return () => {
      unlistenRef.current?.();
    };
  }, []);

  const startExtraction = useCallback(
    async (url: string) => {
      dispatch({ type: "START_PARSING", url });

      // Set up progress listener
      unlistenRef.current?.();
      unlistenRef.current = await onExtractProgress((event) => {
        const percent = STAGE_PERCENT[event.stage as keyof typeof STAGE_PERCENT] ?? 50;
        dispatch({ type: "UPDATE_PROGRESS", stage: event.stage, percent });
      });

      try {
        const settings = getExtractPayload();
        const recipes = await extractRecipe(url, settings);

        // Clean up listener
        unlistenRef.current?.();
        unlistenRef.current = null;

        if (recipes.length > 0) {
          dispatch({ type: "PARSED", recipe: recipes[0] });
        } else {
          dispatch({ type: "ERROR", message: "未提取到任何菜谱信息" });
        }
      } catch (err) {
        unlistenRef.current?.();
        unlistenRef.current = null;
        dispatch({ type: "ERROR", message: String(err) });
      }
    },
    [getExtractPayload]
  );

  const saveEditedRecipe = useCallback(async (recipe: Recipe) => {
    try {
      // Delete old file if exists, then save new
      if (recipe.id) {
        try {
          await deleteRecipe(recipe.id);
        } catch {
          // Old file may not exist, that's fine
        }
      }
      const newId = await saveRecipe(recipe);
      dispatch({ type: "SAVED", recipeId: newId });
      return newId;
    } catch (err) {
      dispatch({ type: "ERROR", message: String(err) });
      throw err;
    }
  }, []);

  const reset = useCallback(() => {
    unlistenRef.current?.();
    unlistenRef.current = null;
    dispatch({ type: "RESET" });
  }, []);

  return {
    state: state as ExtractionState,
    startExtraction,
    saveEditedRecipe,
    reset,
    dispatch,
  };
}
