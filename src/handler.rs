use actix_web::{http::StatusCode, web, HttpResponse, ResponseError};
use anyhow::{anyhow, Context};
use serde::Serialize;
use sqlx::PgPool;
use uuid::Uuid;

use crate::csv::Csv;

#[derive(Serialize, Debug)]
struct DocumentResponse {
    id: Uuid,
    content: serde_json::Value,
}

/// Document must contain valid UTF-8.
#[tracing::instrument(name = "HANDLER__CREATE_DOCUMENT", skip(db_conn_pool))]
pub(crate) async fn create_document(
    request_body: Csv<Vec<String>>,
    db_conn_pool: web::Data<PgPool>,
) -> Result<HttpResponse, CreateDocumentError> {
    let id = Uuid::new_v4();
    let request_body = request_body.into_inner();
    let json_value = serde_json::value::to_value(request_body)?;

    sqlx::query!(
        r#"
        INSERT INTO document
          (id, content)
        VALUES ($1, $2)
        "#,
        id,
        &json_value,
    )
    .execute(&**db_conn_pool)
    .await
    .context("Failed to insert document.")?;

    Ok(HttpResponse::Created().json(&DocumentResponse {
        id,
        content: json_value,
    }))
}

#[derive(thiserror::Error, Debug)]
pub(crate) enum CreateDocumentError {
    #[error(transparent)]
    Unknown(#[from] anyhow::Error),

    #[error("Failed to serialize document to JSON: {}", 0)]
    Serialize(#[from] serde_json::Error),
}

impl ResponseError for CreateDocumentError {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::Unknown(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Self::Serialize(_) => StatusCode::BAD_REQUEST,
        }
    }
}

#[tracing::instrument(name = "HANDLER__GET_DOCUMENT", skip(db_conn_pool))]
pub(crate) async fn get_document(
    path: web::Path<(String,)>,
    db_conn_pool: web::Data<PgPool>,
) -> Result<HttpResponse, GetDocumentError> {
    let id = Uuid::parse_str(&path.0).map_err(|_| GetDocumentError::NotFound)?;
    let record = sqlx::query!(
        r#"
        SELECT id, content FROM document WHERE id = $1
        "#,
        id,
    )
    .fetch_one(&**db_conn_pool)
    .await
    .map_err(|err| match err {
        sqlx::Error::RowNotFound => GetDocumentError::NotFound,
        _ => GetDocumentError::Unknown(anyhow!(err).context("Failed to fetch document.")),
    })?;

    Ok(HttpResponse::Ok().json(&DocumentResponse {
        id: record.id,
        content: record.content,
    }))
}

#[derive(thiserror::Error, Debug)]
pub(crate) enum GetDocumentError {
    #[error(transparent)]
    Unknown(#[from] anyhow::Error),

    #[error("Document not found.")]
    NotFound,
}

impl ResponseError for GetDocumentError {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::Unknown(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Self::NotFound => StatusCode::NOT_FOUND,
        }
    }
}
