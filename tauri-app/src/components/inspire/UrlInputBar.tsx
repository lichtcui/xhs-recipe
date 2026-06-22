import { useState, useCallback } from "react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Link } from "lucide-react";

interface UrlInputBarProps {
  onExtract: (url: string) => void;
  disabled?: boolean;
}

export default function UrlInputBar({ onExtract, disabled }: UrlInputBarProps) {
  const [url, setUrl] = useState("");

  const handleExtract = useCallback(() => {
    const trimmed = url.trim();
    if (!trimmed || disabled) return;
    onExtract(trimmed);
  }, [url, disabled, onExtract]);

  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent) => {
      if (e.key === "Enter") handleExtract();
    },
    [handleExtract]
  );

  return (
    <div>
      <div className="flex gap-2">
        <div className="relative flex-1">
          <Link
            size={16}
            className="absolute left-3.5 top-1/2 -translate-y-1/2 text-gray-400 pointer-events-none"
          />
          <Input
            placeholder="粘贴小红书分享链接..."
            value={url}
            onChange={(e) => setUrl(e.target.value)}
            onKeyDown={handleKeyDown}
            disabled={disabled}
            className="flex-1 h-12 pl-10 rounded-xl border-gray-200 bg-white/80 text-[15px] focus-visible:ring-xhs/30 focus-visible:ring-offset-0 placeholder:text-gray-400"
            autoComplete="off"
          />
        </div>
        <Button
          onClick={handleExtract}
          disabled={disabled || !url.trim()}
          className="h-12 px-5 rounded-xl bg-xhs hover:bg-xhs-hover shrink-0 text-sm font-medium disabled:opacity-50"
        >
          提取
        </Button>
      </div>
      <p className="text-[12px] text-gray-300 mt-2 ml-1">
        支持视频、图文、合集等形式的小红书笔记
      </p>
    </div>
  );
}
