use color_eyre::Result;
use crossterm::event::{Event, EventStream, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use futures::{FutureExt, StreamExt};
use ratatui::{
    DefaultTerminal, Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap},
};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    let terminal = ratatui::init();
    let result = App::new().await?.run(terminal).await;
    ratatui::restore();
    result
}

// Modern color theme inspired by OneHalfDark
pub struct Theme;

impl Theme {
    pub const BACKGROUND: Color = Color::Rgb(40, 44, 52);
    pub const FOREGROUND: Color = Color::Rgb(220, 223, 228);
    pub const ACCENT_PRIMARY: Color = Color::Rgb(97, 175, 239); // Blue
    pub const ACCENT_SECONDARY: Color = Color::Rgb(152, 195, 121); // Green
    pub const ACCENT_WARNING: Color = Color::Rgb(229, 192, 123); // Yellow
    pub const ACCENT_ERROR: Color = Color::Rgb(224, 108, 117); // Red
    pub const ACCENT_MUTED: Color = Color::Rgb(86, 182, 194); // Cyan
    pub const SUBTLE: Color = Color::Rgb(92, 99, 112); // Gray
    pub const BORDER_FOCUSED: Color = Color::Rgb(97, 175, 239); // Blue
    pub const BORDER_UNFOCUSED: Color = Color::Rgb(92, 99, 112); // Gray
    pub const HIGHLIGHT: Color = Color::Rgb(61, 70, 87); // Selection background
    pub const TEXT_DIM: Color = Color::Rgb(145, 148, 158); // Dimmed text
}

#[derive(Debug)]
pub struct App {
    running: bool,
    event_stream: EventStream,
    user_agents: Vec<LaunchAgent>,
    global_agents: Vec<LaunchAgent>,
    apple_agents: Vec<LaunchAgent>,
    current_tab: TabLocation,
    list_state: ListState,
    selected_plist: Option<PlistData>,
    user_agents_dir: PathBuf,
    global_agents_dir: PathBuf,
    apple_agents_dir: PathBuf,
    focus: Focus,
    current_field: FormField,
    editing: bool,
    editing_field: Option<FormField>,
    edit_buffer: String,
    status_message: String,
    status_timer: u32,
    filter_text: String,
    showing_exit_confirmation: bool,
}

#[derive(Debug, Clone)]
pub struct LaunchAgent {
    filename: String,
    label: Option<String>,
    status: AgentStatus,
    enabled: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AgentStatus {
    Running,
    Stopped,
    Error,
    Unknown,
}

#[derive(Debug, Clone, PartialEq)]
enum Focus {
    Search,
    Sidebar,
    Form,
}

#[derive(Debug, Clone, PartialEq)]
enum TabLocation {
    User,
    Global,
    Apple,
}

impl TabLocation {
    fn get_directory(&self) -> Result<PathBuf> {
        match self {
            TabLocation::User => {
                let home_dir = dirs::home_dir()
                    .ok_or_else(|| color_eyre::eyre::eyre!("Could not find home directory"))?;
                Ok(home_dir.join("Library").join("LaunchAgents"))
            }
            TabLocation::Global => Ok(PathBuf::from("/Library/LaunchAgents")),
            TabLocation::Apple => Ok(PathBuf::from("/System/Library/LaunchAgents")),
        }
    }

