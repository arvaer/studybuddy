import { useState, useEffect } from "react";
import { motion, AnimatePresence } from "framer-motion";
import {
  User,
  Bell,
  Moon,
  BookOpen,
  Target,
  Loader2,
  Save,
  Bot,
  Eye,
  EyeOff,
  Trash2,
} from "lucide-react";
import { AppLayout } from "@/components/layout/app-layout";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { Switch } from "@/components/ui/switch";
import { Slider } from "@/components/ui/slider";
import { Label } from "@/components/ui/label";
import { Input } from "@/components/ui/input";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { useAuth } from "@/contexts/AuthContext";
import { fetchSettings, updateSettings, type Settings } from "@/lib/api";
import {
  getLlmConfig,
  saveLlmConfig,
  clearLlmConfig,
  callLlm,
  PROVIDER_DEFAULTS,
  type LlmProvider,
  type LlmConfig,
} from "@/lib/llm";
import { toast } from "sonner";

const defaultSettings: Settings = {
  reinforcementPrompts: true,
  questionFrequency: 50,
  aiGeneratedNotes: true,
  studyTimeGoal: 30,
  dailyQuestions: 10,
  dailyReminders: true,
  streakAlerts: true,
  reviewReminders: false,
  reduceAnimations: false,
};

export default function SettingsPage() {
  const { user } = useAuth();
  const [savedSettings, setSavedSettings] = useState<Settings>(defaultSettings);
  const [draft, setDraft] = useState<Settings>(defaultSettings);
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);

  // AI Provider state (localStorage-backed, independent of server settings)
  const [llmProvider, setLlmProvider] = useState<LlmProvider>("openai");
  const [llmApiKey, setLlmApiKey] = useState("");
  const [llmModel, setLlmModel] = useState("");
  const [llmBaseUrl, setLlmBaseUrl] = useState("");
  const [showApiKey, setShowApiKey] = useState(false);
  const [testingConnection, setTestingConnection] = useState(false);

  useEffect(() => {
    fetchSettings()
      .then((s) => { setSavedSettings(s); setDraft(s); })
      .catch((err) => console.error("Failed to load settings:", err))
      .finally(() => setLoading(false));
  }, []);

  // Load LLM config from localStorage on mount
  useEffect(() => {
    const cfg = getLlmConfig();
    if (cfg) {
      setLlmProvider(cfg.provider);
      setLlmApiKey(cfg.apiKey);
      setLlmModel(cfg.model);
      setLlmBaseUrl(cfg.baseUrl);
    } else {
      const defaults = PROVIDER_DEFAULTS["openai"];
      setLlmModel(defaults.model);
      setLlmBaseUrl(defaults.baseUrl);
    }
  }, []);

  const isDirty = JSON.stringify(draft) !== JSON.stringify(savedSettings);

  const handleSave = async () => {
    setSaving(true);
    try {
      const updated = await updateSettings(draft);
      setSavedSettings(updated);
      setDraft(updated);
    } catch (err) {
      console.error("Failed to save settings:", err);
    } finally {
      setSaving(false);
    }
  };

  const update = (patch: Partial<Settings>) => {
    setDraft((prev) => ({ ...prev, ...patch }));
  };

  const handleProviderChange = (provider: LlmProvider) => {
    setLlmProvider(provider);
    const defaults = PROVIDER_DEFAULTS[provider];
    setLlmModel(defaults.model);
    setLlmBaseUrl(defaults.baseUrl);
    saveLlmConfig({ provider, apiKey: llmApiKey, model: defaults.model, baseUrl: defaults.baseUrl });
  };

  const handleLlmFieldChange = (field: keyof LlmConfig, value: string) => {
    if (field === "apiKey") setLlmApiKey(value);
    else if (field === "model") setLlmModel(value);
    else if (field === "baseUrl") setLlmBaseUrl(value);
    const cfg: LlmConfig = {
      provider: llmProvider,
      apiKey: field === "apiKey" ? value : llmApiKey,
      model: field === "model" ? value : llmModel,
      baseUrl: field === "baseUrl" ? value : llmBaseUrl,
    };
    saveLlmConfig(cfg);
  };

  const handleTestConnection = async () => {
    setTestingConnection(true);
    try {
      await callLlm({ messages: [{ role: "user", content: "Say OK" }], maxTokens: 10 });
      toast.success("Connection successful");
    } catch (err) {
      toast.error(err instanceof Error ? err.message : "Connection failed");
    } finally {
      setTestingConnection(false);
    }
  };

  const handleClearLlm = () => {
    clearLlmConfig();
    setLlmProvider("openai");
    setLlmApiKey("");
    const defaults = PROVIDER_DEFAULTS["openai"];
    setLlmModel(defaults.model);
    setLlmBaseUrl(defaults.baseUrl);
    toast.success("AI provider config cleared");
  };

  if (loading) {
    return (
      <AppLayout>
        <div className="flex items-center justify-center h-full">
          <Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
        </div>
      </AppLayout>
    );
  }

  const initials = user?.displayName
    ? user.displayName
        .split(" ")
        .map((w) => w[0])
        .join("")
        .toUpperCase()
        .slice(0, 2)
    : "??";

  const frequencyLabel = (v: number) => {
    if (v <= 25) return "Low";
    if (v <= 50) return "Medium";
    if (v <= 75) return "High";
    return "Maximum";
  };

  return (
    <AppLayout>
      <div className="p-8 max-w-2xl mx-auto">
        <motion.header
          initial={{ opacity: 0, y: -20 }}
          animate={{ opacity: 1, y: 0 }}
          className="mb-8"
        >
          <h1 className="font-display text-3xl font-bold text-foreground tracking-tight">
            Settings
          </h1>
          <p className="mt-1 text-muted-foreground">
            Customize your learning experience
          </p>
        </motion.header>

        <div className="space-y-6">
          {/* Profile */}
          <Card className="p-6">
            <div className="flex items-center gap-3 mb-6">
              <User className="h-5 w-5 text-muted-foreground" />
              <h2 className="font-display font-semibold text-foreground">Profile</h2>
            </div>
            <div className="flex items-center gap-4">
              <div className="h-16 w-16 rounded-full bg-primary/10 flex items-center justify-center">
                <span className="font-display text-xl font-bold text-primary">{initials}</span>
              </div>
              <div>
                <p className="font-medium text-foreground">{user?.displayName ?? "Unknown"}</p>
                <p className="text-sm text-muted-foreground">{user?.email ?? ""}</p>
              </div>
            </div>
          </Card>

          {/* Learning Preferences */}
          <Card className="p-6">
            <div className="flex items-center gap-3 mb-6">
              <BookOpen className="h-5 w-5 text-muted-foreground" />
              <h2 className="font-display font-semibold text-foreground">Learning Preferences</h2>
            </div>

            <div className="space-y-6">
              <div className="flex items-center justify-between">
                <div>
                  <Label className="text-foreground">Reinforcement Prompts</Label>
                  <p className="text-sm text-muted-foreground">
                    Show quick check questions during reading
                  </p>
                </div>
                <Switch
                  checked={draft.reinforcementPrompts}
                  onCheckedChange={(v) => update({ reinforcementPrompts: v })}
                />
              </div>

              <div className="space-y-3">
                <div className="flex items-center justify-between">
                  <Label className="text-foreground">Question Frequency</Label>
                  <span className="text-sm text-muted-foreground">
                    {frequencyLabel(draft.questionFrequency)}
                  </span>
                </div>
                <Slider
                  value={[draft.questionFrequency]}
                  onValueChange={([v]) => update({ questionFrequency: v })}
                  max={100}
                  step={1}
                />
                <div className="flex justify-between text-xs text-muted-foreground">
                  <span>Fewer</span>
                  <span>More</span>
                </div>
              </div>

              <div className="flex items-center justify-between">
                <div>
                  <Label className="text-foreground">AI-Generated Notes</Label>
                  <p className="text-sm text-muted-foreground">
                    Automatically create study notes
                  </p>
                </div>
                <Switch
                  checked={draft.aiGeneratedNotes}
                  onCheckedChange={(v) => update({ aiGeneratedNotes: v })}
                />
              </div>
            </div>
          </Card>

          {/* AI Provider */}
          <Card className="p-6">
            <div className="flex items-center gap-3 mb-6">
              <Bot className="h-5 w-5 text-muted-foreground" />
              <h2 className="font-display font-semibold text-foreground">AI Provider</h2>
            </div>

            <div className="space-y-4">
              <div className="space-y-2">
                <Label className="text-foreground">Provider</Label>
                <Select value={llmProvider} onValueChange={(v) => handleProviderChange(v as LlmProvider)}>
                  <SelectTrigger>
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="openai">OpenAI</SelectItem>
                    <SelectItem value="anthropic">Anthropic</SelectItem>
                    <SelectItem value="ollama">Ollama</SelectItem>
                    <SelectItem value="custom">Custom</SelectItem>
                  </SelectContent>
                </Select>
              </div>

              <div className="space-y-2">
                <Label className="text-foreground">API Key</Label>
                <div className="relative">
                  <Input
                    type={showApiKey ? "text" : "password"}
                    value={llmApiKey}
                    onChange={(e) => handleLlmFieldChange("apiKey", e.target.value)}
                    placeholder="sk-..."
                    className="pr-10"
                  />
                  <button
                    type="button"
                    onClick={() => setShowApiKey(!showApiKey)}
                    className="absolute right-3 top-1/2 -translate-y-1/2 text-muted-foreground hover:text-foreground"
                  >
                    {showApiKey ? <EyeOff className="h-4 w-4" /> : <Eye className="h-4 w-4" />}
                  </button>
                </div>
              </div>

              <div className="space-y-2">
                <Label className="text-foreground">Model</Label>
                <Input
                  value={llmModel}
                  onChange={(e) => handleLlmFieldChange("model", e.target.value)}
                  placeholder="gpt-4o-mini"
                />
              </div>

              {(llmProvider === "ollama" || llmProvider === "custom") && (
                <div className="space-y-2">
                  <Label className="text-foreground">Base URL</Label>
                  <Input
                    value={llmBaseUrl}
                    onChange={(e) => handleLlmFieldChange("baseUrl", e.target.value)}
                    placeholder="http://localhost:11434"
                  />
                </div>
              )}

              <div className="flex gap-2 pt-2">
                <Button
                  variant="outline"
                  size="sm"
                  onClick={handleTestConnection}
                  disabled={testingConnection || !llmApiKey}
                >
                  {testingConnection ? <Loader2 className="h-4 w-4 animate-spin mr-1.5" /> : null}
                  Test Connection
                </Button>
                <Button variant="ghost" size="sm" onClick={handleClearLlm}>
                  <Trash2 className="h-4 w-4 mr-1.5" />
                  Clear
                </Button>
              </div>
            </div>
          </Card>

          {/* Goals */}
          <Card className="p-6">
            <div className="flex items-center gap-3 mb-6">
              <Target className="h-5 w-5 text-muted-foreground" />
              <h2 className="font-display font-semibold text-foreground">Daily Goals</h2>
            </div>

            <div className="space-y-6">
              <div className="space-y-3">
                <div className="flex items-center justify-between">
                  <Label className="text-foreground">Study Time Goal</Label>
                  <span className="text-sm text-muted-foreground">
                    {draft.studyTimeGoal} minutes
                  </span>
                </div>
                <Slider
                  value={[draft.studyTimeGoal]}
                  onValueChange={([v]) => update({ studyTimeGoal: v })}
                  min={5}
                  max={120}
                  step={1}
                />
              </div>

              <div className="space-y-3">
                <div className="flex items-center justify-between">
                  <Label className="text-foreground">Daily Questions</Label>
                  <span className="text-sm text-muted-foreground">
                    {draft.dailyQuestions} questions
                  </span>
                </div>
                <Slider
                  value={[draft.dailyQuestions]}
                  onValueChange={([v]) => update({ dailyQuestions: v })}
                  min={1}
                  max={50}
                  step={1}
                />
              </div>
            </div>
          </Card>

          {/* Notifications */}
          <Card className="p-6">
            <div className="flex items-center gap-3 mb-6">
              <Bell className="h-5 w-5 text-muted-foreground" />
              <h2 className="font-display font-semibold text-foreground">Notifications</h2>
            </div>

            <div className="space-y-4">
              <div className="flex items-center justify-between">
                <div>
                  <Label className="text-foreground">Daily Reminders</Label>
                  <p className="text-sm text-muted-foreground">
                    Get reminded to study
                  </p>
                </div>
                <Switch
                  checked={draft.dailyReminders}
                  onCheckedChange={(v) => update({ dailyReminders: v })}
                />
              </div>

              <div className="flex items-center justify-between">
                <div>
                  <Label className="text-foreground">Streak Alerts</Label>
                  <p className="text-sm text-muted-foreground">
                    Don't lose your streak!
                  </p>
                </div>
                <Switch
                  checked={draft.streakAlerts}
                  onCheckedChange={(v) => update({ streakAlerts: v })}
                />
              </div>

              <div className="flex items-center justify-between">
                <div>
                  <Label className="text-foreground">Review Reminders</Label>
                  <p className="text-sm text-muted-foreground">
                    When concepts need reinforcement
                  </p>
                </div>
                <Switch
                  checked={draft.reviewReminders}
                  onCheckedChange={(v) => update({ reviewReminders: v })}
                />
              </div>
            </div>
          </Card>

          {/* Appearance */}
          <Card className="p-6">
            <div className="flex items-center gap-3 mb-6">
              <Moon className="h-5 w-5 text-muted-foreground" />
              <h2 className="font-display font-semibold text-foreground">Appearance</h2>
            </div>

            <div className="flex items-center justify-between">
              <div>
                <Label className="text-foreground">Reduce Animations</Label>
                <p className="text-sm text-muted-foreground">
                  For accessibility
                </p>
              </div>
              <Switch
                checked={draft.reduceAnimations}
                onCheckedChange={(v) => update({ reduceAnimations: v })}
              />
            </div>
          </Card>
        </div>

        {/* Sticky save bar */}
        <AnimatePresence>
          {isDirty && (
            <motion.div
              initial={{ opacity: 0, y: 20 }}
              animate={{ opacity: 1, y: 0 }}
              exit={{ opacity: 0, y: 20 }}
              className="sticky bottom-6 flex justify-end mt-6"
            >
              <Button onClick={handleSave} disabled={saving} className="shadow-lg gap-2">
                {saving ? <Loader2 className="h-4 w-4 animate-spin" /> : <Save className="h-4 w-4" />}
                {saving ? "Saving…" : "Save Changes"}
              </Button>
            </motion.div>
          )}
        </AnimatePresence>
      </div>
    </AppLayout>
  );
}
