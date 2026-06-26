use crate::models::HouseholdInfo;
use dioxus::prelude::*;

#[cfg(feature = "server")]
use {crate::auth::session::current_user_id, crate::db::pool, crate::models::HouseholdMember};

/// Get info about the current user's household.
#[server]
pub async fn get_household_info() -> Result<HouseholdInfo, ServerFnError> {
    let user_id = current_user_id().await?;
    let db = pool();

    let row: Option<(uuid::Uuid, String, String)> = sqlx::query_as(
        "SELECT h.id, h.name, h.invite_code \
         FROM households h \
         JOIN users u ON u.household_id = h.id \
         WHERE u.id = $1",
    )
    .bind(user_id)
    .fetch_optional(db)
    .await
    .map_err(|e| ServerFnError::new(e.to_string()))?;

    let (household_id, name, invite_code) =
        row.ok_or_else(|| ServerFnError::new("User has no household"))?;

    let members: Vec<(Option<String>, Option<String>)> =
        sqlx::query_as("SELECT name, email FROM users WHERE household_id = $1 ORDER BY created_at")
            .bind(household_id)
            .fetch_all(db)
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?;

    Ok(HouseholdInfo {
        id: household_id,
        name,
        invite_code,
        members: members
            .into_iter()
            .map(|(name, email)| HouseholdMember { name, email })
            .collect(),
    })
}

/// Create a new household for the current (householdless) user.
#[server]
pub async fn create_household(name: String) -> Result<(), ServerFnError> {
    let user_id = current_user_id().await?;
    let db = pool();

    let invite_code = generate_invite_code();

    let household_id: uuid::Uuid = sqlx::query_scalar(
        "INSERT INTO households (name, invite_code) VALUES ($1, $2) RETURNING id",
    )
    .bind(&name)
    .bind(&invite_code)
    .fetch_one(db)
    .await
    .map_err(|e| ServerFnError::new(e.to_string()))?;

    sqlx::query("UPDATE users SET household_id = $1 WHERE id = $2")
        .bind(household_id)
        .bind(user_id)
        .execute(db)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    Ok(())
}

/// Join an existing household via invite code.
#[server]
pub async fn join_household(invite_code: String) -> Result<(), ServerFnError> {
    let user_id = current_user_id().await?;
    let db = pool();

    let household_id: Option<uuid::Uuid> =
        sqlx::query_scalar("SELECT id FROM households WHERE invite_code = $1")
            .bind(invite_code.trim().to_uppercase())
            .fetch_optional(db)
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?;

    let household_id = household_id.ok_or_else(|| ServerFnError::new("Invalid invite code"))?;

    sqlx::query("UPDATE users SET household_id = $1 WHERE id = $2")
        .bind(household_id)
        .bind(user_id)
        .execute(db)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    Ok(())
}

/// Leave the current household (sets household_id = NULL on the user row).
#[server]
pub async fn leave_household() -> Result<(), ServerFnError> {
    let user_id = current_user_id().await?;
    let db = pool();
    sqlx::query("UPDATE users SET household_id = NULL WHERE id = $1")
        .bind(user_id)
        .execute(db)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    Ok(())
}

/// Regenerate the invite code for the current user's household.
#[server]
pub async fn regenerate_invite_code() -> Result<String, ServerFnError> {
    let user_id = current_user_id().await?;
    let db = pool();

    let new_code = generate_invite_code();

    let rows_affected = sqlx::query(
        "UPDATE households SET invite_code = $1 \
         WHERE id = (SELECT household_id FROM users WHERE id = $2)",
    )
    .bind(&new_code)
    .bind(user_id)
    .execute(db)
    .await
    .map_err(|e| ServerFnError::new(e.to_string()))?
    .rows_affected();

    if rows_affected == 0 {
        return Err(ServerFnError::new("User has no household"));
    }

    Ok(new_code)
}

#[cfg(feature = "server")]
fn generate_invite_code() -> String {
    let id = uuid::Uuid::new_v4();
    let bytes = id.as_bytes();
    format!(
        "{:02X}{:02X}{:02X}{:02X}-{:02X}{:02X}{:02X}{:02X}",
        bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7]
    )
}
