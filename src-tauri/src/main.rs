// ═══════════════════════════════════════════════════════════
//  Rosetta ERP — Tauri Backend (Rust)
//  مخزن بيانات عام (key-value) فوق SQLite حقيقي على القرص
//  كل "store" (invoices, returns, clients, products, delegates,
//  settings) مُمثَّل كصف في جدول واحد، بنفس مبدأ IndexedDB
//  لكن بثبات ملف حقيقي بدل تخزين المتصفح
// ═══════════════════════════════════════════════════════════
use rusqlite::{params, Connection};
use serde::Deserialize;
use std::sync::Mutex;
use tauri::{Manager, State};

struct DbState(Mutex<Connection>);

fn init_db(conn: &Connection) {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS kv_store (
            store TEXT NOT NULL,
            id    TEXT NOT NULL,
            data  TEXT NOT NULL,
            PRIMARY KEY (store, id)
        )",
        [],
    )
    .expect("فشل إنشاء جدول kv_store");
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_kv_store ON kv_store(store)",
        [],
    )
    .expect("فشل إنشاء الفهرس");
}

#[tauri::command]
fn kv_get(state: State<DbState>, store: String, id: String) -> Option<String> {
    let conn = state.0.lock().unwrap();
    conn.query_row(
        "SELECT data FROM kv_store WHERE store = ?1 AND id = ?2",
        params![store, id],
        |row| row.get(0),
    )
    .ok()
}

#[tauri::command]
fn kv_put(state: State<DbState>, store: String, id: String, data: String) -> Result<(), String> {
    let conn = state.0.lock().unwrap();
    conn.execute(
        "INSERT INTO kv_store (store, id, data) VALUES (?1, ?2, ?3)
         ON CONFLICT(store, id) DO UPDATE SET data = excluded.data",
        params![store, id, data],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
fn kv_delete(state: State<DbState>, store: String, id: String) -> Result<(), String> {
    let conn = state.0.lock().unwrap();
    conn.execute(
        "DELETE FROM kv_store WHERE store = ?1 AND id = ?2",
        params![store, id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
fn kv_get_all(state: State<DbState>, store: String) -> Vec<String> {
    let conn = state.0.lock().unwrap();
    let mut stmt = match conn.prepare("SELECT data FROM kv_store WHERE store = ?1") {
        Ok(s) => s,
        Err(_) => return vec![],
    };
    let rows = stmt.query_map(params![store], |row| row.get(0));
    match rows {
        Ok(r) => r.filter_map(|x| x.ok()).collect(),
        Err(_) => vec![],
    }
}

#[tauri::command]
fn kv_clear(state: State<DbState>, store: String) -> Result<(), String> {
    let conn = state.0.lock().unwrap();
    conn.execute("DELETE FROM kv_store WHERE store = ?1", params![store])
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
fn kv_count(state: State<DbState>, store: String) -> i64 {
    let conn = state.0.lock().unwrap();
    conn.query_row(
        "SELECT COUNT(*) FROM kv_store WHERE store = ?1",
        params![store],
        |row| row.get(0),
    )
    .unwrap_or(0)
}

#[derive(Deserialize)]
struct AtomicOp {
    op: String, // "put" أو "delete"
    store: String,
    id: String,
    data: Option<String>,
}

// ✅ معاملة SQLite حقيقية: كل العمليات تنجح معاً أو تفشل معاً (BEGIN/COMMIT/ROLLBACK)
#[tauri::command]
fn kv_atomic(state: State<DbState>, ops: Vec<AtomicOp>) -> Result<(), String> {
    let conn = state.0.lock().unwrap();
    conn.execute_batch("BEGIN").map_err(|e| e.to_string())?;
    for op in ops {
        let result = if op.op == "put" {
            conn.execute(
                "INSERT INTO kv_store (store, id, data) VALUES (?1, ?2, ?3)
                 ON CONFLICT(store, id) DO UPDATE SET data = excluded.data",
                params![op.store, op.id, op.data.unwrap_or_default()],
            )
        } else {
            conn.execute(
                "DELETE FROM kv_store WHERE store = ?1 AND id = ?2",
                params![op.store, op.id],
            )
        };
        if let Err(e) = result {
            let _ = conn.execute_batch("ROLLBACK");
            return Err(e.to_string());
        }
    }
    conn.execute_batch("COMMIT").map_err(|e| e.to_string())?;
    Ok(())
}

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            let app_dir = app
                .path()
                .app_data_dir()
                .expect("لا يمكن تحديد مجلد بيانات التطبيق");
            std::fs::create_dir_all(&app_dir).expect("فشل إنشاء مجلد بيانات التطبيق");
            let db_path = app_dir.join("rosetta.db");
            let conn = Connection::open(&db_path).expect("فشل فتح قاعدة البيانات");
            conn.pragma_update(None, "journal_mode", "WAL").ok();
            init_db(&conn);
            app.manage(DbState(Mutex::new(conn)));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            kv_get, kv_put, kv_delete, kv_get_all, kv_clear, kv_count, kv_atomic,
            save_file_dialog
        ])
        .run(tauri::generate_context!())
        .expect("حدث خطأ أثناء تشغيل التطبيق");
}

// ── نافذة حفظ ملف حقيقية (Windows Save As Dialog) ──────────
#[tauri::command]
fn save_file_dialog(filename: String, content: String) -> Result<Option<String>, String> {
    let ext = if filename.ends_with(".json") { "json" }
              else if filename.ends_with(".csv") { "csv" }
              else { "txt" };

    let path = rfd::FileDialog::new()
        .set_file_name(&filename)
        .add_filter(ext, &[ext])
        .save_file();

    match path {
        Some(p) => {
            std::fs::write(&p, content.as_bytes()).map_err(|e| e.to_string())?;
            Ok(Some(p.to_string_lossy().to_string()))
        }
        None => Ok(None),
    }
}
