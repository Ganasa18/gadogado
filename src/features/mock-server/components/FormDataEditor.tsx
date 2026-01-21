// =============================================================================
// Form Data Editor Component
// Editor for form data items with type selection (text/file)
// =============================================================================

import { AnimatePresence, motion } from "framer-motion";
import { Trash2 } from "lucide-react";
import { Input } from "../../../shared/components/Input";
import { Select } from "../../../shared/components/Select";
import { Button } from "../../../shared/components/Button";
import type { FormDataItem } from "../types";

export interface FormDataEditorProps {
  items: FormDataItem[];
  onChange: (items: FormDataItem[]) => void;
  placeholder?: { key: string; value: string; fileValue: string };
  className?: string;
  emptyMessage?: string;
  showAddButton?: boolean;
  addButtonLabel?: string;
  onAddClick?: () => void;
}

export function FormDataEditor({
  items,
  onChange,
  placeholder = {
    key: "Key",
    value: "Value",
    fileValue: "File path",
  },
  className = "",
  emptyMessage = "No form fields defined",
  showAddButton = true,
  addButtonLabel = "ADD FIELD",
  onAddClick,
}: FormDataEditorProps) {
  const handleAdd = () => {
    if (onAddClick) {
      onAddClick();
    } else {
      onChange([
        ...items,
        { key: "", value: "", type: "text", enabled: true },
      ]);
    }
  };

  const handleUpdate = (
    index: number,
    field: keyof FormDataItem,
    value: string | boolean
  ) => {
    const newItems = [...items];
    newItems[index] = { ...newItems[index], [field]: value };
    onChange(newItems);
  };

  const handleRemove = (index: number) => {
    onChange(items.filter((_, i) => i !== index));
  };

  const typeOptions = [
    { label: "Text", value: "text" },
    { label: "File", value: "file" },
  ];

  return (
    <div className={`space-y-2 ${className}`}>
      <AnimatePresence>
        {items.map((item, idx) => (
          <motion.div
            key={idx}
            className="flex gap-2 items-center animate-in fade-in slide-in-from-left-2"
          >
            <Input
              placeholder={placeholder.key}
              value={item.key}
              onChange={(e) => handleUpdate(idx, "key", e.target.value)}
              className="bg-app-card border-app-border text-xs flex-1"
            />
            <Select
              options={typeOptions}
              value={item.type}
              onChange={(v) => handleUpdate(idx, "type", v as "text" | "file")}
              className="h-8 bg-app-card border-app-border text-xs w-20"
            />
            <Input
              placeholder={
                item.type === "file" ? placeholder.fileValue : placeholder.value
              }
              value={item.value}
              onChange={(e) => handleUpdate(idx, "value", e.target.value)}
              className="bg-app-card border-app-border text-xs flex-1"
            />
            <Button
              size="icon"
              variant="ghost"
              onClick={() => handleRemove(idx)}
              className="text-app-subtext hover:text-red-400"
            >
              <Trash2 className="w-3.5 h-3.5" />
            </Button>
          </motion.div>
        ))}
      </AnimatePresence>
      {items.length === 0 && (
        <div className="text-center py-4 border border-dashed border-app-border rounded-lg text-xs text-app-subtext">
          {emptyMessage}
        </div>
      )}
      {showAddButton && (
        <Button
          size="sm"
          variant="ghost"
          className="h-6 text-[10px] text-app-accent"
          onClick={handleAdd}
        >
          {addButtonLabel}
        </Button>
      )}
    </div>
  );
}
