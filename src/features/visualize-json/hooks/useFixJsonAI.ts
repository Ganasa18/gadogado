import { useState, useCallback } from "react";
import { useLlmConfigBuilder } from "../../../hooks/useLlmConfig";
import { useEnhanceMutation } from "../../../hooks/useLlmApi";

const FIX_JSON_SYSTEM_PROMPT = `You are a JSON repair expert. Your task is to fix malformed or invalid JSON.

Rules:
1. Fix syntax errors (missing quotes, commas, brackets, braces)
2. Correct common mistakes like trailing commas, single quotes instead of double quotes
3. Handle unquoted property names
4. Fix unescaped special characters in strings
5. Return ONLY the corrected JSON, nothing else
6. If the input is completely invalid and cannot be repaired, return a valid empty object: {}
7. Preserve the original structure and data as much as possible
8. Do not add any explanation or comments`;

export interface UseFixJsonAIResult {
  fixJson: (invalidJson: string) => Promise<string | null>;
  isFixing: boolean;
  error: string | null;
  clearError: () => void;
}

export function useFixJsonAI(): UseFixJsonAIResult {
  const buildConfig = useLlmConfigBuilder();
  const enhanceMutation = useEnhanceMutation();
  const [error, setError] = useState<string | null>(null);

  const fixJson = useCallback(
    async (invalidJson: string): Promise<string | null> => {
      setError(null);

      if (!invalidJson.trim()) {
        setError("Input is empty");
        return null;
      }

      try {
        const config = buildConfig({
          maxTokens: 2000,
          temperature: 0.1,
        });

        const result = await enhanceMutation.mutateAsync({
          config,
          content: invalidJson,
          system_prompt: FIX_JSON_SYSTEM_PROMPT,
        });

        const fixedJson = result.result.trim();

        // Validate that the result is valid JSON
        try {
          JSON.parse(fixedJson);
          return fixedJson;
        } catch {
          // Try to extract JSON from the response if AI added extra text
          const jsonMatch = fixedJson.match(/\{[\s\S]*\}|\[[\s\S]*\]/);
          if (jsonMatch) {
            const extracted = jsonMatch[0];
            JSON.parse(extracted); // Validate extracted JSON
            return extracted;
          }
          setError("AI returned invalid JSON. Please try again.");
          return null;
        }
      } catch (err) {
        const message =
          err instanceof Error ? err.message : "Failed to fix JSON with AI";
        setError(message);
        return null;
      }
    },
    [buildConfig, enhanceMutation]
  );

  const clearError = useCallback(() => setError(null), []);

  return {
    fixJson,
    isFixing: enhanceMutation.isPending,
    error,
    clearError,
  };
}
