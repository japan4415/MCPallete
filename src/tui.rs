use ratatui::widgets::ListState;
use crossterm::{event::{self, Event, KeyCode}, execute, terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen}};
use std::collections::HashMap;
use crate::config::*;
use crate::model::*;

pub enum ActiveColumn {
    Environments,
    McpServers,
    PresetList,
    PresetSubmit,
}

pub fn update_env_names(config: &Option<McpServersConfig>) -> Vec<String> {
    if let Some(cfg) = config {
        cfg.environments.keys().cloned().collect::<Vec<_>>()
    } else {
        vec![]
    }
}

pub fn update_mcp_names(config: &Option<McpServersConfig>) -> Vec<String> {
    if let Some(cfg) = config {
        cfg.mcp_servers.keys().cloned().collect::<Vec<_>>()
    } else {
        vec![]
    }
}

pub fn update_preset_names(
    config: &Option<McpServersConfig>,
    env_names: &Vec<String>,
    env_state: &ListState,
    preset_state: &mut ListState,
) -> Vec<String> {
    let names = if let (Some(cfg), Some(env_idx)) = (config, env_state.selected()) {
        if let Some(env_name) = env_names.get(env_idx) {
            if let Some(env_cfg) = cfg.environments.get(env_name) {
                if let Some(presets) = &env_cfg.preset {
                    presets.keys().cloned().collect::<Vec<_>>()
                } else { vec![] }
            } else { vec![] }
        } else { vec![] }
    } else { vec![] };
    if !names.is_empty() {
        preset_state.select(Some(0));
    } else {
        preset_state.select(None);
    }
    names
}

