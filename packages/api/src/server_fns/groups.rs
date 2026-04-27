use crate::models::Group;
use dioxus::prelude::*;
use uuid::Uuid;

#[cfg(feature = "server")]
use {
    crate::auth::session::current_user_id,
    crate::db::pool,
    crate::db_rows::GroupRow,
};

/// List all groups for the current user, newest first.
#[server]
pub async fn list_groups() -> Result<Vec<Group>, ServerFnError> {
    let user_id = current_user_id().await?;
    let db = pool();
    let rows: Vec<GroupRow> = sqlx::query_as(
        "SELECT id, name, description FROM groups \
         WHERE user_id = $1 \
         ORDER BY created_at DESC",
    )
    .bind(user_id)
    .fetch_all(db)
    .await
    .map_err(|e| ServerFnError::new(e.to_string()))?;

    Ok(rows.into_iter().map(Into::into).collect())
}

/// Create a new group for the current user.
#[server]
pub async fn create_group(name: String, description: String) -> Result<Group, ServerFnError> {
    let user_id = current_user_id().await?;
    let db = pool();
    let row: GroupRow = sqlx::query_as(
        "INSERT INTO groups (name, description, user_id) \
         VALUES ($1, $2, $3) \
         RETURNING id, name, description",
    )
    .bind(&name)
    .bind(&description)
    .bind(user_id)
    .fetch_one(db)
    .await
    .map_err(|e| ServerFnError::new(e.to_string()))?;

    Ok(row.into())
}

/// Delete a group owned by the current user and remove all its transaction
/// assignments.
#[server]
pub async fn delete_group(group_id: Uuid) -> Result<(), ServerFnError> {
    let user_id = current_user_id().await?;
    let db = pool();
    // Remove junction rows first (FK may not have CASCADE configured).
    sqlx::query("DELETE FROM transaction_groups WHERE group_id = $1")
        .bind(group_id)
        .execute(db)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    sqlx::query("DELETE FROM groups WHERE id = $1 AND user_id = $2")
        .bind(group_id)
        .bind(user_id)
        .execute(db)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    Ok(())
}

/// Tag a transaction as belonging to a group. Both the transaction and the
/// group must belong to the current user.
#[server]
pub async fn add_to_group(tx_id: Uuid, group_id: Uuid) -> Result<(), ServerFnError> {
    let user_id = current_user_id().await?;
    let db = pool();
    sqlx::query(
        r#"
        INSERT INTO transaction_groups (transaction_id, group_id)
        SELECT $1, $2
        WHERE EXISTS (SELECT 1 FROM transactions WHERE id = $1 AND user_id = $3)
          AND EXISTS (SELECT 1 FROM groups       WHERE id = $2 AND user_id = $3)
        ON CONFLICT DO NOTHING
        "#,
    )
    .bind(tx_id)
    .bind(group_id)
    .bind(user_id)
    .execute(db)
    .await
    .map_err(|e| ServerFnError::new(e.to_string()))?;
    Ok(())
}

/// Remove a transaction from a group, verifying the transaction belongs to the
/// current user.
#[server]
pub async fn remove_from_group(tx_id: Uuid, group_id: Uuid) -> Result<(), ServerFnError> {
    let user_id = current_user_id().await?;
    let db = pool();
    sqlx::query(
        "DELETE FROM transaction_groups \
         WHERE transaction_id = $1 AND group_id = $2 \
           AND EXISTS (SELECT 1 FROM transactions WHERE id = $1 AND user_id = $3)",
    )
    .bind(tx_id)
    .bind(group_id)
    .bind(user_id)
    .execute(db)
    .await
    .map_err(|e| ServerFnError::new(e.to_string()))?;
    Ok(())
}
