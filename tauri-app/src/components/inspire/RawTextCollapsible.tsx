import { Collapsible, CollapsibleContent, CollapsibleTrigger } from "@/components/ui/collapsible";
import { ChevronDown } from "lucide-react";

interface RawTextCollapsibleProps {
  text: string;
}

export default function RawTextCollapsible({ text }: RawTextCollapsibleProps) {
  if (!text) return null;

  return (
    <Collapsible>
      <CollapsibleTrigger className="flex items-center gap-1 text-xs text-gray-400 hover:text-gray-600 transition-colors">
        <ChevronDown size={14} />
        查看原始识别文本
      </CollapsibleTrigger>
      <CollapsibleContent className="mt-2">
        <div className="bg-gray-100 rounded-lg p-3 text-xs text-gray-600 max-h-48 overflow-y-auto whitespace-pre-wrap leading-relaxed">
          {text}
        </div>
      </CollapsibleContent>
    </Collapsible>
  );
}
