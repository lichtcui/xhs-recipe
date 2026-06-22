import { useState, useCallback } from "react";
import ExtractSection from "@/components/home/ExtractSection";
import CookingPage from "@/pages/CookingPage";
import { Card, CardContent } from "@/components/ui/card";
import { truncateUrl } from "@/lib/helpers";
import type { Recipe } from "@/types/recipe";

export default function InspirePage() {
  const [warning, setWarning] = useState<string | null>(null);
  const [viewedRecipe, setViewedRecipe] = useState<Recipe | null>(null);
  const [extractedRecipes, setExtractedRecipes] = useState<Recipe[]>([]);

  const handleExtracted = useCallback((recipes: Recipe[]) => {
    setWarning(null);
    setExtractedRecipes(recipes);

    const nonFood = recipes.filter((r) => !r.is_food);
    if (nonFood.length > 0 && recipes.every((r) => !r.is_food)) {
      const reason = nonFood[0].reason || "无法提取有效菜谱信息";
      setWarning(`⚠ ${reason}`);
    }
  }, []);

  const handleRefine = useCallback((recipe: Recipe) => {
    setViewedRecipe(recipe);
  }, []);

  // Show recipe detail inline when one is selected
  if (viewedRecipe) {
    return (
      <CookingPage
        recipe={viewedRecipe}
        onBack={() => setViewedRecipe(null)}
      />
    );
  }

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

      {extractedRecipes.length > 0 && (
        <div className="mt-8">
          <h3 className="text-[17px] font-semibold text-gray-500 mb-3">
            提取结果
          </h3>
          <div className="flex flex-col gap-2">
            {extractedRecipes.map((r, i) => (
              <Card
                key={r.id || i}
                className="cursor-pointer hover:shadow-md transition-shadow"
                onClick={() => handleRefine(r)}
              >
                <CardContent className="flex items-center justify-between p-3">
                  <div className="flex flex-col gap-0.5 min-w-0">
                    <span className="font-semibold text-[15px]">{r.name}</span>
                    <span className="text-xs text-muted-foreground truncate">
                      {truncateUrl(r.source_url)}
                    </span>
                  </div>
                </CardContent>
              </Card>
            ))}
          </div>
        </div>
      )}
    </div>
  );
}
