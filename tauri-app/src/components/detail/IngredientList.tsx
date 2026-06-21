import { useState, type ReactNode } from "react";
import { Checkbox } from "@/components/ui/checkbox";
import { cn } from "@/lib/utils";
import { fmtAmount } from "@/lib/helpers";
import type { Ingredient } from "@/types/recipe";

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
  const storageKey = `xhs-checked-${label}`;
  const [checked, setChecked] = useState<Set<number>>(() => {
    try {
      const saved = localStorage.getItem(storageKey);
      return saved ? new Set(JSON.parse(saved)) : new Set();
    } catch {
      return new Set();
    }
  });

  const toggle = (idx: number) => {
    setChecked((prev) => {
      const next = new Set(prev);
      if (next.has(idx)) {
        next.delete(idx);
      } else {
        next.add(idx);
        // Micro-bounce animation via temp class handled by CSS
      }
      localStorage.setItem(storageKey, JSON.stringify([...next]));
      return next;
    });
  };

  const toggleAll = () => {
    if (checked.size === items.length) {
      setChecked(new Set());
      localStorage.setItem(storageKey, "[]");
    } else {
      const all = new Set(items.map((_, i) => i));
      setChecked(all);
      localStorage.setItem(storageKey, JSON.stringify([...all]));
    }
  };

  if (items.length === 0) return null;

  return (
    <div className="mb-4">
      <div className="flex items-center justify-between mb-2">
        <span className="font-bold text-gray-600 text-sm">
          {icon} {label}
        </span>
        <button
          onClick={toggleAll}
          className="text-xs text-gray-400 hover:text-xhs transition-colors"
        >
          {checked.size === items.length ? "清除" : "全部勾选"}
        </button>
      </div>
      <div className="space-y-1.5">
        {items.map((item, i) => {
          const isChecked = checked.has(i);
          return (
            <label
              key={i}
              className={cn(
                "flex items-center gap-2.5 px-2 py-1.5 rounded-lg cursor-pointer transition-all duration-200 hover:bg-gray-50",
                isChecked && "bg-gray-50/50"
              )}
            >
              <Checkbox
                checked={isChecked}
                onCheckedChange={() => toggle(i)}
                id={`ing-${label}-${i}`}
              />
              <span
                className={cn(
                  "text-sm transition-all duration-300",
                  isChecked
                    ? "line-through text-gray-300 decoration-xhs/40 decoration-2"
                    : "text-gray-700"
                )}
              >
                {item.name}
                {item.amount && fmtAmount(item.amount)}
                {item.prep && (
                  <span className="text-xs text-gray-400">
                    {" "}
                    ({item.prep})
                  </span>
                )}
              </span>
            </label>
          );
        })}
      </div>
    </div>
  );
}
