import { useState, useEffect, useRef } from "react";
import { useNavigate } from "react-router";
import { createPortal } from "react-dom";
import { motion, AnimatePresence } from "framer-motion";
import { MoreVertical, RefreshCw, Settings, Sliders, Trash2 } from "lucide-react";
import type { DbConnection } from "../../rag/types";

interface ConnectionMenuProps {
  connection: DbConnection;
  onTest: (id: number) => void;
  onDeleteClick: (id: number, name: string) => void;
  onConfigureProfile: (connection: DbConnection) => void;
  onManageTables: (connection: DbConnection) => void;
  onEditConfig?: (connection: DbConnection) => void;
}

interface Position {
  top: number;
  right: number;
}

export function ConnectionMenu({
  connection,
  onTest,
  onDeleteClick,
  onEditConfig,
}: ConnectionMenuProps) {
  const [isOpen, setIsOpen] = useState(false);
  const [position, setPosition] = useState<Position | null>(null);
  const navigate = useNavigate();
  const buttonRef = useRef<HTMLButtonElement>(null);
  const menuRef = useRef<HTMLDivElement>(null);

  // Calculate position when opening
  useEffect(() => {
    if (isOpen && buttonRef.current) {
      const rect = buttonRef.current.getBoundingClientRect();
      setPosition({
        top: rect.bottom + 8,
        right: window.innerWidth - rect.right,
      });
    }
  }, [isOpen]);

  // Close menu when clicking outside
  useEffect(() => {
    const handleClickOutside = (event: MouseEvent) => {
      if (
        menuRef.current &&
        !menuRef.current.contains(event.target as Node) &&
        buttonRef.current &&
        !buttonRef.current.contains(event.target as Node)
      ) {
        setIsOpen(false);
      }
    };

    if (isOpen) {
      document.addEventListener("mousedown", handleClickOutside);
      return () =>
        document.removeEventListener("mousedown", handleClickOutside);
    }
  }, [isOpen]);

  const handleAction = (action: () => void) => {
    action();
    setIsOpen(false);
  };

  const dropdownContent = position ? (
    <>
      {/* Backdrop */}
      <div className="fixed inset-0 z-100" onClick={() => setIsOpen(false)} />
      {/* Dropdown Menu - Fixed positioning to avoid table overflow clipping */}
      <motion.div
        ref={menuRef}
        initial={{ opacity: 0, scale: 0.95, y: -10 }}
        animate={{ opacity: 1, scale: 1, y: 0 }}
        exit={{ opacity: 0, scale: 0.95, y: -10 }}
        transition={{ duration: 0.15 }}
        style={{
          position: "fixed",
          top: `${position.top}px`,
          right: `${position.right}px`,
        }}
        className="w-52 bg-app-panel border border-app-border rounded-lg shadow-xl z-110 overflow-hidden">
        <div className="py-1">
          <button
            onClick={() => handleAction(() => onTest(connection.id))}
            className="w-full px-4 py-2.5 text-left text-sm text-app-text hover:bg-app-card flex items-center gap-3 transition-colors">
            <RefreshCw className="w-4 h-4 text-app-subtext" />
            Test Connection
          </button>
          <div className="h-px bg-app-border/40 my-1" />
          {onEditConfig && (
            <button
              onClick={() => handleAction(() => onEditConfig(connection))}
              className="w-full px-4 py-2.5 text-left text-sm text-app-text hover:bg-app-card flex items-center gap-3 transition-colors">
              <Sliders className="w-4 h-4 text-app-subtext" />
              Edit Config (Limit)
            </button>
          )}
          <button
            onClick={() =>
              handleAction(() => navigate(`/database/setup/${connection.id}`))
            }
            className="w-full px-4 py-2.5 text-left text-sm text-app-text hover:bg-app-card flex items-center gap-3 transition-colors">
            <Settings className="w-4 h-4 text-app-subtext" />
            Setup Tables & Profile
          </button>
          <div className="h-px bg-app-border/40 my-1" />
          <button
            onClick={() =>
              handleAction(() => onDeleteClick(connection.id, connection.name))
            }
            className="w-full px-4 py-2.5 text-left text-sm text-destructive hover:bg-destructive/10 flex items-center gap-3 transition-colors">
            <Trash2 className="w-4 h-4" />
            Delete Connection
          </button>
        </div>
      </motion.div>
    </>
  ) : null;

  return (
    <div className="relative">
      <button
        ref={buttonRef}
        onClick={() => setIsOpen(!isOpen)}
        className="p-2 text-app-subtext hover:text-app-text hover:bg-app-card rounded-lg transition-all"
        title="More options">
        <MoreVertical className="w-4 h-4" />
      </button>

      {isOpen &&
        dropdownContent &&
        createPortal(
          <AnimatePresence>{dropdownContent}</AnimatePresence>,
          document.body,
        )}
    </div>
  );
}
