import { useState, useCallback } from "react";
import ExtractSection from "@/components/home/ExtractSection";
import RecipeList from "@/components/home/RecipeList";
import type { Recipe } from "@/types/recipe";

interface InspirePageProps {
  onViewRecipe: (recipe: Recipe) => void;
}

export default function InspirePage({ onViewRecipe }: InspirePageProps) {
  const [refreshKey, setRefreshKey] = useState(0);
  const [warning, setWarning] = useState<string | null>(null);

  const handleExtracted = useCallback((recipes: Recipe[]) => {
    setRefreshKey((k) => k + 1);
    setWarning(null);

    const nonFood = recipes.filter((r) => !r.is_food);
    if (nonFood.length > 0 && recipes.every((r) => !r.is_food)) {
      const reason = nonFood[0].reason || "无法提取有效菜谱信息";
      setWarning(`⚠ ${reason}`);
    }
  }, []);

  const handleRefine = useCallback(
    (recipe: Recipe) => {
      // Phase 2.4: Temporary — navigate to CookingPage with recipe
      // Phase 3 will replace this with RecipeEditor
      onViewRecipe(recipe);
    },
    [onViewRecipe]
  );

  return (
    <div>
      <h2 className="text-[22px] font-bold text-xhs mb-4">灵感厨房</h2>
      <p className="text-sm text-gray-400 mb-4">从小红书链接一键提取菜谱</p>

      <ExtractSection
        onExtracted={handleExtracted}
        onRefineRecipe={handleRefine}
      />

      {warning && (
        <div className="mt-4 p-3 bg-amber-50 border border-amber-200 rounded-lg text-sm text-amber-800">
          {warning}
        </div>
      )}

      <div className="mt-8">
        <h3 className="text-[17px] font-semibold text-gray-500 mb-3">
          已保存的菜谱
        </h3>
        <RecipeList refreshTrigger={refreshKey} onViewRecipe={onViewRecipe} />
      </div>
    </div>
  );
}
