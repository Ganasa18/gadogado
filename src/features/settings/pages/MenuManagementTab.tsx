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
import {
  DndContext,
  closestCenter,
  KeyboardSensor,
  PointerSensor,
  useSensor,
  useSensors,
  DragEndEvent,
} from "@dnd-kit/core";
import {
  arrayMove,
  SortableContext,
  sortableKeyboardCoordinates,
  verticalListSortingStrategy,
  useSortable,
} from "@dnd-kit/sortable";
import { CSS } from "@dnd-kit/utilities";

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
  
  const sensors = useSensors(
    useSensor(PointerSensor),
    useSensor(KeyboardSensor, {
      coordinateGetter: sortableKeyboardCoordinates,
    })
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

  const handleDragEnd = (event: DragEndEvent) => {
    const { active, over } = event;
    if (!over || active.id === over.id) return;

    // Check if we are dragging a section or an item
    const activeId = active.id as string;
    const overId = over.id as string;

    // Section reordering
    if (NAV_SECTIONS.some(s => s.id === activeId)) {
      const oldIndex = visibleSections.findIndex((s) => s.id === activeId);
      const newIndex = visibleSections.findIndex((s) => s.id === overId);
      
      if (oldIndex !== -1 && newIndex !== -1) {
        const newSections = arrayMove(visibleSections, oldIndex, newIndex);
        newSections.forEach((sec, index) => {
          setSectionOrder(sec.id, index);
        });
      }
    } else {
      // Item reordering - activeId and overId are paths
      // Find which section this item belongs to
      const section = NAV_SECTIONS.find(s => s.items.some(i => i.path === activeId));
      if (!section) return;

      const items = getSectionItems(section.id);
      const oldIndex = items.findIndex((i) => i.path === activeId);
      const newIndex = items.findIndex((i) => i.path === overId);

      if (oldIndex !== -1 && newIndex !== -1) {
        const newItems = arrayMove(items, oldIndex, newIndex);
        newItems.forEach((item, index) => {
          setNavOrder(item.path, index);
        });
      }
    }
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
        <DndContext
          sensors={sensors}
          collisionDetection={closestCenter}
          onDragEnd={handleDragEnd}
        >
          <div className="grid grid-cols-1 lg:grid-cols-2 gap-x-6 gap-y-6 max-w-6xl mx-auto items-start">
            <SortableContext
              items={visibleSections.map(s => s.id)}
              strategy={verticalListSortingStrategy}
            >
              <div className="space-y-6">
                {leftCol.map((section) => (
                  <SortableSection
                    key={section.id}
                    section={section}
                    items={getSectionItems(section.id)}
                    navSettings={navSettings}
                    toggleVis={toggleNavVisibility}
                  />
                ))}
              </div>
              <div className="space-y-6">
                {rightCol.map((section) => (
                  <SortableSection
                    key={section.id}
                    section={section}
                    items={getSectionItems(section.id)}
                    navSettings={navSettings}
                    toggleVis={toggleNavVisibility}
                  />
                ))}
              </div>
            </SortableContext>
          </div>
        </DndContext>
      </div>
    </div>
  );
}

function SortableSection({
  section,
  items,
  navSettings,
  toggleVis,
}: {
  section: NavSection;
  items: NavItem[];
  navSettings: Record<string, { visible: boolean; order: number }>;
  toggleVis: (path: string) => void;
}) {
  const [collapsed, setCollapsed] = useState(true);
  const {
    attributes,
    listeners,
    setNodeRef,
    transform,
    transition,
    isDragging,
  } = useSortable({ id: section.id });

  const style = {
    transform: CSS.Transform.toString(transform),
    transition,
  };

  return (
    <div
      ref={setNodeRef}
      style={style}
      className={cn(
        "bg-app-card rounded-lg border border-app-border overflow-hidden shadow-sm transition-opacity",
        isDragging && "opacity-40 z-50",
      )}
    >
      <div
        className="flex items-center justify-between px-4 py-3 bg-app-card/50 border-b border-app-border/50 cursor-pointer hover:bg-app-panel/50 transition-colors select-none group"
        onClick={() => setCollapsed(!collapsed)}>
        <div className="flex items-center gap-3">
          <div
            {...attributes}
            {...listeners}
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
              <SortableContext
                items={items.map(i => i.path)}
                strategy={verticalListSortingStrategy}
              >
                {items.map((item) => (
                  <SortableItem
                    key={item.path}
                    item={item}
                    visible={navSettings[item.path]?.visible ?? true}
                    toggleVis={toggleVis}
                  />
                ))}
              </SortableContext>
            </div>
          </motion.div>
        )}
      </AnimatePresence>
    </div>
  );
}

function SortableItem({
  item,
  visible,
  toggleVis,
}: {
  item: NavItem;
  visible: boolean;
  toggleVis: (path: string) => void;
}) {
  const isMandatory = item.path === "/general" || item.path === "/menu-management";
  const {
    attributes,
    listeners,
    setNodeRef,
    transform,
    transition,
    isDragging,
  } = useSortable({ 
    id: item.path,
    disabled: isMandatory
  });

  const style = {
    transform: CSS.Transform.toString(transform),
    transition,
  };

  const Icon = item.icon;

  return (
    <div
      ref={setNodeRef}
      style={style}
      className={cn(
        "flex items-center justify-between px-4 py-3 hover:bg-app-panel/30 transition-colors group/item relative",
        isDragging && "opacity-30 bg-app-panel z-50",
        isMandatory && "opacity-80",
      )}
    >
      <div className="flex items-center gap-3">
        <div
          {...attributes}
          {...listeners}
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
          if (isMandatory) return;
          toggleVis(item.path);
        }}
        disabled={isMandatory}
      />
    </div>
  );
}

