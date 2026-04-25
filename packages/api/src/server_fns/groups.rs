use crate::models::Group;
use dioxus::prelude::*;
use uuid::Uuid;

#[cfg(feature = "server")]
use {crate::db::pool, crate::db_rows::GroupRow};

/// List all groups, newest first.
#[server]
pub async fn list_groups() -> Result<Vec<Group>, ServerFnError> {
    let db = pool();
    let rows: Vec<GroupRow> = sqlx::query_as(
        "SELECT id, name, description FROM groups ORDER BY created_at DESC",
    )
    .fetch_all(db)
    .await
    .map_err(|e| ServerFnError::new(e.to_string()))?;

    Ok(rows.into_iter().map(Into::into).collect())
}

/// Create a new group.
#[server]
pub async fn create_group(name: String, description: String) -> Result<Group, ServerFnError> {
    let db = pool();
    let row: GroupRow = sqlx::query_as(
        "INSERT INTO groups (name, description) VALUES ($1, $2) RETURNING id, name, description",
    )
    .bind(&name)
    .bind(&description)
    .fetch_one(db)
    .await
    .map_err(|e| ServerFnError::new(e.to_string()))?;

    Ok(row.into())
}

/// Tag a transaction as belonging to a group.
#[server]
pub async fn add_to_group(tx_id: Uuid, group_id: Uuid) -> Result<(), ServerFnError> {
    let db = pool();
    sqlx::query(
        "INSERT INTO transaction_groups (transaction_id, group_id) VALUES ($1, $2) ON CONFLICT DO NOTHING",
    )
    .bind(tx_id)
    .bind(group_id)
    .execute(db)
    .await
    .map_err(|e| ServerFnError::new(e.to_string()))?;
    Ok(())
}

/// Remove a transaction from a group.
#[server]
pub async fn remove_from_group(tx_id: Uuid, group_id: Uuid) -> Result<(), ServerFnError> {
    let db = pool();
    sqlx::query(
        "DELETE FROM transaction_groups WHERE transaction_id = $1 AND group_id = $2",
    )
    .bind(tx_id)
    .bind(group_id)
    .execute(db)
    .await
    .map_err(|e| ServerFnError::new(e.to_string()))?;
    Ok(())
}
