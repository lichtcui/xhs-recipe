import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { truncateUrl } from "@/lib/helpers";
import type { RecipeSummary } from "@/types/recipe";

interface RecipeCardProps {
  recipe: RecipeSummary;
  onView: () => void;
  onDelete: () => void;
}

export default function RecipeCard({
  recipe,
  onView,
  onDelete,
}: RecipeCardProps) {
  return (
    <Card
      className="flex items-center justify-between px-4 py-3 cursor-pointer hover:shadow-md transition-shadow border-border"
      onClick={onView}
    >
      <div className="flex flex-col gap-0.5 min-w-0">
        <span className="font-semibold text-[15px] text-foreground">
          {recipe.name}
        </span>
        <span className="text-xs text-muted-foreground truncate">
          {truncateUrl(recipe.source_url)}
        </span>
      </div>
      <div className="flex gap-1.5 shrink-0">
        <Button
          variant="outline"
          size="sm"
          className="text-xs h-7"
          onClick={(e) => {
            e.stopPropagation();
            onView();
          }}
        >
          查看
        </Button>
        <Button
          variant="outline"
          size="sm"
          className="text-xs h-7 hover:border-red-500 hover:text-red-500"
          onClick={(e) => {
            e.stopPropagation();
            onDelete();
          }}
        >
          删除
        </Button>
      </div>
    </Card>
  );
}
