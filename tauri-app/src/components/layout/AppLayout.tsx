import type { ReactNode } from "react";
import TabBar, { type Tab } from "./TabBar";

interface AppLayoutProps {
  currentTab: Tab;
  onNavigate: (tab: Tab) => void;
  children: ReactNode;
}

export default function AppLayout({
  currentTab,
  onNavigate,
  children,
}: AppLayoutProps) {
  return (
    <div className="min-h-screen bg-gray-50 pb-16">
      <main className="max-w-[800px] mx-auto p-6">{children}</main>
      <TabBar currentTab={currentTab} onNavigate={onNavigate} />
    </div>
  );
}
