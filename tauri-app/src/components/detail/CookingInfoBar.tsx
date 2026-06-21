import { Clock } from "lucide-react";

interface CookingInfoBarProps {
  totalTime?: string;
}

export default function CookingInfoBar({
  totalTime,
}: CookingInfoBarProps) {
  return (
    <div className="flex items-center gap-4 mb-4 text-sm text-gray-500">
      {totalTime && (
        <div className="flex items-center gap-1">
          <Clock size={16} className="text-xhs" />
          <span>{totalTime}</span>
        </div>
      )}
    </div>
  );
}
