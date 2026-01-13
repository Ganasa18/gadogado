import { useHistoryStore } from "../../store/history";
import { Button } from "../../shared/components/Button";
import {
  Languages,
  Sparkles,
  Trash2,
  Clock,
  ArrowRight,
  Search,
  ArrowLeft,
  Code2,
} from "lucide-react";
import { useEffect, useState } from "react";
import { Input } from "../../shared/components/Input";

export default function HistoryTab() {
  const { items, clearHistory, removeItem } = useHistoryStore();
  const [search, setSearch] = useState("");
  const [selectedId, setSelectedId] = useState<string | null>(null);

  const selectedItem = selectedId
    ? items.find((item) => item.id === selectedId) ?? null
    : null;

  useEffect(() => {
    if (selectedId && !selectedItem) {
      setSelectedId(null);
    }
  }, [selectedId, selectedItem]);

  const filteredItems = items.filter(
    (i) =>
      i.input.toLowerCase().includes(search.toLowerCase()) ||
      i.output.toLowerCase().includes(search.toLowerCase())
  );

  if (selectedItem) {
    return (
      <div className="max-w-6xl mx-auto px-5 py-10 space-y-6 animate-in fade-in slide-in-from-bottom-4 duration-500">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-3">
            <Button
              variant="ghost"
              size="sm"
              onClick={() => setSelectedId(null)}
              className="gap-2">
              <ArrowLeft className="w-4 h-4" />
              Back
            </Button>
            <div className="space-y-1">
              <h3 className="text-2xl font-bold tracking-tight">
                History Detail
              </h3>
              <p className="text-muted-foreground text-sm">
                {selectedItem.type} •{" "}
                {new Date(selectedItem.timestamp).toLocaleDateString()}
              </p>
            </div>
          </div>
          <Button
            variant="ghost"
            size="sm"
            onClick={() => {
              removeItem(selectedItem.id);
              setSelectedId(null);
            }}
            className="text-destructive hover:bg-destructive/10">
            <Trash2 className="w-4 h-4 mr-2" /> Delete
          </Button>
        </div>

        <div className="rounded-xl border border-border bg-card p-4 space-y-4">
          <div className="flex items-center gap-2">
            <div
              className={`p-1.5 rounded-lg ${
                selectedItem.type === "translation"
                  ? "bg-primary/10 text-primary"
                  : selectedItem.type === "enhancement"
                  ? "bg-yellow-500/10 text-yellow-600"
                  : "bg-blue-500/10 text-blue-400"
              }`}>
              {selectedItem.type === "translation" ? (
                <Languages className="w-3 h-3" />
              ) : selectedItem.type === "enhancement" ? (
                <Sparkles className="w-3 h-3" />
              ) : (
                <Code2 className="w-3 h-3" />
              )}
            </div>
            <span className="text-[10px] font-bold uppercase tracking-widest text-muted-foreground">
              {selectedItem.type} Лил{" "}
              {new Date(selectedItem.timestamp).toLocaleDateString()}
            </span>
          </div>

          <div className="grid gap-4 md:grid-cols-3">
            <div className="space-y-1">
              <p className="text-[10px] uppercase tracking-widest text-muted-foreground font-bold">
                Timestamp
              </p>
              <p className="text-sm text-app-text">
                {formatTimestamp(selectedItem.timestamp)}
              </p>
            </div>
            <div className="space-y-1">
              <p className="text-[10px] uppercase tracking-widest text-muted-foreground font-bold">
                Topic
              </p>
              <p className="text-sm text-app-text">
                {selectedItem.topic ?? deriveTopic(selectedItem.input)}
              </p>
            </div>
            <div className="space-y-1">
              <p className="text-[10px] uppercase tracking-widest text-muted-foreground font-bold">
                Type
              </p>
              <p className="text-sm text-app-text capitalize">
                {selectedItem.type}
              </p>
            </div>
          </div>

          <div className="grid gap-4 md:grid-cols-2">
            <div className="space-y-1">
              <p className="text-[10px] uppercase tracking-widest text-muted-foreground font-bold">
                Provider
              </p>
              <p className="text-sm text-app-text">{selectedItem.provider}</p>
            </div>
            <div className="space-y-1">
              <p className="text-[10px] uppercase tracking-widest text-muted-foreground font-bold">
                Model
              </p>
              <p className="text-sm text-app-text">{selectedItem.model}</p>
            </div>
          </div>

          <div className="grid gap-4 md:grid-cols-2">
            <div className="space-y-2">
              <p className="text-[10px] uppercase tracking-widest text-muted-foreground font-bold">
                Input
              </p>
              <div className="rounded-lg border border-border/70 bg-app-panel/40 p-3 text-sm text-app-text whitespace-pre-wrap">
                {selectedItem.input}
              </div>
            </div>
            <div className="space-y-2">
              <p className="text-[10px] uppercase tracking-widest text-muted-foreground font-bold">
                Output
              </p>
              <div className="rounded-lg border border-border/70 bg-app-panel/40 p-3 text-sm text-app-text whitespace-pre-wrap">
                {selectedItem.output}
              </div>
            </div>
          </div>
        </div>
      </div>
    );
  }

  return (
    <div className="max-w-4xl mx-auto p-4 space-y-8 animate-in fade-in slide-in-from-bottom-4 duration-500">
      <div className="flex items-center justify-between">
        <div className="space-y-1">
          <h3 className="text-2xl font-bold tracking-tight">Recent Activity</h3>
          <p className="text-muted-foreground text-sm">
            Your local translation, enhancement, and type generation history.
          </p>
        </div>
        <Button
          variant="ghost"
          size="sm"
          onClick={clearHistory}
          className="text-destructive hover:bg-destructive/10">
          <Trash2 className="w-4 h-4 mr-2" /> Clear All
        </Button>
      </div>

      <div className="relative">
        <Search className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-muted-foreground" />
        <Input
          placeholder="Search history..."
          className="pl-10"
          value={search}
          onInput={(e: any) => setSearch(e.target.value)}
        />
      </div>

      {filteredItems.length === 0 ? (
        <div className="flex flex-col items-center justify-center py-20 bg-muted/20 border-2 border-dashed border-border rounded-xl opacity-50">
          <Clock className="w-12 h-12 mb-4 text-muted-foreground" />
          <p className="text-lg font-medium text-muted-foreground">
            No history found
          </p>
          <p className="text-sm text-muted-foreground">
            Try running a translation or generation first!
          </p>
        </div>
      ) : (
        <div className="space-y-3">
          {filteredItems.map((item) => (
            <div
              key={item.id}
              className="group relative bg-card border border-border p-4 rounded-xl hover:border-primary/30 transition-all hover:shadow-md">
              <div className="flex items-start justify-between mb-2">
                <div className="flex items-center gap-2">
                  <div
                    className={`p-1.5 rounded-lg ${
                      item.type === "translation"
                        ? "bg-primary/10 text-primary"
                        : item.type === "enhancement"
                        ? "bg-yellow-500/10 text-yellow-600"
                        : "bg-blue-500/10 text-blue-400"
                    }`}>
                    {item.type === "translation" ? (
                      <Languages className="w-3 h-3" />
                    ) : item.type === "enhancement" ? (
                      <Sparkles className="w-3 h-3" />
                    ) : (
                      <Code2 className="w-3 h-3" />
                    )}
                  </div>
                  <span className="text-[10px] font-bold uppercase tracking-widest text-muted-foreground">
                    {item.type} �{" "}
                    {new Date(item.timestamp).toLocaleDateString()}
                  </span>
                </div>
                <div className="flex items-center gap-2">
                  <Button
                    variant="ghost"
                    size="sm"
                    className="h-6 px-2 text-[10px] opacity-0 group-hover:opacity-100 transition-opacity"
                    onClick={() => setSelectedId(item.id)}>
                    View
                    <ArrowRight className="w-3 h-3 ml-1" />
                  </Button>
                  <Button
                    variant="ghost"
                    size="icon"
                    className="h-6 w-6 opacity-0 group-hover:opacity-100 transition-opacity text-destructive"
                    onClick={() => removeItem(item.id)}>
                    <Trash2 className="w-3 h-3" />
                  </Button>
                </div>
              </div>

              <div className="grid grid-cols-[1fr,auto,1fr] gap-4 items-center">
                <p className="text-sm font-medium line-clamp-2">{item.input}</p>
                <ArrowRight className="w-3 h-3 text-muted-foreground" />
                <p className="text-sm text-muted-foreground line-clamp-2 italic">
                  {item.output}
                </p>
              </div>

              <div className="mt-3 flex items-center gap-4 text-[10px] text-muted-foreground font-medium">
                <span>Model: {item.model}</span>
                <span>Provider: {item.provider}</span>
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}

function formatTimestamp(timestamp: number) {
  return new Date(timestamp).toLocaleString();
}

function deriveTopic(input: string) {
  const cleaned = input.replace(/\s+/g, " ").trim();
  if (!cleaned) return "General";
  const words = cleaned.split(" ");
  const snippet = words.slice(0, 6).join(" ");
  return words.length > 6 ? `${snippet}...` : snippet;
}
