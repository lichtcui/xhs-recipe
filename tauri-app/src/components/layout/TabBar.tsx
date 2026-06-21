import { Sparkles, BookOpen, CookingPot, User } from "lucide-react";

export type Tab = "inspire" | "recipes" | "cooking" | "profile";

interface TabBarProps {
  currentTab: Tab;
  onNavigate: (tab: Tab) => void;
}

const TABS: { id: Tab; label: string; icon: typeof Sparkles }[] = [
  { id: "inspire", label: "灵感厨房", icon: Sparkles },
  { id: "recipes", label: "我的菜谱", icon: BookOpen },
  { id: "cooking", label: "烹饪台", icon: CookingPot },
  { id: "profile", label: "我的", icon: User },
];

export default function TabBar({ currentTab, onNavigate }: TabBarProps) {
  return (
    <nav className="fixed bottom-0 left-0 right-0 bg-white border-t z-50">
      <div className="max-w-[800px] mx-auto flex justify-around">
        {TABS.map(({ id, label, icon: Icon }) => {
          const active = currentTab === id;
          return (
            <button
              key={id}
              onClick={() => onNavigate(id)}
              className={`flex flex-col items-center gap-0.5 py-2 px-4 min-w-[64px] transition-all duration-150 ${
                active
                  ? "text-xhs scale-105"
                  : "text-gray-400 hover:text-gray-600"
              }`}
            >
              <Icon size={22} strokeWidth={active ? 2.5 : 2} />
              <span className="text-[11px] leading-tight">{label}</span>
            </button>
          );
        })}
      </div>
    </nav>
  );
}
