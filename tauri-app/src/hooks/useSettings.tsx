import {
  createContext,
  useContext,
  useState,
  useEffect,
  useCallback,
  type ReactNode,
} from "react";
import {
  type AppSettings,
  SETTINGS_DEFAULTS,
  toExtractPayload,
  type ExtractSettingsPayload,
} from "@/types/settings";

const SETTINGS_KEY = "xhs-recipe-settings";

function loadSettings(): AppSettings {
  try {
    const saved = localStorage.getItem(SETTINGS_KEY);
    if (saved) return { ...SETTINGS_DEFAULTS, ...JSON.parse(saved) };
  } catch {
    // ignore
  }
  return { ...SETTINGS_DEFAULTS };
}

interface SettingsContextValue {
  settings: AppSettings;
  updateSettings: (partial: Partial<AppSettings>) => void;
  getExtractPayload: () => ExtractSettingsPayload;
}

const SettingsContext = createContext<SettingsContextValue | null>(null);

export function SettingsProvider({ children }: { children: ReactNode }) {
  const [settings, setSettings] = useState<AppSettings>(loadSettings);

  useEffect(() => {
    localStorage.setItem(SETTINGS_KEY, JSON.stringify(settings));
  }, [settings]);

  const updateSettings = useCallback((partial: Partial<AppSettings>) => {
    setSettings((prev) => ({ ...prev, ...partial }));
  }, []);

  const getExtractPayload = useCallback(
    () => toExtractPayload(settings),
    [settings]
  );

  return (
    <SettingsContext.Provider
      value={{ settings, updateSettings, getExtractPayload }}
    >
      {children}
    </SettingsContext.Provider>
  );
}

export function useSettings(): SettingsContextValue {
  const ctx = useContext(SettingsContext);
  if (!ctx)
    throw new Error("useSettings must be used within SettingsProvider");
  return ctx;
}
