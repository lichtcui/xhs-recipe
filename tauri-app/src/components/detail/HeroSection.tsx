import { Star, Pencil } from "lucide-react";

interface HeroSectionProps {
  coverImageUrl?: string;
  name: string;
  isFavorite: boolean;
  onToggleFavorite: () => void;
  onEdit?: () => void;
}

export default function HeroSection({
  coverImageUrl,
  name,
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
      {/* Title + Actions */}
      <div className="absolute bottom-4 left-4 right-4 flex items-center justify-between gap-2">
        <h1 className="text-white text-2xl font-bold drop-shadow-lg">{name}</h1>
        <div className="flex items-center gap-1">
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
