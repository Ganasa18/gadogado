use crate::domain::error::{AppError, Result};

use super::RagRepository;

#[derive(Debug, Clone)]
pub struct StructuredRowWithDoc {
    pub id: i64,
    pub doc_id: i64,
    pub row_index: i64,
    pub category: Option<String>,
    pub source: Option<String>,
    pub title: Option<String>,
    pub created_at_text: Option<String>,
    pub created_at: Option<String>,
    pub content: Option<String>,
    pub data_json: String,
    pub doc_name: String,
}

impl RagRepository {
    pub async fn insert_structured_rows(
        &self,
        doc_id: i64,
        rows: Vec<(
            i64,
            Option<String>,
            Option<String>,
            Option<String>,
            Option<String>,
            Option<String>,
            Option<String>,
            String,
        )>,
    ) -> Result<u64> {
        // rows: (row_index, category, source, title, created_at_text, created_at, content, data_json)
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| AppError::DatabaseError(format!("Failed to begin transaction: {}", e)))?;

        let mut affected: u64 = 0;
        for (
            row_index,
            category,
            source,
            title,
            created_at_text,
            created_at,
            content,
            data_json,
        ) in rows
        {
            let res = sqlx::query(
                "INSERT INTO structured_rows (\n                    doc_id, row_index, category, source, title, created_at_text, created_at, content, data_json\n                 ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
            )
            .bind(doc_id)
            .bind(row_index)
            .bind(category)
            .bind(source)
            .bind(title)
            .bind(created_at_text)
            .bind(created_at)
            .bind(content)
            .bind(data_json)
            .execute(&mut *tx)
            .await
            .map_err(|e| {
                AppError::DatabaseError(format!("Failed to insert structured row: {}", e))
            })?;
            affected += res.rows_affected();
        }

        tx.commit().await.map_err(|e| {
            AppError::DatabaseError(format!("Failed to commit transaction: {}", e))
        })?;

        Ok(affected)
    }

    pub async fn query_structured_rows_by_collection(
        &self,
        collection_id: i64,
        category: Option<&str>,
        source: Option<&str>,
        keyword: Option<&str>,
        limit: i64,
    ) -> Result<Vec<StructuredRowWithDoc>> {
        #[derive(sqlx::FromRow)]
        struct StructuredRowEntity {
            id: i64,
            doc_id: i64,
            row_index: i64,
            category: Option<String>,
            source: Option<String>,
            title: Option<String>,
            created_at_text: Option<String>,
            created_at: Option<String>,
            content: Option<String>,
            data_json: String,
            file_name: String,
        }

        let mut sql = String::from(
            "SELECT sr.id, sr.doc_id, sr.row_index, sr.category, sr.source, sr.title,\n                    sr.created_at_text, sr.created_at, sr.content, sr.data_json,\n                    d.file_name\n             FROM structured_rows sr\n             INNER JOIN documents d ON sr.doc_id = d.id\n             WHERE d.collection_id = ?",
        );

        if category.is_some() {
            sql.push_str(" AND sr.category = ?");
        }

        if source.is_some() {
            sql.push_str(" AND sr.source = ?");
        }

        if keyword.is_some() {
            sql.push_str(" AND COALESCE(sr.content, '') LIKE ?");
        }

        sql.push_str(" ORDER BY sr.row_index ASC LIMIT ?");

        let mut qb = sqlx::query_as::<_, StructuredRowEntity>(&sql).bind(collection_id);
        if let Some(cat) = category {
            qb = qb.bind(cat);
        }
        if let Some(src) = source {
            qb = qb.bind(src);
        }
        if let Some(k) = keyword {
            qb = qb.bind(format!("%{}%", k));
        }
        qb = qb.bind(limit);

        let rows = qb
            .fetch_all(&self.pool)
            .await
            .map_err(|e| AppError::DatabaseError(format!("Failed to query structured rows: {}", e)))?;

        Ok(rows
            .into_iter()
            .map(|r| StructuredRowWithDoc {
                id: r.id,
                doc_id: r.doc_id,
                row_index: r.row_index,
                category: r.category,
                source: r.source,
                title: r.title,
                created_at_text: r.created_at_text,
                created_at: r.created_at,
                content: r.content,
                data_json: r.data_json,
                doc_name: r.file_name,
            })
            .collect())
    }

    pub async fn count_structured_rows_by_collection(
        &self,
        collection_id: i64,
        category: Option<&str>,
        source: Option<&str>,
        keyword: Option<&str>,
    ) -> Result<i64> {
        let mut sql = String::from(
            "SELECT COUNT(*)\n             FROM structured_rows sr\n             INNER JOIN documents d ON sr.doc_id = d.id\n             WHERE d.collection_id = ?",
        );

        if category.is_some() {
            sql.push_str(" AND sr.category = ?");
        }

        if source.is_some() {
            sql.push_str(" AND sr.source = ?");
        }

        if keyword.is_some() {
            sql.push_str(" AND COALESCE(sr.content, '') LIKE ?");
        }

        let mut qb = sqlx::query_scalar::<_, i64>(&sql).bind(collection_id);
        if let Some(cat) = category {
            qb = qb.bind(cat);
        }
        if let Some(src) = source {
            qb = qb.bind(src);
        }
        if let Some(k) = keyword {
            qb = qb.bind(format!("%{}%", k));
        }

        qb.fetch_one(&self.pool)
            .await
            .map_err(|e| AppError::DatabaseError(format!("Failed to count structured rows: {}", e)))
    }
}
