import { memo } from "react";
import { Handle, Position, NodeProps } from "@xyflow/react";
import { Braces, Brackets, Type, Hash, ToggleLeft, Ban, ChevronDown, ChevronRight } from "lucide-react";

const getTypeColor = (type: string) => {
  switch (type) {
    case "object":
      return "text-blue-400";
    case "array":
      return "text-green-400";
    case "string":
      return "text-orange-400";
    case "number":
      return "text-purple-400";
    case "boolean":
      return "text-pink-400";
    case "null":
      return "text-app-subtext";
    default:
      return "text-app-subtext";
  }
};

const getTypeBg = (type: string) => {
  switch (type) {
    case "object":
      return "bg-blue-500/5 border-blue-500/20";
    case "array":
      return "bg-green-500/5 border-green-500/20";
    case "string":
      return "bg-orange-500/5 border-orange-500/20";
    case "number":
      return "bg-purple-500/5 border-purple-500/20";
    case "boolean":
      return "bg-pink-500/5 border-pink-500/20";
    case "null":
      return "bg-app-panel border-app-border";
    default:
      return "bg-app-panel border-app-border";
  }
};

const TypeIcon = ({
  type,
  className,
}: {
  type: string;
  className?: string;
}) => {
  switch (type) {
    case "object":
      return <Braces className={className} size={14} />;
    case "array":
      return <Brackets className={className} size={14} />;
    case "string":
      return <Type className={className} size={14} />;
    case "number":
      return <Hash className={className} size={14} />;
    case "boolean":
      return <ToggleLeft className={className} size={14} />;
    case "null":
      return <Ban className={className} size={14} />;
    default:
      return null;
  }
};

const CustomGraphNode = ({ data }: NodeProps) => {
  const { label, type, value, isRoot, expanded, hasChildren, path } = data as any;
  const onToggle = () => {
    const event = new CustomEvent('toggleNode', { detail: path });
    window.dispatchEvent(event);
  };

  return (
    <div
      className={`shadow-md rounded-lg border  transition-all hover:border-app-accent/40 bg-app-card ${getTypeBg(
        String(type)
      )}`}>
      <Handle
        type="target"
        position={Position.Left}
        className="!bg-app-border !w-1 !h-3 !rounded-sm !border-none"
      />

      <div className="flex items-center justify-between px-3 py-2 border-b border-app-border/40">
        <div className="flex items-center gap-2">
          {hasChildren && (
            <button
              onClick={onToggle}
              className="p-0.5 rounded hover:bg-app-accent/20 hover:text-app-accent transition"
              title={expanded ? "Collapse" : "Expand"}>
              {expanded ? (
                <ChevronDown size={14} />
              ) : (
                <ChevronRight size={14} />
              )}
            </button>
          )}
          <span
            className={`p-1 rounded-md bg-app-bg ${getTypeColor(
              String(type)
            )}`}>
            <TypeIcon type={String(type)} />
          </span>
          <span
            className="font-semibold text-sm text-app-text truncate"
            title={String(label)}>
            {String(label)}
          </span>
        </div>
        {isRoot && (
          <span className="text-[10px] font-bold uppercase tracking-wider text-app-accent bg-app-accent/10 px-1.5 py-0.5 rounded">
            Root
          </span>
        )}
      </div>

      <div className="p-3 bg-app-bg/20">
        <div className="flex justify-between items-center text-xs">
          <span className="text-app-subtext uppercase font-medium tracking-wide text-[10px]">
            {String(type)}
          </span>
          <span
            className="font-mono text-app-text truncate"
            title={String(value)}>
            {String(value)}
          </span>
        </div>
      </div>

      <Handle
        type="source"
        position={Position.Right}
        className="!bg-app-border !w-1 !h-3 !rounded-sm !border-none"
      />
    </div>
  );
};

export default memo(CustomGraphNode);
