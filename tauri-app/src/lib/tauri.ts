import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type {
  Recipe,
  RecipeSummary,
  PrerequisiteStatus,
  ProgressEvent,
  ExtractTextResult,
} from "@/types/recipe";
import type { ExtractSettingsPayload } from "@/types/settings";

export async function extractRecipe(
  url: string,
  settings: ExtractSettingsPayload
): Promise<Recipe[]> {
  return invoke<Recipe[]>("extract", { url, settings });
}

export async function listRecipes(): Promise<RecipeSummary[]> {
  return invoke<RecipeSummary[]>("list_recipes");
}

export async function getRecipe(id: string): Promise<Recipe> {
  return invoke<Recipe>("get_recipe", { id });
}

export async function deleteRecipe(id: string): Promise<void> {
  return invoke<void>("delete_recipe", { id });
}

export async function saveRecipe(recipe: Recipe): Promise<string> {
  return invoke<string>("save_recipe", { recipe });
}

export async function checkPrerequisites(): Promise<PrerequisiteStatus> {
  return invoke<PrerequisiteStatus>("check_prerequisites");
}

export async function checkCookies(): Promise<boolean> {
  return invoke<boolean>("check_cookies");
}

export async function importCookies(cookieJson: string): Promise<string> {
  return invoke<string>("import_cookies", { cookieJson });
}

export function onExtractProgress(
  callback: (event: ProgressEvent) => void
): Promise<UnlistenFn> {
  return listen<ProgressEvent>("extract:progress", (event) => {
    callback(event.payload);
  });
}

export async function extractText(
  url: string,
  settings: ExtractSettingsPayload
): Promise<ExtractTextResult> {
  return invoke<ExtractTextResult>("extract_text", { url, settings });
}

export async function analyzeRecipe(
  text: string,
  model: string,
  apiKey?: string
): Promise<Recipe[]> {
  return invoke<Recipe[]>("analyze_recipe", { text, model, apiKey: apiKey || null });
}
