import type { ReactNode } from "react";
import { fmtAmount } from "@/lib/helpers";
import type { Ingredient } from "@/types/recipe";

interface IngredientListProps {
  icon: ReactNode;
  label: string;
  items: Ingredient[];
}

function fmtIngredient(item: Ingredient): string {
  let s = item.name;
  if (item.amount) s += fmtAmount(item.amount);
  if (item.prep) s += `（${item.prep}）`;
  return s;
}

export default function IngredientList({
  icon,
  label,
  items,
}: IngredientListProps) {
  if (items.length === 0) return null;

  return (
    <div className="mb-4">
      <h3 className="font-semibold text-sm text-gray-500 mb-2 flex items-center gap-1.5">
        {icon}
        <span>{label}</span>
      </h3>
      <p className="text-sm text-gray-700 leading-relaxed">
        {items.map(fmtIngredient).join("、")}
      </p>
    </div>
  );
}
