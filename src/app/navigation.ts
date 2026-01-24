import {
  BookOpen,
  Code2,
  ClipboardCheck,
  ClipboardList,
  History,
  Keyboard,
  KeyRound,
  Languages,
  MessageSquare,
  Settings,
  Wand2,
  Database,
  MessageCircle,
  Server,
  BarChart3,
  Cog,
  Play,
  Download,
  LayoutDashboard,
  GitCompare,
  DatabaseIcon,
} from "lucide-react";
import type { LucideIcon } from "lucide-react";

export type NavSectionId =
  | "Workspace"
  | "Configuration"
  | "QA Tools Automation"
  | "QA Tools"
  | "RAG"
  | "Model Destilation"
  | "Dev Tools";

export interface NavItem {
  path: string;
  label: string;
  icon: LucideIcon;
  section: NavSectionId;
  requiresShortcuts?: boolean;
}

export interface NavSection {
  id: NavSectionId;
  items: NavItem[];
}

export const NAV_SECTIONS: NavSection[] = [
  {
    id: "Workspace",
    items: [
      {
        path: "/translate",
        label: "Translation",
        icon: Languages,
        section: "Workspace",
      },
      {
        path: "/enhance",
        label: "Enhance Prompt",
        icon: Wand2,
        section: "Workspace",
      },

      {
        path: "/history",
        label: "History",
        icon: History,
        section: "Workspace",
      },
    ],
  },
  {
    id: "Dev Tools",
    items: [
      {
        path: "/typegen",
        label: "Type Generator",
        icon: Code2,
        section: "Workspace",
      },
      {
        path: "/token",
        label: "JWT Inspector",
        icon: KeyRound,
        section: "Workspace",
      },
      {
        path: "/visualize-json",
        label: "Visualize Tools",
        icon: KeyRound,
        section: "Workspace",
      },
      {
        path: "/diff-checker",
        label: "Diff Checker",
        icon: GitCompare,
        section: "Workspace",
      },
      {
        path: "/mock-server",
        label: "Mock Server",
        icon: Server,
        section: "Workspace",
      },
    ],
  },
  {
    id: "RAG",
    items: [
      {
        path: "/rag",
        label: "RAG Knowledge Base",
        icon: Database,
        section: "Workspace",
      },
      {
        path: "/rag-chat",
        label: "RAG Chat",
        icon: MessageCircle,
        section: "Workspace",
      },
      {
        path: "/rag/analytics",
        label: "RAG Analytics",
        icon: BarChart3,
        section: "Workspace",
      },
    ],
  },
  {
    id: "QA Tools Automation",
    items: [
      {
        path: "/qa",
        label: "QA Session",
        icon: ClipboardCheck,
        section: "Workspace",
      },
      {
        path: "/qa/history",
        label: "QA History",
        icon: ClipboardList,
        section: "Workspace",
      },
    ],
  },
  {
    id: "Model Destilation",
    items: [
      {
        path: "/model-destilation/setup",
        label: "Setup",
        icon: Cog,
        section: "Model Destilation",
      },
      {
        path: "/model-destilation/train",
        label: "Train",
        icon: Play,
        section: "Model Destilation",
      },
      {
        path: "/model-destilation/evaluate",
        label: "Evaluate",
        icon: BarChart3,
        section: "Model Destilation",
      },
      {
        path: "/model-destilation/export",
        label: "Export",
        icon: Download,
        section: "Model Destilation",
      },
    ],
  },
  {
    id: "Configuration",
    items: [
      {
        path: "/general",
        label: "General",
        icon: Settings,
        section: "Configuration",
      },
      {
        path: "/database",
        label: "Database Setup",
        icon: DatabaseIcon,
        section: "Configuration",
      },
      {
        path: "/menu-management",
        label: "Menu Management", // Or Navigation
        icon: LayoutDashboard,
        section: "Configuration",
      },
      {
        path: "/shortcut",
        label: "Shortcut",
        icon: Keyboard,
        section: "Configuration",
        requiresShortcuts: true,
      },
      {
        path: "/feedback",
        label: "Feedback",
        icon: MessageSquare,
        section: "Configuration",
      },
      {
        path: "/tutorial",
        label: "Tutorial",
        icon: BookOpen,
        section: "Configuration",
      },
    ],
  },
];
