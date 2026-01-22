import { useState } from "react";
import { Card } from "../../shared/components/Card";
import { Button } from "../../shared/components/Button";
import { TextArea } from "../../shared/components/TextArea";
import { ScrollArea } from "../../shared/components/ScrollArea";
import { Copy, Trash2, FileText, GitCompare, Minus, Plus, Equal } from "lucide-react";
import { useToastStore } from "../../store/toast";

interface DiffChange {
  type: "same" | "add" | "remove";
  value: string;
}

interface DiffLine {
  type: "same" | "added" | "removed" | "modified";
  originalContent: string;
  modifiedContent: string;
  lineNumber: number;
  changes?: {
    original?: DiffChange[];
    modified?: DiffChange[];
  };
}

// Compute character-level differences
function computeCharDiff(text1: string, text2: string): DiffChange[] {
  const changes: DiffChange[] = [];
  let i = 0;
  let j = 0;

  while (i < text1.length || j < text2.length) {
    if (i < text1.length && j < text2.length && text1[i] === text2[j]) {
      changes.push({ type: "same", value: text1[i] });
      i++;
      j++;
    } else {
      // Find the next matching character
      let matchFound = false;
      let lookAhead = 1;

      // Look ahead in text2 for match
      while (!matchFound && j + lookAhead < text2.length && lookAhead <= 10) {
        if (text1[i] === text2[j + lookAhead]) {
          // Remove chars from text2 until match
          for (let k = 0; k < lookAhead; k++) {
            changes.push({ type: "add", value: text2[j + k] });
          }
          j += lookAhead;
          matchFound = true;
          break;
        }
        lookAhead++;
      }

      // Look ahead in text1 for match
      if (!matchFound) {
        lookAhead = 1;
        while (!matchFound && i + lookAhead < text1.length && lookAhead <= 10) {
          if (text1[i + lookAhead] === text2[j]) {
            // Remove chars from text1 until match
            for (let k = 0; k < lookAhead; k++) {
              changes.push({ type: "remove", value: text1[i + k] });
            }
            i += lookAhead;
            matchFound = true;
            break;
          }
          lookAhead++;
        }
      }

      // No match found, treat as replace
      if (!matchFound) {
        if (i < text1.length) {
          changes.push({ type: "remove", value: text1[i] });
          i++;
        }
        if (j < text2.length) {
          changes.push({ type: "add", value: text2[j] });
          j++;
        }
      }
    }
  }

  return changes;
}

