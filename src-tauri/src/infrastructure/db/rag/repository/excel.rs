use crate::domain::error::{AppError, Result};
use crate::domain::rag_entities::{RagExcelData, RagExcelDataInput};

use super::entities::RagExcelDataEntity;
use super::RagRepository;

impl RagRepository {
    pub async fn create_excel_data(&self, input: &RagExcelDataInput) -> Result<RagExcelData> {
        let result = sqlx::query_as::<_, RagExcelDataEntity>(
            "INSERT INTO excel_data (doc_id, row_index, data_json, val_a, val_b, val_c)\n             VALUES (?, ?, ?, ?, ?, ?) RETURNING *",
        )
        .bind(input.doc_id)
        .bind(input.row_index)
        .bind(&input.data_json)
        .bind(&input.val_a)
        .bind(&input.val_b)
        .bind(input.val_c)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to create excel data: {}", e)))?;

        Ok(result.into())
    }

    pub async fn get_excel_data(&self, doc_id: i64, limit: i64) -> Result<Vec<RagExcelData>> {
        let data = sqlx::query_as::<_, RagExcelDataEntity>(
            "SELECT id, doc_id, row_index, data_json, val_a, val_b, val_c\n             FROM excel_data WHERE doc_id = ? ORDER BY row_index ASC LIMIT ?",
        )
        .bind(doc_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to fetch excel data: {}", e)))?;

        Ok(data.into_iter().map(|d| d.into()).collect())
    }

    pub async fn search_excel_by_collection(
        &self,
        collection_id: i64,
        limit: i64,
    ) -> Result<Vec<RagExcelData>> {
        let data = sqlx::query_as::<_, RagExcelDataEntity>(
            "SELECT ed.id, ed.doc_id, ed.row_index, ed.data_json, ed.val_a, ed.val_b, ed.val_c\n             FROM excel_data ed\n             INNER JOIN documents d ON ed.doc_id = d.id\n             WHERE d.collection_id = ?\n             ORDER BY ed.row_index ASC\n             LIMIT ?",
        )
        .bind(collection_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| {
            AppError::DatabaseError(format!("Failed to search excel by collection: {}", e))
        })?;

        Ok(data.into_iter().map(|d| d.into()).collect())
    }

    pub async fn search_excel_by_collection_with_filter(
        &self,
        collection_id: i64,
        column_a: Option<&str>,
        column_b: Option<&str>,
        limit: i64,
    ) -> Result<Vec<RagExcelData>> {
        let mut query = String::from(
            "SELECT ed.id, ed.doc_id, ed.row_index, ed.data_json, ed.val_a, ed.val_b, ed.val_c\n             FROM excel_data ed\n             INNER JOIN documents d ON ed.doc_id = d.id\n             WHERE d.collection_id = ?",
        );
        let mut params: Vec<String> = Vec::new();
        let mut param_count = 1;

        if let Some(val) = column_a {
            param_count += 1;
            query.push_str(&format!(" AND ed.val_a = ?{}", param_count));
            params.push(val.to_string());
        }

        if let Some(val) = column_b {
            param_count += 1;
            query.push_str(&format!(" AND ed.val_b = ?{}", param_count));
            params.push(val.to_string());
        }

        query.push_str(&format!(
            " ORDER BY ed.row_index ASC LIMIT ?{}",
            param_count + 1
        ));

        let mut query_builder = sqlx::query_as::<_, RagExcelDataEntity>(&query);
        query_builder = query_builder.bind(collection_id);

        for param in params {
            query_builder = query_builder.bind(param);
        }

        query_builder = query_builder.bind(limit);

        let data = query_builder.fetch_all(&self.pool).await.map_err(|e| {
            AppError::DatabaseError(format!("Failed to search excel with filter: {}", e))
        })?;

        Ok(data.into_iter().map(|d| d.into()).collect())
    }
}
