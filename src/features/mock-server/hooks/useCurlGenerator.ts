// =============================================================================
// cURL Generator Hook
// Generates cURL commands from route configuration
// =============================================================================

import { useMemo } from "react";
import type { MockRoute } from "../types";
import { generateCurlCommand as generateCurlCommandUtil } from "../types";

export interface UseCurlGeneratorReturn {
  generateCurlCommand: (route: MockRoute, baseUrl: string) => string;
}

/**
 * Hook for generating cURL commands from mock route configuration
 * This is a thin wrapper around the utility function for consistency
 */
export function useCurlGenerator(): UseCurlGeneratorReturn {
  const generateCurlCommand = useMemo(
    () => generateCurlCommandUtil,
    []
  );

  return { generateCurlCommand };
}
