import { ComponentChildren, JSX } from "react";
import { cn } from "../../utils/cn";

interface ButtonProps extends JSX.HTMLAttributes<HTMLButtonElement> {
  variant?: "primary" | "secondary" | "ghost" | "destructive";
  size?: "sm" | "md" | "lg" | "icon";
  children?: ComponentChildren;
  disabled?: boolean;
  type?: "button" | "submit" | "reset";
}

export function Button({
  className,
  variant = "primary",
  size = "md",
  children,
  type = "button",
  ...props
}: ButtonProps) {
  const variants = {
    primary:
      "bg-primary text-primary-foreground hover:bg-primary/90 shadow-sm transition-colors",
    secondary:
      "bg-secondary text-secondary-foreground hover:bg-secondary/80 border border-border",
    ghost: "hover:bg-accent hover:text-accent-foreground",
    destructive:
      "bg-destructive text-destructive-foreground hover:bg-destructive/90",
  };

  const sizes = {
    sm: "h-8 px-3 text-xs",
    md: "h-10 px-4 py-2",
    lg: "h-12 px-8 text-lg",
    icon: "h-10 w-10 p-2",
  };

  return (
    <button
      className={cn(
        "inline-flex items-center justify-center rounded-md font-medium focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring disabled:pointer-events-none disabled:opacity-50 active:scale-[0.98] transition-all",
        variants[variant],
        sizes[size],
        className
      )}
      {...props}>
      {children}
    </button>
  );
}
