import { useMemo, useRef, useState, useEffect } from "react";
import { motion, AnimatePresence } from "framer-motion";
import {
  DndContext,
  DragOverlay,
  PointerSensor,
  closestCenter,
  pointerWithin,
  useDroppable,
  useSensor,
  useSensors,
  type CollisionDetection,
  type DragStartEvent,
  type DragEndEvent,
} from "@dnd-kit/core";
import {
  SortableContext,
  useSortable,
  arrayMove,
  horizontalListSortingStrategy,
} from "@dnd-kit/sortable";
import { CSS } from "@dnd-kit/utilities";
import {
  X,
  Save,
  Code,
  FileText,
  Tag,
  Hash,
  Zap,
  AlertCircle,
  Plus,
  GripVertical,
} from "lucide-react";
import { cn } from "../../../utils/cn";
import type {
  QueryTemplate,
  QueryTemplateInput,
  QueryPatternType,
} from "../../rag/types";
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

interface QueryTemplateFormModalProps {
  isOpen: boolean;
  onClose: () => void;
  onSave: (input: QueryTemplateInput) => Promise<void>;
  template?: QueryTemplate | null; // For editing
  profileId: number;
  availableTables: string[];
}

const PATTERN_TYPES: {
  value: QueryPatternType;
  label: string;
  description: string;
  icon: string;
}[] = [
  {
    value: "select_where_eq",
    label: "WHERE =",
    description: "Single value match",
    icon: "=",
  },
  {
    value: "select_where_in",
    label: "WHERE IN",
    description: "Multiple values list",
    icon: "∈",
  },
  {
    value: "select_with_join",
    label: "JOIN",
    description: "Join with another table",
    icon: "⋈",
  },
  {
    value: "aggregate",
    label: "Aggregate",
    description: "COUNT, SUM, AVG, etc",
    icon: "Σ",
  },
  {
    value: "select_where_between",
    label: "BETWEEN",
    description: "Date/value range filter",
    icon: "↔",
  },
  {
    value: "select_where_like",
    label: "LIKE",
    description: "Text pattern search",
    icon: "≈",
  },
  {
    value: "custom",
    label: "Custom",
    description: "Custom query pattern",
    icon: "✎",
  },
];

const PLACEHOLDER_HINTS = [
  { placeholder: "{columns}", description: "Column list from allowlist" },
  { placeholder: "{table}", description: "Table name" },
  { placeholder: "{id}", description: "Single ID value" },
  { placeholder: "{id_list}", description: "Multiple IDs for IN clause" },
  { placeholder: "{id_column}", description: "ID column name" },
  { placeholder: "{date_start}", description: "Start date for BETWEEN" },
  { placeholder: "{date_end}", description: "End date for BETWEEN" },
  { placeholder: "{search_term}", description: "Search text for LIKE" },
  { placeholder: "{filter_column}", description: "Column name for filter" },
  { placeholder: "{order_by_column}", description: "Column for ORDER BY" },
  { placeholder: "{sort_direction}", description: "ASC or DESC" },
  { placeholder: "{group_by_column}", description: "Column for GROUP BY" },
  { placeholder: "{numeric_column}", description: "Numeric column name" },
  { placeholder: "{date_column}", description: "Date column name" },
  { placeholder: "{related_table}", description: "Related/joined table name" },
  {
    placeholder: "{foreign_key_column}",
    description: "Foreign key column for join",
  },
  {
    placeholder: "{main_table_columns}",
    description: "Columns from main table",
  },
  {
    placeholder: "{related_table_columns}",
    description: "Columns from related table",
  },
  {
    placeholder: "{filter_column_1}",
    description: "First filter column for multi-condition WHERE",
  },
  {
    placeholder: "{search_term_1}",
    description: "First search value for multi-condition WHERE",
  },
  {
    placeholder: "{filter_column_2}",
    description: "Second filter column for multi-condition WHERE",
  },
  {
    placeholder: "{search_term_2}",
    description: "Second search value for multi-condition WHERE",
  },
  {
    placeholder: "{text_column}",
    description: "Text column for LIKE search",
  },
];

