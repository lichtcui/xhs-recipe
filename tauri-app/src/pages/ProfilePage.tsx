import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Switch } from "@/components/ui/switch";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { useSettings } from "@/hooks/useSettings";
import { User } from "lucide-react";
import type { ReactNode } from "react";

function FormField({
  id,
  label,
  children,
}: {
  id: string;
  label: string;
  children: ReactNode;
}) {
  return (
    <div className="space-y-1.5">
      <Label htmlFor={id} className="text-sm font-medium">
        {label}
      </Label>
      {children}
    </div>
  );
}

export default function ProfilePage() {
  const { settings, updateSettings } = useSettings();

  return (
    <div>
      <div className="flex items-center gap-2 mb-6">
        <User size={22} className="text-xhs" />
        <h2 className="text-[22px] font-bold text-xhs">设置</h2>
      </div>

      {/* Voice recognition */}
      <div className="mb-6">
        <h3 className="text-base font-semibold text-gray-500 mb-4">语音识别</h3>
        <div className="space-y-4">
          <div className="flex items-center justify-between">
            <Label htmlFor="asr-switch" className="text-sm font-medium cursor-pointer">
              ASR 精度
            </Label>
            <div className="flex items-center gap-2">
              <span className={`text-xs ${settings.asrModel === "qwen3-asr-0.6b" ? "text-xhs font-medium" : "text-gray-400"}`}>高速</span>
              <Switch
                id="asr-switch"
                checked={settings.asrModel === "qwen3-asr-1.7b"}
                onCheckedChange={(v) =>
                  updateSettings({ asrModel: v ? "qwen3-asr-1.7b" : "qwen3-asr-0.6b" })
                }
              />
              <span className={`text-xs ${settings.asrModel === "qwen3-asr-1.7b" ? "text-xhs font-medium" : "text-gray-400"}`}>高精度</span>
            </div>
          </div>
        </div>
      </div>

      {/* LLM API */}
      <div className="border-t border-gray-100 pt-6">
        <h3 className="text-base font-semibold text-gray-500 mb-4">大模型 API</h3>
        <div className="space-y-4">
          <FormField id="llm-model" label="LLM 模型">
            <Select
              value={settings.llmModel}
              onValueChange={(v) => updateSettings({ llmModel: v })}
            >
              <SelectTrigger id="llm-model" className="rounded-xl border-gray-200 bg-white/80 focus:ring-xhs/30 focus:ring-offset-0 px-2.5 py-1.5">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="deepseek-chat">deepseek-chat</SelectItem>
                <SelectItem value="deepseek-reasoner">
                  deepseek-reasoner
                </SelectItem>
              </SelectContent>
            </Select>
          </FormField>

          <FormField id="api-key" label="API Key">
            <Input
              id="api-key"
              type="password"
              placeholder="留空则使用环境变量或钥匙串"
              value={settings.apiKey}
              onChange={(e) => updateSettings({ apiKey: e.target.value })}
              className="rounded-xl border-gray-200 bg-white/80 focus-visible:ring-xhs/30 focus-visible:ring-offset-0"
              autoComplete="off"
            />
          </FormField>

          <FormField id="timeout" label="超时时间（秒）">
            <Input
              id="timeout"
              type="number"
              min={30}
              max={1200}
              value={settings.timeout}
              onChange={(e) =>
                updateSettings({ timeout: parseInt(e.target.value) || 300 })
              }
              className="rounded-xl border-gray-200 bg-white/80 focus-visible:ring-xhs/30 focus-visible:ring-offset-0"
            />
          </FormField>
        </div>
      </div>
    </div>
  );
}
