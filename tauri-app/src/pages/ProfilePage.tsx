import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Switch } from "@/components/ui/switch";
import { Separator } from "@/components/ui/separator";
import { useSettings } from "@/hooks/useSettings";
import CookieManager from "@/components/settings/CookieManager";
import PrerequisiteCheck from "@/components/settings/PrerequisiteCheck";
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
      <h2 className="text-[22px] font-bold text-xhs mb-5">我的</h2>

      <Card>
        <CardHeader className="pb-4">
          <CardTitle className="text-lg">提取配置</CardTitle>
        </CardHeader>
        <CardContent className="space-y-5">
          {/* ASR Model */}
          <FormField id="asr-model" label="ASR 模型">
            <Select
              value={settings.asrModel}
              onValueChange={(v) => updateSettings({ asrModel: v })}
            >
              <SelectTrigger id="asr-model">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="qwen3-asr-0.6b">
                  qwen3-asr-0.6b (快速)
                </SelectItem>
                <SelectItem value="qwen3-asr-1.7b">
                  qwen3-asr-1.7b (高精度)
                </SelectItem>
              </SelectContent>
            </Select>
          </FormField>

          {/* LLM Model */}
          <FormField id="llm-model" label="LLM 模型">
            <Select
              value={settings.llmModel}
              onValueChange={(v) => updateSettings({ llmModel: v })}
            >
              <SelectTrigger id="llm-model">
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

          {/* OCR Toggle */}
          <div className="flex items-center justify-between">
            <Label htmlFor="ocr-toggle" className="text-sm font-medium cursor-pointer">
              启用图片 OCR
            </Label>
            <Switch
              id="ocr-toggle"
              checked={settings.ocrImages}
              onCheckedChange={(v) => updateSettings({ ocrImages: v })}
            />
          </div>

          {/* API Key */}
          <FormField id="api-key" label="API Key">
            <Input
              id="api-key"
              type="password"
              placeholder="留空则使用环境变量或钥匙串"
              value={settings.apiKey}
              onChange={(e) => updateSettings({ apiKey: e.target.value })}
              autoComplete="off"
            />
          </FormField>

          {/* Timeout */}
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
            />
          </FormField>

          <Separator />

          <CookieManager />

          <Separator />

          <PrerequisiteCheck />
        </CardContent>
      </Card>
    </div>
  );
}
