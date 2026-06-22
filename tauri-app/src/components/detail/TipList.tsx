import { Lightbulb } from "lucide-react";

interface TipListProps {
  tips: string[];
}

export default function TipList({ tips }: TipListProps) {
  if (tips.length === 0) return null;

  const cleaned = tips.map((t) => {
    const trimmed = t.trimEnd();
    return trimmed.endsWith("。") ? trimmed.slice(0, -1) : trimmed;
  });

  return (
    <div className="mb-6">
      <h3 className="font-semibold text-sm text-gray-500 mb-2 flex items-center gap-1.5">
        <Lightbulb size={16} />
        小贴士
      </h3>
      <div className="space-y-1">
        {cleaned.map((tip, i) => (
          <p key={i} className="text-sm text-gray-600 leading-relaxed">
            • {tip}
          </p>
        ))}
      </div>
    </div>
  );
}