function SortableHintChip({
  id,
  label,
  title,
}: {
  id: string;
  label: string;
  title?: string;
}) {
  const {
    attributes,
    listeners,
    setNodeRef,
    transform,
    transition,
    isDragging,
  } = useSortable({ id, data: { kind: "hint" } });

  const style: React.CSSProperties = {
    transform: CSS.Transform.toString(transform),
    transition,
  };

  return (
    <div
      ref={setNodeRef}
      style={style}
      title={title}
      className={cn(
        "group inline-flex items-center gap-1.5 px-3 py-1.5 rounded-full border border-app-border/70 bg-app-card/40 hover:bg-app-card/60 text-[11px] font-mono text-app-subtext hover:text-app-text transition-colors select-none",
        isDragging && "opacity-0 ring-2 ring-app-accent/30",
      )}>
      <span
        className="-ml-2 inline-flex items-center text-app-subtext/60 group-hover:text-app-subtext cursor-grab active:cursor-grabbing"
        {...attributes}
        {...listeners}
        aria-label="Drag to reorder">
        <GripVertical className="w-3 h-3" />
      </span>
      <span>{label}</span>
    </div>
  );
}

function HintChipOverlay({ label }: { label: string }) {
  return (
    <div className="inline-flex items-center gap-1.5 px-3 py-1.5 rounded-full border border-app-border/70 bg-app-panel text-[11px] font-mono text-app-text shadow-xl">
      <GripVertical className="w-3 h-3 text-app-subtext/70" />
      <span>{label}</span>
    </div>
  );
}

function QueryPatternDropZone({
  value,
  onChange,
  textareaRef,
}: {
  value: string;
  onChange: (next: string) => void;
  textareaRef: React.MutableRefObject<HTMLTextAreaElement | null>;
}) {
  const { setNodeRef, isOver } = useDroppable({ id: "query-pattern-drop" });

  return (
    <div ref={setNodeRef} className="rounded-xl">
      <textarea
        ref={textareaRef}
        value={value}
        onChange={(e) => onChange(e.target.value)}
        placeholder="SELECT {columns} FROM {table} WHERE id IN ({id_list})"
        rows={4}
        className={cn(
          "w-full bg-app-card border border-app-border rounded-xl px-4 py-3 text-sm text-app-text font-mono focus:ring-2 focus:ring-app-accent/50 focus:border-app-accent outline-none transition-all resize-none",
          isOver && "ring-2 ring-app-accent/30 border-app-accent/40",
        )}
      />
    </div>
  );
}


