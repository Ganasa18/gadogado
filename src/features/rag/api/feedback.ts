import { invoke } from "@tauri-apps/api/core";
import type { FeedbackStats, UserFeedback } from "../types";

export async function submitFeedback(feedback: UserFeedback): Promise<string> {
  return await invoke<string>("rag_submit_feedback", { feedback });
}

export async function getFeedbackStats(): Promise<FeedbackStats> {
  return await invoke<FeedbackStats>("rag_get_feedback_stats");
}

export async function getRecentFeedback(limit?: number): Promise<UserFeedback[]> {
  return await invoke<UserFeedback[]>("rag_get_recent_feedback", { limit });
}

export async function clearFeedback(): Promise<string> {
  return await invoke<string>("rag_clear_feedback");
}