    fn get_display_name(&self) -> &str {
        match self {
            TabLocation::User => "👤 User",
            TabLocation::Global => "🌐 Global",
            TabLocation::Apple => "🍎 Apple",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PlistData {
    #[serde(rename = "Label")]
    pub label: Option<String>,
    #[serde(rename = "ProgramArguments")]
    pub program_arguments: Option<Vec<String>>,
    #[serde(rename = "StartInterval")]
    pub start_interval: Option<i32>,
    #[serde(rename = "RunAtLoad")]
    pub run_at_load: Option<bool>,
    #[serde(rename = "KeepAlive")]
    pub keep_alive: Option<bool>,
    #[serde(rename = "StandardOutPath")]
    pub standard_out_path: Option<String>,
    #[serde(rename = "StandardErrorPath")]
    pub standard_error_path: Option<String>,
    #[serde(rename = "WorkingDirectory")]
    pub working_directory: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
enum FormField {
    Label,
    ProgramArguments,
    StartInterval,
    RunAtLoad,
    KeepAlive,
    StandardOutPath,
    StandardErrorPath,
    WorkingDirectory,
}

impl App {
    pub async fn new() -> Result<Self> {
        let user_agents_dir = TabLocation::User.get_directory()?;
        let global_agents_dir = TabLocation::Global.get_directory()?;
        let apple_agents_dir = TabLocation::Apple.get_directory()?;

        let user_agents = Self::load_launch_agents(&user_agents_dir)?;
        let global_agents = Self::load_launch_agents(&global_agents_dir)?;
        let apple_agents = Self::load_launch_agents(&apple_agents_dir)?;

        let mut list_state = ListState::default();
        if !user_agents.is_empty() {
            list_state.select(Some(0));
        }

        Ok(Self {
            running: false,
            event_stream: EventStream::new(),
            user_agents,
            global_agents,
            apple_agents,
            current_tab: TabLocation::User,
            list_state,
            selected_plist: None,
            user_agents_dir,
            global_agents_dir,
            apple_agents_dir,
            focus: Focus::Sidebar,
            current_field: FormField::Label,
            editing: false,
            editing_field: None,
            edit_buffer: String::new(),
            status_message: String::new(),
            status_timer: 0,
            filter_text: String::new(),
            showing_exit_confirmation: false,
        })
    }

    fn get_current_agents(&self) -> &Vec<LaunchAgent> {
        match self.current_tab {
            TabLocation::User => &self.user_agents,
            TabLocation::Global => &self.global_agents,
            TabLocation::Apple => &self.apple_agents,
        }
    }

    fn get_current_agents_mut(&mut self) -> &mut Vec<LaunchAgent> {
        match self.current_tab {
            TabLocation::User => &mut self.user_agents,
            TabLocation::Global => &mut self.global_agents,
            TabLocation::Apple => &mut self.apple_agents,
        }
    }

    fn get_current_directory(&self) -> &PathBuf {
        match self.current_tab {
            TabLocation::User => &self.user_agents_dir,
            TabLocation::Global => &self.global_agents_dir,
            TabLocation::Apple => &self.apple_agents_dir,
        }
    }

    fn load_launch_agents(dir: &PathBuf) -> Result<Vec<LaunchAgent>> {
        let mut agents = Vec::new();

        if dir.exists() {
            for entry in fs::read_dir(dir)? {
                let entry = entry?;
                let path = entry.path();

                if path.is_file()
                    && path.extension().is_some_and(|ext| ext == "plist")
                    && let Some(filename) = path.file_name().and_then(|n| n.to_str())
                {
                    let label = Self::extract_label_from_file(&path)
                        .unwrap_or_else(|| filename.replace(".plist", ""));

                    let status = Self::check_agent_status(&label);
                    let enabled = Self::check_agent_enabled(&label);

                    agents.push(LaunchAgent {
                        filename: filename.to_string(),
                        label: Some(label),
                        status,
                        enabled,
                    });
                }
            }
        }

        agents.sort_by(|a, b| a.filename.cmp(&b.filename));
        Ok(agents)
    }

    fn extract_label_from_file(path: &PathBuf) -> Option<String> {
        fs::read_to_string(path)
            .ok()
            .and_then(|content| parse_plist_xml(&content).ok())
            .map(|plist| plist.label)?
    }

    fn check_agent_status(label: &str) -> AgentStatus {
        // Check if agent is running using launchctl
        let uid = std::process::Command::new("id")
            .arg("-u")
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
            .unwrap_or_else(|_| "501".to_string());

        if let Ok(output) = std::process::Command::new("launchctl")
            .args(["print", &format!("gui/{}/{}", uid, label)])
            .output()
        {
            let output_str = String::from_utf8_lossy(&output.stdout);
            match output_str.trim() {
                "No such service" => return AgentStatus::Stopped,
                _ if output_str.contains("state = running") => return AgentStatus::Running,
                _ if output_str.contains("state = stopped") => return AgentStatus::Stopped,
                _ => return AgentStatus::Error,
            }
        }
        AgentStatus::Unknown
    }

    fn check_agent_enabled(label: &str) -> bool {
        let uid = std::process::Command::new("id")
            .arg("-u")
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
            .unwrap_or_else(|_| "501".to_string());

        // Check if agent is enabled/loaded
        if let Ok(output) = std::process::Command::new("launchctl")
            .args(["print-disabled", &format!("gui/{}", uid)])
            .output()
        {
            let output_str = String::from_utf8_lossy(&output.stdout);
            !output_str.contains(&format!("\"{}\": false", label))
        } else {
            // If launchctl command fails, assume it's not enabled
            false
        }
    }

    fn load_selected_plist(&mut self) -> Result<()> {
        if let Some(selected) = self.list_state.selected() {
            let filtered_agents = self.get_filtered_agents();
            if let Some(agent) = filtered_agents.get(selected) {
                let file_path = self.get_current_directory().join(&agent.filename);
                let content = fs::read_to_string(file_path)?;

                let plist_data = self.parse_plist(&content)?;
                self.selected_plist = Some(plist_data);
            }
        }
        Ok(())
    }

    pub fn parse_plist(&self, content: &str) -> Result<PlistData> {
        parse_plist_xml(content)
    }

    pub async fn run(mut self, mut terminal: DefaultTerminal) -> Result<()> {
        self.running = true;
        while self.running {
            terminal.draw(|frame| self.draw(frame))?;
            self.handle_crossterm_events().await?;
        }
        Ok(())
    }

    fn draw(&mut self, frame: &mut Frame) {
        // Clear background with theme color
        let background = Block::default().style(Style::default().bg(Theme::BACKGROUND));
        frame.render_widget(background, frame.area());

        let main_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Tab bar
                Constraint::Length(3), // Search bar
                Constraint::Min(5),    // Main content (minimum height)
                Constraint::Length(3), // Status bar
            ])
            .margin(1) // Add margin around the entire layout
            .split(frame.area());

        let content_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(35), Constraint::Percentage(65)])
            .spacing(1) // Add space between panels
            .split(main_chunks[2]);

        self.draw_tab_bar(frame, main_chunks[0]);
        self.draw_search_bar(frame, main_chunks[1]);
        self.draw_sidebar(frame, content_chunks[0]);
        self.draw_main_panel(frame, content_chunks[1]);
        self.draw_status_bar(frame, main_chunks[3]);

        // Draw exit confirmation dialog if showing
        if self.showing_exit_confirmation {
            self.draw_exit_confirmation(frame);
        }
    }

    fn draw_tab_bar(&mut self, frame: &mut Frame, area: Rect) {
        let tabs = [TabLocation::User, TabLocation::Global, TabLocation::Apple];
        let tab_width = area.width / 3;

        let tab_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(tab_width),
                Constraint::Length(tab_width),
                Constraint::Length(tab_width),
            ])
            .split(area);

        for (i, tab) in tabs.iter().enumerate() {
            let is_active = *tab == self.current_tab;
            let agent_count = match tab {
                TabLocation::User => self.user_agents.len(),
                TabLocation::Global => self.global_agents.len(),
                TabLocation::Apple => self.apple_agents.len(),
            };

            let (border_style, title_style, bg_style) = if is_active {
                (
                    Style::default().fg(Theme::BORDER_FOCUSED),
                    Style::default()
                        .fg(Theme::ACCENT_PRIMARY)
                        .add_modifier(Modifier::BOLD),
                    Style::default().bg(Theme::HIGHLIGHT),
                )
            } else {
                (
                    Style::default().fg(Theme::BORDER_UNFOCUSED),
                    Style::default().fg(Theme::TEXT_DIM),
                    Style::default().bg(Theme::BACKGROUND),
                )
            };

            let title = format!("{} ({})", tab.get_display_name(), agent_count);
            let hint = format!("[{}]", i + 1);

            let tab_content = vec![Line::from(vec![
                Span::styled(hint, Style::default().fg(Theme::ACCENT_MUTED)),
                Span::raw(" "),
                Span::styled(title, title_style),
            ])];

            let tab_widget = Paragraph::new(tab_content)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_type(BorderType::Rounded)
                        .border_style(border_style)
                        .style(bg_style),
                )
                .alignment(ratatui::layout::Alignment::Center);

