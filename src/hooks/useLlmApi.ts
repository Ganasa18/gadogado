import { useCallback, useEffect, useState } from "react";
import { useMutation, useQuery } from "@tanstack/react-query";
import type {
  LlmConfig,
  LlmResponse,
  OpenRouterModel,
  OpenRouterProvider,
} from "../shared/api/apiClient";
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

export function useOpenRouterModelsQuery(
  config: LlmConfig,
  enabled: boolean
): QueryResult<OpenRouterModel[]> & { refetch: () => void } {
  const query = useQuery<OpenRouterModel[], unknown>({
    queryKey: ["openrouter-models", config.base_url, config.api_key],
    queryFn: () => llmApi.getOpenRouterModels(config),
    enabled,
    staleTime: 1000 * 60 * 5,
    refetchOnMount: false,
    refetchOnWindowFocus: false,
    refetchOnReconnect: false,
    retry: 1,
  });

  return {
    data: query.data,
    isLoading: query.isLoading,
    error: query.error,
    refetch: () => {
      query.refetch();
    },
  };
}

export function useOpenRouterProvidersQuery(
  config: LlmConfig,
  enabled: boolean
): QueryResult<OpenRouterProvider[]> & { refetch: () => void } {
  const query = useQuery<OpenRouterProvider[], unknown>({
    queryKey: ["openrouter-providers", config.base_url, config.api_key],
    queryFn: () => llmApi.getOpenRouterProviders(config),
    enabled,
    staleTime: 1000 * 60 * 10,
    refetchOnMount: false,
    refetchOnWindowFocus: false,
    refetchOnReconnect: false,
    retry: 1,
  });

  return {
    data: query.data,
    isLoading: query.isLoading,
    error: query.error,
    refetch: () => {
      query.refetch();
    },
  };
}

export function useLogsQuery(
  enabled: boolean
): QueryResult<LogEntry[]> & { clear: () => Promise<void> } {
  const [data, setData] = useState<LogEntry[]>([]);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<unknown | null>(null);
  // Track the last log's unique signature when cleared
  const [clearedAtSignature, setClearedAtSignature] = useState<string | null>(null);

  // Helper to create a unique signature for a log entry
  const getLogSignature = useCallback((log: LogEntry): string => {
    return `${log.time}|${log.level}|${log.source}|${log.message}`;
  }, []);

  useEffect(() => {
    if (!enabled) return;
    let active = true;

    const fetchLogs = async () => {
      setIsLoading(true);
      try {
        const logs = await llmApi.getLogs();
        if (active) {
          // Only show logs that came after the last clear
          let filteredLogs = logs;

          if (clearedAtSignature !== null) {
            // Find the index of the cleared log, then show everything after it
            const clearedIndex = logs.findIndex(log =>
              getLogSignature(log) === clearedAtSignature
            );

            if (clearedIndex !== -1) {
              // Show logs after the cleared one
              filteredLogs = logs.slice(clearedIndex + 1);
            } else {
              // If cleared log not found (might have been rotated out), show all
              filteredLogs = logs;
            }
          }

          setData(filteredLogs);
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
  }, [enabled, clearedAtSignature, getLogSignature]);

  const clear = useCallback(async () => {
    // Remember the last log's signature when clearing
    try {
      const allLogs = await llmApi.getLogs();
      if (allLogs.length > 0) {
        setClearedAtSignature(getLogSignature(allLogs[allLogs.length - 1]));
      } else {
        setClearedAtSignature("");
      }
      setData([]);
    } catch (err) {
      console.error("Failed to get logs for clear:", err);
      setData([]);
    }
  }, [getLogSignature]);

  return { data, isLoading, error, clear };
}