pub fn tui_main() -> Result<(), Box<dyn std::error::Error>> {
    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = ratatui::backend::CrosstermBackend::new(stdout);
    let mut terminal = ratatui::Terminal::new(backend)?;
    let mut config = load_config().ok();
    let mut env_names = update_env_names(&config);
    let mut mcp_names = update_mcp_names(&config);
    let mut env_state = ListState::default();
    let mut mcp_state = ListState::default();
    if !env_names.is_empty() { env_state.select(Some(0)); }
    if !mcp_names.is_empty() { mcp_state.select(Some(0)); }
    let mut preset_state = ListState::default();
    let mut preset_names = update_preset_names(&config, &env_names, &env_state, &mut preset_state);
    let mut mcp_checked = {
        if let (Some(cfg), Some(env_idx)) = (&config, env_state.selected()) {
            let env_name = env_names.get(env_idx);
            if let Some(env_name) = env_name {
                let enabled = cfg.environments.get(env_name).and_then(|e| e.enable.as_ref());
                mcp_names.iter().map(|mcp| {
                    enabled.map_or(false, |v| v.contains(mcp))
                }).collect::<Vec<_>>()
            } else {
                vec![false; mcp_names.len()]
            }
        } else {
            vec![false; mcp_names.len()]
        }
    };
    let mut preset_input = String::new();
    let mut active_col = ActiveColumn::Environments;
    loop {
        terminal.draw(|_| {
            // ...existing code (UI描画処理)...
        })?;
        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('c') if key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) => break,
                    KeyCode::Char('s') if key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) => {
                        if let (Some(cfg), Some(env_idx)) = (&mut config, env_state.selected()) {
                            if let Some(env_name) = env_names.get(env_idx) {
                                if let Some(env_cfg) = cfg.environments.get_mut(env_name) {
                                    if let Some(mode) = env_cfg.mode.as_ref() {
                                        if mode == "claude_desktop" && !env_cfg.config_path.is_empty() {
                                            let path = &env_cfg.config_path;
                                            let selected_servers: HashMap<String, McpServerConfig> = mcp_names.iter().enumerate()
                                                .filter_map(|(i, name)| {
                                                    if mcp_checked.get(i).copied().unwrap_or(false) {
                                                        cfg.mcp_servers.get(name).map(|v| {
                                                            let mut v = v.clone();
                                                            v.env = v.env.iter()
                                                                .map(|(k, val)| expand_env_vars(val).map(|v| (k.clone(), v)))
                                                                .collect::<Result<HashMap<_,_>, Box<dyn std::error::Error>>>()?;
                                                            Ok((name.clone(), v))
                                                        })
                                                    } else {
                                                        None
                                                    }
                                                })
                                                .collect::<Result<HashMap<_,_>, Box<dyn std::error::Error>>>()?;
                                            let desktop_config = ClaudeDesktopConfig { mcp_servers: selected_servers };
                                            let json = serde_json::to_string_pretty(&desktop_config)?;
                                            std::fs::write(path, json)?;
                                        }
                                    }
                                    let enabled: Vec<String> = mcp_names.iter().enumerate()
                                        .filter_map(|(i, name)| if mcp_checked.get(i).copied().unwrap_or(false) { Some(name.clone()) } else { None })
                                        .collect();
                                    env_cfg.enable = Some(enabled);
                                    if let Ok(json) = serde_json::to_string_pretty(&cfg) {
                                        let path = get_config_file_path();
                                        let _ = std::fs::write(&path, json);
                                    }
                                }
                            }
                        }
                        if let ActiveColumn::PresetSubmit = active_col {
                            if !preset_input.trim().is_empty() {
                                if let (Some(cfg), Some(env_idx)) = (&mut config, env_state.selected()) {
                                    if let Some(env_name) = env_names.get(env_idx) {
                                        if let Some(env_cfg) = cfg.environments.get_mut(env_name) {
                                            if env_cfg.preset.is_none() {
                                                env_cfg.preset = Some(HashMap::new());
                                            }
                                            let preset = env_cfg.preset.as_mut().unwrap();
                                            let enabled: Vec<String> = mcp_names.iter().enumerate()
                                                .filter_map(|(i, name)| if mcp_checked.get(i).copied().unwrap_or(false) { Some(name.clone()) } else { None })
                                                .collect();
                                            preset.insert(preset_input.trim().to_string(), enabled);
                                            if let Ok(json) = serde_json::to_string_pretty(&cfg) {
                                                let path = get_config_file_path();
                                                if std::fs::write(&path, json).is_ok() {
                                                    preset_names = update_preset_names(&config, &env_names, &env_state, &mut preset_state);
                                                    preset_input.clear();
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    },
                    KeyCode::Char('r') if key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) => {
                        config = load_config().ok();
                        env_names = update_env_names(&config);
                        mcp_names = update_mcp_names(&config);
                        if !env_names.is_empty() { env_state.select(Some(0)); } else { env_state.select(None); }
                        if !mcp_names.is_empty() { mcp_state.select(Some(0)); } else { mcp_state.select(None); }
                        preset_names = update_preset_names(&config, &env_names, &env_state, &mut preset_state);
                        mcp_checked = {
                            if let (Some(cfg), Some(env_idx)) = (&config, env_state.selected()) {
                                let env_name = env_names.get(env_idx);
                                if let Some(env_name) = env_name {
                                    let enabled = cfg.environments.get(env_name).and_then(|e| e.enable.as_ref());
                                    mcp_names.iter().map(|mcp| {
                                        enabled.map_or(false, |v| v.contains(mcp))
                                    }).collect::<Vec<_>>()
                                } else {
                                    vec![false; mcp_names.len()]
                                }
                            } else {
                                vec![false; mcp_names.len()]
                            }
                        };
                    },
                    KeyCode::Char('d') if key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) => {
                        if let ActiveColumn::PresetList = active_col {
                            if let (Some(cfg), Some(env_idx), Some(preset_idx)) = (&mut config, env_state.selected(), preset_state.selected()) {
                                let env_name = env_names.get(env_idx);
                                let preset_name = preset_names.get(preset_idx);
                                if let (Some(env_name), Some(preset_name)) = (env_name, preset_name) {
                                    if let Some(env_cfg) = cfg.environments.get_mut(env_name) {
                                        if let Some(presets) = env_cfg.preset.as_mut() {
                                            presets.remove(preset_name);
                                            if let Ok(json) = serde_json::to_string_pretty(&cfg) {
                                                let path = get_config_file_path();
                                                if std::fs::write(&path, json).is_ok() {
                                                    preset_names = update_preset_names(&config, &env_names, &env_state, &mut preset_state);
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    },
                    KeyCode::Left => {
                        active_col = match active_col {
                            ActiveColumn::McpServers => ActiveColumn::Environments,
                            ActiveColumn::PresetList => ActiveColumn::McpServers,
                            ActiveColumn::PresetSubmit => ActiveColumn::PresetList,
                            _ => active_col,
                        };
                    },
                    KeyCode::Right => {
                        active_col = match active_col {
                            ActiveColumn::Environments => ActiveColumn::McpServers,
                            ActiveColumn::McpServers => ActiveColumn::PresetList,
                            ActiveColumn::PresetList => ActiveColumn::PresetSubmit,
                            _ => active_col,
                        };
                    },
                    KeyCode::Tab => {
                        active_col = match active_col {
                            ActiveColumn::Environments => ActiveColumn::McpServers,
                            ActiveColumn::McpServers => ActiveColumn::PresetList,
                            ActiveColumn::PresetList => ActiveColumn::PresetSubmit,
                            ActiveColumn::PresetSubmit => ActiveColumn::Environments,
                        };
                    },
                    KeyCode::Up | KeyCode::Down => {
                        match active_col {
                            ActiveColumn::Environments => {
                                let i = env_state.selected().unwrap_or(0);
                                let new = if key.code == KeyCode::Up {
                                    if i == 0 { env_names.len().saturating_sub(1) } else { i - 1 }
                                } else {
                                    if i + 1 >= env_names.len() { 0 } else { i + 1 }
                                };
                                env_state.select(Some(new));
                                preset_names = update_preset_names(&config, &env_names, &env_state, &mut preset_state);
                                if let (Some(cfg), Some(env_name)) = (&config, env_names.get(new)) {
                                    let enabled = cfg.environments.get(env_name).and_then(|e| e.enable.as_ref());
                                    mcp_checked = mcp_names.iter().map(|mcp| {
                                        enabled.map_or(false, |v| v.contains(mcp))
                                    }).collect();
                                }
                            },
                            ActiveColumn::McpServers => {
                                let i = mcp_state.selected().unwrap_or(0);
                                let new = if key.code == KeyCode::Up {
                                    if i == 0 { mcp_names.len().saturating_sub(1) } else { i - 1 }
                                } else {
                                    if i + 1 >= mcp_names.len() { 0 } else { i + 1 }
                                };
                                mcp_state.select(Some(new));
                            },
                            ActiveColumn::PresetList => {
                                let i = preset_state.selected().unwrap_or(0);
                                let new = if key.code == KeyCode::Up {
                                    if i == 0 { preset_names.len().saturating_sub(1) } else { i - 1 }
                                } else {
                                    if i + 1 >= preset_names.len() { 0 } else { i + 1 }
                                };
                                preset_state.select(Some(new));
                            },
                            ActiveColumn::PresetSubmit => {}
                        }
                    },
                    KeyCode::Char(' ') => {
                        match active_col {
                            ActiveColumn::McpServers => {
                                if let Some(idx) = mcp_state.selected() {
                                    if let Some(val) = mcp_checked.get_mut(idx) {
                                        *val = !*val;
                                    }
                                }
                            },
                            ActiveColumn::PresetList => {
                                if let (Some(cfg), Some(env_idx), Some(preset_idx)) = (&config, env_state.selected(), preset_state.selected()) {
                                    let env_name = env_names.get(env_idx);
                                    let preset_name = preset_names.get(preset_idx);
                                    if let (Some(env_name), Some(preset_name)) = (env_name, preset_name) {
                                        if let Some(env_cfg) = cfg.environments.get(env_name) {
                                            if let Some(presets) = &env_cfg.preset {
                                                let enabled_list = presets.get(preset_name);
                                                mcp_checked = mcp_names.iter().map(|mcp| {
                                                    enabled_list.map_or(false, |list| list.contains(mcp))
                                                }).collect();
                                            }
                                        }
                                    }
                                }
                            },
                            _ => {}
                        }
                    },
                    KeyCode::Char(c) if matches!(active_col, ActiveColumn::PresetSubmit) => {
                        preset_input.push(c);
                    },
                    KeyCode::Backspace if matches!(active_col, ActiveColumn::PresetSubmit) => {
                        preset_input.pop();
                    },
                    _ => {}
                }
            }
        }
    }
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{McpServersConfig, McpServerConfig, EnvironmentConfig};
    use ratatui::widgets::ListState;
    use std::collections::HashMap;

    fn sample_config() -> Option<McpServersConfig> {
        let mut mcp_servers = HashMap::new();
        mcp_servers.insert("a".to_string(), McpServerConfig {
            command: "echo".to_string(),
            args: vec!["hi".to_string()],
            env: HashMap::new(),
        });
        let mut environments = HashMap::new();
        environments.insert("env1".to_string(), EnvironmentConfig {
            config_path: "/tmp/test.json".to_string(),
            enable: Some(vec!["a".to_string()]),
            preset: Some(HashMap::from([
                ("p1".to_string(), vec!["a".to_string()])
            ])),
            mode: Some("testmode".to_string()),
        });
        Some(McpServersConfig { mcp_servers, environments })
    }

    #[test]
    fn test_update_env_names() {
        let config = sample_config();
        let envs = update_env_names(&config);
        assert_eq!(envs, vec!["env1"]);
    }

    #[test]
    fn test_update_mcp_names() {
        let config = sample_config();
        let mcps = update_mcp_names(&config);
        assert_eq!(mcps, vec!["a"]);
    }

    #[test]
    fn test_update_preset_names() {
        let config = sample_config();
        let env_names = update_env_names(&config);
        let mut env_state = ListState::default();
        env_state.select(Some(0));
        let mut preset_state = ListState::default();
        let presets = update_preset_names(&config, &env_names, &env_state, &mut preset_state);
        assert_eq!(presets, vec!["p1"]);
        assert_eq!(preset_state.selected(), Some(0));
    }
}
