import type { ReactNode } from "react";
import NavBar from "./NavBar";

interface AppLayoutProps {
  currentPage: string;
  onNavigate: (page: "home" | "settings") => void;
  children: ReactNode;
}

export default function AppLayout({
  currentPage,
  onNavigate,
  children,
}: AppLayoutProps) {
  return (
    <div className="min-h-screen bg-gray-50">
      <NavBar currentPage={currentPage} onNavigate={onNavigate} />
      <main className="max-w-[800px] mx-auto p-6">{children}</main>
    </div>
  );
}
