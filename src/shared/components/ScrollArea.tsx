import type { HTMLAttributes, ReactNode } from "react";
import { cn } from "../../utils/cn";

interface ScrollAreaProps extends HTMLAttributes<HTMLDivElement> {
  children?: ReactNode;
}

export function ScrollArea({ className, children, ...props }: ScrollAreaProps) {
  return (
    <div
      className={cn("overflow-auto", className)}
      {...props}>
      {children}
    </div>
  );
}
