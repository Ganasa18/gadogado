import { motion, AnimatePresence } from 'framer-motion';
import { ChevronRight, ChevronDown, Braces, Brackets, Type, Hash, ToggleLeft, Ban } from 'lucide-react';

interface JsonNode {
  key: string;
  value: any;
  type: 'object' | 'array' | 'string' | 'number' | 'boolean' | 'null';
  path: string;
  depth: number;
  children?: JsonNode[];
  expanded?: boolean;
}

interface JsonListViewProps {
  json: JsonNode | null;
  onToggle: (path: string) => void;
  onSelect?: (path: string) => void;
  activePath?: string;
  mode?: 'dark' | 'light' | 'system';
}

const getTypeColor = (type: string) => {
  switch (type) {
    case 'object': return 'text-blue-400';
    case 'array': return 'text-green-400';
    case 'string': return 'text-orange-400';
    case 'number': return 'text-purple-400';
    case 'boolean': return 'text-pink-400';
    default: return 'text-app-subtext';
  }
};

const TypeIcon = ({ type, className }: { type: string; className?: string }) => {
  switch (type) {
    case 'object': return <Braces className={className} size={12} />;
    case 'array': return <Brackets className={className} size={12} />;
    case 'string': return <Type className={className} size={12} />;
    case 'number': return <Hash className={className} size={12} />;
    case 'boolean': return <ToggleLeft className={className} size={12} />;
    case 'null': return <Ban className={className} size={12} />;
    default: return null;
  }
};

const JsonListView = ({ json, onToggle, onSelect, activePath }: JsonListViewProps) => {
  if (!json) return null;

  const getValuePreview = (node: JsonNode) => {
    if (node.type === 'object') return `{${node.children?.length ?? 0}}`;
    if (node.type === 'array') return `[${node.children?.length ?? 0}]`;
    if (node.type === 'string') return `"${node.value}"`;
    if (node.type === 'boolean') return node.value.toString();
    if (node.type === 'null') return 'null';
    return node.value?.toString();
  };

  const renderNode = (node: JsonNode) => {
    const isExpanded = node.expanded;
    const hasChildren = node.children && node.children.length > 0;

    return (
      <div key={node.path} className="group/node">
        <motion.div
          initial={{ opacity: 0, x: -5 }}
          animate={{ opacity: 1, x: 0 }}
          className={`flex items-center gap-2 py-1.5 px-2 rounded-lg cursor-pointer transition-all border ${
            activePath === node.path 
              ? 'bg-app-accent/10 border-app-accent/30 text-app-accent' 
              : 'border-transparent hover:bg-app-panel hover:border-app-border/30'
          }`}
          onClick={(e) => {
            e.stopPropagation();
            if (hasChildren && e.shiftKey) {
              onToggle(node.path);
            } else if (onSelect) {
              onSelect(node.path);
            } else if (hasChildren) {
              onToggle(node.path);
            }
          }}
        >
          <div className="flex items-center gap-1.5 min-w-0 flex-1">
            <div className="flex-none w-4 flex justify-center">
              {hasChildren ? (
                isExpanded ? <ChevronDown size={14} className="text-app-subtext" /> : <ChevronRight size={14} className="text-app-subtext/50" />
              ) : (
                <div className="w-1 h-1 rounded-full bg-app-border" />
              )}
            </div>
            
            <TypeIcon type={node.type} className={getTypeColor(node.type)} />
            
            <span className={`text-[13px] font-semibold truncate text-app-text`}>
              {node.key || 'root'}
            </span>

            <span className={`text-[11px] font-mono truncate text-app-subtext opacity-60 group-hover/node:opacity-100 transition-opacity`}>
              {getValuePreview(node)}
            </span>
          </div>
          
          <div className="flex-none opacity-0 group-hover/node:opacity-100 transition-opacity">
             <span className="text-[9px] uppercase tracking-widest font-bold text-app-subtext/50">
               {node.type}
             </span>
          </div>
        </motion.div>

        <AnimatePresence>
          {isExpanded && hasChildren && (
            <motion.div 
              initial={{ height: 0, opacity: 0 }}
              animate={{ height: 'auto', opacity: 1 }}
              exit={{ height: 0, opacity: 0 }}
              className="ml-4 pl-2 border-l border-app-border/30 mt-1 space-y-0.5 overflow-hidden"
            >
              {node.children?.map((child: any) => renderNode(child))}
            </motion.div>
          )}
        </AnimatePresence>
      </div>
    );
  };

  return <div className="space-y-0.5 select-none">{renderNode(json)}</div>;
};

export default JsonListView;