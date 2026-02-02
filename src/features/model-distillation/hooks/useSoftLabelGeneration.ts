import { useState } from "react";
import { ModelDistillationAPI } from "../api";
import type { SoftLabelGenerationResult } from "../types";

export function useSoftLabelGeneration() {
  const [result, setResult] = useState<SoftLabelGenerationResult | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<Error | null>(null);

  const generate = async (input: {
    prompts: string[];
    teacherModelId: string;
    temperature: number;
    softLabelType: "logits" | "one_hot" | "text_only";
  }) => {
    try {
      setLoading(true);
      setError(null);
      const data = await ModelDistillationAPI.generateSoftLabels(input);
      setResult(data);
      return data;
    } catch (e) {
      const err = e as Error;
      setError(err);
      throw err;
    } finally {
      setLoading(false);
    }
  };

  const reset = () => {
    setResult(null);
    setError(null);
  };

  return { result, loading, error, generate, reset };
}
