import { JSX, ComponentChildren } from "react";
import { cn } from "../../utils/cn";

interface TextAreaProps extends JSX.HTMLAttributes<HTMLTextAreaElement> {
  children?: ComponentChildren;
  placeholder?: string;
  readOnly?: boolean;
  value?: string;
}

export function TextArea({ className, children, ...props }: TextAreaProps) {
  return (
    <textarea
      className={cn(
        "flex min-h-[120px] w-full rounded-md border border-input bg-background px-3 py-2 text-sm ring-offset-background placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 disabled:cursor-not-allowed disabled:opacity-50 transition-all focus:border-primary/50 resize-none",
        className
      )}
      {...props}>
      {children}
    </textarea>
  );
}
