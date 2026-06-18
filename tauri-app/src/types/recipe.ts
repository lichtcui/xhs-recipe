export interface Ingredient {
  name: string;
  amount?: string;
  prep?: string;
  category?: string;
}

export interface Step {
  title: string;
  time?: string;
  content: string;
}

export interface Recipe {
  name: string;
  total_time?: string;
  ingredients: Ingredient[];
  seasonings: Ingredient[];
  equipment: string[];
  steps: Step[];
  tips: string[];
  source_url: string;
  is_food: boolean;
  reason?: string;
}

export interface RecipeSummary {
  id: string;
  name: string;
  source_url: string;
  saved_at: number;
  is_food: boolean;
}

export interface PrerequisiteStatus {
  ffmpeg: boolean;
  tesseract: boolean;
  qwen_asr: boolean;
  cookies_exist: boolean;
}

export interface ProgressEvent {
  stage: string;
  detail: string;
}
