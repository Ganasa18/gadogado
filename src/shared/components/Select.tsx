import { useState, useRef, useEffect, type ChangeEvent } from "react";
import { ChevronDown, Search, Check, X } from "lucide-react";

interface Option<T extends string = string> {
  value: T;
  label: string;
}

interface SelectProps<T extends string = string> {
  options: Option<T>[];
  value: T | T[];
  onChange: (value: T | T[]) => void;
  placeholder?: string;
  className?: string;
  searchable?: boolean;
  /** Enable multiple selection mode */
  multiple?: boolean;
  /** Maximum number of selections allowed in multiple mode (0 = unlimited) */
  maxSelections?: number;
}

export function Select<T extends string = string>({
  options,
  value,
  onChange,
  placeholder = "Select...",
  className = "",
  searchable = true,
  multiple = false,
  maxSelections = 0,
}: SelectProps<T>) {
  const [isOpen, setIsOpen] = useState(false);
  const [searchTerm, setSearchTerm] = useState("");
  const containerRef = useRef<HTMLDivElement>(null);

  // Handle single value
  const selectedSingle = multiple ? undefined : options.find((opt) => opt.value === (value as T));

  // Handle multiple values
  const selectedMultiple = multiple
    ? options.filter((opt) => (value as T[]).includes(opt.value))
    : [];

  // Check if max selections reached
  const maxReached = multiple && maxSelections > 0 && selectedMultiple.length >= maxSelections;

  useEffect(() => {
    const handleClickOutside = (event: MouseEvent) => {
      if (
        containerRef.current &&
        !containerRef.current.contains(event.target as Node)
      ) {
        setIsOpen(false);
      }
    };
    document.addEventListener("mousedown", handleClickOutside);
    return () => document.removeEventListener("mousedown", handleClickOutside);
  }, []);

  const filteredOptions = options.filter((opt) =>
    opt.label.toLowerCase().includes(searchTerm.toLowerCase())
  );

  const handleSelect = (optValue: T) => {
    if (multiple) {
      const currentValues = value as T[];
      const isSelected = currentValues.includes(optValue);

      if (isSelected) {
        // Remove from selection
        onChange(currentValues.filter((v) => v !== optValue) as T | T[]);
      } else if (!maxReached) {
        // Add to selection
        onChange([...currentValues, optValue] as T | T[]);
      }
    } else {
      onChange(optValue);
      setIsOpen(false);
      setSearchTerm("");
    }
  };

  const handleRemove = (optValue: T, e: React.MouseEvent) => {
    e.stopPropagation();
    onChange((value as T[]).filter((v) => v !== optValue) as T | T[]);
  };

  const handleClearAll = (e: React.MouseEvent) => {
    e.stopPropagation();
    onChange([] as T | T[]);
  };

  return (
    <div className={`relative ${className}`} ref={containerRef}>
      <button
        type="button"
        onClick={() => setIsOpen(!isOpen)}
        className="flex items-center justify-between w-full px-4 py-2 text-sm bg-app-card border border-app-border rounded-lg focus:outline-none focus:ring-2 focus:ring-app-accent/20 transition-all hover:bg-app-card/80 min-h-10">
        <span className="flex-1 text-left truncate">
          {multiple ? (
            selectedMultiple.length > 0 ? (
              <div className="flex items-center gap-1 flex-wrap">
                {selectedMultiple.map((opt) => (
                  <span
                    key={opt.value}
                    className="inline-flex items-center gap-1 px-2 py-0.5 bg-app-accent/10 text-app-accent rounded text-xs font-medium"
                  >
                    {opt.label}
                    <button
                      type="button"
                      onClick={(e) => handleRemove(opt.value, e)}
                      className="hover:text-app-accent/70"
                    >
                      <X className="w-3 h-3" />
                    </button>
                  </span>
                ))}
                {selectedMultiple.length > 1 && (
                  <button
                    type="button"
                    onClick={handleClearAll}
                    className="text-xs text-app-subtext hover:text-app-text underline"
                  >
                    Clear all
                  </button>
                )}
              </div>
            ) : (
              <span className="text-app-subtext">{placeholder}</span>
            )
          ) : selectedSingle ? (
            <span className="text-app-text">{selectedSingle.label}</span>
          ) : (
            <span className="text-app-subtext">{placeholder}</span>
          )}
        </span>
        <ChevronDown
          className={`w-4 h-4 text-app-subtext transition-transform shrink-0 ml-2 ${
            isOpen ? "rotate-180" : ""
          }`}
        />
      </button>

      {isOpen && (
        <div className="absolute z-50 w-full mt-1 bg-app-panel border border-app-border rounded-lg shadow-xl overflow-hidden animate-in fade-in zoom-in-95 duration-200">
          {searchable && (
            <div className="p-2 border-b border-app-border bg-black/10">
              <div className="relative">
                <Search className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-app-subtext" />
                <input
                  type="text"
                  autoFocus
                  className="w-full pl-9 pr-4 py-1.5 text-xs bg-app-bg border border-app-border rounded-md focus:outline-none focus:border-app-accent/50"
                  placeholder={multiple ? "Search templates..." : "Search..."}
                  value={searchTerm}
                  onInput={(e: ChangeEvent<HTMLInputElement>) => setSearchTerm(e.target.value)}
                />
              </div>
            </div>
          )}
          <div className="max-h-60 overflow-y-auto no-scrollbar py-1">
            {filteredOptions.length > 0 ? (
              filteredOptions.map((opt) => {
                const isSelected = multiple
                  ? selectedMultiple.some((s) => s.value === opt.value)
                  : opt.value === (value as T);

                const isDisabled = multiple && maxReached && !isSelected;

                return (
                  <button
                    key={opt.value}
                    type="button"
                    onClick={() => handleSelect(opt.value)}
                    disabled={isDisabled}
                    className={`w-full px-4 py-2 text-left text-sm transition-colors flex items-center gap-2 ${
                      isDisabled
                        ? "text-app-subtext/30 cursor-not-allowed"
                        : isSelected
                        ? "bg-app-accent/5 text-app-accent font-semibold hover:bg-app-accent/10"
                        : "text-app-text hover:bg-app-accent/5"
                    }`}>
                    {multiple && (
                      <div className={`w-4 h-4 rounded border flex items-center justify-center shrink-0 ${
                        isSelected
                          ? "bg-app-accent border-app-accent"
                          : "border-app-border"
                      }`}>
                        {isSelected && <Check className="w-3 h-3 text-white" />}
                      </div>
                    )}
                    <span className="flex-1 truncate">{opt.label}</span>
                  </button>
                );
              })
            ) : (
              <div className="px-4 py-3 text-xs text-app-subtext text-center italic">
                No results found
              </div>
            )}
          </div>
          {multiple && maxSelections > 0 && (
            <div className="px-3 py-2 bg-app-bg/50 border-t border-app-border text-xs text-app-subtext">
              {selectedMultiple.length} / {maxSelections} selected
            </div>
          )}
        </div>
      )}
    </div>
  );
}
