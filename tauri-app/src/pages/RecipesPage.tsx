import { useState, useEffect, useCallback } from "react";
import { listRecipes, getRecipe, deleteRecipe } from "@/lib/tauri";
import type { Recipe, RecipeSummary } from "@/types/recipe";
import RecipeCard from "@/components/home/RecipeCard";

interface RecipesPageProps {
  onViewRecipe: (recipe: Recipe) => void;
}

export default function RecipesPage({ onViewRecipe }: RecipesPageProps) {
  const [recipes, setRecipes] = useState<RecipeSummary[]>([]);
  const [loading, setLoading] = useState(true);

  const loadRecipes = useCallback(async () => {
    try {
      setLoading(true);
      const list = await listRecipes();
      setRecipes(list);
    } catch (err) {
      console.error("Failed to load recipes:", err);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    loadRecipes();
  }, [loadRecipes]);

  const handleView = async (summary: RecipeSummary) => {
    try {
      const recipe = await getRecipe(summary.id);
      onViewRecipe(recipe);
    } catch (err) {
      console.error("Failed to load recipe:", err);
    }
  };

  const handleDelete = async (summary: RecipeSummary) => {
    try {
      await deleteRecipe(summary.id);
      setRecipes((prev) => prev.filter((r) => r.id !== summary.id));
    } catch (err) {
      console.error("Failed to delete recipe:", err);
    }
  };

  if (loading) {
    return (
      <div>
        <h2 className="text-[22px] font-bold text-xhs mb-4">我的菜谱</h2>
        <div className="text-sm text-gray-400">加载中...</div>
      </div>
    );
  }

  return (
    <div>
      <h2 className="text-[22px] font-bold text-xhs mb-4">我的菜谱</h2>
      {recipes.length === 0 ? (
        <div className="text-center py-12 text-gray-400">
          <p className="text-4xl mb-3">📖</p>
          <p className="text-sm">还没有保存的菜谱</p>
          <p className="text-xs mt-1">去「灵感厨房」提取第一条菜谱吧</p>
        </div>
      ) : (
        <div className="space-y-3">
          {recipes.map((r) => (
            <RecipeCard
              key={r.id}
              recipe={r}
              onView={() => handleView(r)}
              onDelete={() => handleDelete(r)}
            />
          ))}
        </div>
      )}
    </div>
  );
}
