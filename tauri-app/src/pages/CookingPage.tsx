import { useState, useCallback } from "react";
import { ChevronLeft, ShoppingBasket, FlaskConical, Wrench, ListChecks } from "lucide-react";
import { toast } from "sonner";
import HeroSection from "@/components/detail/HeroSection";
import RecipeTags from "@/components/detail/RecipeTags";
import CookingInfoBar from "@/components/detail/CookingInfoBar";
import IngredientList from "@/components/detail/IngredientList";
import StepTimeline from "@/components/detail/StepTimeline";
import FrameGallery from "@/components/detail/FrameGallery";
import TipList from "@/components/detail/TipList";
import RecipeEditor from "@/components/inspire/RecipeEditor";
import { getFavorites, favKey } from "@/lib/favorites";
import { saveRecipe, deleteRecipe } from "@/lib/tauri";
import type { Recipe } from "@/types/recipe";

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

      {/* Tags */}
      <RecipeTags tags={currentRecipe.tags} />

      {/* Cooking info */}
      <CookingInfoBar
        totalTime={currentRecipe.total_time}
      />

      {/* Frame gallery (original images) */}
      <FrameGallery imageUrls={currentRecipe.image_urls} />

      {/* Ingredients */}
      <IngredientList
        icon={<ShoppingBasket size={16} />}
        label="食材"
        items={currentRecipe.ingredients}
      />

      {/* Seasonings */}
      <IngredientList
        icon={<FlaskConical size={16} />}
        label="调料"
        items={currentRecipe.seasonings}
      />

      {/* Equipment */}
      {currentRecipe.equipment.length > 0 && (
        <div className="mb-4">
          <h3 className="font-semibold text-sm text-gray-500 mb-2 flex items-center gap-1.5">
            <Wrench size={16} />
            器具
          </h3>
          <p className="text-sm text-gray-700 leading-relaxed">
            {currentRecipe.equipment.join("、")}
          </p>
        </div>
      )}

      {/* Steps */}
      <div className="mb-4">
        <h3 className="font-semibold text-sm text-gray-500 mb-2 flex items-center gap-1.5">
          <ListChecks size={16} />
          烹饪步骤
        </h3>
        <StepTimeline steps={currentRecipe.steps} />
      </div>

      {/* Tips */}
      <TipList tips={currentRecipe.tips} />

      {/* Bottom padding */}
      <div className="h-8" />
    </div>
  );
}
