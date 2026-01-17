import { useState, useRef, useEffect } from "react";
import { ChevronDown, Search } from "lucide-react";

interface Option<T extends string = string> {
  value: T;
  label: string;
}

interface SelectProps<T extends string = string> {
  options: Option<T>[];
  value: T;
  onChange: (value: T) => void;
  placeholder?: string;
  className?: string;
  searchable?: boolean;
}

export function Select<T extends string = string>({
  options,
  value,
  onChange,
  placeholder = "Select...",
  className = "",
  searchable = true,
}: SelectProps<T>) {
  const [isOpen, setIsOpen] = useState(false);
  const [searchTerm, setSearchTerm] = useState("");
  const containerRef = useRef<HTMLDivElement>(null);

  const selectedOption = options.find((opt) => opt.value === value);

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

  return (
    <div className={`relative ${className}`} ref={containerRef}>
      <button
        type="button"
        onClick={() => setIsOpen(!isOpen)}
        className="flex items-center justify-between w-full px-4 py-2 text-sm bg-app-card border border-app-border rounded-lg focus:outline-none focus:ring-2 focus:ring-app-accent/20 transition-all hover:bg-app-card/80">
        <span className={selectedOption ? "text-app-text" : "text-app-subtext"}>
          {selectedOption ? selectedOption.label : placeholder}
        </span>
        <ChevronDown
          className={`w-4 h-4 text-app-subtext transition-transform ${
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
                  placeholder="Search languages..."
                  value={searchTerm}
                  onInput={(e: any) => setSearchTerm(e.target.value)}
                />
              </div>
            </div>
          )}
          <div className="max-h-60 overflow-y-auto no-scrollbar py-1">
            {filteredOptions.length > 0 ? (
              filteredOptions.map((opt) => (
                <button
                  key={opt.value}
                  type="button"
                  onClick={() => {
                    onChange(opt.value);
                    setIsOpen(false);
                    setSearchTerm("");
                  }}
                  className={`w-full px-4 py-2 text-left text-sm hover:bg-app-accent/10 transition-colors ${
                    opt.value === value
                      ? "bg-app-accent/5 text-app-accent font-semibold"
                      : "text-app-text"
                  }`}>
                  {opt.label}
                </button>
              ))
            ) : (
              <div className="px-4 py-3 text-xs text-app-subtext text-center italic">
                No results found
              </div>
            )}
          </div>
        </div>
      )}
    </div>
  );
}
