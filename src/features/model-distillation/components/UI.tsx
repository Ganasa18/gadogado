import { motion } from "framer-motion";
import {
  type LucideIcon,
  CheckCircle,
  Activity,
  AlertCircle,
  Clock,
  XCircle,
} from "lucide-react";

interface MetricCardProps {
  icon?: LucideIcon;
  label: string;
  value: string | number;
  unit?: string;
  trend?: number;
  color?: string;
  delay?: number;
}

export function MetricCard({
  icon: Icon,
  label,
  value,
  unit,
  trend,
  color = "text-blue-400",
  delay = 0,
}: MetricCardProps) {
  return (
    <motion.div
      initial={{ opacity: 0, y: 20 }}
      animate={{ opacity: 1, y: 0 }}
      whileHover={{ y: -2, transition: { duration: 0.2 } }}
      transition={{ duration: 0.4, delay }}
      className="bg-app-card rounded-xl border border-app-border p-5 shadow-lg hover:shadow-xl hover:border-app-subtext/30 transition-all duration-300">
      <div className="flex items-start justify-between mb-3">
        {Icon && (
          <div
            className={`p-2.5 rounded-lg ${color.replace("text", "bg").replace("400", "500/10")} ${color}`}>
            <Icon className="w-5 h-5" />
          </div>
        )}
        {trend !== undefined && (
          <div
            className={`text-xs font-semibold flex items-center gap-1 ${trend >= 0 ? "text-green-500" : "text-red-500"}`}>
            {trend >= 0 ? "↑" : "↓"} {Math.abs(trend).toFixed(1)}%
          </div>
        )}
      </div>
      <div className="text-3xl font-bold text-app-text mb-1">
        {typeof value === "number"
          ? value.toFixed(value % 1 === 0 ? 0 : 2)
          : value}
        {unit && (
          <span className="text-base font-normal text-app-subtext ml-1">
            {unit}
          </span>
        )}
      </div>
      <div className="text-xs text-app-subtext font-medium">{label}</div>
    </motion.div>
  );
}

interface ProgressBarProps {
  value: number;
  max: number;
  color?: string;
  label?: string;
  showPercentage?: boolean;
  animated?: boolean;
}

export function ProgressBar({
  value,
  max,
  color = "bg-blue-500",
  label,
  showPercentage = true,
  animated = true,
}: ProgressBarProps) {
  const percentage = Math.min((value / max) * 100, 100);

  return (
    <div className="w-full">
      {label && (
        <div className="flex justify-between items-center mb-2">
          <span className="text-xs font-medium text-app-text">{label}</span>
          {showPercentage && (
            <span className="text-xs font-semibold text-app-subtext">
              {percentage.toFixed(0)}%
            </span>
          )}
        </div>
      )}
      <div className="h-2.5 bg-background rounded-full overflow-hidden shadow-inner">
        <motion.div
          initial={{ width: 0 }}
          animate={{ width: `${percentage}%` }}
          transition={{ duration: animated ? 0.8 : 0, ease: "easeOut" }}
          className={`h-full rounded-full ${color}`}
        />
      </div>
    </div>
  );
}

interface SelectProps {
  label?: string;
  value: string;
  onChange: (value: string) => void;
  options: { value: string; label: string; icon?: LucideIcon }[];
  disabled?: boolean;
  className?: string;
  error?: string;
}

export function Select({
  label,
  value,
  onChange,
  options,
  disabled,
  className = "",
  error,
}: SelectProps) {
  return (
    <div className={className}>
      {label && (
        <label className="text-xs font-semibold text-app-text mb-2 block">
          {label}
        </label>
      )}
      <div className="relative">
        <select
          value={value}
          onChange={(e) => onChange((e.target as HTMLSelectElement).value)}
          disabled={disabled}
          className={`w-full bg-background border border-app-border rounded-lg px-4 py-3 text-sm appearance-none cursor-pointer hover:border-blue-500/50 transition-all duration-200 outline-none focus:border-blue-500 focus:ring-2 focus:ring-blue-500/10 disabled:opacity-50 disabled:cursor-not-allowed ${error ? "border-red-500" : ""}`}>
          {options.map((option) => (
            <option key={option.value} value={option.value}>
              {option.label}
            </option>
          ))}
        </select>
        <svg
          className="w-4 h-4 absolute right-4 top-1/2 -translate-y-1/2 text-app-subtext pointer-events-none transition-transform duration-200"
          fill="none"
          stroke="currentColor"
          viewBox="0 0 24 24">
          <path
            strokeLinecap="round"
            strokeLinejoin="round"
            strokeWidth={2}
            d="M19 9l-7 7-7-7"
          />
        </svg>
      </div>
      {error && (
        <motion.p
          initial={{ opacity: 0, y: -5 }}
          animate={{ opacity: 1, y: 0 }}
          className="text-xs text-red-500 mt-1.5">
          {error}
        </motion.p>
      )}
    </div>
  );
}

