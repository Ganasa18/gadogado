// =============================================================================
// Mock Server API Client
// Wraps all Tauri invoke calls for the mock server feature
// =============================================================================

import { invoke } from "@tauri-apps/api/core";
import type {
  MockServerConfig,
  MockServerStatus,
  LogEntry,
} from "../types";

/**
 * Mock Server API Client
 * Provides a clean interface to all Tauri backend commands
 */
export const mockServerApi = {
  /**
   * Get the current mock server configuration
   */
  async getConfig(): Promise<MockServerConfig> {
    return invoke<MockServerConfig>("mock_server_get_config");
  },

  /**
   * Update the mock server configuration
   */
  async updateConfig(config: MockServerConfig): Promise<MockServerConfig> {
    return invoke<MockServerConfig>("mock_server_update_config", { config });
  },

  /**
   * Get the current status of the mock server
   */
  async getStatus(): Promise<MockServerStatus> {
    return invoke<MockServerStatus>("mock_server_status");
  },

  /**
   * Start the mock server
   */
  async start(): Promise<MockServerStatus> {
    return invoke<MockServerStatus>("mock_server_start");
  },

  /**
   * Stop the mock server
   */
  async stop(): Promise<MockServerStatus> {
    return invoke<MockServerStatus>("mock_server_stop");
  },

  /**
   * Get all application logs (to be filtered for MockServer source)
   */
  async getLogs(): Promise<LogEntry[]> {
    return invoke<LogEntry[]>("get_logs");
  },
};

/**
 * Helper function to get only MockServer logs
 */
export async function getMockServerLogs(): Promise<LogEntry[]> {
  const allLogs = await mockServerApi.getLogs();
  return allLogs.filter((entry) => entry.source === "MockServer");
}
