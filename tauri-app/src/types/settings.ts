export interface AppSettings {
  asrModel: string;
  llmModel: string;
  ocrImages: boolean;
  apiKey: string;
  timeout: number;
}

export interface ExtractSettingsPayload {
  asr_model: string;
  ocr_images: boolean;
  llm_model: string;
  api_key: string | null;
  timeout_secs: number;
}

export const SETTINGS_DEFAULTS: AppSettings = {
  asrModel: "qwen3-asr-1.7b",
  llmModel: "deepseek-chat",
  ocrImages: true,
  apiKey: "",
  timeout: 300,
};

export function toExtractPayload(settings: AppSettings): ExtractSettingsPayload {
  return {
    asr_model: settings.asrModel,
    ocr_images: settings.ocrImages,
    llm_model: settings.llmModel,
    api_key: settings.apiKey || null,
    timeout_secs: settings.timeout,
  };
}
