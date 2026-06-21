interface HeroSectionProps {
  coverImageUrl?: string;
  name: string;
}

export default function HeroSection({ coverImageUrl, name }: HeroSectionProps) {
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
      {/* Title */}
      <div className="absolute bottom-4 left-4 right-4">
        <h1 className="text-white text-2xl font-bold drop-shadow-lg">{name}</h1>
      </div>
    </div>
  );
}
