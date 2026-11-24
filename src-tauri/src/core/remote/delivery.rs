use serde_json::{json, Value};
use sqlx::{Row, SqlitePool};
use tauri::{AppHandle, Manager};

use crate::app_state::AppState;

pub(crate) async fn init_schema(pool: &SqlitePool) -> Result<(), String> {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS companion_deliveries (
            seq INTEGER PRIMARY KEY AUTOINCREMENT,
            device_id TEXT NOT NULL,
            kind TEXT NOT NULL,
            payload TEXT NOT NULL
        )",
    )
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;
    sqlx::query("CREATE INDEX IF NOT EXISTS companion_delivery_device_seq ON companion_deliveries(device_id, seq)")
        .execute(pool)
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

fn pool(app: &AppHandle) -> Result<SqlitePool, String> {
    let state = app.try_state::<AppState>().ok_or("app state unavailable")?;
    Ok(state.core.chat().conversation().db_pool())
}

pub(crate) fn enqueue(
    app: &AppHandle,
    kind: &'static str,
    mut payload: Value,
) -> Result<(), String> {
    let devices = super::list_paired_devices(app);
    if devices.is_empty() {
        return Ok(());
    }
    let pool = pool(app)?;
    payload["createdAtEpochMs"] = json!(chrono::Utc::now().timestamp_millis());
    let payload = payload.to_string();
    // Tool execution is synchronous. Run the database future on the Tauri runtime
    // and wait for its commit before reporting the tool result.
    let (tx, rx) = std::sync::mpsc::sync_channel(1);
    tauri::async_runtime::spawn(async move {
        let result = async {
            let mut transaction = pool.begin().await?;
            for device in devices {
                sqlx::query(
                    "INSERT INTO companion_deliveries(device_id, kind, payload) VALUES (?, ?, ?)",
                )
                .bind(device.device_id)
                .bind(kind)
                .bind(&payload)
                .execute(&mut *transaction)
                .await?;
            }
            transaction.commit().await
        }
        .await;
        let _ = tx.send(result);
    });
    rx.recv()
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())
}

pub(super) async fn list(app: &AppHandle, device_id: &str, after: i64) -> Result<Value, String> {
    if !super::list_paired_devices(app)
        .iter()
        .any(|device| device.device_id == device_id)
    {
        return Err("device is no longer paired".into());
    }
    list_page(&pool(app)?, device_id, after).await
}

async fn list_page(pool: &SqlitePool, device_id: &str, after: i64) -> Result<Value, String> {
    let rows = sqlx::query(
        "SELECT seq, kind, payload FROM companion_deliveries
         WHERE device_id = ? AND seq > ? ORDER BY seq LIMIT 51",
    )
    .bind(device_id)
    .bind(after)
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())?;
    let mut next = after;
    let mut items = Vec::new();
    for row in rows.iter().take(50) {
        next = row.get("seq");
        let kind: String = row.get("kind");
        let payload: &str = row.get("payload");
        let data: Value = serde_json::from_str(payload).map_err(|e| e.to_string())?;
        items.push(json!({ "kind": kind, "data": data }));
    }
    Ok(json!({ "items": items, "next": next, "hasMore": rows.len() > 50 }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn delivery_pages_resume_independently_for_each_device() {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        init_schema(&pool).await.unwrap();
        for index in 0..52 {
            for device in ["phone-a", "phone-b"] {
                sqlx::query("INSERT INTO companion_deliveries(device_id, kind, payload) VALUES (?, 'file', ?)")
                    .bind(device)
                    .bind(json!({"offerId": format!("{device}-{index}")}).to_string())
                    .execute(&pool)
                    .await
                    .unwrap();
            }
        }

        let first = list_page(&pool, "phone-a", 0).await.unwrap();
        assert_eq!(first["items"].as_array().unwrap().len(), 50);
        assert_eq!(first["hasMore"], true);
        let cursor = first["next"].as_i64().unwrap();
        let second = list_page(&pool, "phone-a", cursor).await.unwrap();
        assert_eq!(second["items"].as_array().unwrap().len(), 2);
        assert_eq!(second["items"][0]["data"]["offerId"], "phone-a-50");
        assert_eq!(second["hasMore"], false);
        let other = list_page(&pool, "phone-b", 0).await.unwrap();
        assert_eq!(other["items"][0]["data"]["offerId"], "phone-b-0");
    }
}
