interface FrameGalleryProps {
  imageUrls?: string[];
}

export default function FrameGallery({ imageUrls }: FrameGalleryProps) {
  if (!imageUrls || imageUrls.length === 0) return null;

  return (
    <div className="mb-4">
      <p className="text-xs text-gray-400 mb-2">原始图片</p>
      <div className="flex gap-2 overflow-x-auto pb-2 -mx-1 px-1">
        {imageUrls.map((url, i) => (
          <img
            key={i}
            src={url}
            alt={`图片 ${i + 1}`}
            className="w-24 h-24 rounded-lg object-cover shrink-0 border border-gray-100"
            onError={(e) => {
              (e.target as HTMLImageElement).style.display = "none";
            }}
          />
        ))}
      </div>
    </div>
  );
}
