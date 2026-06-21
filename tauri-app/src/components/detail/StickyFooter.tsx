import { Button } from "@/components/ui/button";
import { Star } from "lucide-react";
import CookingTimer from "./CookingTimer";

interface StickyFooterProps {
  isFavorite: boolean;
  onToggleFavorite: () => void;
}

export default function StickyFooter({
  isFavorite,
  onToggleFavorite,
}: StickyFooterProps) {
  return (
    <div className="sticky bottom-0 -mx-6 px-4 py-3 bg-white/80 backdrop-blur-md border-t flex items-center gap-2">
      <Button
        variant={isFavorite ? "default" : "outline"}
        size="sm"
        onClick={onToggleFavorite}
        className={`text-xs ${isFavorite ? "bg-amber-400 hover:bg-amber-500 text-white border-amber-400" : ""}`}
      >
        <Star
          size={16}
          className={`mr-1 ${isFavorite ? "fill-white" : ""}`}
        />
        {isFavorite ? "已收藏" : "收藏"}
      </Button>

      <div className="flex-1" />

      <CookingTimer />
    </div>
  );
}
