import { useState, useCallback } from "react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";

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
    <div className="flex gap-2">
      <Input
        placeholder="粘贴小红书分享链接..."
        value={url}
        onChange={(e) => setUrl(e.target.value)}
        onKeyDown={handleKeyDown}
        disabled={disabled}
        className="flex-1"
        autoComplete="off"
      />
      <Button
        onClick={handleExtract}
        disabled={disabled || !url.trim()}
        className="bg-xhs hover:bg-xhs-hover shrink-0"
      >
        提取
      </Button>
    </div>
  );
}
