import { useState } from "react";
import { SettingsProvider } from "@/hooks/useSettings";
import { Toaster } from "sonner";
import AppLayout from "@/components/layout/AppLayout";
import type { Tab } from "@/components/layout/TabBar";
import InspirePage from "@/pages/InspirePage";
import RecipesPage from "@/pages/RecipesPage";
import CookingPage from "@/pages/CookingPage";
import ProfilePage from "@/pages/ProfilePage";
import type { Recipe } from "@/types/recipe";

function App() {
  const [currentTab, setCurrentTab] = useState<Tab>("inspire");
  const [selectedRecipe, setSelectedRecipe] = useState<Recipe | null>(null);

  const navigateToCooking = (recipe: Recipe) => {
    setSelectedRecipe(recipe);
    setCurrentTab("cooking");
  };

  const navigateToInspire = () => {
    setCurrentTab("inspire");
  };

  return (
    <SettingsProvider>
      <Toaster position="top-center" richColors />
      <AppLayout currentTab={currentTab} onNavigate={setCurrentTab}>
        {/* Keep all pages mounted with display:none to preserve state across tab switches */}
        <div style={{ display: currentTab === "inspire" ? "block" : "none" }}>
          <InspirePage onViewRecipe={navigateToCooking} />
        </div>
        <div style={{ display: currentTab === "recipes" ? "block" : "none" }}>
          <RecipesPage onViewRecipe={navigateToCooking} />
        </div>
        <div style={{ display: currentTab === "cooking" ? "block" : "none" }}>
          <CookingPage
            recipe={selectedRecipe}
            onBackToInspire={navigateToInspire}
          />
        </div>
        <div style={{ display: currentTab === "profile" ? "block" : "none" }}>
          <ProfilePage />
        </div>
      </AppLayout>
    </SettingsProvider>
  );
}

export default App;
