import { Lightbulb } from "lucide-react";
import { Card, CardContent } from "@/components/ui/card";

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
    <div className="mb-4">
      <h3 className="font-semibold text-sm text-gray-500 mb-2 flex items-center gap-1.5">
        <Lightbulb size={16} />
        小贴士
      </h3>
      <Card>
        <CardContent className="p-3 space-y-2">
          {cleaned.map((tip, i) => (
            <p key={i} className="text-sm text-gray-700 leading-relaxed">
              • {tip}
            </p>
          ))}
        </CardContent>
      </Card>
    </div>
  );
}
