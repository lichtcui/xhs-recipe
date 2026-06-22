import { useState, useCallback, useRef } from "react";
import { motion, AnimatePresence } from "framer-motion";
import { X, Clock, Sparkles, ExternalLink, BookmarkPlus, BookmarkCheck } from "lucide-react";
import { toast } from "sonner";
import ExtractSection from "@/components/home/ExtractSection";
import CookingPage from "@/pages/CookingPage";
import RecipeEditor from "@/components/inspire/RecipeEditor";
import ErrorBoundary from "@/components/common/ErrorBoundary";
import { Button } from "@/components/ui/button";
import { Card, CardContent } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { truncateUrl } from "@/lib/helpers";
import { saveRecipe, deleteRecipe } from "@/lib/tauri";
import type { Recipe } from "@/types/recipe";

export default function InspirePage() {
  const [warning, setWarning] = useState<string | null>(null);
  const [viewedRecipe, setViewedRecipe] = useState<Recipe | null>(null);
  const [editingRecipe, setEditingRecipe] = useState<Recipe | null>(null);
  const [extractedRecipes, setExtractedRecipes] = useState<Recipe[]>([]);
  const [savedRecipeIds, setSavedRecipeIds] = useState<Set<string>>(new Set());
  const [busy, setBusy] = useState(false);
  const hasRunRef = useRef(false);

  const handleExtracted = useCallback((recipes: Recipe[]) => {
    setWarning(null);
    setExtractedRecipes(recipes);

    const foodRecipes = recipes.filter((r) => r.is_food);
    if (foodRecipes.length > 0) {
      window.dispatchEvent(new CustomEvent("xhs:recipes-changed"));
    }

    const nonFood = recipes.filter((r) => !r.is_food);
    if (nonFood.length > 0 && recipes.every((r) => !r.is_food)) {
      const reason = nonFood[0].reason || "无法提取有效菜谱信息";
      setWarning(`⚠ ${reason}`);
    }
  }, []);

  const handleBusyChange = useCallback((b: boolean) => {
    setBusy(b);
    if (b && !hasRunRef.current) {
      // New extraction started: clear old results
      setExtractedRecipes([]);
      setWarning(null);
      hasRunRef.current = true;
    } else if (!b) {
      // Going idle: reset tracking so next extraction clears old results
      hasRunRef.current = false;
    }
  }, []);

  const handleCloseResults = useCallback(() => {
    setExtractedRecipes([]);
    setWarning(null);
    setSavedRecipeIds(new Set());
  }, []);

  const handleEditSave = useCallback(async (recipe: Recipe) => {
    try {
      if (recipe.id) {
        try { await deleteRecipe(recipe.id); } catch { /* ok */ }
      }
      await saveRecipe(recipe);
      toast.success("菜谱已保存", {
        description: recipe.name,
        action: { label: "查看", onClick: () => setViewedRecipe(recipe) },
      });
      window.dispatchEvent(new CustomEvent("xhs:recipes-changed"));
      setEditingRecipe(null);
    } catch (err) {
      toast.error("保存失败", { description: String(err) });
    }
  }, []);

  const handleQuickSave = useCallback(async (recipe: Recipe, e: React.MouseEvent) => {
    e.stopPropagation();
    try {
      await saveRecipe(recipe);
      const key = recipe.id || recipe.name;
      setSavedRecipeIds((prev) => new Set(prev).add(key));
      window.dispatchEvent(new CustomEvent("xhs:recipes-changed"));
      toast.success("菜谱已保存", {
        description: recipe.name,
        action: { label: "查看", onClick: () => setViewedRecipe(recipe) },
      });
    } catch (err) {
      toast.error("保存失败", { description: String(err) });
    }
  }, []);

  // Show recipe detail when "查看" is clicked from toast
  if (viewedRecipe) {
    return (
      <ErrorBoundary>
        <CookingPage
          recipe={viewedRecipe}
          onBack={() => setViewedRecipe(null)}
        />
      </ErrorBoundary>
    );
  }

  // Show inline recipe editor when clicking a card in extraction results
  if (editingRecipe) {
    return (
      <ErrorBoundary>
        <div className="py-2">
          <RecipeEditor
            recipe={editingRecipe}
            onSave={handleEditSave}
            onCancel={() => setEditingRecipe(null)}
          />
        </div>
      </ErrorBoundary>
    );
  }

  const showContent = warning || extractedRecipes.length > 0 || busy;

  return (
    <div className={showContent ? "" : "flex flex-col justify-center min-h-[calc(100vh-10rem)]"}>
      {/* Header */}
      <div className="flex items-center gap-2 mb-1">
        <Sparkles size={22} className="text-xhs" />
        <h2 className="text-[22px] font-bold text-xhs">提取菜谱</h2>
      </div>
      <p className="text-sm text-gray-400 mb-3">粘贴小红书分享链接，一键提取结构化菜谱</p>

      <ExtractSection
        onExtracted={handleExtracted}
        onBusyChange={handleBusyChange}
      />

      {/* Warning */}
      <AnimatePresence>
        {warning && (
          <motion.div
            initial={{ opacity: 0, y: -8 }}
            animate={{ opacity: 1, y: 0 }}
            exit={{ opacity: 0, y: -8 }}
            className="mt-4 p-3 bg-amber-50 border border-amber-200 rounded-lg text-sm text-amber-800 flex items-center justify-between"
          >
            <span>{warning}</span>
            <button
              onClick={handleCloseResults}
              className="text-xs text-amber-600 underline whitespace-nowrap ml-2"
            >
              重新提取
            </button>
          </motion.div>
        )}
      </AnimatePresence>

      {/* Extraction results */}
      <AnimatePresence>
        {extractedRecipes.length > 0 && (
          <motion.div
            initial={{ opacity: 0, y: 16 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ delay: 0.15 }}
            className="mt-8"
          >
            <div className="flex items-center justify-between mb-3">
              <h3 className="text-[17px] font-semibold text-gray-500 flex items-center gap-1.5">
                <span>提取结果</span>
                <span className="text-xs font-normal text-gray-400">
                  ({extractedRecipes.length} 个菜谱)
                </span>
              </h3>
              <button
                onClick={handleCloseResults}
                className="text-gray-400 hover:text-gray-600 transition-colors p-1"
                title="关闭"
              >
                <X size={18} />
              </button>
            </div>
            <div className="grid gap-3">
              {extractedRecipes.map((r, i) => (
                <motion.div
                  key={r.id || i}
                  initial={{ opacity: 0, y: 12 }}
                  animate={{ opacity: 1, y: 0 }}
                  transition={{ delay: 0.2 + i * 0.08 }}
                >
                  <Card
                    className="cursor-pointer hover:shadow-md transition-all duration-200 hover:-translate-y-0.5 overflow-hidden rounded-xl border-gray-100"
                    onClick={() => setEditingRecipe(r)}
                  >
                    <CardContent className="flex gap-3 p-3">
                      {/* Cover image */}
                      {r.cover_image_url ? (
                        <div className="shrink-0 w-16 h-16 rounded-lg overflow-hidden">
                          <img
                            src={r.cover_image_url}
                            alt={r.name}
                            className="w-full h-full object-cover"
                            onError={(e) => {
                              (e.target as HTMLImageElement).style.display = "none";
                            }}
                          />
                        </div>
                      ) : (
                        <div className="shrink-0 w-16 h-16 rounded-lg bg-gradient-to-br from-xhs/10 to-orange-50 flex items-center justify-center">
                          <Sparkles size={20} className="text-xhs/40" />
                        </div>
                      )}

                      <div className="flex-1 min-w-0 flex flex-col justify-center gap-1">
                        <span className="font-semibold text-[15px] leading-tight line-clamp-1">
                          {r.name}
                        </span>
                        <div className="flex items-center gap-2 flex-wrap">
                          {r.total_time && (
                            <span className="text-[11px] text-gray-400 flex items-center gap-0.5">
                              <Clock size={11} />
                              {r.total_time}
                            </span>
                          )}
                          <span className="text-[11px] text-gray-400 truncate flex items-center gap-0.5">
                            <ExternalLink size={11} />
                            {truncateUrl(r.source_url)}
                          </span>
                        </div>
                        {r.tags && r.tags.length > 0 && (
                          <div className="flex gap-1 mt-0.5">
                            {r.tags.slice(0, 3).map((tag) => (
                              <Badge
                                key={tag}
                                variant="secondary"
                                className="text-[10px] px-1.5 py-0 h-4 font-normal rounded-full"
                              >
                                {tag}
                              </Badge>
                            ))}
                          </div>
                        )}
                      </div>
                      {/* Save button */}
                      <div className="shrink-0 flex items-center">
                        {savedRecipeIds.has(r.id || r.name) ? (
                          <div className="flex items-center gap-1 text-green-500 text-[11px] font-medium">
                            <BookmarkCheck size={16} />
                            <span className="hidden sm:inline">已保存</span>
                          </div>
                        ) : (
                          <Button
                            variant="ghost"
                            size="sm"
                            onClick={(e) => handleQuickSave(r, e)}
                            className="h-8 w-8 p-0 text-gray-400 hover:text-xhs hover:bg-xhs/5 rounded-full"
                            title="保存"
                          >
                            <BookmarkPlus size={16} />
                          </Button>
                        )}
                      </div>
                    </CardContent>
                  </Card>
                </motion.div>
              ))}
            </div>
          </motion.div>
        )}
      </AnimatePresence>
    </div>
  );
}
