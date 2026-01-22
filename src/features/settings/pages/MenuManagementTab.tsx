import { useState } from "react";
import { useSettingsStore } from "../../../store/settings";
import { NAV_SECTIONS, NavSection, NavItem } from "../../../app/navigation";
import {
  GripVertical,
  Save,
  RotateCcw,
  ChevronDown,
  LayoutGrid,
} from "lucide-react";
import { Switch } from "../../../shared/components/Switch";
import { cn } from "../../../utils/cn";
import { useToastStore } from "../../../store/toast";
import { motion, AnimatePresence } from "framer-motion";

export default function MenuManagementTab() {
  const {
    navSettings,
    sectionSettings,
    toggleNavVisibility,
    setNavOrder,
    setSectionOrder,
    resetNavSettings,
  } = useSettingsStore();
  const { addToast } = useToastStore();
  const [draggingId, setDraggingId] = useState<string | null>(null);
  const [draggingSectionId, setDraggingSectionId] = useState<string | null>(
    null,
  );

  // Helper to get items for a section, sorted by order
  const getSectionItems = (sectionId: string) => {
    const section = NAV_SECTIONS.find((s) => s.id === sectionId);
    if (!section) return [];
    return [...section.items].sort((a, b) => {
      const orderA = navSettings[a.path]?.order ?? 0;
      const orderB = navSettings[b.path]?.order ?? 0;
      return orderA - orderB;
    });
  };

  const visibleSections = [...NAV_SECTIONS].sort((a, b) => {
    const orderA = sectionSettings[a.id]?.order ?? 0;
    const orderB = sectionSettings[b.id]?.order ?? 0;
    return orderA - orderB;
  });

  const handleItemReorder = (
    srcPath: string,
    destPath: string,
    sectionId: string,
  ) => {
    const items = getSectionItems(sectionId);
    const fromIndex = items.findIndex((i) => i.path === srcPath);
    const toIndex = items.findIndex((i) => i.path === destPath);

    if (fromIndex === -1 || toIndex === -1 || fromIndex === toIndex) return;

    const newItems = [...items];
    const [moved] = newItems.splice(fromIndex, 1);
    newItems.splice(toIndex, 0, moved);

    newItems.forEach((item, index) => {
      setNavOrder(item.path, index);
    });
  };

  const handleSectionReorder = (srcId: string, destId: string) => {
    if (srcId === destId) return;
    const sections = [...visibleSections];
    const fromIndex = sections.findIndex((s) => s.id === srcId);
    const toIndex = sections.findIndex((s) => s.id === destId);

    if (fromIndex === -1 || toIndex === -1) return;

    const [moved] = sections.splice(fromIndex, 1);
    sections.splice(toIndex, 0, moved);

    sections.forEach((sec, index) => {
      setSectionOrder(sec.id, index);
    });
  };

  // Divide into visually balanced columns based on current sort order
  const midPoint = Math.ceil(visibleSections.length / 2);
  const leftCol = visibleSections.slice(0, midPoint);
  const rightCol = visibleSections.slice(midPoint);

  return (
    <div className="flex flex-col h-full bg-app-bg text-app-text overflow-hidden">
      {/* Header */}
      <div className="flex-none px-6 py-5 flex items-center justify-between border-b border-app-border/50 bg-app-bg z-10 shrink-0">
        <div>
          <h1 className="text-xl font-bold tracking-tight flex items-center gap-2">
            <LayoutGrid className="w-5 h-5 text-app-accent" />
            Menu Management
          </h1>
          <p className="text-app-subtext text-xs mt-0.5 ml-7">
            Configure layout and visibility
          </p>
        </div>
        <div className="flex gap-3">
          <button
            onClick={resetNavSettings}
            className="flex items-center gap-2 px-3 py-1.5 rounded-md bg-app-card border border-app-border text-app-text hover:bg-app-panel transition-colors text-[11px] font-medium uppercase tracking-wide">
            <RotateCcw className="w-3.5 h-3.5" />
            Default
          </button>
          <button
            onClick={() => addToast("Layout saved successfully", "success")}
            className="flex items-center gap-2 px-4 py-1.5 rounded-md bg-blue-600 hover:bg-blue-500 text-white shadow-md shadow-blue-500/10 transition-all text-[11px] font-bold uppercase tracking-wide">
            <Save className="w-3.5 h-3.5" />
            Save Changes
          </button>
        </div>
      </div>

      {/* Scrollable Content */}
      <div className="flex-1 overflow-y-auto p-6">
        <div className="grid grid-cols-1 lg:grid-cols-2 gap-x-6 gap-y-6 max-w-6xl mx-auto items-start">
          {/* Since we can't easily drag between CSS grid columns for reordering sections intuitively without a library,
                   we will render two separate columns but handle drag over logic carefully or just assume reordering is mostly vertical.
                   However, the user wants "Workspace"-like reordering.
                   Dragging a section from Right to Left column is complex with pure HTML5 Drag and Drop without unified list.
                   
                   Workaround: Render all in one list for drag logic simplicity, but CSS Grid makes it look like 2 columns.
                   But vertical reordering in CSS grid (masonry) is hard.
                   
                   Compromise: Reorder visual columns.
                */}
          <div className="space-y-6">
            {leftCol.map((section) => (
              <SectionCard
                key={section.id}
                section={section}
                items={getSectionItems(section.id)}
                navSettings={navSettings}
                toggleVis={toggleNavVisibility}
                onReorderItem={(src, dest) =>
                  handleItemReorder(src, dest, section.id)
                }
                onReorderSection={handleSectionReorder}
                draggingId={draggingId}
                setDraggingId={setDraggingId}
                draggingSectionId={draggingSectionId}
                setDraggingSectionId={setDraggingSectionId}
              />
            ))}
          </div>
          <div className="space-y-6">
            {rightCol.map((section) => (
              <SectionCard
                key={section.id}
                section={section}
                items={getSectionItems(section.id)}
                navSettings={navSettings}
                toggleVis={toggleNavVisibility}
                onReorderItem={(src, dest) =>
                  handleItemReorder(src, dest, section.id)
                }
                onReorderSection={handleSectionReorder}
                draggingId={draggingId}
                setDraggingId={setDraggingId}
                draggingSectionId={draggingSectionId}
                setDraggingSectionId={setDraggingSectionId}
              />
            ))}
          </div>
        </div>
      </div>
    </div>
  );
}

