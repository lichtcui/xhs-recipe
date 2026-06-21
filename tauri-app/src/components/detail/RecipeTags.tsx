import { Badge } from "@/components/ui/badge";

interface RecipeTagsProps {
  tags?: string[];
}

const TAG_COLORS: Record<string, string> = {
  "家常菜": "bg-green-100 text-green-700 hover:bg-green-100",
  "硬菜": "bg-red-100 text-red-700 hover:bg-red-100",
  "快手菜": "bg-blue-100 text-blue-700 hover:bg-blue-100",
  "川菜": "bg-orange-100 text-orange-700 hover:bg-orange-100",
  "粤菜": "bg-teal-100 text-teal-700 hover:bg-teal-100",
  "烘焙": "bg-yellow-100 text-yellow-700 hover:bg-yellow-100",
  "汤羹": "bg-cyan-100 text-cyan-700 hover:bg-cyan-100",
  "凉菜": "bg-emerald-100 text-emerald-700 hover:bg-emerald-100",
  "面食": "bg-amber-100 text-amber-700 hover:bg-amber-100",
};

export default function RecipeTags({ tags }: RecipeTagsProps) {
  if (!tags || tags.length === 0) return null;

  return (
    <div className="flex flex-wrap gap-1.5 mb-3">
      {tags.map((tag) => (
        <Badge
          key={tag}
          variant="secondary"
          className={TAG_COLORS[tag] || "bg-gray-100 text-gray-600"}
        >
          #{tag}
        </Badge>
      ))}
    </div>
  );
}
