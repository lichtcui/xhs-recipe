import { motion } from "framer-motion";
import { Card, CardContent } from "@/components/ui/card";
import { Alert, AlertDescription } from "@/components/ui/alert";
import { AlertTriangle } from "lucide-react";
import RawTextCollapsible from "./RawTextCollapsible";
import AiRefineButton from "./AiRefineButton";
import type { Recipe } from "@/types/recipe";

interface TheBridgeCardProps {
  recipe: Recipe;
  onRefine: (recipe: Recipe) => void;
}

export default function TheBridgeCard({ recipe, onRefine }: TheBridgeCardProps) {
  const isFood = recipe.is_food;
  const hasCover = !!recipe.cover_image_url;
  const rawText = recipe.raw_text || "";
  // Show first 120 chars of raw text as preview
  const preview = rawText.length > 120 ? rawText.slice(0, 120) + "..." : rawText;

  return (
    <motion.div
      initial={{ opacity: 0, scale: 0.95, y: 20 }}
      animate={{ opacity: 1, scale: 1, y: 0 }}
      transition={{ type: "spring", stiffness: 300, damping: 25 }}
    >
      <Card className="overflow-hidden backdrop-blur-md bg-white/90 border shadow-lg">
        <CardContent className="p-4 space-y-3">
          {/* Cover image + preview row */}
          <div className="flex gap-3">
            {hasCover && (
              <div className="shrink-0">
                <img
                  src={recipe.cover_image_url}
                  alt="封面"
                  className="w-20 h-20 rounded-xl object-cover"
                  onError={(e) => {
                    (e.target as HTMLImageElement).style.display = "none";
                  }}
                />
              </div>
            )}
            <div className="flex-1 min-w-0">
              <p className="text-xs text-gray-400 mb-1">AI 画面识别</p>
              {rawText ? (
                <p className="text-sm text-gray-700 leading-relaxed line-clamp-4">
                  {preview}
                </p>
              ) : (
                <p className="text-sm text-gray-400 italic">无文字描述</p>
              )}
            </div>
          </div>

          {/* Collapsible full text */}
          {rawText && <RawTextCollapsible text={rawText} />}

          {/* Non-food warning */}
          {!isFood && (
            <Alert variant="default" className="bg-amber-50 border-amber-200">
              <AlertTriangle className="h-4 w-4 text-amber-600" />
              <AlertDescription className="text-amber-800 text-xs">
                {recipe.reason || "此内容与美食无关，可能无法提取有效菜谱"}
              </AlertDescription>
            </Alert>
          )}

          {/* CTA button */}
          <AiRefineButton
            onClick={() => onRefine(recipe)}
            disabled={!isFood}
          />
        </CardContent>
      </Card>
    </motion.div>
  );
}
