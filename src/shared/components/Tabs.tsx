import { cn } from "../../utils/cn";
// import * as React from 'react';

interface TabsProps {
  children: any;
}

export function Tabs({ children }: TabsProps) {
  return <div className="space-y-4">{children}</div>;
}

export function TabsList({
  className,
  children,
}: {
  className?: string;
  children: any;
}) {
  return (
    <div
      className={cn(
        "inline-flex items-center justify-center rounded-lg bg-muted p-1 text-muted-foreground",
        className
      )}>
      {children}
    </div>
  );
}

export function TabsTrigger({
  value,
  activeValue,
  onClick,
  children,
  className,
}: {
  value: string;
  activeValue: string;
  onClick: () => void;
  children: any;
  className?: string;
}) {
  const isActive = value === activeValue;
  return (
    <button
      onClick={onClick}
      className={cn(
        "inline-flex items-center justify-center whitespace-nowrap rounded-md px-3 py-1.5 text-sm font-medium ring-offset-background transition-all focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 disabled:pointer-events-none disabled:opacity-50",
        isActive
          ? "bg-background text-foreground shadow-sm"
          : "hover:bg-background/50 hover:text-foreground",
        className
      )}>
      {children}
    </button>
  );
}

export function TabsContent({
  value,
  activeValue,
  children,
  className,
}: {
  value: string;
  activeValue: string;
  children: any;
  className?: string;
}) {
  if (value !== activeValue) return null;
  return (
    <div
      className={cn(
        "ring-offset-background focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 animate-in fade-in-50 duration-200",
        className
      )}>
      {children}
    </div>
  );
}
