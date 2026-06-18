import { Button } from "@/components/ui/button";

interface NavBarProps {
  currentPage: string;
  onNavigate: (page: "home" | "settings") => void;
}

export default function NavBar({ currentPage, onNavigate }: NavBarProps) {
  return (
    <nav className="flex gap-0 bg-white border-b border-border px-4">
      <Button
        variant="ghost"
        className={`rounded-none border-b-2 px-5 py-3 h-auto text-[15px] ${
          currentPage === "home"
            ? "border-xhs text-xhs"
            : "border-transparent text-muted-foreground hover:text-foreground"
        }`}
        onClick={() => onNavigate("home")}
      >
        首页
      </Button>
      <Button
        variant="ghost"
        className={`rounded-none border-b-2 px-5 py-3 h-auto text-[15px] ${
          currentPage === "settings"
            ? "border-xhs text-xhs"
            : "border-transparent text-muted-foreground hover:text-foreground"
        }`}
        onClick={() => onNavigate("settings")}
      >
        设置
      </Button>
    </nav>
  );
}
