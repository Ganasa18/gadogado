// =============================================================================
// Method Badge Component
// Displays HTTP method with appropriate styling
// =============================================================================

import { getMethodBadgeColors } from "../types";

export interface MethodBadgeProps {
  method: string;
}

export function MethodBadge({ method }: MethodBadgeProps) {
  const style = getMethodBadgeColors(method);

  return (
    <span className={`px-2 py-0.5 rounded text-[10px] font-bold border ${style}`}>
      {method}
    </span>
  );
}