interface InputProps {
  label?: string;
  type?: string;
  value: string | number;
  onChange: (value: string | number) => void;
  placeholder?: string;
  disabled?: boolean;
  min?: number;
  max?: number;
  step?: number;
  className?: string;
  error?: string;
}

export function Input({
  label,
  type = "text",
  value,
  onChange,
  placeholder,
  disabled,
  min,
  max,
  step,
  className = "",
  error,
}: InputProps) {
  return (
    <div className={className}>
      {label && (
        <label className="text-xs font-semibold text-app-text mb-2 block">
          {label}
        </label>
      )}
      <input
        type={type}
        value={value}
        onChange={(e) => {
          const target = e.target as HTMLInputElement;
          onChange(
            type === "number" ? parseFloat(target.value) || 0 : target.value,
          );
        }}
        placeholder={placeholder}
        disabled={disabled}
        min={min}
        max={max}
        step={step}
        className={`w-full bg-background border border-app-border rounded-lg px-4 py-3 text-sm outline-none hover:border-blue-500/50 transition-all duration-200 focus:border-blue-500 focus:ring-2 focus:ring-blue-500/10 disabled:opacity-50 disabled:cursor-not-allowed ${error ? "border-red-500" : ""}`}
      />
      {error && (
        <motion.p
          initial={{ opacity: 0, y: -5 }}
          animate={{ opacity: 1, y: 0 }}
          className="text-xs text-red-500 mt-1.5">
          {error}
        </motion.p>
      )}
    </div>
  );
}

interface ButtonProps {
  children: React.ReactNode;
  onClick?: () => void;
  variant?: "primary" | "success" | "danger" | "default" | "ghost";
  disabled?: boolean;
  className?: string;
  icon?: LucideIcon;
  loading?: boolean;
  size?: "sm" | "md" | "lg";
}

export function Button({
  children,
  onClick,
  variant = "default",
  disabled,
  className = "",
  icon: Icon,
  loading,
  size = "md",
}: ButtonProps) {
  const variants = {
    primary:
      "bg-blue-500 text-white hover:bg-blue-600 shadow-lg shadow-blue-500/20",
    success:
      "bg-green-500 text-white hover:bg-green-600 shadow-lg shadow-green-500/20",
    danger:
      "bg-red-500 text-white hover:bg-red-600 shadow-lg shadow-red-500/20",
    default:
      "bg-background border border-app-border text-app-text hover:border-blue-500/50 hover:bg-background",
    ghost: "text-app-text hover:bg-app-card/50",
  };

  const sizes = {
    sm: "px-3 py-1.5 text-xs",
    md: "px-5 py-2.5 text-sm",
    lg: "px-7 py-3 text-base",
  };

  return (
    <motion.button
      whileHover={{ scale: disabled ? 1 : 1.02 }}
      whileTap={{ scale: disabled ? 1 : 0.98 }}
      onClick={onClick}
      disabled={disabled || loading}
      className={`rounded-lg transition-all duration-200 flex items-center justify-center gap-2 disabled:opacity-50 disabled:cursor-not-allowed ${variants[variant]} ${sizes[size]} ${className}`}>
      {loading ? (
        <svg className="animate-spin h-4 w-4" fill="none" viewBox="0 0 24 24">
          <circle
            className="opacity-25"
            cx="12"
            cy="12"
            r="10"
            stroke="currentColor"
            strokeWidth="4"></circle>
          <path
            className="opacity-75"
            fill="currentColor"
            d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
        </svg>
      ) : Icon ? (
        <Icon className="w-4 h-4" />
      ) : null}
      {children}
    </motion.button>
  );
}

interface StatusBadgeProps {
  status: string;
  text?: string;
  className?: string;
}

