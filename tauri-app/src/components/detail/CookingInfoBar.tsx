import { Clock, ChefHat, Users } from "lucide-react";

interface CookingInfoBarProps {
  totalTime?: string;
  difficulty?: string;
  servings?: number;
}

const DIFFICULTY_MAP: Record<string, { label: string; color: string }> = {
  easy: { label: "简单", color: "text-green-500" },
  medium: { label: "中等", color: "text-amber-500" },
  hard: { label: "困难", color: "text-red-500" },
};

export default function CookingInfoBar({
  totalTime,
  difficulty,
  servings,
}: CookingInfoBarProps) {
  const diff = difficulty ? DIFFICULTY_MAP[difficulty] || { label: difficulty, color: "text-gray-500" } : null;

  return (
    <div className="flex items-center gap-4 mb-4 text-sm text-gray-500">
      {totalTime && (
        <div className="flex items-center gap-1">
          <Clock size={16} className="text-xhs" />
          <span>{totalTime}</span>
        </div>
      )}
      {diff && (
        <div className="flex items-center gap-1">
          <ChefHat size={16} className={diff.color} />
          <span className={diff.color}>{diff.label}</span>
        </div>
      )}
      {servings != null && servings > 0 && (
        <div className="flex items-center gap-1">
          <Users size={16} className="text-blue-500" />
          <span>{servings}人份</span>
        </div>
      )}
    </div>
  );
}
