import { Image } from "lucide-react";

interface FrameGalleryProps {
  imageUrls?: string[];
}

export default function FrameGallery({ imageUrls }: FrameGalleryProps) {
  if (!imageUrls || imageUrls.length === 0) return null;

  return (
    <div className="mb-4">
      <h3 className="font-semibold text-sm text-gray-500 mb-2 flex items-center gap-1.5">
        <Image size={16} />
        原始图片
      </h3>
      <div className="flex gap-2 overflow-x-auto pb-2">
        {imageUrls.map((url, i) => (
          <img
            key={i}
            src={url}
            alt={`图片 ${i + 1}`}
            className="w-24 h-24 rounded-xl object-cover shrink-0 border border-gray-100"
            onError={(e) => {
              (e.target as HTMLImageElement).style.display = "none";
            }}
          />
        ))}
      </div>
    </div>
  );
}
