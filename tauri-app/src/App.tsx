import { useState } from "react";
import { SettingsProvider } from "@/hooks/useSettings";
import AppLayout from "@/components/layout/AppLayout";
import HomePage from "@/components/home/HomePage";
import RecipeDetail from "@/components/detail/RecipeDetail";
import SettingsPage from "@/components/settings/SettingsPage";
import type { Recipe } from "@/types/recipe";

export type Page = "home" | "detail" | "settings";

function App() {
  const [page, setPage] = useState<Page>("home");
  const [selectedRecipe, setSelectedRecipe] = useState<Recipe | null>(null);

  const navigateToDetail = (recipe: Recipe) => {
    setSelectedRecipe(recipe);
    setPage("detail");
  };

  const navigateToHome = () => {
    setSelectedRecipe(null);
    setPage("home");
  };

  return (
    <SettingsProvider>
      <AppLayout
        currentPage={page}
        onNavigate={(p) => {
          if (p !== "detail") setPage(p);
        }}
      >
        {page === "home" && <HomePage onViewRecipe={navigateToDetail} />}
        {page === "detail" && (
          <RecipeDetail recipe={selectedRecipe} onBack={navigateToHome} />
        )}
        {page === "settings" && <SettingsPage />}
      </AppLayout>
    </SettingsProvider>
  );
}

export default App;
