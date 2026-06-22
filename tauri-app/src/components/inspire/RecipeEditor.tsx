import { useState } from "react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Badge } from "@/components/ui/badge";
import { Separator } from "@/components/ui/separator";
import { Plus, X, Save, ChevronLeft } from "lucide-react";
import type { Recipe, Ingredient, Step } from "@/types/recipe";

interface RecipeEditorProps {
  recipe: Recipe;
  onSave: (recipe: Recipe) => Promise<void>;
  onCancel: () => void;
}

// ── Helpers ──

function newIngredient(): Ingredient {
  return { name: "", amount: "", prep: "", category: "食材" };
}

function newSeasoning(): Ingredient {
  return { name: "", amount: "", category: "调料" };
}

function newStep(): Step {
  return { title: "", time: "", content: "" };
}

function InlineField({
  label,
  value,
  onChange,
  placeholder,
  type = "text",
}: {
  label: string;
  value: string;
  onChange: (v: string) => void;
  placeholder?: string;
  type?: string;
}) {
  return (
    <div className="space-y-1">
      <Label className="text-sm font-semibold">{label}</Label>
      <Input
        type={type}
        value={value}
        onChange={(e) => onChange(e.target.value)}
        placeholder={placeholder}
        className="h-8 text-sm"
      />
    </div>
  );
}

// ── Main Component ──

