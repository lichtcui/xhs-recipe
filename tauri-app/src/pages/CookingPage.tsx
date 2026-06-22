import { useState, useCallback } from "react";
import { ChevronLeft, ShoppingBasket, FlaskConical, Wrench, ListChecks } from "lucide-react";
import { toast } from "sonner";
import HeroSection from "@/components/detail/HeroSection";
import StepTimeline from "@/components/detail/StepTimeline";
import TipList from "@/components/detail/TipList";
import RecipeEditor from "@/components/inspire/RecipeEditor";
import { getFavorites, favKey } from "@/lib/favorites";
import { saveRecipe, deleteRecipe } from "@/lib/tauri";
import { fmtAmount } from "@/lib/helpers";
import type { Recipe, Ingredient } from "@/types/recipe";

function fmtIngredient(item: Ingredient): string {
  let s = item.name;
  if (item.amount) s += fmtAmount(item.amount);
  if (item.prep) s += `（${item.prep}）`;
  return s;
}

interface CookingPageProps {
  recipe: Recipe | null;
  onBack: () => void;
}

export default function CookingPage({ recipe, onBack }: CookingPageProps) {
  const [favorites, setFavorites] = useState<Set<string>>(getFavorites);
  const [editMode, setEditMode] = useState(false);
  const [currentRecipe, setCurrentRecipe] = useState<Recipe | null>(recipe);

  // Sync currentRecipe when the incoming recipe changes
  const [prevRecipe, setPrevRecipe] = useState<Recipe | null>(recipe);
  if (recipe !== prevRecipe) {
    setPrevRecipe(recipe);
    setCurrentRecipe(recipe);
    setEditMode(false);
    window.scrollTo(0, 0);
  }

  const isFavorite = currentRecipe
    ? favorites.has(favKey(currentRecipe.source_url, currentRecipe.name))
    : false;

  const toggleFavorite = useCallback(() => {
    if (!currentRecipe) return;
    const key = favKey(currentRecipe.source_url, currentRecipe.name);
    setFavorites((prev) => {
      const next = new Set(prev);
      if (next.has(key)) {
        next.delete(key);
      } else {
        next.add(key);
      }
      localStorage.setItem("xhs-favorites", JSON.stringify([...next]));
      return next;
    });
  }, [currentRecipe]);

  const handleSave = useCallback(async (edited: Recipe) => {
    if (!currentRecipe) return;
    try {
      // Overwrite the existing recipe file
      if (currentRecipe.id) {
        try { await deleteRecipe(currentRecipe.id); } catch { /* ok */ }
      }
      const newId = await saveRecipe(edited);
      const saved = { ...edited, id: newId };
      setCurrentRecipe(saved);
      setEditMode(false);
      window.scrollTo(0, 0);
      toast.success(`已保存「${edited.name}」`, { duration: 2000 });
    } catch (err) {
      toast.error("保存失败", { description: String(err) });
    }
  }, [currentRecipe]);

  if (!currentRecipe) {
    return (
      <div>
        <h2 className="text-[22px] font-bold text-xhs mb-4">菜谱详情</h2>
        <div className="text-center py-12 text-gray-300">
          <p className="text-sm">选择一个菜谱查看详情</p>
          <button
            onClick={onBack}
            className="mt-3 text-xs text-xhs hover:underline"
          >
            返回
          </button>
        </div>
      </div>
    );
  }

  if (editMode) {
    return (
      <div className="py-2">
        <RecipeEditor
          recipe={currentRecipe}
          onSave={handleSave}
          onCancel={() => { setEditMode(false); window.scrollTo(0, 0); }}
        />
      </div>
    );
  }

  return (
    <div>
      {/* Hero with back button */}
      <div className="relative">
        <HeroSection
          coverImageUrl={currentRecipe.cover_image_url}
          name={currentRecipe.name}
          tags={currentRecipe.tags}
          totalTime={currentRecipe.total_time}
          sourceUrl={currentRecipe.source_url}
          isFavorite={isFavorite}
          onToggleFavorite={toggleFavorite}
          onEdit={() => setEditMode(true)}
        />
        <button
          onClick={onBack}
          className="absolute top-3 left-3 z-10 bg-white/80 backdrop-blur-sm rounded-full p-1.5 shadow hover:bg-white transition-colors"
        >
          <ChevronLeft size={18} className="text-gray-700" />
        </button>
      </div>

      {/* Region: 食材 + 调料 + 器具 */}
      <div className="mb-6 space-y-1.5">
        {currentRecipe.ingredients.length > 0 && (
          <div className="flex items-baseline gap-1.5 text-sm">
            <ShoppingBasket size={14} className="shrink-0 text-gray-400" />
            <span className="font-semibold text-gray-500 shrink-0">食材</span>
            <span className="text-gray-700 leading-relaxed">
              {currentRecipe.ingredients.map(fmtIngredient).join("、")}
            </span>
          </div>
        )}
        {currentRecipe.seasonings.length > 0 && (
          <div className="flex items-baseline gap-1.5 text-sm">
            <FlaskConical size={14} className="shrink-0 text-gray-400" />
            <span className="font-semibold text-gray-500 shrink-0">调料</span>
            <span className="text-gray-700 leading-relaxed">
              {currentRecipe.seasonings.map(fmtIngredient).join("、")}
            </span>
          </div>
        )}
        {currentRecipe.equipment.length > 0 && (
          <div className="flex items-baseline gap-1.5 text-sm">
            <Wrench size={14} className="shrink-0 text-gray-400" />
            <span className="font-semibold text-gray-500 shrink-0">器具</span>
            <span className="text-gray-700 leading-relaxed">
              {currentRecipe.equipment.join("、")}
            </span>
          </div>
        )}
      </div>

      {/* Region: 烹饪步骤 */}
      <div className="mb-6">
        <h3 className="font-semibold text-sm text-gray-500 mb-2 flex items-center gap-1.5">
          <ListChecks size={16} />
          烹饪步骤
        </h3>
        <StepTimeline steps={currentRecipe.steps} />
      </div>

      {/* Region: 小贴士 */}
      <TipList tips={currentRecipe.tips} />

      {/* Bottom padding */}
      <div className="h-8" />
    </div>
  );
}
