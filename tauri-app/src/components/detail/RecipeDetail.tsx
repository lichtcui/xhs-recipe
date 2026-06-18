import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { Alert, AlertDescription } from "@/components/ui/alert";
import { AlertTriangle } from "lucide-react";
import IngredientList from "./IngredientList";
import StepList from "./StepList";
import TipList from "./TipList";
import type { Recipe } from "@/types/recipe";

interface RecipeDetailProps {
  recipe: Recipe | null;
  onBack: () => void;
}

export default function RecipeDetail({ recipe, onBack }: RecipeDetailProps) {
  if (!recipe) {
    return (
      <div className="text-center text-muted-foreground py-12">
        暂无菜谱数据
      </div>
    );
  }

  const { ingredients, seasonings, steps, tips, equipment } = recipe;

  return (
    <div>
      <Button variant="ghost" className="text-xhs pl-0 mb-4" onClick={onBack}>
        ← 返回
      </Button>

      <Card>
        <CardHeader className="pb-3">
          <div className="flex items-center gap-2">
            <CardTitle className="text-xl text-xhs">
              {recipe.name}
            </CardTitle>
            {recipe.total_time && (
              <Badge variant="secondary" className="ml-auto text-xs font-normal">
                ⏱ {recipe.total_time}
              </Badge>
            )}
          </div>
        </CardHeader>

        <CardContent className="space-y-3 pt-0">
          {!recipe.is_food && (
            <Alert>
              <AlertTriangle className="h-4 w-4 text-amber-600" />
              <AlertDescription className="text-amber-800">
                ⚠ 此内容与美食无关
                {recipe.reason ? `: ${recipe.reason}` : ""}
              </AlertDescription>
            </Alert>
          )}

          {ingredients.length > 0 && (
            <IngredientList
              icon="🥩"
              label="食材"
              items={ingredients}
            />
          )}

          {seasonings.length > 0 && (
            <IngredientList
              icon="🧂"
              label="调料"
              items={seasonings}
            />
          )}

          {equipment.length > 0 && (
            <div className="text-sm text-gray-700 leading-relaxed">
              <span className="font-bold text-gray-600">🔧 器具</span>{" "}
              · {equipment.join("、")}
            </div>
          )}

          {steps.length > 0 && <StepList steps={steps} />}

          {tips.length > 0 && <TipList tips={tips} />}
        </CardContent>
      </Card>
    </div>
  );
}
