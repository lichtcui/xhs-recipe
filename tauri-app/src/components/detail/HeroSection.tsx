import { Star, Pencil, Clock, ExternalLink } from "lucide-react";
import { invoke } from "@tauri-apps/api/core";

interface HeroSectionProps {
  coverImageUrl?: string;
  name: string;
  tags?: string[];
  totalTime?: string;
  sourceUrl?: string;
  isFavorite: boolean;
  onToggleFavorite: () => void;
  onEdit?: () => void;
}

export default function HeroSection({
  coverImageUrl,
  name,
  tags,
  totalTime,
  sourceUrl,
  isFavorite,
  onToggleFavorite,
  onEdit,
}: HeroSectionProps) {
  return (
    <div className="relative -mx-6 -mt-6 mb-4 h-48 overflow-hidden rounded-b-2xl">
      {coverImageUrl ? (
        <img
          src={coverImageUrl}
          alt={name}
          className="absolute inset-0 w-full h-full object-cover"
          onError={(e) => {
            (e.target as HTMLImageElement).style.display = "none";
          }}
        />
      ) : (
        <div className="absolute inset-0 bg-gradient-to-br from-xhs/20 to-orange-100" />
      )}
      {/* Gradient overlay */}
      <div className="absolute inset-0 bg-gradient-to-t from-black/60 via-black/20 to-transparent" />
      {/* Title + Time + Actions */}
      <div className="absolute bottom-4 left-4 right-4 flex items-end justify-between gap-2">
        <div>
          <h1 className="text-white text-2xl font-bold drop-shadow-lg">{name}</h1>
          {tags && tags.length > 0 && (
            <div className="flex flex-wrap gap-1.5 mt-1.5">
              {tags.map((tag) => (
                <span
                  key={tag}
                  className="text-[11px] px-2 py-0.5 rounded-full bg-white/20 text-white/90 drop-shadow-lg"
                >
                  {tag}
                </span>
              ))}
            </div>
          )}
          {totalTime && (
            <div className="flex items-center gap-1 mt-1">
              <Clock size={14} className="text-white/80 drop-shadow-lg" />
              <span className="text-white/80 text-sm drop-shadow-lg">{totalTime}</span>
            </div>
          )}
          {sourceUrl && (
            <button
              onClick={() => invoke("open_url", { url: sourceUrl })}
              className="flex items-center gap-1 mt-1 text-white/60 text-xs hover:text-white/90 transition-colors drop-shadow-lg"
              title={sourceUrl}
            >
              <ExternalLink size={12} />
              查看原文
            </button>
          )}
        </div>
        <div className="flex items-center gap-1 shrink-0">
          {onEdit && (
            <button
              onClick={onEdit}
              className="shrink-0 p-1 rounded-full hover:bg-white/10 transition-colors"
              title="修改菜谱"
            >
              <Pencil size={18} className="text-white/90 drop-shadow-lg" />
            </button>
          )}
          <button
            onClick={onToggleFavorite}
            className="shrink-0 p-1 rounded-full hover:bg-white/10 transition-colors"
          >
            <Star
              size={22}
              className={`drop-shadow-lg transition-colors ${
                isFavorite
                  ? "fill-amber-400 text-amber-400"
                  : "text-white/90"
              }`}
            />
          </button>
        </div>
      </div>
    </div>
  );
}
