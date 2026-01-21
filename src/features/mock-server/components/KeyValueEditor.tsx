// =============================================================================
// Key-Value Editor Component
// Generic editor for key-value pairs with enabled toggle
// =============================================================================

import { AnimatePresence, motion } from "framer-motion";
import { Trash2 } from "lucide-react";
import { Input } from "../../../shared/components/Input";
import { Button } from "../../../shared/components/Button";
import type { MockKeyValue } from "../types";
import { createKeyValue } from "../types";

export interface KeyValueEditorProps {
  items: MockKeyValue[];
  onChange: (items: MockKeyValue[]) => void;
  placeholder?: { key: string; value: string };
  className?: string;
  emptyMessage?: string;
  showAddButton?: boolean;
  addButtonLabel?: string;
  onAddClick?: () => void;
}

export function KeyValueEditor({
  items,
  onChange,
  placeholder = { key: "Key", value: "Value" },
  className = "",
  emptyMessage = "No items defined",
  showAddButton = true,
  addButtonLabel = "ADD ITEM",
  onAddClick,
}: KeyValueEditorProps) {
  const handleAdd = () => {
    if (onAddClick) {
      onAddClick();
    } else {
      onChange([...items, createKeyValue()]);
    }
  };

  const handleUpdate = (index: number, field: keyof MockKeyValue, value: string | boolean) => {
    const newItems = [...items];
    newItems[index] = { ...newItems[index], [field]: value };
    onChange(newItems);
  };

  const handleRemove = (index: number) => {
    onChange(items.filter((_, i) => i !== index));
  };

  return (
    <div className={`space-y-2 ${className}`}>
      <AnimatePresence>
        {items.map((item, idx) => (
          <motion.div
            key={idx}
            className="flex gap-2 animate-in fade-in slide-in-from-left-2"
          >
            <Input
              placeholder={placeholder.key}
              value={item.key}
              onChange={(e) => handleUpdate(idx, "key", e.target.value)}
              className="bg-app-card border-app-border text-xs"
            />
            <Input
              placeholder={placeholder.value}
              value={item.value}
              onChange={(e) => handleUpdate(idx, "value", e.target.value)}
              className="bg-app-card border-app-border text-xs"
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
