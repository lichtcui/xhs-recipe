import { useEffect, useState, useCallback } from "react";
import { listRecipes, deleteRecipe, getRecipe } from "@/lib/tauri";
import RecipeCard from "./RecipeCard";
import type { RecipeSummary, Recipe } from "@/types/recipe";

interface RecipeListProps {
  refreshTrigger: number;
  onViewRecipe: (recipe: Recipe) => void;
}

export default function RecipeList({
  refreshTrigger,
  onViewRecipe,
}: RecipeListProps) {
  const [recipes, setRecipes] = useState<RecipeSummary[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const data = await listRecipes();
      setRecipes(data);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    refresh();
  }, [refresh, refreshTrigger]);

  const handleDelete = async (id: string) => {
    try {
      await deleteRecipe(id);
      setRecipes((prev) => prev.filter((r) => r.id !== id));
    } catch (e) {
      setError(String(e));
    }
  };

  const handleView = async (id: string) => {
    try {
      const recipe = await getRecipe(id);
      onViewRecipe(recipe);
    } catch (e) {
      setError(String(e));
    }
  };

  if (loading) {
    return (
      <div className="text-center text-muted-foreground py-6 text-sm">
        加载中...
      </div>
    );
  }

  if (error) {
    return (
      <div className="text-center text-red-500 py-6 text-sm">
        加载失败: {error}
      </div>
    );
  }

  if (recipes.length === 0) {
    return (
      <div className="text-center text-muted-foreground py-6 text-sm">
        暂无保存的菜谱
      </div>
    );
  }

  return (
    <div className="flex flex-col gap-2">
      {recipes.map((r) => (
        <RecipeCard
          key={r.id}
          recipe={r}
          onView={() => handleView(r.id)}
          onDelete={() => handleDelete(r.id)}
        />
      ))}
    </div>
  );
}
