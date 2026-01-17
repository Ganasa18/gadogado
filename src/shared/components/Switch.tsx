import type { InputHTMLAttributes } from "react";
import { cn } from "../../utils/cn";

interface SwitchProps
  extends Omit<InputHTMLAttributes<HTMLInputElement>, "onChange" | "checked"> {
  checked: boolean;
  onCheckedChange: (checked: boolean) => void;
  disabled?: boolean;
}

export function Switch({
  checked,
  onCheckedChange,
  className,
  disabled,
  ...props
}: SwitchProps) {
  return (
    <label
      className={cn(
        "relative inline-flex items-center cursor-pointer",
        className
      )}>
      <input
        type="checkbox"
        className="sr-only peer"
        checked={checked}
        onChange={(e) => onCheckedChange(e.currentTarget.checked)}
        disabled={disabled}
        {...props}
      />
      <div
        className={cn(
          "w-11 h-6 bg-muted rounded-full peer peer-focus:ring-2 peer-focus:ring-ring dark:bg-gray-700 peer-checked:after:translate-x-full rtl:peer-checked:after:-translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:start-[2px] after:bg-white after:border-gray-300 after:border after:rounded-full after:h-5 after:w-5 after:transition-all dark:border-gray-600 peer-checked:bg-primary",
          disabled && "opacity-50 cursor-not-allowed"
        )}></div>
    </label>
  );
}
