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
import CookieManager from "./CookieManager";
import PrerequisiteCheck from "./PrerequisiteCheck";

export default function SettingsPage() {
  const { settings, updateSettings } = useSettings();

  return (
    <div>
      <h2 className="text-[22px] font-bold text-xhs mb-5">设置</h2>

      <Card>
        <CardHeader className="pb-4">
          <CardTitle className="text-lg">提取配置</CardTitle>
        </CardHeader>
        <CardContent className="space-y-4">
          {/* ASR Model */}
          <div className="space-y-1.5">
            <Label className="text-sm font-medium">ASR 模型</Label>
            <Select
              value={settings.asrModel}
              onValueChange={(v) => updateSettings({ asrModel: v })}
            >
              <SelectTrigger>
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
          </div>

          {/* LLM Model */}
          <div className="space-y-1.5">
            <Label className="text-sm font-medium">LLM 模型</Label>
            <Select
              value={settings.llmModel}
              onValueChange={(v) => updateSettings({ llmModel: v })}
            >
              <SelectTrigger>
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="deepseek-chat">deepseek-chat</SelectItem>
                <SelectItem value="deepseek-reasoner">
                  deepseek-reasoner
                </SelectItem>
              </SelectContent>
            </Select>
          </div>

          {/* OCR Toggle */}
          <div className="flex items-center justify-between">
            <Label className="text-sm font-medium">启用图片 OCR</Label>
            <Switch
              checked={settings.ocrImages}
              onCheckedChange={(v) => updateSettings({ ocrImages: v })}
            />
          </div>

          {/* API Key */}
          <div className="space-y-1.5">
            <Label className="text-sm font-medium">API Key</Label>
            <Input
              type="password"
              placeholder="留空则使用环境变量或钥匙串"
              value={settings.apiKey}
              onChange={(e) => updateSettings({ apiKey: e.target.value })}
              autoComplete="off"
            />
          </div>

          {/* Timeout */}
          <div className="space-y-1.5">
            <Label className="text-sm font-medium">超时时间（秒）</Label>
            <Input
              type="number"
              min={30}
              max={1200}
              value={settings.timeout}
              onChange={(e) =>
                updateSettings({ timeout: parseInt(e.target.value) || 300 })
              }
            />
          </div>

          <Separator />

          <CookieManager />

          <Separator />

          <PrerequisiteCheck />
        </CardContent>
      </Card>
    </div>
  );
}
