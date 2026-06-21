import RecipeDetail from "@/components/detail/RecipeDetail";
import type { Recipe } from "@/types/recipe";

interface CookingPageProps {
  recipe: Recipe | null;
  onBackToInspire: () => void;
}

export default function CookingPage({ recipe, onBackToInspire }: CookingPageProps) {
  if (!recipe) {
    return (
      <div>
        <h2 className="text-[22px] font-bold text-xhs mb-4">烹饪台</h2>
        <div className="text-center py-12 text-gray-400">
          <p className="text-4xl mb-3">🍳</p>
          <p className="text-sm">选择一个菜谱开始烹饪</p>
          <button
            onClick={onBackToInspire}
            className="mt-3 text-xs text-xhs hover:underline"
          >
            去灵感厨房提取菜谱
          </button>
        </div>
      </div>
    );
  }

  return (
    <div>
      <RecipeDetail
        recipe={recipe}
        onBack={onBackToInspire}
      />
    </div>
  );
}
