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
    <div className="mb-4 text-sm text-gray-600">
      <span className="font-bold">{icon} {label}</span>
      <span className="ml-1">
        · {items.map(fmtIngredient).join("、")}
      </span>
    </div>
  );
}