export function QueryTemplateFormModal({
  isOpen,
  onClose,
  onSave,
  template,
  profileId,
  availableTables,
}: QueryTemplateFormModalProps) {
  const [name, setName] = useState("");
  const [description, setDescription] = useState("");
  const [intentKeywords, setIntentKeywords] = useState<string[]>([]);
  const [keywordInput, setKeywordInput] = useState("");
  const [exampleQuestion, setExampleQuestion] = useState("");
  const [queryPattern, setQueryPattern] = useState("");
  const queryPatternRef = useRef<HTMLTextAreaElement | null>(null);
  const [patternType, setPatternType] =
    useState<QueryPatternType>("select_where_eq");
  const [tablesUsed, setTablesUsed] = useState<string[]>([]);
  const [priority, setPriority] = useState(10);
  const [isPatternAgnostic, setIsPatternAgnostic] = useState(false);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const [hintOrder, setHintOrder] = useState<string[]>(() =>
    PLACEHOLDER_HINTS.map((h) => h.placeholder),
  );
  const [hintSearch, setHintSearch] = useState("");
  const [activeDragLabel, setActiveDragLabel] = useState<string | null>(null);

  const hintByPlaceholder = useMemo(() => {
    const map = new Map<string, (typeof PLACEHOLDER_HINTS)[number]>();
    for (const h of PLACEHOLDER_HINTS) map.set(h.placeholder, h);
    return map;
  }, []);

  const orderedHints = useMemo(() => {
    const seen = new Set<string>();
    const fromOrder = hintOrder
      .map((ph) => hintByPlaceholder.get(ph))
      .filter((h): h is (typeof PLACEHOLDER_HINTS)[number] => Boolean(h))
      .filter((h) => {
        if (seen.has(h.placeholder)) return false;
        seen.add(h.placeholder);
        return true;
      });
    const missing = PLACEHOLDER_HINTS.filter((h) => !seen.has(h.placeholder));
    return [...fromOrder, ...missing];
  }, [hintOrder, hintByPlaceholder]);

  const visibleHints = useMemo(() => {
    const q = hintSearch.trim().toLowerCase();
    if (!q) return orderedHints;
    return orderedHints.filter((h) => {
      return (
        h.placeholder.toLowerCase().includes(q) ||
        h.description.toLowerCase().includes(q)
      );
    });
  }, [hintSearch, orderedHints]);

  const visiblePlaceholders = useMemo(
    () => visibleHints.map((h) => h.placeholder),
    [visibleHints],
  );

  const sensors = useSensors(
    useSensor(PointerSensor, { activationConstraint: { distance: 6 } }),
  );

  const collisionDetection: CollisionDetection = (args) => {
    const pointerCollisions = pointerWithin(args);
    if (pointerCollisions.length > 0) return pointerCollisions;
    return closestCenter(args);
  };

  const handleHintDragStart = (event: DragStartEvent) => {
    setActiveDragLabel(String(event.active.id));
  };

  const handleHintDragCancel = () => {
    setActiveDragLabel(null);
  };

  // Reset form when opened or template changes
  useEffect(() => {
    if (isOpen) {
      if (template) {
        setName(template.name);
        setDescription(template.description || "");
        setIntentKeywords(template.intent_keywords);
        setExampleQuestion(template.example_question);
        setQueryPattern(template.query_pattern);
        setPatternType(template.pattern_type);
        setTablesUsed(template.tables_used);
        setPriority(template.priority);
        setIsPatternAgnostic(template.is_pattern_agnostic ?? false);
      } else {
        setName("");
        setDescription("");
        setIntentKeywords([]);
        setKeywordInput("");
        setExampleQuestion("");
        setQueryPattern("");
        setPatternType("select_where_eq");
        setTablesUsed([]);
        setPriority(10);
        setIsPatternAgnostic(false);
      }
      setError(null);
    }
  }, [isOpen, template]);

  // Keep hint order in sync if hints list changes.
  useEffect(() => {
    setHintOrder((prev) => {
      const base = PLACEHOLDER_HINTS.map((h) => h.placeholder);
      const next = prev.filter((ph) => base.includes(ph));
      for (const ph of base) {
        if (!next.includes(ph)) next.push(ph);
      }
      return next;
    });
  }, []);

  const insertPlaceholderAtCaret = (placeholder: string) => {
    const el = queryPatternRef.current;
    if (!el) {
      setQueryPattern((prev) => prev + placeholder);
      return;
    }

    const start = el.selectionStart ?? el.value.length;
    const end = el.selectionEnd ?? el.value.length;
    const before = queryPattern.slice(0, start);
    const after = queryPattern.slice(end);
    const next = before + placeholder + after;
    const nextCursor = start + placeholder.length;

    setQueryPattern(next);
    requestAnimationFrame(() => {
      el.focus();
      try {
        el.setSelectionRange(nextCursor, nextCursor);
      } catch {
        // ignore
      }
    });
  };

  const handleHintDragEnd = (event: DragEndEvent) => {
    const { active, over } = event;
    setActiveDragLabel(null);
    if (!over) return;

    if (String(over.id) === "query-pattern-drop") {
      insertPlaceholderAtCaret(String(active.id));
      return;
    }

    if (active.id === over.id) return;

    setHintOrder((prev) => {
      const oldIndex = prev.indexOf(String(active.id));
      const newIndex = prev.indexOf(String(over.id));
      if (oldIndex === -1 || newIndex === -1) return prev;
      return arrayMove(prev, oldIndex, newIndex);
    });
  };

  const handleAddKeyword = () => {
    const trimmed = keywordInput.trim().toLowerCase();
    if (trimmed && !intentKeywords.includes(trimmed)) {
      setIntentKeywords([...intentKeywords, trimmed]);
      setKeywordInput("");
    }
  };

  const handleRemoveKeyword = (keyword: string) => {
    setIntentKeywords(intentKeywords.filter((k) => k !== keyword));
  };

  const handleKeywordKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === "Enter") {
      e.preventDefault();
      handleAddKeyword();
    }
  };

  const handleToggleTable = (tableName: string) => {
    if (tablesUsed.includes(tableName)) {
      setTablesUsed(tablesUsed.filter((t) => t !== tableName));
    } else {
      setTablesUsed([...tablesUsed, tableName]);
    }
  };

  const validate = (): string | null => {
    if (!name.trim()) return "Template name is required";
    if (intentKeywords.length === 0)
      return "At least one intent keyword is required";
    if (!exampleQuestion.trim()) return "Example question is required";
    if (!queryPattern.trim()) return "Query pattern is required";
    // Pattern-agnostic templates don't require specific tables
    if (!isPatternAgnostic && tablesUsed.length === 0) {
      return "At least one table must be selected (or enable Pattern-Agnostic mode)";
    }
    return null;
  };

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    add_log("QueryTemplateForm", "handleSubmit: Starting validation");

    const validationError = validate();
    if (validationError) {
      add_log("QueryTemplateForm", "handleSubmit: Validation failed", {
        error: validationError,
      });
      setError(validationError);
      return;
    }

    add_log("QueryTemplateForm", "handleSubmit: Validation passed");

    const templateInput = {
      allowlist_profile_id: profileId,
      name: name.trim(),
      description: description.trim() || undefined,
      intent_keywords: intentKeywords,
      example_question: exampleQuestion.trim(),
      query_pattern: queryPattern.trim(),
      pattern_type: patternType,
      tables_used: tablesUsed,
      priority,
      is_pattern_agnostic: isPatternAgnostic,
    } as QueryTemplateInput;
    add_log("QueryTemplateForm", "handleSubmit: Calling onSave", {
      input: templateInput,
    });

    setSaving(true);
    setError(null);

    try {
      await onSave(templateInput);
      add_log(
        "QueryTemplateForm",
        "handleSubmit: onSave completed successfully",
      );
      onClose();
    } catch (err) {
      console.error("Failed to save template:", err);
      add_log("QueryTemplateForm", "handleSubmit: ERROR in onSave", {
        error: err,
      });
      setError(err instanceof Error ? err.message : "Failed to save template");
    } finally {
      setSaving(false);
    }
  };

  if (!isOpen) return null;

  return (
    <AnimatePresence>
      <motion.div
        initial={{ opacity: 0 }}
        animate={{ opacity: 1 }}
        exit={{ opacity: 0 }}
        className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm p-4"
        onClick={(e) => e.target === e.currentTarget && onClose()}>
        <motion.div
          initial={{ opacity: 0, scale: 0.95, y: 20 }}
          animate={{ opacity: 1, scale: 1, y: 0 }}
          exit={{ opacity: 0, scale: 0.95, y: 20 }}
          className="bg-app-panel border border-app-border rounded-2xl shadow-2xl w-full max-w-3xl max-h-[90vh] overflow-hidden flex flex-col">
          {/* Header */}
          <div className="px-6 py-4 border-b border-app-border flex items-center justify-between bg-app-card/30">
            <div className="flex items-center gap-3">
              <div className="w-10 h-10 rounded-xl bg-app-accent/10 flex items-center justify-center">
                <Code className="w-5 h-5 text-app-accent" />
              </div>
              <div>
                <h2 className="text-lg font-bold text-app-text">
                  {template ? "Edit Query Template" : "Create Query Template"}
                </h2>
                <p className="text-xs text-app-subtext">
                  Define SQL pattern for LLM to use as reference
                </p>
              </div>
            </div>
            <button
              onClick={onClose}
              className="p-2 rounded-lg hover:bg-app-border/40 transition-colors text-app-subtext hover:text-app-text">
              <X className="w-5 h-5" />
            </button>
          </div>

          {/* Form Content */}
          <form
            onSubmit={handleSubmit}
            className="flex-1 overflow-y-auto p-6 space-y-6 custom-scrollbar">
            {/* Error Banner */}
            {error && (
              <div className="p-4 bg-destructive/10 border border-destructive/20 rounded-xl flex items-center gap-3 text-destructive">
                <AlertCircle className="w-5 h-5 shrink-0" />
                <p className="text-sm">{error}</p>
              </div>
            )}

            {/* Basic Info */}
            <div className="grid grid-cols-2 gap-4">
              <div className="col-span-2">
                <label className="block text-xs font-bold uppercase tracking-wider text-app-subtext mb-2">
                  Template Name *
                </label>
                <input
                  type="text"
                  value={name}
                  onChange={(e) => setName(e.target.value)}
                  placeholder="e.g., Find users by IDs"
                  className="w-full bg-app-card border border-app-border rounded-xl px-4 py-3 text-sm text-app-text focus:ring-2 focus:ring-app-accent/50 focus:border-app-accent outline-none transition-all"
                />
              </div>

              <div className="col-span-2">
                <label className="block text-xs font-bold uppercase tracking-wider text-app-subtext mb-2">
                  Description
                </label>
                <input
                  type="text"
                  value={description}
                  onChange={(e) => setDescription(e.target.value)}
                  placeholder="When should LLM use this template?"
                  className="w-full bg-app-card border border-app-border rounded-xl px-4 py-3 text-sm text-app-text focus:ring-2 focus:ring-app-accent/50 focus:border-app-accent outline-none transition-all"
                />
              </div>
            </div>

            {/* Intent Keywords */}
            <div>
              <label className="block text-xs font-bold uppercase tracking-wider text-app-subtext mb-2">
                <Tag className="w-3 h-3 inline mr-1" />
                Intent Keywords *
              </label>
              <p className="text-[11px] text-app-subtext mb-2">
                Keywords that trigger this template (e.g., "find user", "get",
                "lookup")
              </p>
              <div className="flex gap-2 mb-2">
                <input
                  type="text"
                  value={keywordInput}
                  onChange={(e) => setKeywordInput(e.target.value)}
                  onKeyDown={handleKeywordKeyDown}
                  placeholder="Type keyword and press Enter"
                  className="flex-1 bg-app-card border border-app-border rounded-xl px-4 py-2.5 text-sm text-app-text focus:ring-2 focus:ring-app-accent/50 focus:border-app-accent outline-none transition-all"
                />
                <button
                  type="button"
                  onClick={handleAddKeyword}
                  className="px-4 py-2.5 bg-app-border/40 hover:bg-app-border rounded-xl text-sm font-semibold transition-colors">
                  <Plus className="w-4 h-4" />
                </button>
              </div>
              <div className="flex flex-wrap gap-2 min-h-[32px]">
                {intentKeywords.map((keyword) => (
                  <span
                    key={keyword}
                    className="inline-flex items-center gap-1.5 px-3 py-1 bg-app-accent/10 text-app-accent rounded-full text-xs font-semibold">
                    {keyword}
                    <button
                      type="button"
                      onClick={() => handleRemoveKeyword(keyword)}
                      className="hover:text-destructive transition-colors">
                      <X className="w-3 h-3" />
                    </button>
                  </span>
                ))}
                {intentKeywords.length === 0 && (
                  <span className="text-xs text-app-subtext italic">
                    No keywords added yet
                  </span>
                )}
              </div>
            </div>

            {/* Example Question */}
            <div>
              <label className="block text-xs font-bold uppercase tracking-wider text-app-subtext mb-2">
                <FileText className="w-3 h-3 inline mr-1" />
                Example Question *
              </label>
              <input
                type="text"
                value={exampleQuestion}
                onChange={(e) => setExampleQuestion(e.target.value)}
                placeholder="e.g., Find users with IDs 1, 2, and 3"
                className="w-full bg-app-card border border-app-border rounded-xl px-4 py-3 text-sm text-app-text focus:ring-2 focus:ring-app-accent/50 focus:border-app-accent outline-none transition-all"
              />
            </div>

            {/* Pattern Type */}
            <div>
              <label className="block text-xs font-bold uppercase tracking-wider text-app-subtext mb-2">
                <Zap className="w-3 h-3 inline mr-1" />
                Pattern Type *
              </label>
              <div className="grid grid-cols-5 gap-2">
                {PATTERN_TYPES.map((pt) => (
                  <button
                    key={pt.value}
                    type="button"
                    onClick={() => setPatternType(pt.value)}
                    className={cn(
                      "p-3 rounded-xl border text-center transition-all",
                      patternType === pt.value
                        ? "bg-app-accent/10 border-app-accent/30 text-app-accent"
                        : "bg-app-card/50 border-app-border text-app-subtext hover:border-app-subtext/40 hover:text-app-text",
                    )}>
                    <div className="text-xl mb-1">{pt.icon}</div>
                    <div className="text-xs font-bold">{pt.label}</div>
                  </button>
                ))}
              </div>
            </div>

            {/* Query Pattern */}
            <div>
              <label className="block text-xs font-bold uppercase tracking-wider text-app-subtext mb-2">
                <Code className="w-3 h-3 inline mr-1" />
                Query Pattern *
              </label>
              <DndContext
                sensors={sensors}
                collisionDetection={collisionDetection}
                onDragStart={handleHintDragStart}
                onDragCancel={handleHintDragCancel}
                onDragEnd={handleHintDragEnd}>
                <QueryPatternDropZone
                  value={queryPattern}
                  onChange={setQueryPattern}
                  textareaRef={queryPatternRef}
                />

                <DragOverlay>
                  {activeDragLabel ? (
                    <HintChipOverlay label={activeDragLabel} />
                  ) : null}
                </DragOverlay>

                {/* Query Hints (pill labels + drag/drop into Query Pattern) */}
                <div className="mt-3 p-3 bg-app-card/40 rounded-xl border border-app-border/60">
                  <div className="flex items-center justify-between gap-3 mb-2">
                    <p className="text-[10px] font-bold text-app-subtext uppercase tracking-wider">
                      Query Hints
                    </p>
                    <div className="w-[280px] max-w-full">
                      <input
                        value={hintSearch}
                        onChange={(e) => setHintSearch(e.target.value)}
                        placeholder="Search hints..."
                        className="w-full bg-app-card border border-app-border rounded-lg px-3 py-2 text-xs text-app-text font-mono focus:ring-2 focus:ring-app-accent/40 focus:border-app-accent outline-none transition-all"
                        aria-label="Search query hints"
                      />
                    </div>
                  </div>

                  <p className="text-[11px] text-app-subtext mb-2">
                    Drag pills into Query Pattern. Drag handle to reorder.
                  </p>

                  <SortableContext
                    items={visiblePlaceholders}
                    strategy={horizontalListSortingStrategy}>
                    <div className="max-h-40 overflow-y-auto custom-scrollbar pr-1">
                      <div className="flex flex-wrap gap-2">
                        {visibleHints.map((hint) => (
                        <SortableHintChip
                          key={hint.placeholder}
                          id={hint.placeholder}
                          label={hint.placeholder}
                          title={hint.description}
                        />
                      ))}
                      </div>
                    </div>
                  </SortableContext>
                </div>
              </DndContext>

              {/* Examples for ORDER BY and GROUP BY */}
              <div className="mt-3 p-3 bg-app-card/50 rounded-xl border border-app-border/50">
                <p className="text-[10px] font-bold text-app-subtext uppercase tracking-wider mb-2">
                  Examples for ORDER BY and GROUP BY
                </p>
                <div className="space-y-2 text-[10px] font-mono text-app-subtext">
                  <div>
                    <span className="text-app-accent font-semibold">ORDER BY:</span>
                    <pre className="mt-1 bg-app-bg/30 p-2 rounded text-[9px] overflow-x-auto">
                      SELECT {"{columns}"} FROM {"{table}"} WHERE role = '{'{'}search_term{'}'}' ORDER BY {"{order_by_column}"} {"{sort_direction}"}
                    </pre>
                    <p className="mt-1 italic text-app-subtext/70">
                      Example: "tampilkan user role admin, urutkan by name descending"
                    </p>
                  </div>
                  <div>
                    <span className="text-app-accent font-semibold">GROUP BY:</span>
                    <pre className="mt-1 bg-app-bg/30 p-2 rounded text-[9px] overflow-x-auto">
                      SELECT {"{group_by_column}"}, COUNT(*) as count FROM {"{table}"} GROUP BY {"{group_by_column}"}
                    </pre>
                    <p className="mt-1 italic text-app-subtext/70">
                      Example: "tampilkan jumlah user per role"
                    </p>
                  </div>
                </div>
              </div>
            </div>

            {/* Tables Used */}
            <div>
              <label className="block text-xs font-bold uppercase tracking-wider text-app-subtext mb-2">
                Tables Used *
              </label>
              <p className="text-[11px] text-app-subtext mb-2">
                Select which tables this template can query
              </p>
              <div className="flex flex-wrap gap-2 p-4 bg-app-card/30 rounded-xl border border-app-border min-h-[60px]">
                {availableTables.length === 0 ? (
                  <span className="text-xs text-app-subtext italic">
                    No tables available
                  </span>
                ) : (
                  availableTables.map((table) => (
                    <button
                      key={table}
                      type="button"
                      onClick={() => handleToggleTable(table)}
                      className={cn(
                        "px-3 py-1.5 rounded-lg text-xs font-mono font-semibold transition-all",
                        tablesUsed.includes(table)
                          ? "bg-app-success/20 text-app-success border border-app-success/30"
                          : "bg-app-border/40 text-app-subtext hover:text-app-text border border-transparent",
                      )}>
                      {table}
                    </button>
                  ))
                )}
              </div>
            </div>

            {/* Priority */}
            <div>
              <label className="block text-xs font-bold uppercase tracking-wider text-app-subtext mb-2">
                <Hash className="w-3 h-3 inline mr-1" />
                Priority
              </label>
              <p className="text-[11px] text-app-subtext mb-2">
                Higher priority templates are preferred when multiple match
              </p>
              <div className="flex items-center gap-4">
                <input
                  type="range"
                  min={1}
                  max={100}
                  value={priority}
                  onChange={(e) => setPriority(parseInt(e.target.value))}
                  className="flex-1 accent-app-accent"
                />
                <span className="w-12 text-center text-sm font-bold text-app-text bg-app-card/50 rounded-lg py-1">
                  {priority}
                </span>
              </div>
            </div>

            {/* Pattern-Agnostic Mode */}
            <div className="p-4 bg-linear-to-r from-app-accent/5 to-purple-500/5 border border-app-accent/20 rounded-xl">
              <label className="flex items-start gap-3 cursor-pointer">
                <input
                  type="checkbox"
                  checked={isPatternAgnostic}
                  onChange={(e) => setIsPatternAgnostic(e.target.checked)}
                  className="mt-1 w-5 h-5 rounded border-app-border bg-app-card text-app-accent focus:ring-app-accent focus:ring-offset-0"
                />
                <div className="flex-1">
                  <div className="flex items-center gap-2">
                    <span className="text-sm font-bold text-app-text">
                      Pattern-Agnostic Mode
                    </span>
                  </div>
                  <p className="text-xs text-app-subtext mt-1">
                    Template works as an abstract SQL pattern across any table.
                    LLM adapts column/table names from the query instead of
                    requiring exact table matches.
                  </p>
                  <div className="mt-2 p-2 bg-app-card/50 rounded-lg">
                    <p className="text-[10px] text-app-subtext">
                      <span className="font-semibold text-app-accent">
                        Example:
                      </span>
                      Pattern "SELECT * FROM users WHERE id =
                      '&lbrace;id&rbrace;'" can match queries like "Find
                      merchant with ID M_B8RjeABb" (LLM maps
                      users→ms_loan_merchant, id→merchant_id)
                    </p>
                  </div>
                </div>
              </label>
            </div>
          </form>

          {/* Footer */}
          <div className="px-6 py-4 border-t border-app-border bg-app-card/30 flex justify-end gap-3">
            <button
              type="button"
              onClick={onClose}
              className="px-5 py-2.5 text-sm font-semibold text-app-subtext hover:text-app-text transition-colors">
              Cancel
            </button>
            <button
              onClick={handleSubmit}
              disabled={saving}
              className="px-6 py-2.5 bg-app-accent text-white rounded-xl text-sm font-bold shadow-lg shadow-app-accent/20 hover:bg-app-accent/90 transition-all hover:scale-[1.02] active:scale-[0.98] disabled:opacity-50 disabled:cursor-not-allowed flex items-center gap-2">
              <Save className="w-4 h-4" />
              {saving
                ? "Saving..."
                : template
                  ? "Update Template"
                  : "Create Template"}
            </button>
          </div>
        </motion.div>
      </motion.div>
    </AnimatePresence>
  );
}