function SectionCard({
  section,
  items,
  navSettings,
  toggleVis,
  onReorderItem,
  onReorderSection,
  draggingId,
  setDraggingId,
  draggingSectionId,
  setDraggingSectionId,
}: {
  section: NavSection;
  items: NavItem[];
  navSettings: Record<string, { visible: boolean; order: number }>;
  toggleVis: (path: string) => void;
  onReorderItem: (src: string, dest: string) => void;
  onReorderSection: (src: string, dest: string) => void;
  draggingId: string | null;
  setDraggingId: (id: string | null) => void;
  draggingSectionId: string | null;
  setDraggingSectionId: (id: string | null) => void;
}) {
  // Default collapsed as requested
  const [collapsed, setCollapsed] = useState(true);

  return (
    <div
      className={cn(
        "bg-app-card rounded-lg border border-app-border overflow-hidden shadow-sm transition-opacity",
        draggingSectionId === section.id && "opacity-40",
      )}
      draggable
      onDragStart={(e) => {
        // Only start drag if not dragging an item
        if (draggingId) {
          e.preventDefault();
          return;
        }
        setDraggingSectionId(section.id);
        e.dataTransfer.effectAllowed = "move";
        e.stopPropagation();
      }}
      onDragOver={(e) => {
        e.preventDefault();
        // If dragging a section and hovering over another section
        if (draggingSectionId && draggingSectionId !== section.id) {
          e.dataTransfer.dropEffect = "move";
        }
      }}
      onDrop={(e) => {
        e.preventDefault();
        e.stopPropagation();
        if (draggingSectionId && draggingSectionId !== section.id) {
          onReorderSection(draggingSectionId, section.id);
        }
        setDraggingSectionId(null);
      }}
      onDragEnd={() => setDraggingSectionId(null)}>
      <div
        className="flex items-center justify-between px-4 py-3 bg-app-card/50 border-b border-app-border/50 cursor-pointer hover:bg-app-panel/50 transition-colors select-none group"
        onClick={() => setCollapsed(!collapsed)}>
        <div className="flex items-center gap-3">
          <div
            className={cn(
              "text-app-subtext/30 group-hover:text-app-subtext p-0.5 rounded transition-colors cursor-grab active:cursor-grabbing",
            )}>
            <GripVertical className="w-4 h-4" />
          </div>
          <div className="flex items-center gap-2">
            <motion.div
              animate={{ rotate: collapsed ? -90 : 0 }}
              transition={{ duration: 0.2 }}>
              <ChevronDown className="w-4 h-4 text-app-subtext" />
            </motion.div>
            <span className="text-[10px] uppercase font-bold text-app-subtext tracking-widest">
              {section.id}
            </span>
          </div>
        </div>

        {collapsed ? (
          <span className="text-[10px] bg-app-panel border border-app-border px-1.5 py-0.5 rounded text-app-subtext/70 uppercase tracking-wider">
            Collapsed
          </span>
        ) : (
          <span className="text-[10px] bg-app-panel border border-app-border px-1.5 py-0.5 rounded text-app-subtext font-mono">
            {items.length} ITEMS
          </span>
        )}
      </div>

      <AnimatePresence initial={false}>
        {!collapsed && (
          <motion.div
            initial={{ height: 0, opacity: 0 }}
            animate={{ height: "auto", opacity: 1 }}
            exit={{ height: 0, opacity: 0 }}
            transition={{ duration: 0.3, ease: "easeInOut" }}>
            <div className="divide-y divide-app-border/30">
              {items.map((item) => {
                const visible = navSettings[item.path]?.visible ?? true;
                const Icon = item.icon;
                const isDragging = draggingId === item.path;
                const isMandatory =
                  item.path === "/general" || item.path === "/menu-management";

                return (
                  <div
                    key={item.path}
                    draggable={!isMandatory}
                    onDragStart={(e) => {
                      if (isMandatory) return;
                      setDraggingId(item.path);
                      e.dataTransfer.effectAllowed = "move";
                      e.stopPropagation();
                    }}
                    onDragOver={(e) => {
                      e.preventDefault();
                      e.stopPropagation();
                      if (
                        draggingId &&
                        draggingId !== item.path &&
                        !isMandatory
                      ) {
                        e.dataTransfer.dropEffect = "move";
                      }
                    }}
                    onDrop={(e) => {
                      e.preventDefault();
                      e.stopPropagation();
                      if (
                        draggingId &&
                        draggingId !== item.path &&
                        !isMandatory
                      ) {
                        onReorderItem(draggingId, item.path);
                      }
                      setDraggingId(null);
                    }}
                    onDragEnd={(e) => {
                      e.stopPropagation();
                      setDraggingId(null);
                    }}
                    className={cn(
                      "flex items-center justify-between px-4 py-3 hover:bg-app-panel/30 transition-colors group/item relative",
                      isDragging && "opacity-30 bg-app-panel",
                      isMandatory && "opacity-80",
                    )}>
                    <div className="flex items-center gap-3">
                      <div
                        className={cn(
                          "text-app-subtext/30 group-hover/item:text-app-subtext p-0.5 rounded transition-colors",
                          isMandatory
                            ? "cursor-not-allowed opacity-50"
                            : "cursor-grab active:cursor-grabbing hover:bg-app-panel",
                        )}>
                        <GripVertical className="w-4 h-4" />
                      </div>
                      <div
                        className={cn(
                          "p-1.5 rounded transition-colors",
                          visible
                            ? "bg-app-accent/10 text-app-accent"
                            : "bg-app-panel text-app-subtext",
                        )}>
                        <Icon className="w-4 h-4" />
                      </div>
                      <span
                        className={cn(
                          "text-sm font-medium transition-opacity",
                          !visible && "text-app-subtext/60 line-through",
                        )}>
                        {item.label}
                        {isMandatory && (
                          <span className="ml-2 text-[9px] uppercase tracking-wider text-app-subtext/60 border border-app-border px-1 rounded">
                            Required
                          </span>
                        )}
                      </span>
                    </div>
                    <Switch
                      checked={visible}
                      onCheckedChange={() => {
                        if (isMandatory) {
                          // Prevent toggling visibility for mandatory items
                          return;
                        }
                        toggleVis(item.path);
                      }}
                      disabled={isMandatory}
                    />
                  </div>
                );
              })}
            </div>
          </motion.div>
        )}
      </AnimatePresence>
    </div>
  );
}
