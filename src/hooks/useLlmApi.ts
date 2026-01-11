import { useCallback, useEffect, useState } from "react";
import { useMutation, useQuery } from "@tanstack/react-query";
import type { LlmConfig, LlmResponse } from "../shared/api/apiClient";
import {
  llmApi,
  type EnhancePayload,
  type LogEntry,
  type TranslatePayload,
  type TypeGenPayload,
} from "../shared/api/apiClient";

type QueryResult<TData> = {
  data: TData | undefined;
  isLoading: boolean;
  error: unknown | null;
};

export function useTranslateMutation() {
  return useMutation<LlmResponse, unknown, TranslatePayload>({
    mutationFn: (payload) => llmApi.translate(payload),
    retry: 0,
  });
}

export function useEnhanceMutation() {
  return useMutation<LlmResponse, unknown, EnhancePayload>({
    mutationFn: (payload) => llmApi.enhance(payload),
    retry: 0,
  });
}

export function useTypegenMutation() {
  return useMutation<LlmResponse, unknown, TypeGenPayload>({
    mutationFn: (payload) => llmApi.typegen(payload),
    retry: 0,
  });
}

export function useModelsQuery(
  config: LlmConfig,
  enabled: boolean
): QueryResult<string[]> {
  const query = useQuery<string[], unknown>({
    queryKey: ["models", config.provider, config.base_url, config.api_key],
    queryFn: () => llmApi.getModels(config),
    enabled,
    staleTime: 1000 * 60 * 5,
    refetchOnMount: false,
    refetchOnWindowFocus: false,
    refetchOnReconnect: false,
    retry: 1,
  });

  return { data: query.data, isLoading: query.isLoading, error: query.error };
}

export function useLogsQuery(
  enabled: boolean
): QueryResult<LogEntry[]> & { clear: () => void } {
  const [data, setData] = useState<LogEntry[]>([]);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<unknown | null>(null);

  useEffect(() => {
    if (!enabled) return;
    let active = true;

    const fetchLogs = async () => {
      setIsLoading(true);
      try {
        const logs = await llmApi.getLogs();
        if (active) {
          setData(logs);
        }
      } catch (err) {
        if (active) {
          setError(err);
        }
      } finally {
        if (active) {
          setIsLoading(false);
        }
      }
    };

    fetchLogs();
    const intervalId = window.setInterval(fetchLogs, 2000);

    return () => {
      active = false;
      window.clearInterval(intervalId);
    };
  }, [enabled]);

  const clear = useCallback(() => setData([]), []);

  return { data, isLoading, error, clear };
}
