import React, { useState } from "react";
import { motion } from "framer-motion";
import type { FeedbackRating, UserFeedback } from "../types";
import { submitFeedback } from "../api";

interface FeedbackButtonsProps {
  queryId: string;
  queryText: string;
  responseText: string;
  collectionId?: number;
  chunksUsed?: string[];
  onFeedbackSubmitted?: (rating: FeedbackRating) => void;
}

export const FeedbackButtons: React.FC<FeedbackButtonsProps> = ({
  queryId,
  queryText,
  responseText,
  collectionId,
  chunksUsed,
  onFeedbackSubmitted,
}) => {
  const [selectedRating, setSelectedRating] = useState<FeedbackRating | null>(null);
  const [showCommentInput, setShowCommentInput] = useState(false);
  const [comment, setComment] = useState("");
  const [submitting, setSubmitting] = useState(false);
  const [submitted, setSubmitted] = useState(false);

  const handleRatingClick = async (rating: FeedbackRating) => {
    if (submitted) return;

    setSelectedRating(rating);

    // If thumbs down, show comment input for more details
    if (rating === "ThumbsDown") {
      setShowCommentInput(true);
      return;
    }

    // Submit immediately for thumbs up
    await submitFeedbackData(rating);
  };

  const submitFeedbackData = async (rating: FeedbackRating, feedbackComment?: string) => {
    setSubmitting(true);
    try {
      const feedback: UserFeedback = {
        query_id: queryId,
        query_text: queryText,
        response_text: responseText,
        rating,
        comment: feedbackComment || undefined,
        timestamp: Date.now(),
        collection_id: collectionId,
        chunks_used: chunksUsed,
      };

      await submitFeedback(feedback);
      setSubmitted(true);
      onFeedbackSubmitted?.(rating);
    } catch (err) {
      console.error("Failed to submit feedback:", err);
    } finally {
      setSubmitting(false);
    }
  };

  const handleCommentSubmit = async () => {
    if (!selectedRating) return;
    await submitFeedbackData(selectedRating, comment);
    setShowCommentInput(false);
  };

  if (submitted) {
    return (
      <motion.div
        initial={{ opacity: 0, scale: 0.9 }}
        animate={{ opacity: 1, scale: 1 }}
        className="flex items-center gap-2 text-sm"
      >
        <span className="text-green-400">Thank you for your feedback!</span>
        {selectedRating === "ThumbsUp" && <span>👍</span>}
        {selectedRating === "ThumbsDown" && <span>👎</span>}
      </motion.div>
    );
  }

  return (
    <div className="flex flex-col gap-2">
      <div className="flex items-center gap-2">
        <span className="text-xs text-neutral-500">Was this helpful?</span>
        <button
          onClick={() => handleRatingClick("ThumbsUp")}
          disabled={submitting}
          className={`p-1.5 rounded transition-all ${
            selectedRating === "ThumbsUp"
              ? "bg-green-500/20 text-green-400"
              : "text-neutral-400 hover:text-green-400 hover:bg-green-500/10"
          } ${submitting ? "opacity-50 cursor-not-allowed" : ""}`}
          title="Helpful"
        >
          <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
            <path
              strokeLinecap="round"
              strokeLinejoin="round"
              strokeWidth={2}
              d="M14 10h4.764a2 2 0 011.789 2.894l-3.5 7A2 2 0 0115.263 21h-4.017c-.163 0-.326-.02-.485-.06L7 20m7-10V5a2 2 0 00-2-2h-.095c-.5 0-.905.405-.905.905 0 .714-.211 1.412-.608 2.006L7 11v9m7-10h-2M7 20H5a2 2 0 01-2-2v-6a2 2 0 012-2h2.5"
            />
          </svg>
        </button>
        <button
          onClick={() => handleRatingClick("ThumbsDown")}
          disabled={submitting}
          className={`p-1.5 rounded transition-all ${
            selectedRating === "ThumbsDown"
              ? "bg-red-500/20 text-red-400"
              : "text-neutral-400 hover:text-red-400 hover:bg-red-500/10"
          } ${submitting ? "opacity-50 cursor-not-allowed" : ""}`}
          title="Not helpful"
        >
          <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
            <path
              strokeLinecap="round"
              strokeLinejoin="round"
              strokeWidth={2}
              d="M10 14H5.236a2 2 0 01-1.789-2.894l3.5-7A2 2 0 018.736 3h4.018a2 2 0 01.485.06l3.76.94m-7 10v5a2 2 0 002 2h.096c.5 0 .905-.405.905-.904 0-.715.211-1.413.608-2.008L17 13V4m-7 10h2m5-10h2a2 2 0 012 2v6a2 2 0 01-2 2h-2.5"
            />
          </svg>
        </button>
      </div>

      {/* Comment input for negative feedback */}
      {showCommentInput && (
        <motion.div
          initial={{ opacity: 0, height: 0 }}
          animate={{ opacity: 1, height: "auto" }}
          className="space-y-2"
        >
          <textarea
            value={comment}
            onChange={(e) => setComment(e.target.value)}
            placeholder="What could be improved? (optional)"
            className="w-full bg-neutral-800 text-neutral-200 text-sm p-2 rounded border border-neutral-600 resize-none h-16"
          />
          <div className="flex gap-2">
            <button
              onClick={handleCommentSubmit}
              disabled={submitting}
              className="px-3 py-1 bg-blue-600 hover:bg-blue-500 disabled:bg-neutral-700 text-white text-sm rounded transition-colors"
            >
              {submitting ? "Submitting..." : "Submit"}
            </button>
            <button
              onClick={() => {
                setShowCommentInput(false);
                setSelectedRating(null);
              }}
              className="px-3 py-1 bg-neutral-700 hover:bg-neutral-600 text-white text-sm rounded transition-colors"
            >
              Cancel
            </button>
          </div>
        </motion.div>
      )}
    </div>
  );
};

export default FeedbackButtons;
