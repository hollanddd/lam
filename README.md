# LAM - Launch Agent Manager

<div align="center">
  <img src="assets/loop.gif" alt="LAM Logo" />
</div>

---

## 🐑 Overview

LAM (Launch Agent Manager) is a modern, feature-rich terminal user interface (TUI) application built with Rust and Ratatui for managing macOS LaunchAgent plist files. Inspired by systemctl-tui, LAM provides an intuitive, vim-style interface for viewing, editing, and managing launch agents across User, Global, and Apple directories.

## ✨ Features

### 🎯 **Service-Style Agent Browser**

- **Three-tab interface**: User (`~/Library/LaunchAgents`), Global (`/Library/LaunchAgents`), and Apple (`/System/Library/LaunchAgents`)
- **Real-time status indicators**:
  - ● **Running** (green) / ● **Stopped** (red) / ✗ **Error** (magenta) / ? **Unknown** (gray)
  - ◉ **Enabled** (cyan) / ○ **Disabled** (gray)
- **Agent count display**: Shows total number of LaunchAgents in each category

### 🔍 **Smart Search & Filtering**

- **Real-time search**: Filter agents by filename or label as you type
- **Live counter**: Shows filtered results (e.g., "LaunchAgents (3/15)")
- **Case-insensitive**: Searches both filename and internal agent labels

### ⚡ **Status Integration**

- **launchctl integration**: Live status detection using macOS launchctl
- **Automatic refresh**: Status updates after save operations
- **Multi-state support**: Running, stopped, error, and unknown states

### 📝 **Plist Editor**

- **Structured form interface**: Edit plist properties in organized form fields
- **Editing**: Navigate and edit values with vim-style keybindings
- **Comprehensive property support**: Common LaunchAgent properties
- **Save functionality**: Save changes with Ctrl-S and automatic agent reload

### 🎨 **Modern Design**

- **OneHalfDark theme**: Professional color scheme with excellent readability
- **Rounded borders**: Modern UI elements with consistent styling
- **Focus indicators**: Clear visual feedback for current selection
- **Loading screen**: Animated startup with progress indicators

### ⌨️ **Vim-Style Navigation**

- **j/k navigation**: Move through agent lists and form fields
- **g/G shortcuts**: Jump to first/last items
- **Tab cycling**: Switch between Search → Sidebar → Form panels
- **Intuitive keybindings**: Familiar patterns for efficient workflow

## 🛠 Installation

### Prerequisites

- macOS (required for LaunchAgent functionality)
- Rust toolchain (latest stable)

### Build from Source

```bash
git clone https://github.com/hollanddd/lam.git
cd lam
cargo build --release
./target/release/lam
```

### Development Build

```bash
cargo run
```

## 📖 Usage

### Navigation

- **Tab**: Cycle focus between Search → Sidebar → Form
- **1/2/3**: Switch between User/Global/Apple tabs
- **/**: Jump to search bar
- **q/Esc**: Show exit confirmation

### Search Bar

- **Type**: Filter agents by name or label
- **Backspace**: Remove filter characters
- **Enter**: Move focus to sidebar

### Sidebar Navigation

- **j/k** or **Arrow keys**: Navigate agent list
- **g**: Go to first agent
- **G**: Go to last agent
- **Enter**: Load selected agent for editing

### Form Editor

- **j/k** or **Arrow keys**: Navigate form fields
- **Enter**: Start editing current field
- **Ctrl-S**: Save changes and reload agent
- **PgUp/PgDn**: Scroll through long forms

### Edit Mode

- **Type**: Modify field values
- **Enter**: Save field changes
- **Esc**: Cancel editing
- **Backspace**: Delete characters

### Exit

- **q/Esc/Ctrl-C**: Show exit confirmation
- **Y**: Confirm exit
- **N**: Cancel exit

## 🔧 Supported LaunchAgent Properties

LAM supports editing all common LaunchAgent plist properties:

| Property | Type | Description |
|----------|------|-------------|
| **Label** | String | Unique identifier for the agent |
| **Program** | String | Path to executable program |
| **ProgramArguments** | Array | Command and arguments to execute |
| **StartInterval** | Integer | Run interval in seconds |
| **ThrottleInterval** | Integer | Minimum seconds between launches |
| **RunAtLoad** | Boolean | Start at system boot |
| **KeepAlive** | Boolean | Restart if process exits |
| **AbandonProcessGroup** | Boolean | Prevent process group management |
| **StandardOutPath** | String | Path for stdout logging |
| **StandardErrorPath** | String | Path for stderr logging |
| **WorkingDirectory** | String | Working directory for process |
| **POSIXSpawnType** | String | Process spawn method |
| **EnablePressuredExit** | Boolean | Allow system-initiated termination |
| **EnableTransactions** | Boolean | Enable transaction support |
| **EventMonitor** | Boolean | Monitor system events |
| **LimitLoadToSessionType** | String/Array | Session type restrictions |
| **AssociatedBundleIdentifiers** | Array | Related bundle identifiers |
| **EnvironmentVariables** | Dictionary | Custom environment variables |

## 🎯 Auto-reload Process

When you save changes with **Ctrl-S**, LAM automatically:

1. **Saves** the XML file to the appropriate LaunchAgents directory
2. **Unloads** the agent using `launchctl unload`
3. **Loads** the agent using `launchctl load`  
4. **Refreshes** status indicators to show new state
5. **Provides feedback** in the status bar

**Error Handling:**

- Unload failures (agent not loaded) are ignored
- Load failures display error messages in status bar
- File saves always succeed, even if reload fails

## 🏗 Architecture

### Core Technologies

- **Rust**: Systems programming language for performance and safety
- **Ratatui**: Terminal UI framework for rich text interfaces
- **Crossterm**: Cross-platform terminal manipulation
- **Tokio**: Async runtime for responsive UI
- **Serde**: Serialization for plist data handling
- **Quick-XML**: Fast XML parsing with serde integration

### Key Components

- **Single-file application**: Entire logic in `src/main.rs`
- **Event-driven architecture**: Async event handling with Crossterm
- **State management**: Centralized app state with focus tracking
- **Real-time integration**: Live launchctl status checking
- **Theme system**: Consistent OneHalfDark color palette

## 🧪 Development

### Running Tests

```bash
cargo test
```

### Code Quality

```bash
# Format code
cargo fmt

# Run linter
cargo clippy

# Check compilation
cargo check
```

### Project Structure

```text
lam/
├── src/
│   └── main.rs          # Complete application logic
├── assets/
│   └── lamb.png         # Application logo
├── Cargo.toml           # Rust dependencies and metadata
├── README.md            # This documentation
├── LICENSE              # MIT license
└── CLAUDE.md           # AI assistant instructions
```

## 🤝 Contributing

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

## 📄 License

Copyright (c) Darren <me@darrenholland.com>

This project is licensed under the MIT license ([LICENSE] or <http://opensource.org/licenses/MIT>)

[LICENSE]: ./LICENSE

---

<div align="center">
  <p><em>Built with ❤️ in Rust for macOS power users</em></p>
</div>
