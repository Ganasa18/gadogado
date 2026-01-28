// =============================================================================
// Mock Server Layout Component
// Manages the 3-column layout and collapsible sidebars
// =============================================================================

import React, { useState } from "react";
import { AnimatePresence, motion } from "framer-motion";
import { ChevronLeft, ChevronRight, Menu, Square } from "lucide-react";

interface MockServerLayoutProps {
  leftSidebar: React.ReactNode;
  rightSidebar: React.ReactNode;
  children: React.ReactNode;
}

export function MockServerLayout({
  leftSidebar,
  rightSidebar,
  children,
}: MockServerLayoutProps) {
  const [showLeft, setShowLeft] = useState(true);
  const [showRight, setShowRight] = useState(true);

  return (
    <div className="flex h-full bg-app-bg text-app-text overflow-hidden font-sans relative">
      {/* Left Sidebar Toggle (when hidden) */}
      {!showLeft && (
        <button
          onClick={() => setShowLeft(true)}
          className="absolute left-4 top-8 z-50 p-2 bg-app-card border border-app-border rounded-lg text-app-subtext hover:text-app-text transition-all shadow-lg"
          title="Show Sidebar">
          <Menu className="w-4 h-4" />
        </button>
      )}

      {/* Left Sidebar */}
      <AnimatePresence mode="wait">
        {showLeft && (
          <motion.div
            initial={{ width: 0, opacity: 0 }}
            animate={{ width: 256, opacity: 1 }}
            exit={{ width: 0, opacity: 0 }}
            transition={{ type: "spring", damping: 25, stiffness: 200 }}
            className="flex-shrink-0 flex flex-col h-full bg-app-bg border-r border-app-border relative group">
            {/* Collapse Trigger (Overlay on hover) */}
            <button
              onClick={() => setShowLeft(false)}
              className="absolute -right-3 top-24 z-50 w-6 h-12 bg-app-card border border-app-border rounded-full flex items-center justify-center text-app-subtext hover:text-app-text transition-all opacity-0 group-hover:opacity-100 shadow-md translate-x-1">
              <ChevronLeft className="w-4 h-4" />
            </button>
            <div className="h-full overflow-hidden w-64">{leftSidebar}</div>
          </motion.div>
        )}
      </AnimatePresence>

      {/* Main Content Area */}
      <div className="flex-1 flex flex-col min-w-0 bg-app-bg relative overflow-hidden">
        {children}
      </div>

      {/* Right Sidebar */}
      <AnimatePresence mode="wait">
        {showRight && (
          <motion.div
            initial={{ width: 0, opacity: 0 }}
            animate={{ width: 320, opacity: 1 }}
            exit={{ width: 0, opacity: 0 }}
            transition={{ type: "spring", damping: 25, stiffness: 200 }}
            className="flex-shrink-0 flex flex-col h-full bg-app-bg border-l border-app-border relative group">
            {/* Collapse Trigger */}
            <button
              onClick={() => setShowRight(false)}
              className="absolute -left-3 top-24 z-50 w-6 h-12 bg-app-card border border-app-border rounded-full flex items-center justify-center text-app-subtext hover:text-app-text transition-all opacity-0 group-hover:opacity-100 shadow-md -translate-x-1">
              <ChevronRight className="w-4 h-4" />
            </button>
            <div className="h-full overflow-hidden w-80">{rightSidebar}</div>
          </motion.div>
        )}
      </AnimatePresence>

      {/* Right Sidebar Toggle (when hidden) */}
      {!showRight && (
        <button
          onClick={() => setShowRight(true)}
          className="absolute right-4 top-6 z-50 p-2 bg-app-card border border-app-border rounded-lg text-app-subtext hover:text-app-text transition-all shadow-lg"
          title="Show Quick Switcher">
          <Square className="w-4 h-4 opacity-50 rotate-90" />
        </button>
      )}
    </div>
  );
}