export default function RecipeEditor({
  recipe,
  onSave,
  onCancel,
}: RecipeEditorProps) {
  const [edited, setEdited] = useState<Recipe>({ ...recipe,
    ingredients: recipe.ingredients.map((i) => ({ ...i })),
    seasonings: recipe.seasonings.map((s) => ({ ...s })),
    steps: recipe.steps.map((s) => ({ ...s })),
    tips: [...recipe.tips],
    equipment: [...recipe.equipment],
    tags: [...(recipe.tags || [])],
  });
  const [saving, setSaving] = useState(false);
  const [isDirty, setIsDirty] = useState(false);

  // ── Field updaters ──

  const updateField = (field: keyof Recipe, value: unknown) => {
    setEdited((prev) => ({ ...prev, [field]: value }));
    setIsDirty(true);
  };

  // ── Ingredients ──

  const updateIngredient = (idx: number, field: keyof Ingredient, value: string) => {
    setEdited((prev) => {
      const items = [...prev.ingredients];
      items[idx] = { ...items[idx], [field]: value };
      return { ...prev, ingredients: items };
    });
    setIsDirty(true);
  };

  const addIngredient = () => {
    setEdited((prev) => ({ ...prev, ingredients: [...prev.ingredients, newIngredient()] }));
    setIsDirty(true);
  };

  const removeIngredient = (idx: number) => {
    setEdited((prev) => ({
      ...prev,
      ingredients: prev.ingredients.filter((_, i) => i !== idx),
    }));
    setIsDirty(true);
  };

  // ── Seasonings ──

  const updateSeasoning = (idx: number, field: keyof Ingredient, value: string) => {
    setEdited((prev) => {
      const items = [...prev.seasonings];
      items[idx] = { ...items[idx], [field]: value };
      return { ...prev, seasonings: items };
    });
    setIsDirty(true);
  };

  const addSeasoning = () => {
    setEdited((prev) => ({ ...prev, seasonings: [...prev.seasonings, newSeasoning()] }));
    setIsDirty(true);
  };

  const removeSeasoning = (idx: number) => {
    setEdited((prev) => ({
      ...prev,
      seasonings: prev.seasonings.filter((_, i) => i !== idx),
    }));
    setIsDirty(true);
  };

  // ── Steps ──

  const updateStep = (idx: number, field: keyof Step, value: string) => {
    setEdited((prev) => {
      const items = [...prev.steps];
      items[idx] = { ...items[idx], [field]: value };
      return { ...prev, steps: items };
    });
    setIsDirty(true);
  };

  const addStep = () => {
    setEdited((prev) => ({ ...prev, steps: [...prev.steps, newStep()] }));
    setIsDirty(true);
  };

  const removeStep = (idx: number) => {
    setEdited((prev) => ({
      ...prev,
      steps: prev.steps.filter((_, i) => i !== idx),
    }));
    setIsDirty(true);
  };

  const moveStep = (idx: number, dir: -1 | 1) => {
    setEdited((prev) => {
      const steps = [...prev.steps];
      const target = idx + dir;
      if (target < 0 || target >= steps.length) return prev;
      [steps[idx], steps[target]] = [steps[target], steps[idx]];
      return { ...prev, steps };
    });
    setIsDirty(true);
  };

  // ── Tips ──

  const updateTip = (idx: number, value: string) => {
    setEdited((prev) => {
      const tips = [...prev.tips];
      tips[idx] = value;
      return { ...prev, tips };
    });
    setIsDirty(true);
  };

  const addTip = () => {
    setEdited((prev) => ({ ...prev, tips: [...prev.tips, ""] }));
    setIsDirty(true);
  };

  const removeTip = (idx: number) => {
    setEdited((prev) => ({ ...prev, tips: prev.tips.filter((_, i) => i !== idx) }));
    setIsDirty(true);
  };

  // ── Save ──

  const handleSave = async () => {
    setSaving(true);
    try {
      // Filter out empty entries
      const toSave: Recipe = {
        ...edited,
        ingredients: edited.ingredients.filter((i) => i.name.trim()),
        seasonings: edited.seasonings.filter((s) => s.name.trim()),
        steps: edited.steps.filter((s) => s.title.trim() || s.content.trim()),
        tips: edited.tips.filter((t) => t.trim()),
        equipment: edited.equipment.filter((e) => e.trim()),
      };
      await onSave(toSave);
      setIsDirty(false);
    } finally {
      setSaving(false);
    }
  };

  // ── Tags ──

  const addTag = (tag: string) => {
    const trimmed = tag.trim();
    if (!trimmed) return;
    setEdited((prev) => ({
      ...prev,
      tags: [...(prev.tags || []), trimmed],
    }));
    setIsDirty(true);
  };

  const removeTag = (idx: number) => {
    setEdited((prev) => ({
      ...prev,
      tags: (prev.tags || []).filter((_, i) => i !== idx),
    }));
    setIsDirty(true);
  };

  // ── Equipment ──

  const updateEquipment = (idx: number, value: string) => {
    setEdited((prev) => {
      const eq = [...prev.equipment];
      eq[idx] = value;
      return { ...prev, equipment: eq };
    });
    setIsDirty(true);
  };

  const addEquipment = () => {
    setEdited((prev) => ({ ...prev, equipment: [...prev.equipment, ""] }));
    setIsDirty(true);
  };

  const removeEquipment = (idx: number) => {
    setEdited((prev) => ({ ...prev, equipment: prev.equipment.filter((_, i) => i !== idx) }));
    setIsDirty(true);
  };

  return (
    <div className="space-y-5">
      {/* Back / Cancel */}
      <button
        onClick={() => {
          if (isDirty) {
            if (!window.confirm("菜谱未保存，确定离开？")) return;
          }
          onCancel();
        }}
        className="flex items-center gap-1 text-sm text-gray-400 hover:text-gray-600 transition-colors"
      >
        <ChevronLeft size={16} />
        返回
      </button>

      {/* ── Basic Info ── */}
      <div className="space-y-3">
        {/* Name */}
        <InlineField
          label="菜名"
          value={edited.name}
          onChange={(v) => updateField("name", v)}
          placeholder="菜名"
        />

        {/* Time + Tags row */}
        <div className="grid grid-cols-3 gap-3">
          <InlineField
            label="烹饪时间"
            value={edited.total_time || ""}
            onChange={(v) => updateField("total_time", v || undefined)}
            placeholder="如 45分钟"
          />
          <div className="col-span-2" />
        </div>

        {/* Tags */}
        <div className="space-y-1.5">
          <Label className="text-sm font-semibold">标签</Label>
          <div className="flex flex-wrap gap-1.5">
            {(edited.tags || []).map((tag, i) => (
              <Badge key={i} variant="secondary" className="gap-1 pr-1">
                {tag}
                <button onClick={() => removeTag(i)} className="hover:text-red-500">
                  <X size={12} />
                </button>
              </Badge>
            ))}
            <TagInput onAdd={addTag} />
          </div>
        </div>
      </div>

      <Separator />

      {/* ── Ingredients ── */}
      <div className="space-y-3">
        <div className="flex items-center justify-between">
          <Label className="text-sm font-semibold">📋 食材</Label>
          <Button variant="ghost" size="sm" onClick={addIngredient} className="text-xs h-7">
            <Plus size={14} className="mr-1" />添加
          </Button>
        </div>
        {edited.ingredients.map((ing, i) => (
          <div key={i} className="flex gap-2 items-start">
            <Input
              value={ing.name}
              onChange={(e) => updateIngredient(i, "name", e.target.value)}
              placeholder="名称"
              className="h-8 text-sm flex-[2]"
            />
            <Input
              value={ing.amount || ""}
              onChange={(e) => updateIngredient(i, "amount", e.target.value)}
              placeholder="用量"
              className="h-8 text-sm flex-1"
            />
            <Input
              value={ing.prep || ""}
              onChange={(e) => updateIngredient(i, "prep", e.target.value)}
              placeholder="处理"
              className="h-8 text-sm flex-1"
            />
            <Button
              variant="ghost"
              size="sm"
              onClick={() => removeIngredient(i)}
              className="h-8 w-8 p-0 text-gray-400 hover:text-red-500 shrink-0"
            >
              <X size={14} />
            </Button>
          </div>
        ))}
      </div>

      <Separator />

      {/* ── Seasonings ── */}
      <div className="space-y-3">
        <div className="flex items-center justify-between">
          <Label className="text-sm font-semibold">🧂 调料</Label>
          <Button variant="ghost" size="sm" onClick={addSeasoning} className="text-xs h-7">
            <Plus size={14} className="mr-1" />添加
          </Button>
        </div>
        {edited.seasonings.map((s, i) => (
          <div key={i} className="flex gap-2 items-start">
            <Input
              value={s.name}
              onChange={(e) => updateSeasoning(i, "name", e.target.value)}
              placeholder="名称"
              className="h-8 text-sm flex-[2]"
            />
            <Input
              value={s.amount || ""}
              onChange={(e) => updateSeasoning(i, "amount", e.target.value)}
              placeholder="用量"
              className="h-8 text-sm flex-1"
            />
            <Button
              variant="ghost"
              size="sm"
              onClick={() => removeSeasoning(i)}
              className="h-8 w-8 p-0 text-gray-400 hover:text-red-500 shrink-0"
            >
              <X size={14} />
            </Button>
          </div>
        ))}
        {edited.seasonings.length === 0 && (
          <p className="text-xs text-gray-400 text-center py-2">暂无调料，点击添加</p>
        )}
      </div>

      <Separator />

      {/* ── Steps ── */}
      <div className="space-y-3">
        <div className="flex items-center justify-between">
          <Label className="text-sm font-semibold">📝 烹饪步骤</Label>
          <Button variant="ghost" size="sm" onClick={addStep} className="text-xs h-7">
            <Plus size={14} className="mr-1" />添加步骤
          </Button>
        </div>
        {edited.steps.map((step, i) => (
          <div key={i} className="border rounded-lg p-3 space-y-2 bg-gray-50/50">
            <div className="flex items-center justify-between">
              <span className="text-xs font-medium text-gray-500">步骤 {i + 1}</span>
              <div className="flex gap-0.5">
                <Button
                  variant="ghost"
                  size="sm"
                  onClick={() => moveStep(i, -1)}
                  disabled={i === 0}
                  className="h-6 w-6 p-0 text-gray-400"
                >
                  ↑
                </Button>
                <Button
                  variant="ghost"
                  size="sm"
                  onClick={() => moveStep(i, 1)}
                  disabled={i === edited.steps.length - 1}
                  className="h-6 w-6 p-0 text-gray-400"
                >
                  ↓
                </Button>
                <Button
                  variant="ghost"
                  size="sm"
                  onClick={() => removeStep(i)}
                  className="h-6 w-6 p-0 text-gray-400 hover:text-red-500"
                >
                  <X size={14} />
                </Button>
              </div>
            </div>
            <div className="flex gap-2">
              <Input
                value={step.title}
                onChange={(e) => updateStep(i, "title", e.target.value)}
                placeholder="步骤标题"
                className="h-8 text-sm flex-[2]"
              />
              <Input
                value={step.time || ""}
                onChange={(e) => updateStep(i, "time", e.target.value)}
                placeholder="耗时"
                className="h-8 text-sm flex-1"
              />
            </div>
            <Input
              value={step.content}
              onChange={(e) => updateStep(i, "content", e.target.value)}
              placeholder="详细说明..."
              className="h-8 text-sm"
            />
          </div>
        ))}
      </div>

      <Separator />

      {/* ── Tips ── */}
      <div className="space-y-3">
        <div className="flex items-center justify-between">
          <Label className="text-sm font-semibold">💡 小贴士</Label>
          <Button variant="ghost" size="sm" onClick={addTip} className="text-xs h-7">
            <Plus size={14} className="mr-1" />添加
          </Button>
        </div>
        {edited.tips.map((tip, i) => (
          <div key={i} className="flex gap-2">
            <Input
              value={tip}
              onChange={(e) => updateTip(i, e.target.value)}
              placeholder="小贴士"
              className="h-8 text-sm"
            />
            <Button
              variant="ghost"
              size="sm"
              onClick={() => removeTip(i)}
              className="h-8 w-8 p-0 text-gray-400 hover:text-red-500 shrink-0"
            >
              <X size={14} />
            </Button>
          </div>
        ))}
      </div>

      <Separator />

      {/* ── Equipment ── */}
      <div className="space-y-3">
        <div className="flex items-center justify-between">
          <Label className="text-sm font-semibold">🔧 器具</Label>
          <Button variant="ghost" size="sm" onClick={addEquipment} className="text-xs h-7">
            <Plus size={14} className="mr-1" />添加
          </Button>
        </div>
        {edited.equipment.map((eq, i) => (
          <div key={i} className="flex gap-2">
            <Input
              value={eq}
              onChange={(e) => updateEquipment(i, e.target.value)}
              placeholder="器具名称"
              className="h-8 text-sm"
            />
            <Button
              variant="ghost"
              size="sm"
              onClick={() => removeEquipment(i)}
              className="h-8 w-8 p-0 text-gray-400 hover:text-red-500 shrink-0"
            >
              <X size={14} />
            </Button>
          </div>
        ))}
        {edited.equipment.length === 0 && (
          <p className="text-xs text-gray-400 text-center py-2">暂无器具</p>
        )}
      </div>

      {/* ── Actions ── */}
      <Separator />
      <div className="flex gap-3 pb-4">
        <Button
          onClick={handleSave}
          disabled={saving || !edited.name.trim()}
          className="flex-1 bg-xhs hover:bg-xhs-hover"
        >
          <Save size={16} className="mr-1.5" />
          {saving ? "保存中..." : "保存入库"}
        </Button>
      </div>
    </div>
  );
}

// ── Tag Input ──

function TagInput({ onAdd }: { onAdd: (tag: string) => void }) {
  const [value, setValue] = useState("");

  const handleAdd = () => {
    onAdd(value);
    setValue("");
  };

  return (
    <div className="flex gap-1 items-center">
      <Input
        value={value}
        onChange={(e) => setValue(e.target.value)}
        onKeyDown={(e) => {
          if (e.key === "Enter") {
            e.preventDefault();
            handleAdd();
          }
        }}
        placeholder="添加标签"
        className="h-7 w-24 text-xs"
      />
      <Button
        variant="ghost"
        size="sm"
        onClick={handleAdd}
        className="h-7 w-7 p-0"
        disabled={!value.trim()}
      >
        <Plus size={14} />
      </Button>
    </div>
  );
}