export default function DiffCheckerPage() {
  const [text1, setText1] = useState("");
  const [text2, setText2] = useState("");
  const [diffs, setDiffs] = useState<DiffLine[]>([]);
  const [hasCompared, setHasCompared] = useState(false);
  const { addToast } = useToastStore();

  const computeDiff = () => {
    if (!text1.trim() && !text2.trim()) {
      addToast("Please enter text in at least one field", "error");
      return;
    }

    const lines1 = text1.split("\n");
    const lines2 = text2.split("\n");
    const result: DiffLine[] = [];

    const maxLines = Math.max(lines1.length, lines2.length);

    for (let i = 0; i < maxLines; i++) {
      const line1 = lines1[i] ?? "";
      const line2 = lines2[i] ?? "";

      if (line1 === line2) {
        result.push({
          type: "same",
          originalContent: line1,
          modifiedContent: line2,
          lineNumber: i + 1,
        });
      } else {
        // Compute character-level diff
        const originalChanges = computeCharDiff(line1, line2);
        const modifiedChanges = computeCharDiff(line2, line1);

        result.push({
          type: line1 && line2 ? "modified" : line1 ? "removed" : "added",
          originalContent: line1,
          modifiedContent: line2,
          lineNumber: i + 1,
          changes: {
            original: originalChanges,
            modified: modifiedChanges,
          },
        });
      }
    }

    setDiffs(result);
    setHasCompared(true);

    const diffCount = result.filter((d) => d.type !== "same").length;
    addToast(
      `Found ${diffCount} difference${diffCount !== 1 ? "s" : ""}`,
      "success"
    );
  };

  const clearAll = () => {
    setText1("");
    setText2("");
    setDiffs([]);
    setHasCompared(false);
  };

  const copyResult = () => {
    const resultText = diffs
      .map((d) => {
        let line = "";
        if (d.type === "same") {
          line = `  ${d.originalContent}`;
        } else if (d.type === "removed") {
          line = `- ${d.originalContent}`;
        } else if (d.type === "added") {
          line = `+ ${d.modifiedContent}`;
        } else {
          line = `~ ${d.modifiedContent}`;
        }
        return line;
      })
      .join("\n");

    navigator.clipboard.writeText(resultText);
    addToast("Diff result copied to clipboard", "success");
  };

  const getDiffStats = () => {
    const added = diffs.filter((d) => d.type === "added").length;
    const removed = diffs.filter((d) => d.type === "removed").length;
    const modified = diffs.filter((d) => d.type === "modified").length;
    const unchanged = diffs.filter((d) => d.type === "same").length;
    return { added, removed, modified, unchanged, total: diffs.length };
  };

  const stats = getDiffStats();

  const renderHighlightedText = (
    content: string,
    changes?: DiffChange[]
  ) => {
    if (!changes || changes.length === 0) {
      return <span className="text-app-subtext">{content || <span className="text-app-subtext/30">(empty)</span>}</span>;
    }

    return (
      <span className="font-mono text-sm">
        {changes.map((change, idx) => {
          const isRemove = change.type === "remove";
          const isAdd = change.type === "add";
          const isSame = change.type === "same";

          return (
            <span
              key={idx}
              className={
                isRemove
                  ? "bg-red-500/20 text-red-400 line-through decoration-red-400"
                  : isAdd
                    ? "bg-green-500/20 text-green-400"
                    : isSame
                      ? "text-app-subtext"
                      : ""
              }
            >
              {change.value}
            </span>
          );
        })}
      </span>
    );
  };

  const getLineIcon = (type: DiffLine["type"]) => {
    switch (type) {
      case "added":
        return <Plus className="w-3.5 h-3.5 text-green-500" />;
      case "removed":
        return <Minus className="w-3.5 h-3.5 text-red-500" />;
      case "modified":
        return <GitCompare className="w-3.5 h-3.5 text-yellow-500" />;
      default:
        return <Equal className="w-3.5 h-3.5 text-app-subtext/30" />;
    }
  };

  return (
    <div className="flex flex-col h-full gap-4 p-6">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-3">
          <div className="p-2 rounded-lg bg-app-accent/10">
            <GitCompare className="w-5 h-5 text-app-accent" />
          </div>
          <div>
            <h1 className="text-xl font-semibold text-app-text">
              Diff Checker
            </h1>
            <p className="text-sm text-app-subtext">
              Compare two texts and find differences
            </p>
          </div>
        </div>

        {hasCompared && (
          <div className="flex items-center gap-2">
            <div className="flex items-center gap-4 px-4 py-2 rounded-lg bg-app-card border border-app-border">
              <div className="flex items-center gap-2">
                <Plus className="w-3.5 h-3.5 text-green-500" />
                <span className="text-sm text-app-subtext">
                  Added: <span className="font-medium text-app-text">{stats.added}</span>
                </span>
              </div>
              <div className="w-px h-4 bg-app-border" />
              <div className="flex items-center gap-2">
                <Minus className="w-3.5 h-3.5 text-red-500" />
                <span className="text-sm text-app-subtext">
                  Removed: <span className="font-medium text-app-text">{stats.removed}</span>
                </span>
              </div>
              <div className="w-px h-4 bg-app-border" />
              <div className="flex items-center gap-2">
                <GitCompare className="w-3.5 h-3.5 text-yellow-500" />
                <span className="text-sm text-app-subtext">
                  Modified: <span className="font-medium text-app-text">{stats.modified}</span>
                </span>
              </div>
              <div className="w-px h-4 bg-app-border" />
              <span className="text-sm text-app-subtext">
                Total: <span className="font-medium text-app-text">{stats.total}</span> lines
              </span>
            </div>
          </div>
        )}
      </div>

      {/* Input Section */}
      <div className="grid grid-cols-2 gap-4 flex-1 min-h-0">
        {/* Text 1 */}
        <Card className="flex flex-col p-4 bg-app-panel border-app-border">
          <div className="flex items-center justify-between mb-3">
            <div className="flex items-center gap-2">
              <FileText className="w-4 h-4 text-app-subtext" />
              <label className="text-sm font-medium text-app-text">
                Original Text
              </label>
            </div>
            {text1 && (
              <button
                onClick={() => setText1("")}
                className="flex items-center gap-1 h-7 text-xs text-app-subtext hover:text-app-text transition-colors">
                <Trash2 className="w-3 h-3" />
                Clear
              </button>
            )}
          </div>
          <TextArea
            value={text1}
            onChange={(e) => setText1(e.target.value)}
            placeholder="Enter original text here..."
            className="flex-1 min-h-0 resize-none bg-app-card border-app-border text-app-text placeholder:text-app-subtext/50 focus-visible:ring-app-accent"
          />
          <div className="mt-2 text-xs text-app-subtext text-right">
            {text1.split("\n").length} lines · {text1.length} characters
          </div>
        </Card>

        {/* Text 2 */}
        <Card className="flex flex-col p-4 bg-app-panel border-app-border">
          <div className="flex items-center justify-between mb-3">
            <div className="flex items-center gap-2">
              <FileText className="w-4 h-4 text-app-subtext" />
              <label className="text-sm font-medium text-app-text">
                Modified Text
              </label>
            </div>
            {text2 && (
              <button
                onClick={() => setText2("")}
                className="flex items-center gap-1 h-7 text-xs text-app-subtext hover:text-app-text transition-colors">
                <Trash2 className="w-3 h-3" />
                Clear
              </button>
            )}
          </div>
          <TextArea
            value={text2}
            onChange={(e) => setText2(e.target.value)}
            placeholder="Enter modified text here..."
            className="flex-1 min-h-0 resize-none bg-app-card border-app-border text-app-text placeholder:text-app-subtext/50 focus-visible:ring-app-accent"
          />
          <div className="mt-2 text-xs text-app-subtext text-right">
            {text2.split("\n").length} lines · {text2.length} characters
          </div>
        </Card>
      </div>

      {/* Action Buttons */}
      <div className="flex items-center justify-center gap-3">
        <Button
          onClick={clearAll}
          variant="secondary"
          disabled={!text1 && !text2 && !hasCompared}>
          <Trash2 className="w-4 h-4 mr-2" />
          Clear All
        </Button>
        <Button
          onClick={computeDiff}
          disabled={!text1.trim() && !text2.trim()}
          className="bg-app-accent hover:bg-app-accent/90 text-white min-w-[140px]">
          <GitCompare className="w-4 h-4 mr-2" />
          Compare
        </Button>
      </div>

      {/* Result Section - Side by Side Comparison */}
      {hasCompared && (
        <Card className="flex flex-col p-4 bg-app-panel border-app-border flex-1 min-h-0">
          <div className="flex items-center justify-between mb-3">
            <h3 className="text-sm font-medium text-app-text">
              Comparison Result
            </h3>
            <button
              onClick={copyResult}
              className="flex items-center gap-1 h-7 text-xs text-app-subtext hover:text-app-text transition-colors">
              <Copy className="w-3 h-3" />
              Copy
            </button>
          </div>

          {/* Column Headers */}
          <div className="grid grid-cols-[auto,1fr,auto,1fr,auto] gap-2 px-3 pb-2 border-b border-app-border">
            <div className="w-8 text-xs text-app-subtext font-medium">#</div>
            <div className="text-xs text-app-subtext font-medium">Original</div>
            <div className="w-8 flex justify-center">
              <GitCompare className="w-3.5 h-3.5 text-app-accent" />
            </div>
            <div className="text-xs text-app-subtext font-medium">Modified</div>
            <div className="w-8"></div>
          </div>

          <ScrollArea className="flex-1">
            <div className="space-y-1 pr-4">
              {diffs.length === 0 ? (
                <div className="flex items-center justify-center py-8 text-app-subtext">
                  No differences found
                </div>
              ) : (
                diffs.map((diff, index) => (
                  <div
                    key={index}
                    className={`grid grid-cols-[auto,1fr,auto,1fr,auto] gap-2 items-start py-2 px-3 rounded-lg ${
                      diff.type === "added"
                        ? "bg-green-500/5"
                        : diff.type === "removed"
                          ? "bg-red-500/5"
                          : diff.type === "modified"
                            ? "bg-yellow-500/5"
                            : "hover:bg-app-card/50"
                    }`}>
                    {/* Line Number */}
                    <div className="text-xs text-app-subtext w-8 pt-1 shrink-0">
                      {diff.lineNumber}
                    </div>

                    {/* Original Content */}
                    <div
                      className={`py-1 px-2 rounded ${
                        diff.type === "removed" || diff.type === "modified"
                          ? "bg-red-500/10 border-l-2 border-red-500"
                          : ""
                      }`}>
                      {renderHighlightedText(
                        diff.originalContent,
                        diff.changes?.original
                      )}
                    </div>

                    {/* Status Icon */}
                    <div className="w-8 flex justify-center pt-1">
                      {getLineIcon(diff.type)}
                    </div>

                    {/* Modified Content */}
                    <div
                      className={`py-1 px-2 rounded ${
                        diff.type === "added" || diff.type === "modified"
                          ? "bg-green-500/10 border-l-2 border-green-500"
                          : ""
                      }`}>
                      {renderHighlightedText(
                        diff.modifiedContent,
                        diff.changes?.modified
                      )}
                    </div>

                    {/* Actions */}
                    <div className="w-8"></div>
                  </div>
                ))
              )}
            </div>
          </ScrollArea>
        </Card>
      )}

      {/* Legend */}
      {hasCompared && diffs.length > 0 && (
        <div className="flex items-center justify-center gap-6 text-xs text-app-subtext">
          <div className="flex items-center gap-2">
            <span className="px-2 py-0.5 rounded bg-green-500/10 border-l-2 border-green-500 text-green-400">
              + Added
            </span>
          </div>
          <div className="flex items-center gap-2">
            <span className="px-2 py-0.5 rounded bg-red-500/10 border-l-2 border-red-500 text-red-400 line-through">
              - Removed
            </span>
          </div>
          <div className="flex items-center gap-2">
            <span className="px-2 py-0.5 rounded bg-yellow-500/10 border-l-2 border-yellow-500 text-yellow-400">
              ~ Modified
            </span>
          </div>
          <div className="flex items-center gap-2">
            <span className="px-2 py-0.5 rounded bg-app-card border-l-2 border-app-border">
              Unchanged
            </span>
          </div>
        </div>
      )}
    </div>
  );
}
