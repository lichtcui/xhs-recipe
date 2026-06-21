import { Button } from "@/components/ui/button";
import { Sparkles } from "lucide-react";

interface AiRefineButtonProps {
  onClick: () => void;
  disabled?: boolean;
}

export default function AiRefineButton({
  onClick,
  disabled,
}: AiRefineButtonProps) {
  return (
    <Button
      onClick={onClick}
      disabled={disabled}
      className="w-full bg-gradient-to-r from-orange-400 to-red-500 hover:from-orange-500 hover:to-red-600 text-white font-semibold text-base py-5 rounded-xl shadow-lg shadow-orange-200 transition-all duration-200 hover:scale-[1.02] disabled:opacity-50 disabled:cursor-not-allowed disabled:hover:scale-100"
    >
      <Sparkles size={18} className="mr-2" />
      AI 提炼菜谱
    </Button>
  );
}
