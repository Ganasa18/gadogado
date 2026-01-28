import { useState, useEffect } from "react";
import { motion, AnimatePresence } from "framer-motion";
import {
  Code,
  Plus,
  Trash2,
  Edit2,
  ToggleLeft,
  ToggleRight,
  ChevronDown,
  ChevronUp,
  Tag,
  Table as TableIcon,
  AlertCircle,
  Loader2,
  FileText,
  Search,
  Copy,
  Check,
} from "lucide-react";
import { cn } from "../../../utils/cn";
import type { QueryTemplate, QueryTemplateInput, QueryPatternType } from "../../rag/types";
import {
  dbListQueryTemplates,
  dbCreateQueryTemplate,
  dbUpdateQueryTemplate,
  dbDeleteQueryTemplate,
  dbToggleQueryTemplate,
} from "../../rag/api";
import { QueryTemplateFormModal } from "./QueryTemplateFormModal";
import { Pagination } from "./Pagination";

const TEMPLATES_PER_PAGE = 5;

interface QueryTemplatesSectionProps {
  profileId: number | null;
  availableTables: string[];
}

// Logging utility for debugging
const add_log = (category: string, message: string, data?: unknown) => {
  const timestamp = new Date().toISOString();
  const logEntry = `[${timestamp}] [${category}] ${message}`;
  if (data) {
    console.log(logEntry, data);
  } else {
    console.log(logEntry);
  }
};

const PATTERN_TYPE_LABELS: Record<QueryPatternType, { label: string; color: string }> = {
  select_where_eq: { label: "WHERE =", color: "bg-blue-500/20 text-blue-400" },
  select_where_in: { label: "WHERE IN", color: "bg-emerald-500/20 text-emerald-400" },
  select_with_join: { label: "JOIN", color: "bg-purple-500/20 text-purple-400" },
  aggregate: { label: "AGG", color: "bg-amber-500/20 text-amber-400" },
  custom: { label: "CUSTOM", color: "bg-gray-500/20 text-gray-400" },
};

