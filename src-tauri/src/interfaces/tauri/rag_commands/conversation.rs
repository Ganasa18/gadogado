//! Conversation Persistence Commands
//!
//! This module provides Tauri commands for:
//! - Creating, listing, and deleting conversations
//! - Adding and retrieving conversation messages

use crate::application::use_cases::conversation_service::{Conversation, ConversationMessage};
use crate::domain::error::Result;
use crate::interfaces::http::add_log;
use std::sync::Arc;
use tauri::State;

#[tauri::command]
pub async fn rag_create_conversation(
    state: State<'_, Arc<super::AppState>>,
    collection_id: Option<i64>,
    title: Option<String>,
) -> Result<i64> {
    add_log(
        &state.logs,
        "INFO",
        "RAG",
        &format!("Creating conversation for collection: {:?}", collection_id),
    );

    state
        .conversation_service
        .create_conversation(collection_id, title.as_deref())
        .await
        .map_err(|e| {
            add_log(
                &state.logs,
                "ERROR",
                "RAG",
                &format!("Failed to create conversation: {}", e),
            );
            e
        })
}

/// Add a message to a conversation

#[tauri::command]
pub async fn rag_add_conversation_message(
    state: State<'_, Arc<super::AppState>>,
    conversation_id: i64,
    role: String,
    content: String,
    sources: Option<Vec<i64>>,
) -> Result<i64> {
    add_log(
        &state.logs,
        "INFO",
        "RAG",
        &format!(
            "Adding {} message to conversation {}",
            role, conversation_id
        ),
    );

    // Convert sources to JSON string if provided
    let sources_json = sources.map(|s| serde_json::to_string(&s).unwrap_or_default());

    state
        .conversation_service
        .add_message(conversation_id, &role, &content, sources_json.as_deref())
        .await
        .map_err(|e| {
            add_log(
                &state.logs,
                "ERROR",
                "RAG",
                &format!("Failed to add message: {}", e),
            );
            e
        })
}

/// Get messages for a conversation

#[tauri::command]
pub async fn rag_get_conversation_messages(
    state: State<'_, Arc<super::AppState>>,
    conversation_id: i64,
    limit: Option<i64>,
) -> Result<Vec<ConversationMessage>> {
    add_log(
        &state.logs,
        "INFO",
        "RAG",
        &format!("Getting messages for conversation {}", conversation_id),
    );

    state
        .conversation_service
        .get_messages(conversation_id, limit.unwrap_or(100))
        .await
        .map_err(|e| {
            add_log(
                &state.logs,
                "ERROR",
                "RAG",
                &format!("Failed to get messages: {}", e),
            );
            e
        })
}

/// List conversations for a collection (or all if no collection_id)

#[tauri::command]
pub async fn rag_list_conversations(
    state: State<'_, Arc<super::AppState>>,
    collection_id: Option<i64>,
) -> Result<Vec<Conversation>> {
    add_log(
        &state.logs,
        "INFO",
        "RAG",
        &format!("Listing conversations for collection: {:?}", collection_id),
    );

    state
        .conversation_service
        .list_conversations(collection_id)
        .await
        .map_err(|e| {
            add_log(
                &state.logs,
                "ERROR",
                "RAG",
                &format!("Failed to list conversations: {}", e),
            );
            e
        })
}

/// Delete a conversation and all its messages

#[tauri::command]
pub async fn rag_delete_conversation(
    state: State<'_, Arc<super::AppState>>,
    conversation_id: i64,
) -> Result<()> {
    add_log(
        &state.logs,
        "INFO",
        "RAG",
        &format!("Deleting conversation {}", conversation_id),
    );

    state
        .conversation_service
        .delete_conversation(conversation_id)
        .await
        .map_err(|e| {
            add_log(
                &state.logs,
                "ERROR",
                "RAG",
                &format!("Failed to delete conversation: {}", e),
            );
            e
        })
}

