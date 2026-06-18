interface TipListProps {
  tips: string[];
}

export default function TipList({ tips }: TipListProps) {
  const cleaned = tips.map((t) => {
    const trimmed = t.trimEnd();
    return trimmed.endsWith("。") ? trimmed.slice(0, -1) : trimmed;
  });

  return (
    <div className="text-sm text-gray-700 leading-relaxed">
      <span className="font-bold text-gray-600">💡 小贴士</span>{" "}
      {cleaned.join(" · ")}
    </div>
  );
}
