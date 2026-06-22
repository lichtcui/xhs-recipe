import { useState, useCallback } from "react";
import { motion, AnimatePresence } from "framer-motion";
import ExtractSection from "@/components/home/ExtractSection";
import CookingPage from "@/pages/CookingPage";
import { Card, CardContent } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { Clock, Sparkles, ExternalLink } from "lucide-react";
import { truncateUrl } from "@/lib/helpers";
import type { Recipe } from "@/types/recipe";

export default function InspirePage() {
  const [warning, setWarning] = useState<string | null>(null);
  const [viewedRecipe, setViewedRecipe] = useState<Recipe | null>(null);
  const [extractedRecipes, setExtractedRecipes] = useState<Recipe[]>([]);
  const [busy, setBusy] = useState(false);

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

  const handleRetry = useCallback(() => {
    setWarning(null);
    setExtractedRecipes([]);
  }, []);

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
        onRefineRecipe={handleRefine}
        onBusyChange={setBusy}
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
              onClick={handleRetry}
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
            <h3 className="text-[17px] font-semibold text-gray-500 mb-3 flex items-center gap-1.5">
              <span>提取结果</span>
              <span className="text-xs font-normal text-gray-400">
                ({extractedRecipes.length} 个菜谱)
              </span>
            </h3>
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
                    onClick={() => handleRefine(r)}
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
