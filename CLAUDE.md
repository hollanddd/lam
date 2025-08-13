# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

This is "lam" (Launch Agent Manager), a terminal user interface (TUI) application built with Rust and Ratatui for managing macOS LaunchAgent plist files located in `~/Library/LaunchAgents`. The application provides a sidebar for file navigation with vim-style keybindings and displays parsed plist content in a structured form.

## Development Commands

### Build and Run
```bash
# Build the project
cargo build

# Run the application
cargo run

# Build for release
cargo build --release
```

### Testing and Linting
```bash
# Run tests
cargo test

# Check code without building
cargo check

# Format code
cargo fmt

# Run clippy for linting
cargo clippy
```

## Architecture

### Core Structure
- **Single-file application**: The entire application logic is contained in `src/main.rs`
- **Async architecture**: Built on Tokio runtime with async/await patterns
- **Event-driven UI**: Uses Crossterm for terminal events and Ratatui for rendering

### Key Components

#### App Struct (`src/main.rs:20-26`)
- Main application state container
- Manages the application lifecycle through the `running` boolean
- Handles terminal event stream via `EventStream`

#### Event Loop (`src/main.rs:35-42`)
- Async main loop that alternates between drawing UI and handling events
- Uses `tokio::select!` for concurrent event processing
- Includes timeout mechanism to prevent busy waiting

#### UI Rendering (`src/main.rs:49-63`)
- Currently displays a simple welcome screen with bordered paragraph
- Template includes placeholder text and basic styling
- Ready for extension with additional widgets and layouts

### Key Features
- **Service-Style Agent Browser**: Left sidebar displays all LaunchAgents with real-time status indicators
  - **Status Icons**: ● = Running (green), ○ = Stopped (red), ✗ = Error (magenta), ? = Unknown (gray)
  - **Enabled Status**: ◉ = Enabled (cyan), ○ = Disabled (gray)
  - **Agent Count**: Shows total number of LaunchAgents in title
- **Real-time Search & Filtering**: Top search bar filters agents by filename or label
  - **Live Filtering**: Results update as you type
  - **Counter Display**: Shows filtered count (e.g., "LaunchAgents (3/15)")
  - **Case-insensitive**: Searches both filename and agent labels
- **Real-time Status Detection**: Integrates with `launchctl` to show live agent status
- **Vim Navigation**: Navigate the agent list using `j`/`k` (down/up), `g` (first), `G` (last)
- **Editable Form Interface**: Press Enter to load and edit plist content in structured form fields
- **Real-time Editing**: Navigate form fields with vim keys, press Enter to edit values inline
- **Auto-save Support**: Save changes back to XML with Ctrl-S
- **Automatic Reload**: After saving, automatically runs `launchctl unload` then `launchctl load` to apply changes
- **Three-Panel Focus Management**: Tab cycles between Search → Sidebar → Form
- **Professional TUI Design**: Inspired by systemctl-tui with clear visual hierarchy
- **Status Bar**: Context-aware help showing available commands and status legend
- **Edit Mode Indicators**: Clear visual cues when editing with cursor indicator and yellow highlighting
- **Exit Confirmation**: Prevents accidental exits with a confirmation dialog

### Key Bindings
#### Global
- `q`, `Esc`, `Ctrl-C`: Show exit confirmation dialog
- `Tab`: Cycle focus between Search → Sidebar → Form
- `Ctrl-S`: Save changes and automatically reload with launchctl
- `/`: Jump to search bar

#### Search Bar (when focused)
- `Type`: Filter agents by filename or label
- `Backspace`: Remove characters from search
- `Enter`: Move focus to sidebar
- `Tab`: Move focus to sidebar

#### Exit Confirmation Dialog
- `Y` or `y`: Confirm exit
- `Esc`: Confirm exit (second Esc press)
- `N` or `n`: Cancel exit and return to application

#### Sidebar (when focused)
- `j` or `Down`: Move down in filtered agent list
- `k` or `Up`: Move up in filtered agent list
- `g`: Go to first agent
- `G`: Go to last agent
- `Enter`: Load and display selected plist file
- `/`: Jump to search bar

#### Form Panel (when focused)
- `j` or `Down`: Move to next form field
- `k` or `Up`: Move to previous form field
- `Enter`: Start editing the current field

#### Edit Mode (when editing a field)
- `Type`: Modify field value
- `Enter`: Save changes to field
- `Esc`: Cancel editing and revert changes
- `Backspace`: Delete characters
- **Navigation Disabled**: Arrow keys, Tab, and other navigation keys are ignored during editing

### Dependencies
- **ratatui**: Terminal UI framework for building rich text user interfaces
- **crossterm**: Cross-platform terminal manipulation library
- **tokio**: Async runtime with full feature set
- **color-eyre**: Enhanced error reporting and handling
- **futures**: Additional async utilities for stream processing
- **serde**: Serialization framework for Rust
- **quick-xml**: Fast XML parser with serde support
- **dirs**: Platform-specific standard locations finder

### Data Structures

#### PlistData (`src/main.rs:42-59`)
Represents the common LaunchAgent plist properties:
- `Label`: Unique identifier for the agent
- `ProgramArguments`: Command and arguments to execute
- `StartInterval`: Run interval in seconds
- `RunAtLoad`: Whether to run at system startup
- `KeepAlive`: Whether to restart if the process exits
- `StandardOutPath`: Path for stdout logging
- `StandardErrorPath`: Path for stderr logging
- `WorkingDirectory`: Working directory for the process

### Automatic Reload Process
When you save changes with `Ctrl-S`, the application automatically:
1. **Saves the XML file** to `~/Library/LaunchAgents/`
2. **Unloads the agent** with `launchctl unload <file>`
3. **Loads the agent** with `launchctl load <file>`
4. **Refreshes status indicators** to show the new running state
5. **Provides feedback** in the status bar about success or failure

**Error Handling:**
- If unload fails because the agent wasn't loaded, this is ignored
- If load fails, the error message is displayed in the status bar
- The file is always saved successfully, even if reload fails

### Visual Design

#### Modern Color Theme
The application uses a sophisticated OneHalfDark-inspired color palette:
- **Background**: Dark blue-gray (#282c34) for reduced eye strain
- **Foreground**: Light gray (#dcdfe4) for high readability  
- **Primary Accent**: Blue (#61afef) for focused elements and interactive components
- **Secondary Accent**: Green (#98c379) for success states and running services
- **Warning Accent**: Yellow (#e5c07b) for edit mode and warnings
- **Error Accent**: Red (#e06c75) for errors and stopped services
- **Muted Accent**: Cyan (#56b6c2) for secondary information
- **Subtle Elements**: Gray (#5c636c) for borders and disabled states

#### Enhanced Visual Hierarchy
- **Rounded Borders**: Modern BorderType::Rounded for all panels
- **Contextual Styling**: Focus-aware color changes throughout the interface
- **Icon Integration**: Meaningful emojis enhance visual communication
- **Improved Spacing**: Better padding and margins for cleaner layout
- **Status Indicators**: Bold, colorful symbols for immediate status recognition

#### Professional Layout
- **35/65 Split**: Optimized sidebar-to-main ratio for better content display
- **Unified Margins**: Consistent 1-unit margin around the entire interface
- **Panel Spacing**: 1-unit spacing between major UI components
- **Enhanced Typography**: Bold formatting for labels, italic for hints

### Extension Points
The application is structured to easily add:
- Additional plist properties support
- File operations (create, delete, enable/disable)
- Manual start/stop/restart commands
- Log viewing capabilities
- Syntax highlighting for plist content
- Theme customization system
- Keyboard shortcut customization