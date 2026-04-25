use crate::models::Category;
use dioxus::prelude::*;
use uuid::Uuid;

#[cfg(feature = "server")]
use {crate::db::pool, crate::db_rows::CategoryRow};

/// List all categories ordered so that each parent is immediately followed by
/// its subcategories: `COALESCE(parent_id, id), parent_id NULLS FIRST, name`.
#[server]
pub async fn list_categories() -> Result<Vec<Category>, ServerFnError> {
    let db = pool();
    let rows: Vec<CategoryRow> = sqlx::query_as(
        "SELECT id, name, color, parent_id FROM categories \
         ORDER BY COALESCE(parent_id, id), parent_id NULLS FIRST, name",
    )
    .fetch_all(db)
    .await
    .map_err(|e| ServerFnError::new(e.to_string()))?;

    Ok(rows.into_iter().map(Into::into).collect())
}

/// Create a new category. Pass `parent_id = Some(…)` to create a subcategory.
#[server]
pub async fn create_category(
    name: String,
    color: String,
    parent_id: Option<Uuid>,
) -> Result<Category, ServerFnError> {
    let db = pool();
    let row: CategoryRow = sqlx::query_as(
        "INSERT INTO categories (name, color, parent_id) \
         VALUES ($1, $2, $3) \
         RETURNING id, name, color, parent_id",
    )
    .bind(&name)
    .bind(&color)
    .bind(parent_id)
    .fetch_one(db)
    .await
    .map_err(|e| ServerFnError::new(e.to_string()))?;

    Ok(row.into())
}

/// Rename and/or recolor an existing category (works for both tiers).
#[server]
pub async fn update_category(
    id: Uuid,
    name: String,
    color: String,
) -> Result<Category, ServerFnError> {
    let db = pool();
    let row: CategoryRow = sqlx::query_as(
        "UPDATE categories SET name = $1, color = $2 WHERE id = $3 \
         RETURNING id, name, color, parent_id",
    )
    .bind(&name)
    .bind(&color)
    .bind(id)
    .fetch_one(db)
    .await
    .map_err(|e| ServerFnError::new(e.to_string()))?;

    Ok(row.into())
}

/// Delete a category by id. Deleting a top-level category cascades to its
/// subcategories; the existing ON DELETE SET NULL on transactions.category_id
/// then unclassifies any transactions that were in those subcategories.
#[server]
pub async fn delete_category(id: Uuid) -> Result<(), ServerFnError> {
    let db = pool();
    sqlx::query("DELETE FROM categories WHERE id = $1")
        .bind(id)
        .execute(db)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    Ok(())
}

/// Assign a category to a transaction (pass `None` to un-classify).
#[server]
pub async fn classify_transaction(
    tx_id: Uuid,
    category_id: Option<Uuid>,
) -> Result<(), ServerFnError> {
    let db = pool();
    sqlx::query("UPDATE transactions SET category_id = $1 WHERE id = $2")
        .bind(category_id)
        .bind(tx_id)
        .execute(db)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    Ok(())
}
