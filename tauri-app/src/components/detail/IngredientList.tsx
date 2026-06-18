import type { ReactNode } from "react";
import type { Ingredient } from "@/types/recipe";
import { fmtAmount } from "@/lib/helpers";

interface IngredientListProps {
  icon: ReactNode;
  label: string;
  items: Ingredient[];
}

export default function IngredientList({
  icon,
  label,
  items,
}: IngredientListProps) {
  const parts = items.map((item) => {
    let s = item.name;
    if (item.amount) {
      const fa = fmtAmount(item.amount);
      if (fa) s += fa;
    }
    if (item.prep) s += `（${item.prep}）`;
    return s;
  });

  return (
    <div className="text-sm text-gray-700 leading-relaxed mb-3">
      <span className="font-bold text-gray-600 mr-1">{icon}</span>
      <span className="font-bold text-gray-600 mr-1">{label}</span>
      <span>· {parts.join("、")}</span>
    </div>
  );
}
