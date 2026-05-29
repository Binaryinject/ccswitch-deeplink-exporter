#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use base64::Engine;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
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
    notes: Option<String>,
    endpoints: Vec<Endpoint>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct Endpoint {
    url: String,
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

fn read_providers(db: &Connection) -> Result<Vec<Provider>, String> {
    let mut stmt = db
        .prepare("SELECT id, app_type, name, settings_config, website_url, icon, is_current, notes FROM providers")
        .map_err(|e| e.to_string())?;

    let providers = stmt
        .query_map([], |row| {
            let settings_raw: Option<String> = row.get(3)?;
            let settings_config = settings_raw
                .as_ref()
                .and_then(|s| serde_json::from_str(s).ok());

            Ok(Provider {
                id: row.get(0)?,
                app_type: row.get(1)?,
                name: row.get(2)?,
                settings_config,
                website_url: row.get(4)?,
                icon: row.get(5)?,
                is_current: row.get::<_, i32>(6).unwrap_or(0) != 0,
                notes: row.get(7)?,
                endpoints: Vec::new(),
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    Ok(providers)
}

fn read_endpoints(db: &Connection) -> Result<Vec<(String, String)>, String> {
    let mut stmt = db
        .prepare("SELECT provider_id, url FROM provider_endpoints")
        .map_err(|e| e.to_string())?;

    let eps = stmt
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    Ok(eps)
}

fn read_skill_repos(db: &Connection) -> Result<Vec<SkillRepo>, String> {
    let mut stmt = db
        .prepare("SELECT owner, name, branch FROM skill_repos")
        .map_err(|e| e.to_string())?;

    let skills = stmt
        .query_map([], |row| {
            Ok(SkillRepo {
                owner: row.get(0)?,
                name: row.get(1)?,
                branch: row.get(2)?,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    Ok(skills)
}

fn utf8_to_b64(s: &str) -> String {
    base64::engine::general_purpose::STANDARD.encode(s.as_bytes())
}

fn build_provider_deeplink(p: &Provider) -> String {
    let config = p.settings_config.clone().unwrap_or(serde_json::json!({}));
    let config_json = serde_json::to_string(&config).unwrap_or_default();
    let config_b64 = utf8_to_b64(&config_json);

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

    // Endpoints
    if !p.endpoints.is_empty() {
        let urls: Vec<&str> = p.endpoints.iter().map(|e| e.url.as_str()).collect();
        params.push(("endpoint".into(), urls.join(",")));
    }

    if p.is_current {
        params.push(("enabled".into(), "true".into()));
    }

    let query: String = params
        .iter()
        .map(|(k, v)| format!("{}={}", urlencoding::encode(k), urlencoding::encode(v)))
        .collect::<Vec<_>>()
        .join("&");

    format!("ccswitch://v1/import?{}", query)
}

fn build_skill_deeplink(s: &SkillRepo) -> String {
    let repo = format!("{}/{}", s.owner, s.name);
    let params = vec![
        ("resource", "skill"),
        ("repo", &repo),
        ("branch", &s.branch),
    ];

    let query: String = params
        .iter()
        .map(|(k, v)| format!("{}={}", urlencoding::encode(k), urlencoding::encode(v)))
        .collect::<Vec<_>>()
        .join("&");

    format!("ccswitch://v1/import?{}", query)
}

fn extract_info(p: &Provider) -> (String, String) {
    let config = p.settings_config.clone().unwrap_or(serde_json::json!({}));
    let env = config.get("env").cloned().unwrap_or(serde_json::json!({}));

    let endpoint = env
        .get("ANTHROPIC_BASE_URL")
        .or_else(|| env.get("GEMINI_BASE_URL"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let model = env
        .get("ANTHROPIC_MODEL")
        .or_else(|| env.get("GEMINI_MODEL"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    (endpoint, model)
}

#[tauri::command]
fn load_deeplinks() -> Result<Vec<DeeplinkItem>, String> {
    let db_path = get_db_path().ok_or("无法找到用户目录")?;

    if !db_path.exists() {
        return Err(format!("未找到 CC Switch 数据库: {}", db_path.display()));
    }

    let db = Connection::open_with_flags(&db_path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
        .map_err(|e| format!("打开数据库失败: {}", e))?;

    let mut providers = read_providers(&db)?;
    let endpoints = read_endpoints(&db)?;
    let skills = read_skill_repos(&db)?;

    // Merge endpoints into providers
    let ep_map: std::collections::HashMap<String, Vec<Endpoint>> = endpoints.into_iter().fold(
        std::collections::HashMap::new(),
        |mut acc, (pid, url)| {
            acc.entry(pid)
                .or_default()
                .push(Endpoint { url });
            acc
        },
    );

    for p in &mut providers {
        if let Some(eps) = ep_map.get(&p.id) {
            p.endpoints = eps.clone();
        }
    }

    let mut items: Vec<DeeplinkItem> = Vec::new();

    for p in &providers {
        let (endpoint, model) = extract_info(p);
        items.push(DeeplinkItem {
            item_type: "provider".into(),
            name: p.name.clone(),
            app: p.app_type.clone(),
            deeplink: build_provider_deeplink(p),
            endpoint,
            model,
            is_current: p.is_current,
        });
    }

    for s in &skills {
        items.push(DeeplinkItem {
            item_type: "skill".into(),
            name: format!("{}/{}", s.owner, s.name),
            app: "skill".into(),
            deeplink: build_skill_deeplink(s),
            endpoint: String::new(),
            model: format!("branch: {}", s.branch),
            is_current: false,
        });
    }

    Ok(items)
}

#[tauri::command]
fn get_db_path_str() -> Result<String, String> {
    let db_path = get_db_path().ok_or("无法找到用户目录")?;
    Ok(db_path.to_string_lossy().to_string())
}

#[tauri::command]
fn read_raw_config() -> Result<String, String> {
    let db_path = get_db_path().ok_or("无法找到用户目录")?;
    let db = Connection::open_with_flags(&db_path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
        .map_err(|e| format!("打开数据库失败: {}", e))?;

    let mut stmt = db
        .prepare("SELECT id, app_type, name, settings_config, website_url, icon, is_current, notes FROM providers")
        .map_err(|e| e.to_string())?;

    let mut providers: Vec<serde_json::Value> = Vec::new();
    let rows = stmt
        .query_map([], |row| {
            let id: String = row.get(0)?;
            let app_type: String = row.get(1)?;
            let name: String = row.get(2)?;
            let settings_raw: Option<String> = row.get(3)?;
            let website_url: Option<String> = row.get(4)?;
            let icon: Option<String> = row.get(5)?;
            let is_current: i32 = row.get(6)?;
            let notes: Option<String> = row.get(7)?;

            let settings_config: serde_json::Value = settings_raw
                .as_ref()
                .and_then(|s| serde_json::from_str(s).ok())
                .unwrap_or(serde_json::json!({}));

            Ok(serde_json::json!({
                "id": id,
                "app_type": app_type,
                "name": name,
                "settings_config": settings_config,
                "website_url": website_url,
                "icon": icon,
                "is_current": is_current != 0,
                "notes": notes,
            }))
        })
        .map_err(|e| e.to_string())?;

    for row in rows {
        if let Ok(r) = row {
            providers.push(r);
        }
    }

    // Endpoints
    let mut ep_stmt = db
        .prepare("SELECT provider_id, url FROM provider_endpoints")
        .map_err(|e| e.to_string())?;
    let eps: Vec<serde_json::Value> = ep_stmt
        .query_map([], |row| {
            Ok(serde_json::json!({
                "provider_id": row.get::<_, String>(0)?,
                "url": row.get::<_, String>(1)?,
            }))
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    // Skills
    let mut skill_stmt = db
        .prepare("SELECT owner, name, branch FROM skill_repos")
        .map_err(|e| e.to_string())?;
    let skills: Vec<serde_json::Value> = skill_stmt
        .query_map([], |row| {
            Ok(serde_json::json!({
                "owner": row.get::<_, String>(0)?,
                "name": row.get::<_, String>(1)?,
                "branch": row.get::<_, String>(2)?,
            }))
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    serde_json::to_string_pretty(&serde_json::json!({
        "providers": providers,
        "endpoints": eps,
        "skill_repos": skills,
    }))
    .map_err(|e| e.to_string())
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            load_deeplinks,
            get_db_path_str,
            read_raw_config,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
