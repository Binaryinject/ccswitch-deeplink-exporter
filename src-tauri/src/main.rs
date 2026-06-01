#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use base64::Engine;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Clone)]
struct Provider {
    id: String,
    app_type: String,
    name: String,
    settings_config: Option<serde_json::Value>,
    website_url: Option<String>,
    icon: Option<String>,
    is_current: bool,
    endpoints: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct SkillRepo {
    owner: String,
    name: String,
    branch: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct DeeplinkItem {
    item_type: String,
    name: String,
    app: String,
    deeplink: String,
    endpoint: String,
    model: String,
    is_current: bool,
}

fn get_db_path() -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join(".cc-switch").join("cc-switch.db"))
}

fn open_db(path: &PathBuf) -> Result<Connection, String> {
    Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
        .map_err(|e| format!("打开数据库失败: {}", e))
}

fn query_rows<T, F>(db: &Connection, sql: &str, mapper: F) -> Result<Vec<T>, String>
where
    F: FnMut(&rusqlite::Row) -> rusqlite::Result<T>,
{
    let mut stmt = db.prepare(sql).map_err(|e| e.to_string())?;
    let rows: Vec<T> = stmt
        .query_map([], mapper)
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();
    Ok(rows)
}

fn read_providers(db: &Connection) -> Result<Vec<Provider>, String> {
    query_rows(db,
        "SELECT id, app_type, name, settings_config, website_url, icon, is_current FROM providers",
        |row| {
            let raw: Option<String> = row.get(3)?;
            Ok(Provider {
                id: row.get(0)?,
                app_type: row.get(1)?,
                name: row.get(2)?,
                settings_config: raw.as_ref().and_then(|s| serde_json::from_str(s).ok()),
                website_url: row.get(4)?,
                icon: row.get(5)?,
                is_current: row.get::<_, i32>(6).unwrap_or(0) != 0,
                endpoints: Vec::new(),
            })
        },
    )
}

fn read_endpoints(db: &Connection) -> Result<HashMap<String, Vec<String>>, String> {
    let pairs: Vec<(String, String)> = query_rows(db,
        "SELECT provider_id, url FROM provider_endpoints",
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?;
    Ok(pairs.into_iter().fold(HashMap::new(), |mut acc, (pid, url)| {
        acc.entry(pid).or_default().push(url);
        acc
    }))
}

fn read_skill_repos(db: &Connection) -> Result<Vec<SkillRepo>, String> {
    query_rows(db,
        "SELECT owner, name, branch FROM skill_repos",
        |row| Ok(SkillRepo { owner: row.get(0)?, name: row.get(1)?, branch: row.get(2)? }),
    )
}

fn build_query(params: &[(&str, &str)]) -> String {
    params
        .iter()
        .map(|(k, v)| format!("{}={}", urlencoding::encode(k), urlencoding::encode(v)))
        .collect::<Vec<_>>()
        .join("&")
}

fn build_provider_deeplink(p: &Provider) -> String {
    let config = p.settings_config.as_ref().unwrap_or(&serde_json::Value::Object(Default::default()));
    let config_b64 = base64::engine::general_purpose::STANDARD.encode(
        serde_json::to_string(config).unwrap_or_default().as_bytes(),
    );

    let mut params: Vec<(String, String)> = vec![
        ("resource".into(), "provider".into()),
        ("app".into(), p.app_type.clone()),
        ("name".into(), p.name.clone()),
        ("config".into(), config_b64),
        ("configFormat".into(), "json".into()),
    ];

    if let Some(ref url) = p.website_url {
        params.push(("homepage".into(), url.clone()));
    }
    if let Some(ref icon) = p.icon {
        params.push(("icon".into(), icon.clone()));
    }
    if !p.endpoints.is_empty() {
        params.push(("endpoint".into(), p.endpoints.join(",")));
    }
    if p.is_current {
        params.push(("enabled".into(), "true".into()));
    }

    let refs: Vec<(&str, &str)> = params.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();
    format!("ccswitch://v1/import?{}", build_query(&refs))
}

fn build_skill_deeplink(s: &SkillRepo) -> String {
    let repo = format!("{}/{}", s.owner, s.name);
    format!(
        "ccswitch://v1/import?{}",
        build_query(&[("resource", "skill"), ("repo", &repo), ("branch", &s.branch)])
    )
}

fn extract_info(p: &Provider) -> (String, String) {
    let env = p
        .settings_config
        .as_ref()
        .and_then(|c| c.get("env"))
        .cloned()
        .unwrap_or(serde_json::json!({}));

    let find = |keys: &[&str]| -> String {
        keys.iter()
            .find_map(|k| env.get(k).and_then(|v| v.as_str()))
            .unwrap_or("")
            .to_string()
    };

    (find(&["ANTHROPIC_BASE_URL", "GEMINI_BASE_URL"]), find(&["ANTHROPIC_MODEL", "GEMINI_MODEL"]))
}

#[tauri::command]
fn load_deeplinks() -> Result<Vec<DeeplinkItem>, String> {
    let db_path = get_db_path().ok_or("无法找到用户目录")?;
    if !db_path.exists() {
        return Err(format!("未找到 CC Switch 数据库: {}", db_path.display()));
    }

    let db = open_db(&db_path)?;
    let mut providers = read_providers(&db)?;
    let ep_map = read_endpoints(&db)?;
    let skills = read_skill_repos(&db)?;

    for p in &mut providers {
        if let Some(eps) = ep_map.get(&p.id) {
            p.endpoints = eps.clone();
        }
    }

    let mut items: Vec<DeeplinkItem> = providers
        .iter()
        .map(|p| {
            let (endpoint, model) = extract_info(p);
            DeeplinkItem {
                item_type: "provider".into(),
                name: p.name.clone(),
                app: p.app_type.clone(),
                deeplink: build_provider_deeplink(p),
                endpoint,
                model,
                is_current: p.is_current,
            }
        })
        .collect();

    items.extend(skills.iter().map(|s| DeeplinkItem {
        item_type: "skill".into(),
        name: format!("{}/{}", s.owner, s.name),
        app: "skill".into(),
        deeplink: build_skill_deeplink(s),
        endpoint: String::new(),
        model: format!("branch: {}", s.branch),
        is_current: false,
    }));

    Ok(items)
}

#[tauri::command]
fn get_db_path_str() -> Result<String, String> {
    let db_path = get_db_path().ok_or("无法找到用户目录")?;
    Ok(db_path.to_string_lossy().to_string())
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![load_deeplinks, get_db_path_str])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
