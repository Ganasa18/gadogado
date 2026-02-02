import { ChevronLeft, ChevronRight } from "lucide-react";
import { cn } from "../../../utils/cn";

interface PaginationProps {
  currentPage: number;
  totalItems: number;
  itemsPerPage: number;
  onPageChange: (page: number) => void;
  className?: string;
}

export function Pagination({
  currentPage,
  totalItems,
  itemsPerPage,
  onPageChange,
  className,
}: PaginationProps) {
  const totalPages = Math.ceil(totalItems / itemsPerPage);

  if (totalPages <= 1) return null;

  return (
    <div className={cn("flex items-center justify-between px-2 py-3", className)}>
      <div className="text-[10px] text-app-subtext font-medium uppercase tracking-wider">
        Showing {Math.min((currentPage - 1) * itemsPerPage + 1, totalItems)} - {Math.min(currentPage * itemsPerPage, totalItems)} of {totalItems}
      </div>
      <div className="flex items-center gap-1">
        <button
          onClick={() => onPageChange(currentPage - 1)}
          disabled={currentPage === 1}
          className="p-1.5 rounded-lg border border-app-border bg-app-card/40 text-app-subtext hover:text-app-text hover:bg-app-border/40 disabled:opacity-30 disabled:cursor-not-allowed transition-all"
        >
          <ChevronLeft className="w-3.5 h-3.5" />
        </button>
        <div className="flex items-center gap-1 mx-1">
          {Array.from({ length: totalPages }, (_, i) => i + 1).map((page) => {
            // Simple logic to show only few pages if there are many
            if (
              totalPages > 5 &&
              page !== 1 &&
              page !== totalPages &&
              Math.abs(page - currentPage) > 1
            ) {
              if (page === 2 || page === totalPages - 1) {
                return <span key={page} className="text-app-subtext px-1">...</span>;
              }
              return null;
            }

            return (
              <button
                key={page}
                onClick={() => onPageChange(page)}
                className={cn(
                  "w-7 h-7 flex items-center justify-center rounded-lg text-xs font-bold transition-all",
                  currentPage === page
                    ? "bg-app-accent text-white shadow-sm shadow-app-accent/20"
                    : "text-app-subtext hover:text-app-text hover:bg-app-card"
                )}
              >
                {page}
              </button>
            );
          })}
        </div>
        <button
          onClick={() => onPageChange(currentPage + 1)}
          disabled={currentPage === totalPages}
          className="p-1.5 rounded-lg border border-app-border bg-app-card/40 text-app-subtext hover:text-app-text hover:bg-app-border/40 disabled:opacity-30 disabled:cursor-not-allowed transition-all"
        >
          <ChevronRight className="w-3.5 h-3.5" />
        </button>
      </div>
    </div>
  );
}
