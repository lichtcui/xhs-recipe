interface SkeletonCardProps {
  count?: number;
}

export default function SkeletonCard({ count = 3 }: SkeletonCardProps) {
  return (
    <div className="animate-pulse space-y-3">
      {Array.from({ length: count }).map((_, i) => (
        <div key={i} className="flex gap-3 p-3 bg-gray-100 rounded-xl">
          <div className="w-16 h-16 bg-gray-200 rounded-lg shrink-0" />
          <div className="flex-1 space-y-2 py-1">
            <div className="h-3 bg-gray-200 rounded w-2/3" />
            <div className="h-2.5 bg-gray-200 rounded w-1/2" />
          </div>
        </div>
      ))}
    </div>
  );
}
