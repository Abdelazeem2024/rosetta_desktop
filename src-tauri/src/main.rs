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
            kv_get, kv_put, kv_delete, kv_get_all, kv_clear, kv_count, kv_atomic, kv_list_stores,
            save_file_dialog,
            backup_pick_folder, backup_default_dir, backup_write, backup_list, backup_delete
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

// ── قائمة أسماء مخازن البيانات الموجودة فعلياً في قاعدة البيانات ──
// تُستخدم في التصدير للتأكد أن أي مخزن بيانات (حتى لو لم يكن معروفاً للواجهة) لا يُفقد من النسخة
#[tauri::command]
fn kv_list_stores(state: State<DbState>) -> Vec<String> {
    let conn = state.0.lock().unwrap();
    let mut stmt = match conn.prepare("SELECT DISTINCT store FROM kv_store") {
        Ok(s) => s,
        Err(_) => return vec![],
    };
    let rows = stmt.query_map([], |row| row.get(0));
    match rows {
        Ok(r) => r.filter_map(|x| x.ok()).collect(),
        Err(_) => vec![],
    }
}

// ══════════ النسخ الاحتياطي التلقائي (مجلد يحدده المستخدم) ══════════
// الاسم الوحيد المسموح بكتابته وقراءته وحذفه: rikaz_auto_backup_YYYY-MM-DD.json
// أي ملف آخر داخل المجلد (نسخ يدوية أو أي ملفات أخرى) لا يُقرأ ولا يُكتب ولا يُحذف إطلاقاً.
fn is_auto_backup_name(name: &str) -> bool {
    let prefix: &[u8] = b"rikaz_auto_backup_";
    let suffix: &[u8] = b".json";
    let nb: &[u8] = name.as_bytes();
    if nb.len() != prefix.len() + 10 + suffix.len() {
        return false;
    }
    if &nb[..prefix.len()] != prefix {
        return false;
    }
    if &nb[nb.len() - suffix.len()..] != suffix {
        return false;
    }
    let date = &nb[prefix.len()..prefix.len() + 10];
    for (i, c) in date.iter().enumerate() {
        if i == 4 || i == 7 {
            if *c != b'-' {
                return false;
            }
        } else if *c < b'0' || *c > b'9' {
            return false;
        }
    }
    true
}

// نافذة اختيار مجلد (نفس مكتبة rfd المستخدمة في نافذة الحفظ)
#[tauri::command]
fn backup_pick_folder() -> Option<String> {
    let picked = rfd::FileDialog::new().pick_folder();
    match picked {
        Some(p) => Some(p.to_string_lossy().to_string()),
        None => None,
    }
}

// المجلد الافتراضي: المستندات ▸ Rikaz Pro Backups (يُنشأ عند أول حفظ)
#[tauri::command]
fn backup_default_dir() -> String {
    let base = std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .unwrap_or_default();
    if base.is_empty() {
        return String::new();
    }
    let mut p = std::path::PathBuf::from(base);
    p.push("Documents");
    p.push("Rikaz Pro Backups");
    p.to_string_lossy().to_string()
}

// كتابة نسخة: تُكتب أولاً في ملف مؤقت ثم يُتحقق من حجمها ثم تُنقل لاسمها النهائي،
// فلا يبقى ملف ناقص أبداً. تُرجع عدد البايتات المكتوبة ليتحقق منه الطرف الآخر.
#[tauri::command]
fn backup_write(dir: String, filename: String, content: String) -> Result<u64, String> {
    if !is_auto_backup_name(&filename) {
        return Err("اسم ملف غير مسموح".to_string());
    }
    if dir.trim().is_empty() {
        return Err("لم يتم تحديد مجلد".to_string());
    }
    let d = std::path::PathBuf::from(&dir);
    std::fs::create_dir_all(&d).map_err(|e| format!("تعذر إنشاء المجلد: {}", e))?;
    let final_path = d.join(&filename);
    let tmp_path = d.join(format!("{}.tmp", filename));
    let bytes = content.as_bytes();
    std::fs::write(&tmp_path, bytes).map_err(|e| format!("تعذر كتابة الملف: {}", e))?;
    let written = std::fs::metadata(&tmp_path)
        .map_err(|e| format!("تعذر فحص الملف: {}", e))?
        .len();
    if written != bytes.len() as u64 {
        let _ = std::fs::remove_file(&tmp_path);
        return Err("لم يُكتب الملف كاملاً".to_string());
    }
    std::fs::rename(&tmp_path, &final_path).map_err(|e| format!("تعذر حفظ الملف: {}", e))?;
    Ok(written)
}

// قائمة النسخ التلقائية فقط (مرتبة تصاعدياً بالتاريخ) — لا تُرجع أي ملف آخر
#[tauri::command]
fn backup_list(dir: String) -> Result<Vec<String>, String> {
    let rd = std::fs::read_dir(&dir).map_err(|e| format!("تعذر قراءة المجلد: {}", e))?;
    let mut names: Vec<String> = Vec::new();
    for entry in rd {
        if let Ok(en) = entry {
            let is_file = match en.file_type() {
                Ok(ft) => ft.is_file(),
                Err(_) => false,
            };
            if !is_file {
                continue;
            }
            let fname = en.file_name();
            if let Some(name) = fname.to_str() {
                if is_auto_backup_name(name) {
                    names.push(name.to_string());
                }
            }
        }
    }
    names.sort();
    Ok(names)
}

// حذف نسخة تلقائية قديمة — يرفض أي اسم ملف لا يطابق صيغة النسخ التلقائية بالضبط
#[tauri::command]
fn backup_delete(dir: String, filename: String) -> Result<(), String> {
    if !is_auto_backup_name(&filename) {
        return Err("اسم ملف غير مسموح".to_string());
    }
    let p = std::path::PathBuf::from(&dir).join(&filename);
    if !p.is_file() {
        return Ok(());
    }
    std::fs::remove_file(&p).map_err(|e| format!("تعذر حذف الملف: {}", e))
}
