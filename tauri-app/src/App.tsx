import { useState, useEffect } from "react";
import { SettingsProvider } from "@/hooks/useSettings";
import { Toaster } from "sonner";
import AppLayout from "@/components/layout/AppLayout";
import type { Tab } from "@/components/layout/TabBar";
import InspirePage from "@/pages/InspirePage";
import RecipesPage from "@/pages/RecipesPage";
import ProfilePage from "@/pages/ProfilePage";

function App() {
  const [currentTab, setCurrentTab] = useState<Tab>("inspire");

  // Keyboard shortcuts
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      const mod = e.metaKey || e.ctrlKey;
      if (mod && e.key === "n") {
        e.preventDefault();
        setCurrentTab("inspire");
      }
    };
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, []);

  return (
    <SettingsProvider>
      <Toaster position="top-center" richColors />
      <AppLayout currentTab={currentTab} onNavigate={setCurrentTab}>
        <div style={{ display: currentTab === "inspire" ? "block" : "none" }}>
          <InspirePage />
        </div>
        <div style={{ display: currentTab === "recipes" ? "block" : "none" }}>
          <RecipesPage />
        </div>
        <div style={{ display: currentTab === "profile" ? "block" : "none" }}>
          <ProfilePage />
        </div>
      </AppLayout>
    </SettingsProvider>
  );
}

export default App;
