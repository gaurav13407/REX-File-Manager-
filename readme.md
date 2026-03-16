A fast, keyboard-driven terminal file manager and system dashboard — built in Rust. One binary. Vim-style modal keys. Always-on system stats. Zero dependencies.
Features
⌨
Vim modal keys
hjkl navigation, visual select, command mode — fully remappable via config.toml
▣
Dual-pane layout
Side-by-side panes, Tab to switch, resize with < / >
◈
Live system stats
CPU%, temp, RAM, disk, GPU always visible. Press S for full dashboard
⌥
Git awareness
Inline M / U / S indicators per file, branch name in pane header
◎
Fuzzy search
/ triggers live search powered by the nucleo engine (same as Helix)
≡
Bulk rename
Visual select + R opens selection in $EDITOR. Save = rename.
Install
build from source
copy
git clone https://github.com/gaurav/rex
cd rex
cargo build --release
sudo cp target/release/rex /usr/local/bin/
Config
~/.config/rex/config.toml — auto-generated on first run
copy
[keys]
up         = ["k", "Up"]
down       = ["j", "Down"]
left       = ["h", "Left", "Backspace"]
right      = ["l", "Right", "Enter"]
search     = ["/"]
toggle_stats = ["S"]
quit       = ["q"]

[stats]
poll_interval_ms = 1000
show_gpu         = true
show_cpu_temp    = true
Roadmap
v0.1
Foundation ← current
Dual pane · vim keys · file ops · stats bar · TOML config
v0.2
Power features
Fuzzy search · git status · detailed stats panel · preview pane
v0.3
Workflow
Bulk rename · bookmarks · zoxide · extension open rules · trash
v0.4
Polish
Themes · image preview (kitty protocol) · hooks · session restore
Stack
ratatui
TUI rendering engine
crossterm
Terminal input / raw mode
sysinfo
CPU, RAM, disk stats
nvml-wrapper
NVIDIA GPU stats
nucleo
Fuzzy search engine
git2
Git status integration
tokio
Async stat polling
serde + toml
Config parsing
Built on Linux · Hyprland / Wayland · AMD Ryzen 7 7700X · RTX 5060 Ti
MIT License
