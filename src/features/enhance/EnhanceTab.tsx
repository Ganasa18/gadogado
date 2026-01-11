import { useState } from "react";
import { TextArea } from "../../shared/components/TextArea";
import { Button } from "../../shared/components/Button";
import {
  Sparkles,
  Copy,
  History,
  Check,
  Zap,
  RefreshCw,
  Settings2,
  Plus,
  Trash2,
  Edit3,
  RotateCcw,
  X,
  ChevronDown,
} from "lucide-react";
import { useSettingsStore, PromptTemplate } from "../../store/settings";
import { useHistoryStore } from "../../store/history";
import { useLlmConfigBuilder } from "../../hooks/useLlmConfig";
import { useEnhanceMutation } from "../../hooks/useLlmApi";

export default function EnhanceTab() {
  const [input, setInput] = useState("");
  const [output, setOutput] = useState("");
  const [copied, setCopied] = useState(false);
  const [showTemplateModal, setShowTemplateModal] = useState(false);
  const [editingTemplate, setEditingTemplate] = useState<PromptTemplate | null>(
    null
  );
  const [newTemplateName, setNewTemplateName] = useState("");
  const [newTemplatePrompt, setNewTemplatePrompt] = useState("");

  const {
    provider,
    model,
    promptTemplates,
    activeTemplateId,
    setActiveTemplateId,
    addPromptTemplate,
    updatePromptTemplate,
    deletePromptTemplate,
    restoreDefaultTemplates,
  } = useSettingsStore();
  const { addItem } = useHistoryStore();
  const buildConfig = useLlmConfigBuilder();
  const enhanceMutation = useEnhanceMutation();
  const isLoading = enhanceMutation.isPending;

  const activeTemplate =
    promptTemplates.find((t) => t.id === activeTemplateId) ||
    promptTemplates[0];

  const handleEnhance = async () => {
    if (!input.trim() || isLoading) return;

    window.dispatchEvent(
      new CustomEvent("global-loading:start", { detail: { id: "enhance" } })
    );
    try {
      const result = await enhanceMutation.mutateAsync({
        config: buildConfig({ maxTokens: 2000, temperature: 0.8 }),
        content: input,
        system_prompt: activeTemplate?.systemPrompt,
      });

      setOutput(result.result);
      addItem({
        type: "enhancement",
        input,
        output: result.result,
        provider,
        model,
      });
    } catch (e: any) {
      console.error("Enhancement failed", e);
      setOutput(`Error: ${e.message || "Check backend logs"}`);
    } finally {
      window.dispatchEvent(
        new CustomEvent("global-loading:end", { detail: { id: "enhance" } })
      );
    }
  };

  const handleRegenerate = async () => {
    if (!input.trim() || isLoading) return;
    await handleEnhance();
  };

  const copyToClipboard = () => {
    navigator.clipboard.writeText(output);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  const handleSaveTemplate = () => {
    if (!newTemplateName.trim() || !newTemplatePrompt.trim()) return;

    if (editingTemplate) {
      updatePromptTemplate(editingTemplate.id, {
        name: newTemplateName,
        systemPrompt: newTemplatePrompt,
      });
    } else {
      addPromptTemplate({
        name: newTemplateName,
        systemPrompt: newTemplatePrompt,
      });
    }

    setNewTemplateName("");
    setNewTemplatePrompt("");
    setEditingTemplate(null);
  };

  const handleEditTemplate = (template: PromptTemplate) => {
    setEditingTemplate(template);
    setNewTemplateName(template.name);
    setNewTemplatePrompt(template.systemPrompt);
  };

  const handleCancelEdit = () => {
    setEditingTemplate(null);
    setNewTemplateName("");
    setNewTemplatePrompt("");
  };

  return (
    <div className="max-w-3xl mx-auto p-6 space-y-8 animate-in fade-in slide-in-from-bottom-4 duration-500">
      <div className="space-y-2">
        <h3 className="text-2xl font-bold tracking-tight flex items-center gap-2">
          <Sparkles className="w-6 h-6 text-yellow-500" />
          Prompt Enhancement
        </h3>
        <p className="text-muted-foreground text-sm">
          Optimize your prompts for better LLM performance, structure, and
          clarity.
        </p>
      </div>

      {/* Template Selector */}
      <div className="flex items-center gap-3">
        <div className="flex-1 relative">
          <select
            value={activeTemplateId}
            onChange={(e: any) => setActiveTemplateId(e.target.value)}
            className="w-full bg-app-card border border-app-border rounded-lg p-3 pr-10 text-sm appearance-none cursor-pointer hover:border-gray-500 transition outline-none text-app-text">
            {promptTemplates.map((template) => (
              <option key={template.id} value={template.id}>
                {template.name}
                {template.isDefault ? " (Built-in)" : ""}
              </option>
            ))}
          </select>
          <ChevronDown className="w-4 h-4 absolute right-3 top-3.5 text-gray-500 pointer-events-none" />
        </div>
        <Button
          variant="ghost"
          size="sm"
          onClick={() => setShowTemplateModal(true)}
          className="h-10 px-3 border border-app-border"
          title="Manage Templates">
          <Settings2 className="w-4 h-4" />
        </Button>
      </div>

      <div className="space-y-6">
        <div className="space-y-3">
          <label className="text-xs font-bold uppercase tracking-widest text-muted-foreground px-1">
            Input Text
          </label>
          <TextArea
            placeholder="Paste your text here..."
            className="min-h-[160px] text-base p-4 bg-card/40 focus:ring-yellow-500/20"
            value={input}
            onInput={(e: any) => setInput(e.target.value)}
          />
        </div>

        <div className="flex justify-center">
          <Button
            size="lg"
            className="gap-2 px-12 bg-gradient-to-r from-yellow-500 to-amber-600 hover:from-yellow-600 hover:to-amber-700 text-white border-none shadow-lg shadow-yellow-500/20 h-14"
            onClick={handleEnhance}
            disabled={!input.trim() || isLoading}>
            <Zap className="w-5 h-5 fill-current" />
            {isLoading ? "Processing..." : "Enhance"}
          </Button>
        </div>

        {output && (
          <div className="space-y-3 animate-in fade-in zoom-in-95 duration-500">
            <div className="flex items-center justify-between px-1">
              <span className="text-xs font-bold uppercase tracking-widest text-muted-foreground">
                Enhanced Version
              </span>
              <div className="flex items-center gap-2">
                <div className="relative group">
                  <Button
                    variant="ghost"
                    size="sm"
                    onClick={handleRegenerate}
                    className="h-6 px-2 text-[10px]"
                    disabled={isLoading}>
                    <RefreshCw
                      className={`w-3 h-3 mr-1 ${
                        isLoading ? "animate-spin" : ""
                      }`}
                    />
                    Regenerate
                  </Button>
                  <div className="absolute bottom-full left-1/2 -translate-x-1/2 mb-2 px-2 py-1 bg-app-card border border-app-border rounded text-[10px] text-app-text opacity-0 group-hover:opacity-100 transition-opacity whitespace-nowrap pointer-events-none z-10 shadow-lg">
                    Regenerate prompt with same input
                  </div>
                </div>
                <Button
                  variant="ghost"
                  size="sm"
                  onClick={copyToClipboard}
                  className="h-6 px-2 text-[10px]">
                  {copied ? (
                    <Check className="w-3 h-3 mr-1 text-green-500" />
                  ) : (
                    <Copy className="w-3 h-3 mr-1" />
                  )}
                  Copy Result
                </Button>
              </div>
            </div>
            <TextArea
              readOnly
              className="min-h-[240px] text-base p-4 bg-yellow-500/5 border-yellow-500/20 font-medium"
              value={output}
            />
          </div>
        )}
      </div>

      {!output && (
        <div className="p-8 border-2 border-dashed border-border rounded-xl flex flex-col items-center justify-center text-center space-y-3 bg-muted/10 opacity-60">
          <History className="w-8 h-8 text-muted-foreground" />
          <p className="text-sm text-muted-foreground max-w-xs italic">
            "Better prompts lead to better results. Enhance your structure
            today."
          </p>
        </div>
      )}

      {/* Template Management Modal */}
      {showTemplateModal && (
        <div className="fixed inset-0 bg-black/60 flex items-center justify-center z-50 p-4">
          <div className="bg-app-bg border border-app-border rounded-xl w-full max-w-2xl max-h-[80vh] overflow-hidden flex flex-col shadow-2xl">
            {/* Modal Header */}
            <div className="flex items-center justify-between p-4 border-b border-app-border">
              <h3 className="text-lg font-bold text-app-text">
                Manage Prompt Templates
              </h3>
              <button
                onClick={() => {
                  setShowTemplateModal(false);
                  handleCancelEdit();
                }}
                className="text-gray-500 hover:text-white transition">
                <X className="w-5 h-5" />
              </button>
            </div>

            {/* Modal Content */}
            <div className="flex-1 overflow-y-auto p-4 space-y-4">
              {/* Template List */}
              <div className="space-y-2">
                {promptTemplates.map((template) => (
                  <div
                    key={template.id}
                    className={`p-3 rounded-lg border transition ${
                      activeTemplateId === template.id
                        ? "border-yellow-500/50 bg-yellow-500/10"
                        : "border-app-border bg-app-card hover:border-gray-600"
                    }`}>
                    <div className="flex items-center justify-between">
                      <div className="flex items-center gap-2">
                        <span className="font-medium text-app-text">
                          {template.name}
                        </span>
                        {template.isDefault && (
                          <span className="text-[10px] px-2 py-0.5 rounded bg-gray-700 text-gray-300">
                            Built-in
                          </span>
                        )}
                      </div>
                      <div className="flex items-center gap-1">
                        {!template.isDefault && (
                          <>
                            <button
                              onClick={() => handleEditTemplate(template)}
                              className="p-1.5 text-gray-500 hover:text-white transition rounded hover:bg-white/10"
                              title="Edit template">
                              <Edit3 className="w-3.5 h-3.5" />
                            </button>
                            <button
                              onClick={() => deletePromptTemplate(template.id)}
                              className="p-1.5 text-gray-500 hover:text-red-400 transition rounded hover:bg-red-500/10"
                              title="Delete template">
                              <Trash2 className="w-3.5 h-3.5" />
                            </button>
                          </>
                        )}
                      </div>
                    </div>
                    <p className="text-xs text-gray-500 mt-1 line-clamp-2">
                      {template.systemPrompt}
                    </p>
                  </div>
                ))}
              </div>

              {/* Add/Edit Template Form */}
              <div className="border-t border-app-border pt-4 space-y-3">
                <h4 className="text-sm font-bold text-app-text">
                  {editingTemplate ? "Edit Template" : "Add New Template"}
                </h4>
                <input
                  type="text"
                  placeholder="Template name"
                  value={newTemplateName}
                  onInput={(e: any) => setNewTemplateName(e.target.value)}
                  className="w-full bg-app-card border border-app-border rounded-lg p-2.5 text-sm outline-none focus:border-gray-500 transition text-app-text placeholder-gray-500"
                  maxLength={50}
                />
                <textarea
                  placeholder="System prompt..."
                  value={newTemplatePrompt}
                  onInput={(e: any) => setNewTemplatePrompt(e.target.value)}
                  className="w-full bg-app-card border border-app-border rounded-lg p-2.5 text-sm outline-none focus:border-gray-500 transition text-app-text placeholder-gray-500 min-h-[100px] resize-none"
                  maxLength={4096}
                />
                <div className="flex items-center gap-2">
                  <Button
                    size="sm"
                    onClick={handleSaveTemplate}
                    disabled={
                      !newTemplateName.trim() || !newTemplatePrompt.trim()
                    }
                    className="gap-1">
                    {editingTemplate ? (
                      <>
                        <Check className="w-3.5 h-3.5" />
                        Save Changes
                      </>
                    ) : (
                      <>
                        <Plus className="w-3.5 h-3.5" />
                        Add Template
                      </>
                    )}
                  </Button>
                  {editingTemplate && (
                    <Button
                      variant="ghost"
                      size="sm"
                      onClick={handleCancelEdit}>
                      Cancel
                    </Button>
                  )}
                </div>
              </div>
            </div>

            {/* Modal Footer */}
            <div className="p-4 border-t border-app-border flex justify-between">
              <Button
                variant="ghost"
                size="sm"
                onClick={() => {
                  restoreDefaultTemplates();
                  handleCancelEdit();
                }}
                className="gap-1 text-yellow-500 hover:text-yellow-400">
                <RotateCcw className="w-3.5 h-3.5" />
                Restore Defaults
              </Button>
              <Button
                variant="ghost"
                size="sm"
                onClick={() => {
                  setShowTemplateModal(false);
                  handleCancelEdit();
                }}>
                Close
              </Button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
