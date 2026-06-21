export function getFavorites(): Set<string> {
  try {
    return new Set<string>(JSON.parse(localStorage.getItem("xhs-favorites") || "[]"));
  } catch {
    return new Set<string>();
  }
}

export function favKey(sourceUrl: string, name: string): string {
  return `${sourceUrl}::${name}`;
}