            frame.render_widget(tab_widget, tab_chunks[i]);
        }
    }

    fn draw_search_bar(&mut self, frame: &mut Frame, area: Rect) {
        let search_text = if self.focus == Focus::Search {
            if self.filter_text.is_empty() {
                "│".to_string()
            } else {
                format!("{}│", self.filter_text)
            }
        } else if self.filter_text.is_empty() {
            "Type to filter agents...".to_string()
        } else {
            self.filter_text.clone()
        };

        let (search_style, border_style, title_style) = if self.focus == Focus::Search {
            (
                Style::default()
                    .fg(Theme::BACKGROUND)
                    .bg(Theme::ACCENT_PRIMARY),
                Style::default().fg(Theme::BORDER_FOCUSED),
                Style::default()
                    .fg(Theme::ACCENT_PRIMARY)
                    .add_modifier(Modifier::BOLD),
            )
        } else {
            (
                Style::default().fg(Theme::FOREGROUND).bg(Theme::BACKGROUND),
                Style::default().fg(Theme::BORDER_UNFOCUSED),
                Style::default().fg(Theme::TEXT_DIM),
            )
        };

        let title = if self.filter_text.is_empty() {
            "🔍 Search"
        } else {
            "🔍 Filtering"
        };

        let search_widget = Paragraph::new(search_text)
            .block(
                Block::default()
                    .title(Line::from(vec![Span::styled(title, title_style)]))
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(border_style)
                    .style(Style::default().bg(Theme::BACKGROUND)),
            )
            .style(search_style);

        frame.render_widget(search_widget, area);
    }

    fn get_filtered_agents(&self) -> Vec<&LaunchAgent> {
        let current_agents = self.get_current_agents();
        if self.filter_text.is_empty() {
            current_agents.iter().collect()
        } else {
            current_agents
                .iter()
                .filter(|agent| {
                    let search_text = self.filter_text.to_lowercase();
                    agent.filename.to_lowercase().contains(&search_text)
                        || agent
                            .label
                            .as_ref()
                            .map(|label| label.to_lowercase().contains(&search_text))
                            .unwrap_or(false)
                })
                .collect()
        }
    }

    fn draw_sidebar(&mut self, frame: &mut Frame, area: Rect) {
        let filtered_agents: Vec<LaunchAgent> =
            self.get_filtered_agents().into_iter().cloned().collect();
        let items: Vec<ListItem> = filtered_agents
            .iter()
            .map(|agent| {
                let (status_icon, status_color) = match agent.status {
                    AgentStatus::Running => ("●", Theme::ACCENT_SECONDARY),
                    AgentStatus::Stopped => ("●", Theme::ACCENT_ERROR),
                    AgentStatus::Error => ("✗", Theme::ACCENT_ERROR),
                    AgentStatus::Unknown => ("?", Theme::SUBTLE),
                };

                let (enabled_icon, enabled_color) = if agent.enabled {
                    ("◉", Theme::ACCENT_MUTED)
                } else {
                    ("○", Theme::SUBTLE)
                };

                let label = agent.label.as_deref().unwrap_or(&agent.filename);
                let display_name = if label.len() > 35 {
                    format!("{}...", &label[..32])
                } else {
                    label.to_string()
                };

                ListItem::new(Line::from(vec![
                    Span::styled(
                        status_icon,
                        Style::default()
                            .fg(status_color)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::raw(" "),
                    Span::styled(enabled_icon, Style::default().fg(enabled_color)),
                    Span::raw("  "),
                    Span::styled(display_name, Style::default().fg(Theme::FOREGROUND)),
                ]))
            })
            .collect();

        let (border_style, title_style) = if self.focus == Focus::Sidebar {
            (
                Style::default().fg(Theme::BORDER_FOCUSED),
                Style::default()
                    .fg(Theme::ACCENT_PRIMARY)
                    .add_modifier(Modifier::BOLD),
            )
        } else {
            (
                Style::default().fg(Theme::BORDER_UNFOCUSED),
                Style::default().fg(Theme::TEXT_DIM),
            )
        };

        let current_agents_count = self.get_current_agents().len();
        let title = if self.filter_text.is_empty() {
            format!("📋 Agents ({})", current_agents_count)
        } else {
            format!(
                "📋 Agents ({}/{})",
                filtered_agents.len(),
                current_agents_count
            )
        };

        let list = List::new(items)
            .block(
                Block::default()
                    .title(Line::from(vec![Span::styled(title, title_style)]))
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(border_style)
                    .style(Style::default().bg(Theme::BACKGROUND)),
            )
            .highlight_style(
                Style::default()
                    .bg(Theme::HIGHLIGHT)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("▶ ");

        frame.render_stateful_widget(list, area, &mut self.list_state);
    }

    fn draw_main_panel(&mut self, frame: &mut Frame, area: Rect) {
        let (border_style, title_style) = if self.focus == Focus::Form {
            (
                Style::default().fg(Theme::BORDER_FOCUSED),
                Style::default()
                    .fg(Theme::ACCENT_PRIMARY)
                    .add_modifier(Modifier::BOLD),
            )
        } else {
            (
                Style::default().fg(Theme::BORDER_UNFOCUSED),
                Style::default().fg(Theme::TEXT_DIM),
            )
        };

        if let Some(plist) = &self.selected_plist {
            let mut text = Vec::new();

            let start_interval_str = plist
                .start_interval
                .map(|i| i.to_string())
                .unwrap_or_default();
            let run_at_load_str = if plist.run_at_load.unwrap_or(false) {
                "true"
            } else {
                "false"
            };
            let keep_alive_str = if plist.keep_alive.unwrap_or(false) {
                "true"
            } else {
                "false"
            };

            let fields = vec![
                (
                    FormField::Label,
                    "🏷️  Label",
                    plist.label.as_deref().unwrap_or(""),
                ),
                (
                    FormField::StartInterval,
                    "⏰ Start Interval",
                    &start_interval_str,
                ),
                (FormField::RunAtLoad, "🚀 Run At Load", run_at_load_str),
                (FormField::KeepAlive, "💓 Keep Alive", keep_alive_str),
                (
                    FormField::StandardOutPath,
                    "📄 Stdout Path",
                    plist.standard_out_path.as_deref().unwrap_or(""),
                ),
                (
                    FormField::StandardErrorPath,
                    "📄 Stderr Path",
                    plist.standard_error_path.as_deref().unwrap_or(""),
                ),
                (
                    FormField::WorkingDirectory,
                    "📁 Working Directory",
                    plist.working_directory.as_deref().unwrap_or(""),
                ),
            ];

            for (i, (field, label, value)) in fields.iter().enumerate() {
                let is_current = self.focus == Focus::Form && self.current_field == *field;
                let is_editing = self.editing && self.editing_field.as_ref() == Some(field);

                let (label_style, value_style) = if is_editing {
                    (
                        Style::default()
                            .fg(Theme::ACCENT_WARNING)
                            .add_modifier(Modifier::BOLD),
                        Style::default()
                            .fg(Theme::BACKGROUND)
                            .bg(Theme::ACCENT_WARNING)
                            .add_modifier(Modifier::BOLD),
                    )
                } else if is_current {
                    (
                        Style::default()
                            .fg(Theme::ACCENT_PRIMARY)
                            .add_modifier(Modifier::BOLD),
                        Style::default()
                            .fg(Theme::ACCENT_PRIMARY)
                            .bg(Theme::HIGHLIGHT)
                            .add_modifier(Modifier::BOLD),
                    )
                } else {
                    (
                        Style::default()
                            .fg(Theme::ACCENT_MUTED)
                            .add_modifier(Modifier::BOLD),
                        Style::default().fg(Theme::FOREGROUND),
                    )
                };

                let display_value = if is_editing {
                    format!("{}│", &self.edit_buffer)
                } else {
                    value.to_string()
                };

                // Add some spacing between fields
                if i > 0 {
                    text.push(Line::from(""));
                }

                text.push(Line::from(vec![
                    Span::styled(format!("{:<20}", label), label_style),
                    Span::styled(display_value, value_style),
                ]));
            }

            text.push(Line::from(""));
            text.push(Line::from(""));

            if let Some(args) = &plist.program_arguments {
                let is_current =
                    self.focus == Focus::Form && self.current_field == FormField::ProgramArguments;
                let is_editing = self.editing
                    && self.editing_field.as_ref() == Some(&FormField::ProgramArguments);

                let label_style = if is_current {
                    Style::default()
                        .fg(Theme::ACCENT_PRIMARY)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                        .fg(Theme::ACCENT_MUTED)
                        .add_modifier(Modifier::BOLD)
                };

                text.push(Line::from(vec![Span::styled(
                    "⚙️  Program Arguments:",
                    label_style,
                )]));
                text.push(Line::from(""));

                for (i, arg) in args.iter().enumerate() {
                    let arg_style = if is_editing {
                        Style::default()
                            .fg(Theme::BACKGROUND)
                            .bg(Theme::ACCENT_WARNING)
                    } else if is_current {
                        Style::default()
                            .fg(Theme::ACCENT_PRIMARY)
                            .bg(Theme::HIGHLIGHT)
                    } else {
                        Style::default().fg(Theme::FOREGROUND)
                    };
                    text.push(Line::from(vec![
                        Span::raw("    "),
                        Span::styled(format!("[{}] ", i), Style::default().fg(Theme::TEXT_DIM)),
                        Span::styled(arg, arg_style),
                    ]));
                }
            }

            let paragraph = Paragraph::new(text)
                .block(
                    Block::default()
                        .title(Line::from(vec![Span::styled(
                            "⚙️  Agent Editor",
                            title_style,
                        )]))
                        .borders(Borders::ALL)
                        .border_type(BorderType::Rounded)
                        .border_style(border_style)
                        .style(Style::default().bg(Theme::BACKGROUND))
                        .padding(ratatui::widgets::Padding::uniform(1)),
                )
                .wrap(Wrap { trim: true });

            frame.render_widget(paragraph, area);
        } else {
            let paragraph = Paragraph::new(Line::from(vec![
                Span::styled("📝 ", Style::default().fg(Theme::TEXT_DIM)),
                Span::styled(
                    "Select an agent from the sidebar to view and edit its configuration",
                    Style::default()
                        .fg(Theme::TEXT_DIM)
                        .add_modifier(Modifier::ITALIC),
                ),
            ]))
            .block(
                Block::default()
                    .title(Line::from(vec![Span::styled(
                        "⚙️  Agent Editor",
                        title_style,
                    )]))
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(border_style)
                    .style(Style::default().bg(Theme::BACKGROUND)),
            )
            .style(Style::default().fg(Theme::TEXT_DIM))
            .alignment(ratatui::layout::Alignment::Center)
            .wrap(Wrap { trim: true });

            frame.render_widget(paragraph, area);
        }
    }

    fn draw_status_bar(&mut self, frame: &mut Frame, area: Rect) {
        // Update status timer
        if self.status_timer > 0 {
            self.status_timer -= 1;
            if self.status_timer == 0 {
                self.status_message.clear();
            }
        }

        let (status_text, status_style, icon) = if !self.status_message.is_empty() {
            let icon = if self.status_message.contains('✓') {
                "✅"
            } else if self.status_message.contains('✗') {
                "❌"
            } else {
                "ℹ️"
            };
            (
                self.status_message.clone(),
                Style::default()
                    .fg(Theme::ACCENT_SECONDARY)
                    .add_modifier(Modifier::BOLD),
                icon,
            )
        } else if self.editing {
            (
                format!(
                    "EDITING: {} | Enter=Save, Esc=Cancel",
                    self.get_editing_field_name()
                ),
                Style::default()
                    .fg(Theme::ACCENT_WARNING)
                    .add_modifier(Modifier::BOLD),
                "✏️",
            )
        } else {
            let (text, icon) = match self.focus {
                Focus::Search => (
                    "Type to filter agents | Enter=Focus Sidebar, Tab=Next Panel, 1/2/3=Switch Tabs".to_string(),
                    "🔍",
                ),
                Focus::Sidebar => (
                    "●=Running ○=Stopped ◉=Enabled | j/k=Navigate, Enter=Load, /=Search, 1/2/3=Switch Tabs"
                        .to_string(),
                    "📋",
                ),
                Focus::Form => (
                    "j/k=Navigate Fields, Enter=Edit, Ctrl+S=Save | Tab=Switch Panel, 1/2/3=Switch Tabs".to_string(),
                    "⚙️",
                ),
            };
            (text, Style::default().fg(Theme::ACCENT_MUTED), icon)
        };

        let status_line = Line::from(vec![
            Span::styled(
                format!("{} ", icon),
                Style::default().fg(Theme::ACCENT_PRIMARY),
            ),
            Span::styled(status_text, status_style),
        ]);

        let status_paragraph = Paragraph::new(vec![status_line])
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(Theme::BORDER_UNFOCUSED))
                    .style(Style::default().bg(Theme::BACKGROUND)),
            )
            .style(Style::default().bg(Theme::BACKGROUND));

        frame.render_widget(status_paragraph, area);
    }

    fn draw_exit_confirmation(&mut self, frame: &mut Frame) {
        let area = frame.area();

        // Create a centered popup area
        let popup_area = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(35),
                Constraint::Length(9),
                Constraint::Percentage(35),
            ])
            .split(area)[1];

        let popup_area = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(20),
                Constraint::Percentage(60),
                Constraint::Percentage(20),
            ])
            .split(popup_area)[1];

        // Clear the background
        frame.render_widget(Clear, popup_area);

        // Create the confirmation dialog
        let confirmation_text = vec![
            Line::from(""),
            Line::from(vec![Span::styled(
                "  🚪 Quit LaunchAgent Manager?",
                Style::default()
                    .fg(Theme::ACCENT_WARNING)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from(""),
            Line::from(vec![
                Span::styled("  ", Style::default()),
                Span::styled(
                    "[Y]",
                    Style::default()
                        .fg(Theme::ACCENT_SECONDARY)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled("es  ", Style::default().fg(Theme::FOREGROUND)),
                Span::styled(
                    "[N]",
                    Style::default()
                        .fg(Theme::ACCENT_ERROR)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled("o  ", Style::default().fg(Theme::FOREGROUND)),
                Span::styled(
                    "[Esc]",
                    Style::default()
                        .fg(Theme::ACCENT_MUTED)
                        .add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(""),
            Line::from(vec![Span::styled(
                "  Press any key to choose",
                Style::default()
                    .fg(Theme::TEXT_DIM)
                    .add_modifier(Modifier::ITALIC),
            )]),
            Line::from(""),
        ];

        let confirmation_dialog = Paragraph::new(confirmation_text)
            .block(
                Block::default()
                    .title(Line::from(vec![Span::styled(
                        " ⚠️  Confirm Exit ",
                        Style::default()
                            .fg(Theme::ACCENT_WARNING)
                            .add_modifier(Modifier::BOLD),
                    )]))
                    .borders(Borders::ALL)
                    .border_type(BorderType::Thick)
                    .border_style(Style::default().fg(Theme::ACCENT_WARNING))
                    .style(Style::default().bg(Theme::BACKGROUND)),
            )
            .style(Style::default().bg(Theme::BACKGROUND))
            .alignment(ratatui::layout::Alignment::Left);

        frame.render_widget(confirmation_dialog, popup_area);
    }

    #[allow(dead_code)]
    fn get_current_field_name(&self) -> &str {
        match self.current_field {
            FormField::Label => "Label",
            FormField::ProgramArguments => "Program Arguments",
            FormField::StartInterval => "Start Interval",
            FormField::RunAtLoad => "Run At Load",
            FormField::KeepAlive => "Keep Alive",
            FormField::StandardOutPath => "Standard Out Path",
            FormField::StandardErrorPath => "Standard Error Path",
            FormField::WorkingDirectory => "Working Directory",
        }
    }

    fn get_editing_field_name(&self) -> &str {
        if let Some(editing_field) = &self.editing_field {
            match editing_field {
                FormField::Label => "Label",
                FormField::ProgramArguments => "Program Arguments",
                FormField::StartInterval => "Start Interval",
                FormField::RunAtLoad => "Run At Load",
                FormField::KeepAlive => "Keep Alive",
                FormField::StandardOutPath => "Standard Out Path",
                FormField::StandardErrorPath => "Standard Error Path",
                FormField::WorkingDirectory => "Working Directory",
            }
        } else {
            "Unknown"
        }
    }

    fn set_status_message(&mut self, message: String) {
        self.status_message = message;
        self.status_timer = 100; // Show for ~2 seconds at 50ms update rate
    }

    async fn handle_crossterm_events(&mut self) -> Result<()> {
        tokio::select! {
            event = self.event_stream.next().fuse() => {
                if let Some(Ok(evt)) = event {
                    match evt {
                        Event::Key(key) if key.kind == KeyEventKind::Press => {
                            self.on_key_event(key)?;
                        }
                        Event::Mouse(_) => {}
                        Event::Resize(_, _) => {}
                        _ => {}
                    }
                }
            }
            _ = tokio::time::sleep(tokio::time::Duration::from_millis(50)) => {}
        }
        Ok(())
    }

    fn on_key_event(&mut self, key: KeyEvent) -> Result<()> {
        if self.showing_exit_confirmation {
            self.handle_exit_confirmation_keys(key)?;
        } else if self.editing {
            self.handle_edit_keys(key)?;
        } else {
            match (key.modifiers, key.code) {
                (_, KeyCode::Esc | KeyCode::Char('q')) => {
                    self.showing_exit_confirmation = true;
                }
                (KeyModifiers::CONTROL, KeyCode::Char('c') | KeyCode::Char('C')) => {
                    self.showing_exit_confirmation = true;
                }
                (_, KeyCode::Tab) => {
                    self.focus = match self.focus {
                        Focus::Search => Focus::Sidebar,
                        Focus::Sidebar => Focus::Form,
                        Focus::Form => Focus::Search,
                    };
                }
                (KeyModifiers::CONTROL, KeyCode::Char('s') | KeyCode::Char('S')) => {
                    self.save_plist()?;
                }
                (_, KeyCode::Char('/')) => {
                    self.focus = Focus::Search;
                }
                (_, KeyCode::Char('1')) => {
                    self.switch_to_tab(TabLocation::User);
                }
                (_, KeyCode::Char('2')) => {
                    self.switch_to_tab(TabLocation::Global);
                }
                (_, KeyCode::Char('3')) => {
                    self.switch_to_tab(TabLocation::Apple);
                }
                _ => match self.focus {
                    Focus::Search => self.handle_search_keys(key)?,
                    Focus::Sidebar => self.handle_sidebar_keys(key)?,
                    Focus::Form => self.handle_form_keys(key)?,
                },
            }
        }
        Ok(())
    }

    fn handle_exit_confirmation_keys(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Esc => {
                self.quit();
            }
            KeyCode::Char('n') | KeyCode::Char('N') => {
                self.showing_exit_confirmation = false;
            }
            _ => {}
        }
        Ok(())
    }

    fn handle_search_keys(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Char(c) => {
                self.filter_text.push(c);
                // Reset selection when filter changes
                self.list_state
                    .select(if self.get_filtered_agents().is_empty() {
                        None
                    } else {
                        Some(0)
                    });
            }
            KeyCode::Backspace => {
                self.filter_text.pop();
                // Reset selection when filter changes
                self.list_state
                    .select(if self.get_filtered_agents().is_empty() {
                        None
                    } else {
                        Some(0)
                    });
            }
            KeyCode::Enter => {
                self.focus = Focus::Sidebar;
            }
            _ => {}
        }
        Ok(())
    }

    fn handle_sidebar_keys(&mut self, key: KeyEvent) -> Result<()> {
        let filtered_count = self.get_filtered_agents().len();
        if filtered_count == 0 {
            return Ok(());
        }

        match key.code {
            KeyCode::Char('j') | KeyCode::Down => {
                let i = match self.list_state.selected() {
                    Some(i) => {
                        if i >= filtered_count - 1 {
                            0
                        } else {
                            i + 1
                        }
                    }
                    _ => 0,
                };
                self.list_state.select(Some(i));
            }
            KeyCode::Char('k') | KeyCode::Up => {
                let i = match self.list_state.selected() {
                    Some(i) => {
                        if i == 0 {
                            filtered_count - 1
                        } else {
                            i - 1
                        }
                    }
                    _ => 0,
                };
                self.list_state.select(Some(i));
            }
            KeyCode::Char('g') => {
                self.list_state.select(Some(0));
            }
            KeyCode::Char('G') => {
                self.list_state.select(Some(filtered_count - 1));
            }
            KeyCode::Enter => {
                self.load_selected_plist()?;
            }
            _ => {}
        }
        Ok(())
    }

    fn handle_form_keys(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Char('j') | KeyCode::Down => {
                self.current_field = match self.current_field {
                    FormField::Label => FormField::ProgramArguments,
                    FormField::ProgramArguments => FormField::StartInterval,
                    FormField::StartInterval => FormField::RunAtLoad,
                    FormField::RunAtLoad => FormField::KeepAlive,
                    FormField::KeepAlive => FormField::StandardOutPath,
                    FormField::StandardOutPath => FormField::StandardErrorPath,
                    FormField::StandardErrorPath => FormField::WorkingDirectory,
                    FormField::WorkingDirectory => FormField::Label,
                };
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.current_field = match self.current_field {
                    FormField::Label => FormField::WorkingDirectory,
                    FormField::ProgramArguments => FormField::Label,
                    FormField::StartInterval => FormField::ProgramArguments,
                    FormField::RunAtLoad => FormField::StartInterval,
                    FormField::KeepAlive => FormField::RunAtLoad,
                    FormField::StandardOutPath => FormField::KeepAlive,
                    FormField::StandardErrorPath => FormField::StandardOutPath,
                    FormField::WorkingDirectory => FormField::StandardErrorPath,
                };
            }
            KeyCode::Enter => {
                self.start_editing()?;
            }
            _ => {}
        }
        Ok(())
    }

    fn start_editing(&mut self) -> Result<()> {
        if let Some(plist) = &self.selected_plist {
            self.editing = true;
            self.editing_field = Some(self.current_field.clone());
            self.edit_buffer = match self.current_field {
                FormField::Label => plist.label.clone().unwrap_or_default(),
                FormField::StartInterval => plist
                    .start_interval
                    .map(|i| i.to_string())
                    .unwrap_or_default(),
                FormField::RunAtLoad => if plist.run_at_load.unwrap_or(false) {
                    "true"
                } else {
                    "false"
                }
                .to_string(),
                FormField::KeepAlive => if plist.keep_alive.unwrap_or(false) {
                    "true"
                } else {
                    "false"
                }
                .to_string(),
                FormField::StandardOutPath => plist.standard_out_path.clone().unwrap_or_default(),
                FormField::StandardErrorPath => {
                    plist.standard_error_path.clone().unwrap_or_default()
                }
                FormField::WorkingDirectory => plist.working_directory.clone().unwrap_or_default(),
                FormField::ProgramArguments => {
                    if let Some(args) = &plist.program_arguments {
                        args.join("\n")
                    } else {
                        String::new()
                    }
                }
            };
        }
        Ok(())
    }

    fn handle_edit_keys(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Esc => {
                self.editing = false;
                self.editing_field = None;
                self.edit_buffer.clear();
                self.set_status_message("✗ Edit cancelled".to_string());
            }
            KeyCode::Enter => {
                self.save_field_edit()?;
                self.editing = false;
                self.editing_field = None;
            }
            KeyCode::Backspace => {
                self.edit_buffer.pop();
            }
            // Block vim navigation keys during editing
            KeyCode::Char('j') | KeyCode::Char('k') | KeyCode::Char('g') | KeyCode::Char('G') => {
                // These are vim navigation keys - ignore them during editing
                // Don't add them to the edit buffer
            }
            KeyCode::Char(c) => {
                self.edit_buffer.push(c);
            }
            // Ignore arrow keys and other navigation keys during editing
            KeyCode::Up | KeyCode::Down | KeyCode::Left | KeyCode::Right => {
                // Do nothing - prevent navigation during editing
            }
            KeyCode::Tab => {
                // Tab should not change focus while editing
            }
            KeyCode::Home | KeyCode::End | KeyCode::PageUp | KeyCode::PageDown => {
                // Ignore other navigation keys during editing
            }
            _ => {}
        }
        Ok(())
    }

    fn save_field_edit(&mut self) -> Result<()> {
        if let (Some(plist), Some(editing_field)) = (&mut self.selected_plist, &self.editing_field)
        {
            match editing_field {
                FormField::Label => {
                    plist.label = (!self.edit_buffer.is_empty()).then(|| self.edit_buffer.clone());
                }
                FormField::StartInterval => {
                    plist.start_interval = self.edit_buffer.parse().ok();
                }
                FormField::RunAtLoad => {
                    plist.run_at_load = Some(self.edit_buffer == "true");
                }
                FormField::KeepAlive => {
                    plist.keep_alive = Some(self.edit_buffer == "true");
                }
                FormField::StandardOutPath => {
                    plist.standard_out_path =
                        (!self.edit_buffer.is_empty()).then(|| self.edit_buffer.clone());
                }
                FormField::StandardErrorPath => {
                    plist.standard_error_path =
                        (!self.edit_buffer.is_empty()).then(|| self.edit_buffer.clone());
                }
                FormField::WorkingDirectory => {
                    plist.working_directory =
                        (!self.edit_buffer.is_empty()).then(|| self.edit_buffer.clone());
                }
                FormField::ProgramArguments => {
                    let args: Vec<String> = self
                        .edit_buffer
                        .lines()
                        .map(|line| line.trim().to_string())
                        .filter(|line| !line.is_empty())
                        .collect();
                    plist.program_arguments = (!args.is_empty()).then_some(args);
                }
            }
            self.set_status_message(format!("✓ Updated {}", self.get_editing_field_name()));
        }
        self.edit_buffer.clear();
        Ok(())
    }

    fn save_plist(&mut self) -> Result<()> {
        if let Some(plist) = &self.selected_plist {
            if let Some(selected) = self.list_state.selected() {
                let filtered_agents = self.get_filtered_agents();
                if let Some(agent) = filtered_agents.get(selected) {
                    let file_path = self.get_current_directory().join(&agent.filename);
                    let xml_content = self.plist_to_xml(plist)?;
                    fs::write(&file_path, xml_content)?;

                    // Reload the agent with launchctl
                    match self.reload_agent(file_path.to_owned()) {
                        Ok(()) => {
                            self.set_status_message(format!(
                                "✓ Saved and reloaded {}",
                                agent.filename
                            ));
                            // Refresh the agent status after successful reload
                            self.refresh_agent_status();
                        }
                        Err(e) => {
                            self.set_status_message(format!(
                                "✓ Saved {} but reload failed: {}",
                                agent.filename, e
                            ));
                        }
                    }
                } else {
                    self.set_status_message("✗ No agent selected".to_string());
                }
            } else {
                self.set_status_message("✗ No agent selected".to_string());
            }
        } else {
            self.set_status_message("✗ No plist data to save".to_string());
        }
        Ok(())
    }

    fn reload_agent(&self, file_path: PathBuf) -> Result<()> {
        // First unload the agent (ignore errors if it wasn't loaded)
        let unload_result = std::process::Command::new("launchctl")
            .args(["unload", &file_path.to_string_lossy()])
            .output();

        match unload_result {
            Ok(output) => {
                if !output.status.success() {
                    // Unload failed, but that's okay if the agent wasn't loaded
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    if !stderr.contains("Could not find specified service") {
                        return Err(color_eyre::eyre::eyre!("Unload failed: {}", stderr));
                    }
                }
            }
            Err(e) => {
                return Err(color_eyre::eyre::eyre!(
                    "Failed to run launchctl unload: {}",
                    e
                ));
            }
        }

        // Now load the agent
        let load_result = std::process::Command::new("launchctl")
            .args(["load", &file_path.to_string_lossy()])
            .output();

        match load_result {
            Ok(output) => {
                if !output.status.success() {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    return Err(color_eyre::eyre::eyre!("Load failed: {}", stderr));
                }
            }
            Err(e) => {
                return Err(color_eyre::eyre::eyre!(
                    "Failed to run launchctl load: {}",
                    e
                ));
            }
        }

        Ok(())
    }

    fn refresh_agent_status(&mut self) {
        // Refresh the status of agents in the current tab
        let current_agents = self.get_current_agents_mut();
        for agent in current_agents {
            if let Some(label) = &agent.label {
                agent.status = Self::check_agent_status(label);
                agent.enabled = Self::check_agent_enabled(label);
            }
        }
    }

    fn switch_to_tab(&mut self, new_tab: TabLocation) {
        if self.current_tab != new_tab {
            self.current_tab = new_tab;
            self.selected_plist = None; // Clear selected plist when switching tabs
            self.filter_text.clear(); // Clear search filter

            // Reset list selection to first item if available
            let current_agents = self.get_current_agents();
            self.list_state.select(if current_agents.is_empty() {
                None
            } else {
                Some(0)
            });
        }
    }

    pub fn plist_to_xml(&self, plist: &PlistData) -> Result<String> {
        let mut xml = String::new();
        xml.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
        xml.push_str("<!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n");
        xml.push_str("<plist version=\"1.0\">\n");
        xml.push_str("<dict>\n");

        if let Some(label) = &plist.label {
            xml.push_str("    <key>Label</key>\n");
            xml.push_str(&format!("    <string>{}</string>\n", label));
            xml.push_str("    \n");
        }

        if let Some(args) = &plist.program_arguments {
            xml.push_str("    <key>ProgramArguments</key>\n");
            xml.push_str("    <array>\n");
            for arg in args {
                xml.push_str(&format!("        <string>{}</string>\n", arg));
            }
            xml.push_str("    </array>\n");
            xml.push_str("    \n");
        }

        if let Some(interval) = plist.start_interval {
            xml.push_str("    <key>StartInterval</key>\n");
            xml.push_str(&format!("    <integer>{}</integer>\n", interval));
            xml.push_str("    \n");
        }

        if let Some(run_at_load) = plist.run_at_load {
            xml.push_str("    <key>RunAtLoad</key>\n");
            xml.push_str(&format!(
                "    <{}/>\n",
                if run_at_load { "true" } else { "false" }
            ));
            xml.push_str("    \n");
        }

        if let Some(keep_alive) = plist.keep_alive {
            xml.push_str("    <key>KeepAlive</key>\n");
            xml.push_str(&format!(
                "    <{}/>\n",
                if keep_alive { "true" } else { "false" }
            ));
            xml.push_str("    \n");
        }

        if let Some(stdout) = &plist.standard_out_path {
            xml.push_str("    <key>StandardOutPath</key>\n");
            xml.push_str(&format!("    <string>{}</string>\n", stdout));
            xml.push_str("    \n");
        }

        if let Some(stderr) = &plist.standard_error_path {
            xml.push_str("    <key>StandardErrorPath</key>\n");
            xml.push_str(&format!("    <string>{}</string>\n", stderr));
            xml.push_str("    \n");
        }

        if let Some(workdir) = &plist.working_directory {
            xml.push_str("    <key>WorkingDirectory</key>\n");
            xml.push_str(&format!("    <string>{}</string>\n", workdir));
        }

        xml.push_str("</dict>\n");
        xml.push_str("</plist>\n");
        Ok(xml)
    }

    fn quit(&mut self) {
        self.running = false;
    }
}

fn parse_plist_xml(content: &str) -> Result<PlistData> {
    let mut plist_data = PlistData::default();

    let lines: Vec<&str> = content.lines().collect();
    let mut i = 0;
    let mut in_dict = false;
    let mut current_key = String::new();
    let mut program_args = Vec::new();
    let mut collecting_array = false;

    while i < lines.len() {
        let line = lines[i].trim();

        if line == "<dict>" {
            in_dict = true;
        } else if line == "</dict>" {
            in_dict = false;
        } else if in_dict && line.starts_with("<key>") && line.ends_with("</key>") {
            current_key = line[5..line.len() - 6].to_string();
        } else if in_dict && !current_key.is_empty() {
            match current_key.as_str() {
                "Label" if line.starts_with("<string>") => {
                    plist_data.label = Some(line[8..line.len() - 9].to_string());
                }
                "StartInterval" if line.starts_with("<integer>") => {
                    if let Ok(val) = line[9..line.len() - 10].parse() {
                        plist_data.start_interval = Some(val);
                    }
                }
                "RunAtLoad" if line == "<true/>" => {
                    plist_data.run_at_load = Some(true);
                }
                "RunAtLoad" if line == "<false/>" => {
                    plist_data.run_at_load = Some(false);
                }
                "KeepAlive" if line == "<true/>" => {
                    plist_data.keep_alive = Some(true);
                }
                "KeepAlive" if line == "<false/>" => {
                    plist_data.keep_alive = Some(false);
                }
                "StandardOutPath" if line.starts_with("<string>") => {
                    plist_data.standard_out_path = Some(line[8..line.len() - 9].to_string());
                }
                "StandardErrorPath" if line.starts_with("<string>") => {
                    plist_data.standard_error_path = Some(line[8..line.len() - 9].to_string());
                }
                "WorkingDirectory" if line.starts_with("<string>") => {
                    plist_data.working_directory = Some(line[8..line.len() - 9].to_string());
                }
                "ProgramArguments" if line == "<array>" => {
                    collecting_array = true;
                    program_args.clear();
                }
                "ProgramArguments" if line == "</array>" => {
                    collecting_array = false;
                    plist_data.program_arguments = Some(program_args.clone());
                }
                _ => {}
            }

            if collecting_array && line.starts_with("<string>") && line.ends_with("</string>") {
                program_args.push(line[8..line.len() - 9].to_string());
            }

            if !collecting_array {
                current_key.clear();
            }
        }
        i += 1;
    }

    Ok(plist_data)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_label_element() {
        let xml = r#"<dict>
    <key>Label</key>
    <string>com.user.test</string>
</dict>"#;

        let parsed = parse_plist_xml(xml).unwrap();
        assert_eq!(parsed.label, Some("com.user.test".to_string()));
    }

    #[test]
    fn test_parse_start_interval_element() {
        let xml = r#"<dict>
    <key>StartInterval</key>
    <integer>300</integer>
</dict>"#;

        let parsed = parse_plist_xml(xml).unwrap();
        assert_eq!(parsed.start_interval, Some(300));
    }

    #[test]
    fn test_parse_boolean_true_element() {
        let xml = r#"<dict>
    <key>RunAtLoad</key>
    <true/>
</dict>"#;

        let parsed = parse_plist_xml(xml).unwrap();
        assert_eq!(parsed.run_at_load, Some(true));
    }

    #[test]
    fn test_parse_boolean_false_element() {
        let xml = r#"<dict>
    <key>KeepAlive</key>
    <false/>
</dict>"#;

        let parsed = parse_plist_xml(xml).unwrap();
        assert_eq!(parsed.keep_alive, Some(false));
    }

    #[test]
    fn test_parse_program_arguments_array() {
        let xml = r#"<dict>
    <key>ProgramArguments</key>
    <array>
        <string>/usr/bin/test</string>
        <string>--flag</string>
    </array>
</dict>"#;

        let parsed = parse_plist_xml(xml).unwrap();
        assert_eq!(
            parsed.program_arguments,
            Some(vec!["/usr/bin/test".to_string(), "--flag".to_string()])
        );
    }

    #[test]
    fn test_parse_string_paths() {
        let xml = r#"<dict>
    <key>StandardOutPath</key>
    <string>/tmp/out.log</string>
    <key>StandardErrorPath</key>
    <string>/tmp/err.log</string>
    <key>WorkingDirectory</key>
    <string>/tmp</string>
</dict>"#;

        let parsed = parse_plist_xml(xml).unwrap();
        assert_eq!(parsed.standard_out_path, Some("/tmp/out.log".to_string()));
        assert_eq!(parsed.standard_error_path, Some("/tmp/err.log".to_string()));
        assert_eq!(parsed.working_directory, Some("/tmp".to_string()));
    }

    #[test]
    fn test_example_plist_file() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.user.price-checker-eth</string>
    
    <key>ProgramArguments</key>
    <array>
        <string>/Users/dev/.local/bin/price-checker-eth</string>
    </array>
    
    <key>StartInterval</key>
    <integer>600</integer>
    
    <key>RunAtLoad</key>
    <true/>
    
    <key>KeepAlive</key>
    <false/>
    
    <key>StandardOutPath</key>
    <string>/Users/dev/Documents/github.com/hollanddd/price-checker-eth/logs/price-checker-eth.log</string>
    
    <key>StandardErrorPath</key>
    <string>/Users/dev/Documents/github.com/hollanddd/price-checker-eth/logs/price-checker-eth.error.log</string>
    
    <key>WorkingDirectory</key>
    <string>/Users/dev/Documents/github.com/hollanddd/price-checker-eth</string>
</dict>
</plist>"#;

        let parsed = parse_plist_xml(xml).unwrap();

        assert_eq!(parsed.label, Some("com.user.price-checker-eth".to_string()));
        assert_eq!(
            parsed.program_arguments,
            Some(vec!["/Users/dev/.local/bin/price-checker-eth".to_string()])
        );
        assert_eq!(parsed.start_interval, Some(600));
        assert_eq!(parsed.run_at_load, Some(true));
        assert_eq!(parsed.keep_alive, Some(false));
        assert_eq!(parsed.standard_out_path, Some("/Users/dev/Documents/github.com/hollanddd/price-checker-eth/logs/price-checker-eth.log".to_string()));
        assert_eq!(parsed.standard_error_path, Some("/Users/dev/Documents/github.com/hollanddd/price-checker-eth/logs/price-checker-eth.error.log".to_string()));
        assert_eq!(
            parsed.working_directory,
            Some("/Users/dev/Documents/github.com/hollanddd/price-checker-eth".to_string())
        );
    }
}
