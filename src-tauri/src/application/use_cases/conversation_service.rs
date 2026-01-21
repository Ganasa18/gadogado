use crate::domain::error::{AppError, Result};
use crate::infrastructure::db::rag::repository::RagRepository;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Maximum number of messages to include in context
const MAX_CONTEXT_MESSAGES: usize = 10;
/// Maximum total characters for context
const MAX_CONTEXT_CHARS: usize = 4000;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conversation {
    pub id: i64,
    pub collection_id: Option<i64>,
    pub title: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationMessage {
    pub id: i64,
    pub conversation_id: i64,
    pub role: String, // "user", "assistant", "system"
    pub content: String,
    pub sources: Option<String>, // JSON array of cited chunk IDs
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationContext {
    /// Summary of previous conversation
    pub summary: Option<String>,
    /// Recent messages for context
    pub recent_messages: Vec<ConversationMessage>,
    /// Extracted entities/topics from conversation
    pub entities: Vec<String>,
}

pub struct ConversationService {
    rag_repository: Arc<RagRepository>,
}

impl ConversationService {
    pub fn new(rag_repository: Arc<RagRepository>) -> Self {
        Self { rag_repository }
    }

    /// Create a new conversation
    pub async fn create_conversation(
        &self,
        collection_id: Option<i64>,
        title: Option<&str>,
    ) -> Result<i64> {
        let sql = r#"
            INSERT INTO conversations (collection_id, title)
            VALUES (?, ?)
        "#;

        let result = sqlx::query(sql)
            .bind(collection_id)
            .bind(title)
            .execute(self.rag_repository.pool())
            .await
            .map_err(|e| AppError::Internal(format!("Failed to create conversation: {}", e)))?;

        Ok(result.last_insert_rowid())
    }

    /// Add a message to a conversation
    pub async fn add_message(
        &self,
        conversation_id: i64,
        role: &str,
        content: &str,
        sources: Option<&str>,
    ) -> Result<i64> {
        let sql = r#"
            INSERT INTO conversation_messages (conversation_id, role, content, sources)
            VALUES (?, ?, ?, ?)
        "#;

        let result = sqlx::query(sql)
            .bind(conversation_id)
            .bind(role)
            .bind(content)
            .bind(sources)
            .execute(self.rag_repository.pool())
            .await
            .map_err(|e| AppError::Internal(format!("Failed to add message: {}", e)))?;

        // Update conversation timestamp
        let update_sql = r#"
            UPDATE conversations SET updated_at = CURRENT_TIMESTAMP WHERE id = ?
        "#;
        let _ = sqlx::query(update_sql)
            .bind(conversation_id)
            .execute(self.rag_repository.pool())
            .await;

        Ok(result.last_insert_rowid())
    }

    /// Get messages from a conversation (ascending order)
    pub async fn get_messages(
        &self,
        conversation_id: i64,
        limit: i64,
    ) -> Result<Vec<ConversationMessage>> {
        self.get_recent_messages(conversation_id, limit as usize)
            .await
    }

    /// Get recent messages from a conversation
    pub async fn get_recent_messages(
        &self,
        conversation_id: i64,
        limit: usize,
    ) -> Result<Vec<ConversationMessage>> {
        let sql = r#"
            SELECT id, conversation_id, role, content, sources, created_at
            FROM conversation_messages
            WHERE conversation_id = ?
            ORDER BY created_at DESC
            LIMIT ?
        "#;

        let rows: Vec<(i64, i64, String, String, Option<String>, String)> = sqlx::query_as(sql)
            .bind(conversation_id)
            .bind(limit as i64)
            .fetch_all(self.rag_repository.pool())
            .await
            .map_err(|e| AppError::Internal(format!("Failed to get messages: {}", e)))?;

        let mut messages: Vec<ConversationMessage> = rows
            .into_iter()
            .map(
                |(id, conv_id, role, content, sources, created_at)| ConversationMessage {
                    id,
                    conversation_id: conv_id,
                    role,
                    content,
                    sources,
                    created_at,
                },
            )
            .collect();

        // Reverse to get chronological order
        messages.reverse();
        Ok(messages)
    }

    /// Build conversation context for a new query
    pub async fn build_context(&self, conversation_id: i64) -> Result<ConversationContext> {
        let messages = self
            .get_recent_messages(conversation_id, MAX_CONTEXT_MESSAGES)
            .await?;

        // Extract entities from conversation
        let entities = self.extract_entities(&messages);

        // Summarize if too many messages
        let (summary, recent_messages) = if messages.len() > 5 {
            let summary = self.summarize_conversation(&messages[..messages.len() - 3]);
            (Some(summary), messages[messages.len() - 3..].to_vec())
        } else {
            (None, messages)
        };

        Ok(ConversationContext {
            summary,
            recent_messages,
            entities,
        })
    }

    /// Build a prompt with conversation context
    pub fn build_contextual_prompt(
        &self,
        context: &ConversationContext,
        new_query: &str,
        rag_context: &str,
    ) -> String {
        let mut prompt = String::new();

        // Add conversation summary if available
        if let Some(ref summary) = context.summary {
            prompt.push_str("Previous conversation summary:\n");
            prompt.push_str(summary);
            prompt.push_str("\n\n");
        }

        // Add recent messages
        if !context.recent_messages.is_empty() {
            prompt.push_str("Recent conversation:\n");
            for msg in &context.recent_messages {
                let role_label = match msg.role.as_str() {
                    "user" => "User",
                    "assistant" => "Assistant",
                    _ => &msg.role,
                };
                prompt.push_str(&format!("{}: {}\n", role_label, msg.content));
            }
            prompt.push_str("\n");
        }

        // Add entities for context
        if !context.entities.is_empty() {
            prompt.push_str(&format!(
                "Key topics discussed: {}\n\n",
                context.entities.join(", ")
            ));
        }

        // Add RAG context
        prompt.push_str("Retrieved information:\n");
        prompt.push_str(rag_context);
        prompt.push_str("\n\n");

        // Add new query
        prompt.push_str(&format!("Current question: {}\n", new_query));
        prompt.push_str("\nPlease answer based on the retrieved information and conversation context. Cite sources when applicable.");

        prompt
    }

    /// Extract key entities/topics from messages
    fn extract_entities(&self, messages: &[ConversationMessage]) -> Vec<String> {
        let mut entities = Vec::new();
        let mut seen = std::collections::HashSet::new();

        for msg in messages {
            // Extract capitalized words as potential entities
            for word in msg.content.split_whitespace() {
                let clean = word.trim_matches(|c: char| !c.is_alphanumeric());
                if clean.len() > 3
                    && clean
                        .chars()
                        .next()
                        .map(|c| c.is_uppercase())
                        .unwrap_or(false)
                    && !seen.contains(&clean.to_lowercase())
                {
                    seen.insert(clean.to_lowercase());
                    entities.push(clean.to_string());
                }
            }
        }

        // Limit entities
        entities.truncate(10);
        entities
    }

    /// Create a simple summary of messages
    fn summarize_conversation(&self, messages: &[ConversationMessage]) -> String {
        let mut summary = String::new();
        let mut total_chars = 0;

        for msg in messages {
            if total_chars > MAX_CONTEXT_CHARS / 2 {
                break;
            }

            let role = if msg.role == "user" { "Q" } else { "A" };
            let truncated = if msg.content.len() > 200 {
                format!("{}...", &msg.content[..200])
            } else {
                msg.content.clone()
            };

            summary.push_str(&format!("{}: {}\n", role, truncated));
            total_chars += truncated.len();
        }

        summary
    }

    /// List all conversations for a collection
    pub async fn list_conversations(
        &self,
        collection_id: Option<i64>,
    ) -> Result<Vec<Conversation>> {
        let sql = if collection_id.is_some() {
            r#"
                SELECT id, collection_id, title, created_at, updated_at
                FROM conversations
                WHERE collection_id = ?
                ORDER BY updated_at DESC
            "#
        } else {
            r#"
                SELECT id, collection_id, title, created_at, updated_at
                FROM conversations
                ORDER BY updated_at DESC
            "#
        };

        let rows: Vec<(i64, Option<i64>, Option<String>, String, String)> = if let Some(cid) =
            collection_id
        {
            sqlx::query_as(sql)
                .bind(cid)
                .fetch_all(self.rag_repository.pool())
                .await
                .map_err(|e| AppError::Internal(format!("Failed to list conversations: {}", e)))?
        } else {
            sqlx::query_as(sql)
                .fetch_all(self.rag_repository.pool())
                .await
                .map_err(|e| AppError::Internal(format!("Failed to list conversations: {}", e)))?
        };

        Ok(rows
            .into_iter()
            .map(
                |(id, collection_id, title, created_at, updated_at)| Conversation {
                    id,
                    collection_id,
                    title,
                    created_at,
                    updated_at,
                },
            )
            .collect())
    }

    /// Delete a conversation and all its messages
    pub async fn delete_conversation(&self, conversation_id: i64) -> Result<()> {
        let sql = "DELETE FROM conversations WHERE id = ?";
        sqlx::query(sql)
            .bind(conversation_id)
            .execute(self.rag_repository.pool())
            .await
            .map_err(|e| AppError::Internal(format!("Failed to delete conversation: {}", e)))?;

        Ok(())
    }
}
