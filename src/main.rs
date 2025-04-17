use std::env;
use std::fs;
use std::path::PathBuf;
use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use ratatui::widgets::{ListState, List, ListItem, Paragraph, Wrap, Block, Borders};
use ratatui::style::{Color, Style};
use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};

#[derive(Debug, Serialize, Deserialize)]
pub struct McpServersConfig {
    #[serde(rename = "mcpServers")]
    pub mcp_servers: HashMap<String, McpServerConfig>,
    #[serde(rename = "environments")]
    pub environments: HashMap<String, EnvironmentConfig>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct McpServerConfig {
    pub command: String,
    pub args: Vec<String>,
    pub env: HashMap<String, String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EnvironmentConfig {
    #[serde(rename = "configPath")]
    pub config_path: String,
    pub enable: Option<Vec<String>>,
    pub preset: Option<HashMap<String, Vec<String>>>,
}

fn ensure_config() -> std::io::Result<()> {
    // XDG_CONFIG_HOME優先、なければ$HOME/.config
    let config_dir = match env::var("XDG_CONFIG_HOME") {
        Ok(val) => PathBuf::from(val).join("mcpallete"),
        Err(_) => {
            let home = env::var("HOME").expect("HOME環境変数が必要です");
            PathBuf::from(home).join(".config/mcpallete")
        }
    };
    if !config_dir.exists() {
        fs::create_dir_all(&config_dir)?;
    }
    let config_file = config_dir.join("basic_config.json");
    if !config_file.exists() {
        use std::io::Write;
        let mut file = fs::File::create(config_file)?;
        file.write_all(b"{\n  \"mcpServers\": {}\n}\n")?;
    }
    Ok(())
}

fn get_config_file_path() -> PathBuf {
    let config_dir = match env::var("XDG_CONFIG_HOME") {
        Ok(val) => PathBuf::from(val).join("mcpallete"),
        Err(_) => {
            let home = env::var("HOME").expect("HOME環境変数が必要です");
            PathBuf::from(home).join(".config/mcpallete")
        }
    };
    config_dir.join("basic_config.json")
}

fn load_config() -> Option<McpServersConfig> {
    let path = get_config_file_path();
    match std::fs::read_to_string(&path) {
        Ok(content) => serde_json::from_str::<McpServersConfig>(&content).ok(),
        Err(_) => None,
    }
}

fn read_json_pretty() -> String {
    let path = get_config_file_path();
    match std::fs::read_to_string(&path) {
        Ok(content) => {
            match serde_json::from_str::<serde_json::Value>(&content) {
                Ok(v) => serde_json::to_string_pretty(&v).unwrap_or_else(|_| content.clone()),
                Err(_) => content,
            }
        }
        Err(e) => format!("ファイル読み込みエラー: {}", e),
    }
}

fn update_env_names(config: &Option<McpServersConfig>) -> Vec<String> {
    if let Some(cfg) = config {
        cfg.environments.keys().cloned().collect::<Vec<_>>()
    } else {
        vec![]
    }
}

fn update_mcp_names(config: &Option<McpServersConfig>) -> Vec<String> {
    if let Some(cfg) = config {
        cfg.mcp_servers.keys().cloned().collect::<Vec<_>>()
    } else {
        vec![]
    }
}

fn update_preset_names(
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

enum ActiveColumn {
    Environments,
    McpServers,
    PresetList,
    PresetSubmit,
}

fn tui_main() -> Result<(), Box<dyn std::error::Error>> {
    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = ratatui::backend::CrosstermBackend::new(stdout);
    let mut terminal = ratatui::Terminal::new(backend)?;
    let mut json_text = read_json_pretty();
    let mut config = load_config();
    // 初回・Ctrl+R時のみEnvironments, MCP Servers, Presets, mcp_checkedを更新
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
        terminal.draw(|f| {
            let area = f.area();
            // 上部に1行分のスペースとキー操作説明
            let help_text = "Ctrl+R: Reload  Ctrl+S: Save  Space: Toggle/Apply  ↑↓←→: Move";
            let help_area = ratatui::layout::Rect {
                x: area.x,
                y: area.y,
                width: area.width,
                height: 1,
            };
            let help_para = Paragraph::new(help_text);
            f.render_widget(help_para, help_area);
            // 残りのUIを1行下にずらす
            let area = ratatui::layout::Rect {
                x: area.x,
                y: area.y + 1,
                width: area.width,
                height: area.height.saturating_sub(1),
            };
            let layout = ratatui::layout::Layout::default()
                .direction(ratatui::layout::Direction::Horizontal)
                .margin(1)
                .constraints([
                    ratatui::layout::Constraint::Percentage(20), // Environments
                    ratatui::layout::Constraint::Percentage(20), // McpServers
                    ratatui::layout::Constraint::Percentage(20), // PresetList+PresetSubmit
                    ratatui::layout::Constraint::Percentage(40), // JSON
                ])
                .split(area);
            // 左: 環境名リスト
            let env_items: Vec<ListItem> = env_names.iter().map(|n| ListItem::new(n.as_str())).collect();
            let env_list = List::new(env_items)
                .block(Block::default().title("Environments").borders(Borders::ALL))
                .highlight_style(Style::default().bg(match active_col { ActiveColumn::Environments => Color::Blue, _ => Color::Reset }).fg(Color::White))
                .highlight_symbol("▶ ");
            f.render_stateful_widget(env_list, layout[0], &mut env_state);
            // 中央左: MCPサーバー名リスト（チェックボックス付き）
            let mcp_items: Vec<ListItem> = mcp_names.iter().enumerate().map(|(i, n)| {
                let checked = if mcp_checked.get(i).copied().unwrap_or(false) { "[x]" } else { "[ ]" };
                ListItem::new(format!("{} {}", checked, n))
            }).collect();
            let mcp_list = List::new(mcp_items)
                .block(Block::default().title("MCP Servers").borders(Borders::ALL))
                .highlight_style(Style::default().bg(match active_col { ActiveColumn::McpServers => Color::Blue, _ => Color::Reset }).fg(Color::White))
                .highlight_symbol("▶ ");
            f.render_stateful_widget(mcp_list, layout[1], &mut mcp_state);
            // 中央右: PresetList + PresetSubmit
            let preset_chunks = ratatui::layout::Layout::default()
                .direction(ratatui::layout::Direction::Vertical)
                .constraints([
                    ratatui::layout::Constraint::Min(5), // PresetList
                    ratatui::layout::Constraint::Length(3), // PresetSubmit
                ])
                .split(layout[2]);
            // PresetList
            let preset_items: Vec<ListItem> = preset_names.iter().map(|n| ListItem::new(n.as_str())).collect();
            let preset_list = List::new(preset_items)
                .block(Block::default().title("Preset List").borders(Borders::ALL))
                .highlight_style(Style::default().bg(match active_col { ActiveColumn::PresetList => Color::Blue, _ => Color::Reset }).fg(Color::White))
                .highlight_symbol("▶ ");
            f.render_stateful_widget(preset_list, preset_chunks[0], &mut preset_state);
            // PresetSubmit
            let input_block = Block::default()
                .title("Preset Submit")
                .borders(Borders::ALL)
                .border_style(
                    if let ActiveColumn::PresetSubmit = active_col {
                        Style::default().fg(Color::Yellow)
                    } else {
                        Style::default()
                    }
                );
            let input_para = Paragraph::new(preset_input.as_str())
                .block(input_block);
            f.render_widget(input_para, preset_chunks[1]);
            if let ActiveColumn::PresetSubmit = active_col {
                f.set_cursor(preset_chunks[1].x + preset_input.len() as u16 + 1, preset_chunks[1].y + 1);
            }
            // 右: JSON
            let para = Paragraph::new(json_text.as_str())
                .block(Block::default().title("Config JSON").borders(Borders::ALL))
                .wrap(Wrap { trim: false });
            f.render_widget(para, layout[3]);
        })?;
        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('c') if key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) => break,
                    KeyCode::Char('s') if key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) => {
                        // ctrl+sでのみenableフィールドとjsonを保存・更新
                        if let (Some(cfg), Some(env_idx)) = (&mut config, env_state.selected()) {
                            if let Some(env_name) = env_names.get(env_idx) {
                                if let Some(env_cfg) = cfg.environments.get_mut(env_name) {
                                    let enabled: Vec<String> = mcp_names.iter().enumerate()
                                        .filter_map(|(i, name)| if mcp_checked.get(i).copied().unwrap_or(false) { Some(name.clone()) } else { None })
                                        .collect();
                                    env_cfg.enable = Some(enabled);
                                    if let Ok(json) = serde_json::to_string_pretty(&cfg) {
                                        let path = get_config_file_path();
                                        if std::fs::write(&path, json).is_ok() {
                                            json_text = read_json_pretty();
                                        }
                                    }
                                }
                            }
                        }
                        // 既存のプリセット保存処理
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
                                                    json_text = read_json_pretty();
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
                        json_text = read_json_pretty();
                        config = load_config();
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
                                // enableやjsonの保存・更新はここでは行わない
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
                                // enableやjsonの保存・更新はここでは行わない
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

fn main() {
    if let Err(e) = ensure_config() {
        eprintln!("設定ディレクトリ作成失敗: {}", e);
        std::process::exit(1);
    }
    // 起動時に設定ファイルを読み込む
    let config = load_config();
    if config.is_none() {
        eprintln!("設定ファイルの読み込みに失敗しました");
    }
    if let Err(e) = tui_main() {
        eprintln!("TUIエラー: {}", e);
        std::process::exit(1);
    }
}
