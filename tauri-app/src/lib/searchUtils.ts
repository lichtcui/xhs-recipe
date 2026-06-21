import type { RecipeSummary } from "@/types/recipe";

export function fuzzyMatch(text: string, query: string): boolean {
  if (!query.trim()) return true;
  const lower = text.toLowerCase();
  const q = query.toLowerCase().trim();
  // Simple includes match for Chinese + English
  return lower.includes(q);
}

export function searchRecipes(
  recipes: RecipeSummary[],
  query: string
): RecipeSummary[] {
  if (!query.trim()) return recipes;
  return recipes.filter(
    (r) =>
      fuzzyMatch(r.name, query) ||
      r.tags?.some((t) => fuzzyMatch(t, query))
  );
}

export function filterByTag(
  recipes: RecipeSummary[],
  tag: string | null
): RecipeSummary[] {
  if (!tag) return recipes;
  return recipes.filter((r) => r.tags?.includes(tag));
}

export function collectTags(recipes: RecipeSummary[]): string[] {
  const tagSet = new Set<string>();
  for (const r of recipes) {
    for (const t of r.tags || []) {
      tagSet.add(t);
    }
  }
  return [...tagSet].sort();
}
