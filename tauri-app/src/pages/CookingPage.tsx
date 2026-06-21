import { useState, useCallback } from "react";
import { ChevronLeft } from "lucide-react";
import HeroSection from "@/components/detail/HeroSection";
import RecipeTags from "@/components/detail/RecipeTags";
import CookingInfoBar from "@/components/detail/CookingInfoBar";
import IngredientList from "@/components/detail/IngredientList";
import StepTimeline from "@/components/detail/StepTimeline";
import FrameGallery from "@/components/detail/FrameGallery";
import TipList from "@/components/detail/TipList";
import { getFavorites, favKey } from "@/lib/favorites";
import type { Recipe } from "@/types/recipe";

interface CookingPageProps {
  recipe: Recipe | null;
  onBackToInspire: () => void;
}

export default function CookingPage({ recipe, onBackToInspire }: CookingPageProps) {
  const [favorites, setFavorites] = useState<Set<string>>(getFavorites);

  const isFavorite = recipe ? favorites.has(favKey(recipe.source_url, recipe.name)) : false;

  const toggleFavorite = useCallback(() => {
    if (!recipe) return;
    const key = favKey(recipe.source_url, recipe.name);
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
  }, [recipe]);

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
      {/* Hero with back button */}
      <div className="relative">
        <HeroSection
          coverImageUrl={recipe.cover_image_url}
          name={recipe.name}
          isFavorite={isFavorite}
          onToggleFavorite={toggleFavorite}
        />
        <button
          onClick={onBackToInspire}
          className="absolute top-3 left-3 z-10 bg-white/80 backdrop-blur-sm rounded-full p-1.5 shadow hover:bg-white transition-colors"
        >
          <ChevronLeft size={18} className="text-gray-700" />
        </button>
      </div>

      {/* Tags */}
      <RecipeTags tags={recipe.tags} />

      {/* Cooking info */}
      <CookingInfoBar
        totalTime={recipe.total_time}
      />

      {/* Frame gallery (original images) */}
      <FrameGallery imageUrls={recipe.image_urls} />

      {/* Ingredients with checkboxes */}
      <IngredientList
        icon="📋"
        label="食材"
        items={recipe.ingredients}
      />

      {/* Seasonings with checkboxes */}
      <IngredientList
        icon="🧂"
        label="调料"
        items={recipe.seasonings}
      />

      {/* Equipment */}
      {recipe.equipment.length > 0 && (
        <div className="mb-4 text-sm text-gray-600">
          <span className="font-bold">🔧 器具</span>
          <span className="ml-1">· {recipe.equipment.join("、")}</span>
        </div>
      )}

      {/* Steps timeline */}
      <div className="mb-4">
        <p className="font-bold text-gray-600 text-sm mb-3">📝 烹饪步骤</p>
        <StepTimeline steps={recipe.steps} />
      </div>

      {/* Tips */}
      <TipList tips={recipe.tips} />

      {/* Bottom padding */}
      <div className="h-8" />
    </div>
  );
}
