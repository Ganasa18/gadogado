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
} from "lucide-react";
import type { LucideIcon } from "lucide-react";

export type NavSectionId = "Workspace" | "Configuration" | "QA Tools Automation" | "QA Tools";

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
        path: "/typegen",
        label: "Type Generator",
        icon: Code2,
        section: "Workspace",
      },
      {
        path: "/history",
        label: "History",
        icon: History,
        section: "Workspace",
      },

      {
        path: "/token",
        label: "JWT Inspector",
        icon: KeyRound,
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
    id: "Configuration",
    items: [
      {
        path: "/general",
        label: "General",
        icon: Settings,
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