export function StatusBadge({
  status,
  text,
  className = "",
}: StatusBadgeProps) {
  const statusConfig: Record<
    string,
    { color: string; bg: string; icon: LucideIcon; defaultText: string }
  > = {
    completed: {
      color: "text-green-400",
      bg: "bg-green-500/10",
      icon: CheckCircle,
      defaultText: "Completed",
    },
    running: {
      color: "text-blue-400",
      bg: "bg-blue-500/10",
      icon: Activity,
      defaultText: "Running",
    },
    failed: {
      color: "text-red-400",
      bg: "bg-red-500/10",
      icon: XCircle,
      defaultText: "Failed",
    },
    queued: {
      color: "text-yellow-400",
      bg: "bg-yellow-500/10",
      icon: Clock,
      defaultText: "Queued",
    },
    cancelled: {
      color: "text-gray-400",
      bg: "bg-gray-500/10",
      icon: XCircle,
      defaultText: "Cancelled",
    },
    promoted: {
      color: "text-green-400",
      bg: "bg-green-500/10",
      icon: CheckCircle,
      defaultText: "Active",
    },
    candidate: {
      color: "text-orange-400",
      bg: "bg-orange-500/10",
      icon: AlertCircle,
      defaultText: "Candidate",
    },
  };

  const config = statusConfig[status] || statusConfig.running;
  const Icon = config.icon;

  return (
    <motion.div
      initial={{ scale: 0.9 }}
      animate={{ scale: 1 }}
      className={`inline-flex items-center gap-1.5 px-3 py-1 rounded-full ${config.bg} ${config.color} text-xs font-semibold ${className}`}>
      <Icon className="w-3.5 h-3.5" />
      <span>{text || config.defaultText}</span>
    </motion.div>
  );
}

interface CardProps {
  title: string;
  icon?: LucideIcon;
  iconColor?: string;
  children: React.ReactNode;
  className?: string;
}

export function Card({
  title,
  icon: Icon,
  iconColor = "text-blue-400",
  children,
  className = "",
}: CardProps) {
  return (
    <motion.div
      initial={{ opacity: 0, y: 10 }}
      animate={{ opacity: 1, y: 0 }}
      transition={{ duration: 0.3 }}
      className={`bg-app-card rounded-xl border border-app-border shadow-md hover:shadow-lg transition-shadow duration-300 ${className}`}>
      <div className="p-6">
        {title && (
          <div className="flex items-center gap-3 mb-6 border-b border-app-border/50 pb-4">
            {Icon && (
              <div
                className={`p-2 rounded-lg ${iconColor.replace("text", "bg").replace("400", "500/10")} ${iconColor}`}>
                <Icon className="w-5 h-5" />
              </div>
            )}
            <h3 className="text-lg font-semibold text-app-text tracking-tight">
              {title}
            </h3>
          </div>
        )}
        {children}
      </div>
    </motion.div>
  );
}

interface InfoBoxProps {
  type: "success" | "warning" | "info" | "error";
  children: React.ReactNode;
  icon?: LucideIcon;
  className?: string;
}

export function InfoBox({
  type,
  children,
  icon: Icon,
  className = "",
}: InfoBoxProps) {
  const types = {
    success: {
      bg: "bg-green-500/10",
      border: "border-green-500/20",
      text: "text-green-400",
      defaultIcon: CheckCircle,
    },
    warning: {
      bg: "bg-yellow-500/10",
      border: "border-yellow-500/20",
      text: "text-yellow-400",
      defaultIcon: AlertCircle,
    },
    info: {
      bg: "bg-blue-500/10",
      border: "border-blue-500/20",
      text: "text-blue-400",
      defaultIcon: Activity,
    },
    error: {
      bg: "bg-red-500/10",
      border: "border-red-500/20",
      text: "text-red-400",
      defaultIcon: XCircle,
    },
  };

  const config = types[type];
  const DefaultIcon = config.defaultIcon;
  const ComponentIcon = Icon || DefaultIcon;

  return (
    <motion.div
      initial={{ opacity: 0, x: -10 }}
      animate={{ opacity: 1, x: 0 }}
      className={`${config.bg} border ${config.border} ${config.text} rounded-xl p-4 flex items-start gap-3 ${className}`}>
      <ComponentIcon className="w-5 h-5 mt-0.5 flex-shrink-0" />
      <div className="text-sm leading-relaxed">{children}</div>
    </motion.div>
  );
}
