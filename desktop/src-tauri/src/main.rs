mod leases;
mod platform;
mod workspace;

use platform::{Connection, Frame, Runtime, SessionInfo};
use std::sync::{Arc, Mutex};
use tauri::Manager;

struct State {
    runtime: Runtime,
    connection: Mutex<leases::Leases<Arc<Mutex<Connection>>>>,
}
fn controller(state: &State, id: &str) -> Result<Arc<Mutex<Connection>>, String> {
    Ok(state
        .connection
        .lock()
        .map_err(|_| "Session lock failed")?
        .get(id)?
        .clone())
}
#[derive(serde::Serialize)]
struct Attached {
    #[serde(flatten)]
    info: SessionInfo,
    connection_id: String,
}
#[tauri::command]
async fn initialize(state: tauri::State<'_, State>) -> Result<SessionInfo, String> {
    state.runtime.initialize()
}

#[tauri::command]
async fn connect(
    state: tauri::State<'_, State>,
    cols: u16,
    rows: u16,
    terminal_id: Option<String>,
) -> Result<Attached, String> {
    platform::validate_size(cols, rows)?;
    if !state
        .connection
        .lock()
        .map_err(|_| "Session lock failed")?
        .has_capacity()
    {
        return Err("At most 32 terminal views may be attached".into());
    }
    let (next, info) = match terminal_id {
        Some(id) => state.runtime.connect_to(cols, rows, Some(id))?,
        None => state.runtime.connect(cols, rows)?,
    };
    let connection_id = state
        .connection
        .lock()
        .map_err(|_| "Session lock failed")?
        .insert(Arc::new(Mutex::new(next)))?;
    Ok(Attached {
        info,
        connection_id,
    })
}
#[tauri::command]
async fn disconnect(state: tauri::State<'_, State>, connection_id: String) -> Result<(), String> {
    let removed = state
        .connection
        .lock()
        .map_err(|_| "Session lock failed")?
        .remove(&connection_id);
    // Release the pool lock before a controller's destructor waits for its child.
    drop(removed);
    Ok(())
}
#[tauri::command]
async fn input(
    state: tauri::State<'_, State>,
    connection_id: String,
    text: String,
    paste: bool,
) -> Result<(), String> {
    if text.len() > 262144 {
        return Err("Input is too large".into());
    }
    let text = if paste {
        platform::paste_input(&text)?
    } else {
        text
    };
    let handle = controller(&state, &connection_id)?;
    let mut connection = handle.lock().map_err(|_| "Controller lock failed")?;
    connection.send(serde_json::json!({"type":"terminal.input","text":text}))
}
#[tauri::command]
async fn resize(
    state: tauri::State<'_, State>,
    connection_id: String,
    cols: u16,
    rows: u16,
) -> Result<(), String> {
    platform::validate_size(cols, rows)?;
    let handle = controller(&state, &connection_id)?;
    let mut connection = handle.lock().map_err(|_| "Controller lock failed")?;
    connection.send(serde_json::json!({"type":"terminal.resize","cols":cols,"rows":rows}))
}
#[tauri::command]
async fn scroll(
    state: tauri::State<'_, State>,
    direction: String,
    lines: u16,
    connection_id: String,
) -> Result<(), String> {
    if !matches!(direction.as_str(), "up" | "down") || !(1..=100).contains(&lines) {
        return Err("Invalid scroll request".into());
    }
    let handle = controller(&state, &connection_id)?;
    let mut connection = handle.lock().map_err(|_| "Controller lock failed")?;
    connection
        .send(serde_json::json!({"type":"terminal.scroll","direction":direction,"lines":lines}))
}
#[tauri::command]
async fn read_frame(
    state: tauri::State<'_, State>,
    connection_id: String,
) -> Result<Option<Frame>, String> {
    let handle = controller(&state, &connection_id)?;
    let mut connection = handle.lock().map_err(|_| "Controller lock failed")?;
    connection.read_frame()
}
#[tauri::command]
async fn navigation(state: tauri::State<'_, State>) -> Result<serde_json::Value, String> {
    state.runtime.navigation()
}
#[tauri::command]
async fn create_space(
    state: tauri::State<'_, State>,
    pane_id: String,
) -> Result<serde_json::Value, String> {
    let params = workspace::create_params(&state.runtime.navigation()?, &pane_id)?;
    state.runtime.request("workspace.create", params)
}
#[tauri::command]
async fn create_tab(
    state: tauri::State<'_, State>,
    workspace_id: String,
    pane_id: String,
) -> Result<serde_json::Value, String> {
    if workspace_id.is_empty() || workspace_id.len() > 128 {
        return Err("Invalid Space".into());
    }
    let snapshot = state.runtime.navigation()?;
    let pane = snapshot["panes"]
        .as_array()
        .and_then(|panes| {
            panes
                .iter()
                .find(|pane| pane["pane_id"] == pane_id && pane["workspace_id"] == workspace_id)
        })
        .ok_or("Selected pane no longer exists")?;
    let cwd = pane["foreground_cwd"]
        .as_str()
        .or_else(|| pane["cwd"].as_str())
        .ok_or("Selected pane has no working directory")?;
    state.runtime.request(
        "tab.create",
        serde_json::json!({"workspace_id":workspace_id,"cwd":cwd,"focus":false}),
    )
}
#[tauri::command]
async fn split_pane(
    state: tauri::State<'_, State>,
    pane_id: String,
    direction: String,
) -> Result<serde_json::Value, String> {
    if !matches!(direction.as_str(), "right" | "down") {
        return Err("Invalid split direction".into());
    }
    let snapshot = state.runtime.navigation()?;
    let pane = snapshot["panes"]
        .as_array()
        .and_then(|panes| panes.iter().find(|pane| pane["pane_id"] == pane_id))
        .ok_or("Selected pane no longer exists")?;
    let cwd = pane["foreground_cwd"]
        .as_str()
        .or_else(|| pane["cwd"].as_str())
        .ok_or("Selected pane has no directory")?;
    state.runtime.request("pane.split", serde_json::json!({"target_pane_id":pane_id,"direction":direction,"ratio":0.5,"cwd":cwd,"focus":false}))
}
#[tauri::command]
async fn pane_action(
    state: tauri::State<'_, State>,
    pane_id: String,
    action: String,
    direction: String,
    target_pane_id: Option<String>,
) -> Result<serde_json::Value, String> {
    if !matches!(direction.as_str(), "left" | "right" | "up" | "down") {
        return Err("Invalid direction".into());
    }
    let snapshot = state.runtime.navigation()?;
    let panes = snapshot["panes"].as_array().ok_or("Missing panes")?;
    let source = panes
        .iter()
        .find(|pane| pane["pane_id"] == pane_id)
        .ok_or("Selected pane no longer exists")?;
    match action.as_str() {
        "resize" => state.runtime.request(
            "pane.resize",
            serde_json::json!({"pane_id":pane_id,"direction":direction,"amount":0.05}),
        ),
        "move" => {
            let target = target_pane_id.ok_or("Missing target pane")?;
            if target == pane_id
                || !panes
                    .iter()
                    .any(|pane| pane["pane_id"] == target && pane["tab_id"] == source["tab_id"])
            {
                return Err("Move requires a different pane in the same tab".into());
            }
            state.runtime.request("pane.reposition",serde_json::json!({"pane_id":pane_id,"target_pane_id":target,"direction":direction}))
        }
        _ => Err("Unsupported pane action".into()),
    }
}
fn main() {
    let result = tauri::Builder::default()
        .setup(|app| {
            let root = app.path().app_data_dir()?;
            let binary = app.path().resource_dir()?.join("runtime/herdr");
            let runtime = Runtime::new(root, binary)?;
            app.manage(State {
                runtime,
                connection: Mutex::new(leases::Leases::default()),
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            connect,
            initialize,
            disconnect,
            input,
            resize,
            scroll,
            read_frame,
            navigation,
            create_space,
            create_tab,
            split_pane,
            pane_action
        ])
        .build(tauri::generate_context!());
    match result {
        Ok(app) => app.run(|app, event| {
            if matches!(event, tauri::RunEvent::Exit) {
                if let Some(state) = app.try_state::<State>() {
                    if let Ok(mut connection) = state.connection.try_lock() {
                        connection.clear();
                    }
                }
            }
        }),
        Err(error) => eprintln!("hi3 could not start: {error}"),
    }
}
