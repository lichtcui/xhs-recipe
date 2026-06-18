import { useState, useCallback } from "react";
import ExtractSection from "./ExtractSection";
import RecipeList from "./RecipeList";
import type { Recipe } from "@/types/recipe";

interface HomePageProps {
  onViewRecipe: (recipe: Recipe) => void;
}

export default function HomePage({ onViewRecipe }: HomePageProps) {
  const [refreshKey, setRefreshKey] = useState(0);

  const handleExtracted = useCallback(
    (recipes: Recipe[]) => {
      setRefreshKey((k) => k + 1);
      // Navigate to first recipe detail
      if (recipes.length > 0) {
        setTimeout(() => onViewRecipe(recipes[0]), 1500);
      }
    },
    [onViewRecipe]
  );

  return (
    <div>
      <h2 className="text-[22px] font-bold text-xhs mb-4">小红书菜谱提取</h2>

      <ExtractSection onExtracted={handleExtracted} />

      <div className="mt-8">
        <h3 className="text-[17px] font-semibold text-gray-500 mb-3">
          已保存的菜谱
        </h3>
        <RecipeList refreshTrigger={refreshKey} onViewRecipe={onViewRecipe} />
      </div>
    </div>
  );
}
