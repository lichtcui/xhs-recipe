import type { Recipe, ExtractTextResult } from "@/types/recipe";

export type ExtractionStatus =
  | "IDLE"
  | "PARSING"
  | "PARSED"
  | "GENERATING"
  | "GENERATED"
  | "SAVED";

export interface ExtractionState {
  status: ExtractionStatus;
  url?: string;
  progress?: { stage: string; percent: number };
  rawText?: string;
  coverImageUrl?: string;
  recipe?: Recipe;
  streamedText?: string;
  recipeId?: string;
  error?: string;
  isDirty?: boolean;
}

export type ExtractionAction =
  | { type: "START_PARSING"; url: string }
  | { type: "UPDATE_PROGRESS"; stage: string; percent: number }
  | { type: "PARSED"; recipe: Recipe }
  | { type: "PARSED_TEXT"; result: ExtractTextResult }
  | { type: "START_GENERATING" }
  | { type: "TOKEN_RECEIVED"; token: string }
  | { type: "GENERATED"; recipe: Recipe }
  | { type: "SAVED"; recipeId: string }
  | { type: "RESET" }
  | { type: "ERROR"; message: string };

export const initialExtractionState: ExtractionState = {
  status: "IDLE",
};

export function extractionReducer(
  state: ExtractionState,
  action: ExtractionAction
): ExtractionState {
  switch (action.type) {
    case "START_PARSING":
      return {
        ...initialExtractionState,
        status: "PARSING",
        url: action.url,
        progress: { stage: "fetching", percent: 5 },
      };

    case "UPDATE_PROGRESS":
      return {
        ...state,
        progress: { stage: action.stage, percent: action.percent },
      };

    case "PARSED":
      return {
        ...state,
        status: "PARSED",
        recipe: action.recipe,
        rawText: action.recipe.raw_text,
        coverImageUrl: action.recipe.cover_image_url,
        progress: { stage: "done", percent: 100 },
      };

    case "PARSED_TEXT":
      return {
        ...state,
        status: "PARSED",
        rawText: action.result.raw_text,
        coverImageUrl: action.result.cover_image_url,
        progress: { stage: "done", percent: 100 },
      };

    case "START_GENERATING":
      return {
        ...state,
        status: "GENERATING",
        streamedText: "",
      };

    case "TOKEN_RECEIVED":
      return {
        ...state,
        streamedText: (state.streamedText || "") + action.token,
      };

    case "GENERATED":
      return {
        ...state,
        status: "GENERATED",
        recipe: action.recipe,
        isDirty: false,
      };

    case "SAVED":
      return {
        ...state,
        status: "SAVED",
        recipeId: action.recipeId,
        isDirty: false,
      };

    case "RESET":
      return initialExtractionState;

    case "ERROR":
      return {
        ...state,
        error: action.message,
      };

    default:
      return state;
  }
}