export function QueryTemplatesSection({
  profileId,
  availableTables,
}: QueryTemplatesSectionProps) {
  const [templates, setTemplates] = useState<QueryTemplate[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [expanded, setExpanded] = useState<number | null>(null);
  const [showModal, setShowModal] = useState(false);
  const [editingTemplate, setEditingTemplate] = useState<QueryTemplate | null>(null);
  const [searchQuery, setSearchQuery] = useState("");
  const [copiedId, setCopiedId] = useState<number | null>(null);
  const [deleteConfirm, setDeleteConfirm] = useState<number | null>(null);

  // Pagination State
  const [currentPage, setCurrentPage] = useState(1);

  // Load templates when profileId changes
  useEffect(() => {
    if (profileId) {
      loadTemplates();
    }
  }, [profileId]);

  const loadTemplates = async () => {
    if (!profileId) {
      add_log("QueryTemplates", "loadTemplates: No profileId provided");
      return;
    }
    add_log("QueryTemplates", "loadTemplates: Starting", { profileId });
    setLoading(true);
    setError(null);
    try {
      const data = await dbListQueryTemplates(profileId);
      add_log("QueryTemplates", "loadTemplates: Success", { count: data.length, templates: data });
      setTemplates(data);
    } catch (err) {
      console.error("Failed to load templates:", err);
      add_log("QueryTemplates", "loadTemplates: ERROR", { error: err });
      setError("Failed to load query templates");
    } finally {
      setLoading(false);
    }
  };

  const handleCreate = async (input: QueryTemplateInput) => {
    add_log("QueryTemplates", "handleCreate: Starting", { input });
    try {
      const created = await dbCreateQueryTemplate(input);
      add_log("QueryTemplates", "handleCreate: Success", { created });
      setTemplates([...templates, created]);
    } catch (err) {
      console.error("Failed to create template:", err);
      add_log("QueryTemplates", "handleCreate: ERROR", { error: err, input });
      throw err;
    }
  };

  const handleUpdate = async (input: QueryTemplateInput) => {
    if (!editingTemplate) {
      add_log("QueryTemplates", "handleUpdate: No editing template");
      return;
    }
    add_log("QueryTemplates", "handleUpdate: Starting", { templateId: editingTemplate.id, input });
    try {
      const updated = await dbUpdateQueryTemplate(editingTemplate.id, input);
      add_log("QueryTemplates", "handleUpdate: Success", { updated });
      setTemplates(templates.map((t) => (t.id === updated.id ? updated : t)));
      setEditingTemplate(null);
    } catch (err) {
      console.error("Failed to update template:", err);
      add_log("QueryTemplates", "handleUpdate: ERROR", { error: err, templateId: editingTemplate.id, input });
      throw err;
    }
  };

  const handleDelete = async (templateId: number) => {
    add_log("QueryTemplates", "handleDelete: Starting", { templateId });
    try {
      await dbDeleteQueryTemplate(templateId);
      add_log("QueryTemplates", "handleDelete: Success", { templateId });
      setTemplates(templates.filter((t) => t.id !== templateId));
      setDeleteConfirm(null);
    } catch (err) {
      console.error("Failed to delete template:", err);
      add_log("QueryTemplates", "handleDelete: ERROR", { error: err, templateId });
      throw err;
    }
  };

  const handleToggle = async (template: QueryTemplate) => {
    const newEnabledState = !template.is_enabled;
    add_log("QueryTemplates", "handleToggle: Starting", { templateId: template.id, newEnabledState });
    try {
      const updated = await dbToggleQueryTemplate(template.id, newEnabledState);
      add_log("QueryTemplates", "handleToggle: Success", { updated });
      setTemplates(templates.map((t) => (t.id === updated.id ? updated : t)));
    } catch (err) {
      console.error("Failed to toggle template:", err);
      add_log("QueryTemplates", "handleToggle: ERROR", { error: err, templateId: template.id, newEnabledState });
      throw err;
    }
  };

  const handleCopyPattern = (template: QueryTemplate) => {
    navigator.clipboard.writeText(template.query_pattern);
    setCopiedId(template.id);
    setTimeout(() => setCopiedId(null), 2000);
  };

  const filteredTemplates = templates.filter((t) =>
    t.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
    t.intent_keywords.some((k) => k.toLowerCase().includes(searchQuery.toLowerCase()))
  );

  const sortedTemplates = [...filteredTemplates].sort((a, b) => b.priority - a.priority);

  const paginatedTemplates = sortedTemplates.slice(
    (currentPage - 1) * TEMPLATES_PER_PAGE,
    currentPage * TEMPLATES_PER_PAGE
  );

  // Reset pagination when search query or profile changes
  useEffect(() => {
    setCurrentPage(1);
  }, [searchQuery, profileId]);

  if (!profileId) {
    return (
      <div className="bg-app-panel/40 border border-app-border rounded-2xl p-6">
        <div className="text-center py-8 text-app-subtext">
          <AlertCircle className="w-10 h-10 mx-auto mb-3 opacity-40" />
          <p className="text-sm">Select a profile to manage query templates</p>
        </div>
      </div>
    );
  }

  return (
    <div className="bg-app-panel/40 border border-app-border rounded-2xl overflow-hidden">
      {/* Header */}
      <div className="px-6 py-4 bg-app-card/30 border-b border-app-border">
        <div className="flex items-center justify-between mb-3">
          <div className="flex items-center gap-3">
            <div className="w-10 h-10 rounded-xl bg-app-accent/10 flex items-center justify-center">
              <Code className="w-5 h-5 text-app-accent" />
            </div>
            <div>
              <h3 className="text-sm font-bold text-app-text">Query Reference Templates</h3>
              <p className="text-[11px] text-app-subtext">
                Define SQL patterns for LLM to use instead of generating arbitrary queries
              </p>
            </div>
          </div>
          <button
            onClick={() => {
              setEditingTemplate(null);
              setShowModal(true);
            }}
            className="px-4 py-2 bg-app-accent text-white rounded-xl text-xs font-bold shadow-lg shadow-app-accent/20 hover:bg-app-accent/90 transition-all hover:scale-[1.02] active:scale-[0.98] flex items-center gap-2"
          >
            <Plus className="w-4 h-4" />
            Add Template
          </button>
        </div>

        {/* Search */}
        {templates.length > 0 && (
          <div className="relative">
            <Search className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-app-subtext" />
            <input
              type="text"
              placeholder="Search templates..."
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              className="w-full bg-app-card/40 border border-app-border rounded-xl pl-10 pr-4 py-2 text-sm focus:ring-2 focus:ring-app-accent/50 focus:border-app-accent outline-none transition-all"
            />
          </div>
        )}
      </div>

      {/* Content */}
      <div className="p-4">
        {loading ? (
          <div className="flex justify-center py-12">
            <Loader2 className="w-8 h-8 animate-spin text-app-subtext/40" />
          </div>
        ) : error ? (
          <div className="text-center py-8">
            <AlertCircle className="w-10 h-10 mx-auto mb-3 text-destructive/60" />
            <p className="text-sm text-destructive">{error}</p>
            <button
              onClick={loadTemplates}
              className="mt-3 text-xs text-app-accent hover:underline"
            >
              Try again
            </button>
          </div>
        ) : sortedTemplates.length === 0 ? (
          <div className="text-center py-12">
            <div className="w-16 h-16 mx-auto mb-4 rounded-2xl bg-app-border/30 flex items-center justify-center">
              <FileText className="w-8 h-8 text-app-subtext/40" />
            </div>
            <h4 className="text-sm font-bold text-app-text mb-1">No Templates Yet</h4>
            <p className="text-xs text-app-subtext max-w-xs mx-auto">
              Create query templates to control how LLM generates SQL queries. Templates ensure consistency and prevent arbitrary query generation.
            </p>
          </div>
        ) : (
          <div className="space-y-4">
            <div className="space-y-2 min-h-[300px]">
              {paginatedTemplates.map((template) => (
                <motion.div
                  key={template.id}
                  layout
                  initial={{ opacity: 0, y: 10 }}
                  animate={{ opacity: 1, y: 0 }}
                  className={cn(
                    "border rounded-xl transition-all overflow-hidden",
                    template.is_enabled
                      ? "bg-app-card/50 border-app-border"
                      : "bg-app-card/20 border-app-border/50 opacity-60"
                  )}
                >
                  {/* ... same content ... */}
                  <div
                    className="px-4 py-3 flex items-center gap-3 cursor-pointer hover:bg-app-card/30 transition-colors"
                    onClick={() => setExpanded(expanded === template.id ? null : template.id)}
                  >
                    {/* Priority Badge */}
                    <div className="w-8 h-8 rounded-lg bg-app-border/40 flex items-center justify-center text-xs font-bold text-app-subtext shrink-0">
                      {template.priority}
                    </div>

                    {/* Info */}
                    <div className="flex-1 min-w-0">
                      <div className="flex items-center gap-2 mb-0.5">
                        <span className="text-sm font-bold text-app-text truncate">
                          {template.name}
                        </span>
                        <span
                          className={cn(
                            "px-2 py-0.5 rounded text-[10px] font-bold uppercase",
                            PATTERN_TYPE_LABELS[template.pattern_type].color
                          )}
                        >
                          {PATTERN_TYPE_LABELS[template.pattern_type].label}
                        </span>
                      </div>
                      <div className="flex items-center gap-2 text-[11px] text-app-subtext">
                        <span className="flex items-center gap-1">
                          <TableIcon className="w-3 h-3" />
                          {template.tables_used.join(", ")}
                        </span>
                        <span>•</span>
                        <span className="flex items-center gap-1">
                          <Tag className="w-3 h-3" />
                          {template.intent_keywords.length} keywords
                        </span>
                      </div>
                    </div>

                    {/* Actions */}
                    <div className="flex items-center gap-1 shrink-0" onClick={(e) => e.stopPropagation()}>
                      <button
                        onClick={() => handleCopyPattern(template)}
                        className="p-2 rounded-lg hover:bg-app-border/40 transition-colors text-app-subtext hover:text-app-text"
                        title="Copy SQL pattern"
                      >
                        {copiedId === template.id ? (
                          <Check className="w-4 h-4 text-app-success" />
                        ) : (
                          <Copy className="w-4 h-4" />
                        )}
                      </button>
                      <button
                        onClick={() => handleToggle(template)}
                        className={cn(
                          "p-2 rounded-lg transition-colors",
                          template.is_enabled
                            ? "text-app-success hover:bg-app-success/10"
                            : "text-app-subtext hover:bg-app-border/40"
                        )}
                        title={template.is_enabled ? "Disable template" : "Enable template"}
                      >
                        {template.is_enabled ? (
                          <ToggleRight className="w-4 h-4" />
                        ) : (
                          <ToggleLeft className="w-4 h-4" />
                        )}
                      </button>
                      <button
                        onClick={() => {
                          setEditingTemplate(template);
                          setShowModal(true);
                        }}
                        className="p-2 rounded-lg hover:bg-app-border/40 transition-colors text-app-subtext hover:text-app-text"
                        title="Edit template"
                      >
                        <Edit2 className="w-4 h-4" />
                      </button>
                      <button
                        onClick={() => setDeleteConfirm(template.id)}
                        className="p-2 rounded-lg hover:bg-destructive/10 transition-colors text-app-subtext hover:text-destructive"
                        title="Delete template"
                      >
                        <Trash2 className="w-4 h-4" />
                      </button>
                    </div>

                    {/* Expand Icon */}
                    <div className="text-app-subtext">
                      {expanded === template.id ? (
                        <ChevronUp className="w-4 h-4" />
                      ) : (
                        <ChevronDown className="w-4 h-4" />
                      )}
                    </div>
                  </div>

                  {/* Expanded Details */}
                  <AnimatePresence>
                    {expanded === template.id && (
                      <motion.div
                        initial={{ height: 0, opacity: 0 }}
                        animate={{ height: "auto", opacity: 1 }}
                        exit={{ height: 0, opacity: 0 }}
                        transition={{ duration: 0.2 }}
                        className="border-t border-app-border"
                      >
                        <div className="p-4 space-y-4 bg-app-bg/30">
                          {/* Description */}
                          {template.description && (
                            <div>
                              <label className="block text-[10px] font-bold uppercase tracking-wider text-app-subtext mb-1">
                                Description
                              </label>
                              <p className="text-xs text-app-text">{template.description}</p>
                            </div>
                          )}

                          {/* Example Question */}
                          <div>
                            <label className="block text-[10px] font-bold uppercase tracking-wider text-app-subtext mb-1">
                              Example Question
                            </label>
                            <p className="text-xs text-app-text italic">"{template.example_question}"</p>
                          </div>

                          {/* Query Pattern */}
                          <div>
                            <label className="block text-[10px] font-bold uppercase tracking-wider text-app-subtext mb-1">
                              SQL Pattern
                            </label>
                            <pre className="p-3 bg-app-card/50 rounded-lg text-xs font-mono text-app-text overflow-x-auto whitespace-pre-wrap">
                              {template.query_pattern}
                            </pre>
                          </div>

                          {/* Intent Keywords */}
                          <div>
                            <label className="block text-[10px] font-bold uppercase tracking-wider text-app-subtext mb-1">
                              Intent Keywords
                            </label>
                            <div className="flex flex-wrap gap-1">
                              {template.intent_keywords.map((keyword) => (
                                <span
                                  key={keyword}
                                  className="px-2 py-0.5 bg-app-accent/10 text-app-accent rounded text-[10px] font-semibold"
                                >
                                  {keyword}
                                </span>
                              ))}
                            </div>
                          </div>
                        </div>
                      </motion.div>
                    )}
                  </AnimatePresence>

                  {/* Delete Confirmation */}
                  <AnimatePresence>
                    {deleteConfirm === template.id && (
                      <motion.div
                        initial={{ height: 0, opacity: 0 }}
                        animate={{ height: "auto", opacity: 1 }}
                        exit={{ height: 0, opacity: 0 }}
                        className="border-t border-destructive/30 bg-destructive/5"
                      >
                        <div className="p-4 flex items-center justify-between">
                          <p className="text-xs text-destructive">Delete this template?</p>
                          <div className="flex items-center gap-2">
                            <button
                              onClick={() => setDeleteConfirm(null)}
                              className="px-3 py-1.5 text-xs font-semibold text-app-subtext hover:text-app-text"
                            >
                              Cancel
                            </button>
                            <button
                              onClick={() => handleDelete(template.id)}
                              className="px-3 py-1.5 bg-destructive text-white rounded-lg text-xs font-bold hover:bg-destructive/90"
                            >
                              Delete
                            </button>
                          </div>
                        </div>
                      </motion.div>
                    )}
                  </AnimatePresence>
                </motion.div>
              ))}
            </div>
            
            <Pagination
              currentPage={currentPage}
              totalItems={sortedTemplates.length}
              itemsPerPage={TEMPLATES_PER_PAGE}
              onPageChange={setCurrentPage}
              className="mt-2 pt-4 border-t border-app-border/40"
            />
          </div>
        )
      }
    </div>

      {/* Template Form Modal */}
      <QueryTemplateFormModal
        isOpen={showModal}
        onClose={() => {
          setShowModal(false);
          setEditingTemplate(null);
        }}
        onSave={editingTemplate ? handleUpdate : handleCreate}
        template={editingTemplate}
        profileId={profileId}
        availableTables={availableTables}
      />
    </div>
  );
}
