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
  id?: string;
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
  cover_image_url?: string;
  image_urls?: string[];
  tags?: string[];
  difficulty?: string;
  servings?: number;
  raw_text?: string;
}

export interface RecipeSummary {
  id: string;
  name: string;
  source_url: string;
  saved_at: number;
  is_food: boolean;
  cover_image_url?: string;
  total_time?: string;
  difficulty?: string;
  tags: string[];
}

export interface ExtractTextResult {
  raw_text: string;
  cover_image_url?: string;
  image_urls: string[];
  title: string;
  source_url: string;
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
