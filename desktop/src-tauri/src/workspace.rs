use serde_json::{json, Value};

pub fn create_params(snapshot: &Value, pane_id: &str) -> Result<Value, String> {
    let pane = snapshot["panes"]
        .as_array()
        .and_then(|panes| panes.iter().find(|pane| pane["pane_id"] == pane_id))
        .ok_or("Selected pane no longer exists")?;
    let cwd = pane["foreground_cwd"]
        .as_str()
        .filter(|cwd| !cwd.is_empty())
        .or_else(|| pane["cwd"].as_str())
        .ok_or("Selected pane has no working directory")?;
    if !std::path::Path::new(cwd).is_absolute() {
        return Err("Selected pane directory is not absolute".into());
    }
    // Explicit cwd honors this GUI's selection; no shared server-focus side effect.
    Ok(json!({"cwd":cwd,"focus":false}))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn current_pane_directory_wins_over_server_focus_and_label_is_default() {
        let snapshot = json!({"panes":[{"pane_id":"a","focused":true,"cwd":"/other"},{"pane_id":"b","cwd":"/project","foreground_cwd":"/project/subdir"}]});
        assert_eq!(
            create_params(&snapshot, "b").unwrap(),
            json!({"cwd":"/project/subdir","focus":false})
        );
        assert!(create_params(&snapshot, "gone").is_err());
    }
    #[test]
    fn missing_or_relative_directory_does_not_silently_create_elsewhere() {
        assert!(create_params(&json!({"panes":[{"pane_id":"a"}]}), "a").is_err());
        assert!(create_params(&json!({"panes":[{"pane_id":"a","cwd":"relative"}]}), "a").is_err());
        assert_eq!(
            create_params(&json!({"panes":[{"pane_id":"a","cwd":"/project"}]}), "a").unwrap()
                ["cwd"],
            "/project"
        );
    }
}
