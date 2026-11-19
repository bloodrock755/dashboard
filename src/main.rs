#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use slint::{ModelRc, SharedString, Timer, TimerMode, VecModel};
use std::env;
use std::fs;
use std::net::{Ipv4Addr, SocketAddr, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

#[cfg(windows)]
use std::os::windows::process::CommandExt;

fn new_command(program: impl AsRef<std::ffi::OsStr>) -> Command {
    let mut cmd = Command::new(program);
    cmd.stdin(Stdio::null());
    #[cfg(windows)]
    {
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    cmd
}

slint::slint! {
    import { Button, VerticalBox, HorizontalBox, LineEdit, ListView } from "std-widgets.slint";

    component GrantButton inherits Rectangle {
        in property <string> text: "Grant";
        callback clicked <=> touch.clicked;

        height: 44px;
        min-width: 100px;
        border-radius: 6px;
        border-width: 1px;
        border-color: #00e676;
        background: touch.pressed ? #222222 : (touch.has-hover ? #141414 : #00e676);

        touch := TouchArea { mouse-cursor: pointer; }

        Text {
            text: root.text;
            color: #000000;
            font-size: 20px;
            font-weight: 700;
            horizontal-alignment: center;
            vertical-alignment: center;
        }
    }

    component RevokeButton inherits Rectangle {
        in property <string> text: "Revoke";
        callback clicked <=> touch.clicked;

        height: 44px;
        min-width: 100px;
        border-radius: 6px;
        border-width: 1px;
        border-color: #f39c12;
        background: touch.pressed ? #222222 : (touch.has-hover ? #141414 : #000000);

        touch := TouchArea { mouse-cursor: pointer; }

        Text {
            text: root.text;
            color: #f39c12;
            font-size: 20px;
            font-weight: 700;
            horizontal-alignment: center;
            vertical-alignment: center;
        }
    }

    component ImportButton inherits Rectangle {
        in property <string> text: "Import";
        callback clicked <=> touch.clicked;

        height: 44px;
        min-width: 110px;
        border-radius: 6px;
        border-width: 1px;
        border-color: #00ffff;
        background: touch.pressed ? #222222 : (touch.has-hover ? #141414 : #000000);

        touch := TouchArea { mouse-cursor: pointer; }

        Text {
            text: root.text;
            color: #00ffff;
            font-size: 20px;
            font-weight: 700;
            horizontal-alignment: center;
            vertical-alignment: center;
        }
    }

    component DeleteButton inherits Rectangle {
        in property <string> text: "Delete User";
        callback clicked <=> touch.clicked;

        height: 44px;
        min-width: 140px;
        border-radius: 6px;
        border-width: 1px;
        border-color: #e74c3c;
        background: touch.pressed ? #222222 : (touch.has-hover ? #141414 : #000000);

        touch := TouchArea { mouse-cursor: pointer; }

        Text {
            text: root.text;
            color: #e74c3c;
            font-size: 20px;
            font-weight: 700;
            horizontal-alignment: center;
            vertical-alignment: center;
        }
    }

    component ServerControlButton inherits Rectangle {
        in property <string> text;
        in property <bool> enabled: true;
        in property <color> accent-color: #ffffff;
        callback clicked;

        height: 45px;
        border-radius: 6px;
        border-width: 2px;
        border-color: root.enabled ? root.accent-color : #333333;
        background: !root.enabled ? #0a0a0a : (touch.pressed ? #222222 : (touch.has-hover ? #141414 : #000000));

        touch := TouchArea {
            enabled: root.enabled;
            mouse-cursor: root.enabled ? pointer : default;
            clicked => { root.clicked(); }
        }

        Text {
            text: root.text;
            color: root.enabled ? root.accent-color : #555555;
            font-size: 20px;
            font-weight: 700;
            horizontal-alignment: center;
            vertical-alignment: center;
        }
    }

    export component AppWindow inherits Window {
        title: "Silvermere Server Panel V1.2.0";
        min-width: 980px;
        min-height: 740px;
        default-font-size: 24px;

        in-out property <bool> is-running: false;
        in-out property <string> status-text: "Unknown";
        in-out property <brush> status-color: #ffd700;
        in-out property <string> server-version: "V 0.00";
        in-out property <string> command-feedback: "";
        in-out property <string> online-header: "Who's Online (0)";
        in-out property <[string]> online-players: ["Server stopped."];
        in-out property <bool> show-logs: false;
        in-out property <[string]> log-lines: ["No log output yet."];
        in-out property <string> confirm-action: "";
        in-out property <string> confirm-target: "";
        in-out property <string> confirm-message: "";

        callback start-clicked();
        callback shutdown-clicked();
        callback webserver-clicked();
        callback refresh-clicked();
        callback import-clicked();
        callback delete-user-clicked(string);
        callback exit-clicked();
        callback grant-clicked(string, string);
        callback revoke-clicked(string, string);
        callback broadcast-clicked(string);
        callback reload-world-clicked();

        VerticalBox {
            spacing: 12px;
            padding: 20px;

            // 1. Server Status Header (Top)
            Rectangle {
                height: 46px;

                HorizontalBox {
                    padding: 0px;
                    alignment: center;
                    Text {
                        text: "Server Status: ";
                        font-size: 38px;
                    }
                    Text {
                        text: root.status-text;
                        font-size: 36px;
                        color: root.status-color;
                        font-weight: 700;
                    }
                }

                Text {
                    text: root.server-version;
                    font-size: 28px;
                    color: #cccccc;
                    x: parent.width - self.width - 10px;
                    y: (parent.height - self.height) / 2;
                }
            }

            // 2. Webserver & Start / Stop Controls
            HorizontalBox {
                spacing: 0px;

                ServerControlButton {
                    text: "🌐  Open Webserver";
                    width: 245px;
                    accent-color: #00ffff;
                    clicked => { root.webserver-clicked(); }
                }

                Rectangle { horizontal-stretch: 1; }

                HorizontalBox {
                    padding: 0px;
                    spacing: 48px;

                    ServerControlButton {
                        text: "▶  Start Server";
                        width: 195px;
                        accent-color: #00e676;
                        enabled: !root.is-running;
                        clicked => {
                            root.confirm-action = "";
                            root.start-clicked();
                        }
                    }

                    ServerControlButton {
                        text: "■  Shutdown Server";
                        width: 245px;
                        accent-color: #00e676;
                        enabled: root.is-running;
                        clicked => {
                            root.confirm-action = "shutdown";
                            root.confirm-target = "";
                            root.confirm-message = "Are you sure you want to shut down the server?";
                        }
                    }
                }
            }

            // Inline Confirmation Banner
            if root.confirm-action != "" : Rectangle {
                border-radius: 6px;
                border-width: 1px;
                border-color: #e74c3c;
                background: #2c1515;
                height: 52px;

                HorizontalBox {
                    padding-left: 12px;
                    padding-right: 12px;
                    spacing: 12px;
                    alignment: space-between;

                    Text {
                        text: root.confirm-message;
                        font-size: 18px;
                        color: #ffffff;
                        vertical-alignment: center;
                    }

                    HorizontalBox {
                        spacing: 8px;
                        Button {
                            text: "Yes, Confirm";
                            height: 36px;
                            clicked => {
                                if (root.confirm-action == "shutdown") {
                                    root.shutdown-clicked();
                                } else if (root.confirm-action == "delete") {
                                    root.delete-user-clicked(root.confirm-target);
                                }
                                root.confirm-action = "";
                                root.confirm-target = "";
                            }
                        }

                        Button {
                            text: "Cancel";
                            height: 36px;
                            clicked => {
                                root.confirm-action = "";
                                root.confirm-target = "";
                                root.command-feedback = "Action canceled.";
                            }
                        }
                    }
                }
            }

            // 3. Account Management (Grant & Revoke Role)
            VerticalBox {
                spacing: 8px;
                padding-top: 4px;

                Text {
                    text: "Account Management";
                    font-size: 22px;
                    font-weight: 700;
                }

                HorizontalBox {
                    spacing: 12px;
                    user-input := LineEdit {
                        placeholder-text: "Username";
                        height: 44px;
                    }
                    option-input := LineEdit {
                        placeholder-text: "Role (e.g. sysop)";
                        height: 44px;
                    }
                    GrantButton {
                        text: "Grant";
                        clicked => { root.grant-clicked(user-input.text, option-input.text); }
                    }
                    RevokeButton {
                        text: "Revoke";
                        clicked => { root.revoke-clicked(user-input.text, option-input.text); }
                    }
                }
            }

            // 4. Import & Delete User Section
            VerticalBox {
                spacing: 8px;

                HorizontalBox {
                    spacing: 24px;

                    VerticalBox {
                        spacing: 8px;
                        Text {
                            text: "Import Section";
                            font-size: 22px;
                            font-weight: 700;
                        }
                        HorizontalBox {
                            alignment: start;
                            ImportButton {
                                text: "Import";
                                clicked => { root.import-clicked() }
                            }
                        }
                    }

                    VerticalBox {
                        spacing: 8px;
                        horizontal-stretch: 1;
                        Text {
                            text: "Delete Account";
                            font-size: 22px;
                            font-weight: 700;
                        }
                        HorizontalBox {
                            spacing: 12px;
                            delete-user-input := LineEdit {
                                placeholder-text: "Username to delete";
                                height: 44px;
                            }
                            DeleteButton {
                                text: "Delete User";
                                clicked => {
                                    if (delete-user-input.text == "") {
                                        root.command-feedback = "Please enter a username to delete.";
                                    } else {
                                        root.confirm-action = "delete";
                                        root.confirm-target = delete-user-input.text;
                                        root.confirm-message = "Delete user '" + delete-user-input.text + "' permanently?";
                                        delete-user-input.text = "";
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // 5. Sysop Controls (Broadcast & Reload World)
            VerticalBox {
                spacing: 8px;

                Text {
                    text: "Sysop Controls";
                    font-size: 22px;
                    font-weight: 700;
                }

                HorizontalBox {
                    spacing: 12px;
                    broadcast-input := LineEdit {
                        placeholder-text: "Broadcast message to all online players...";
                        height: 44px;
                        enabled: root.is-running;
                    }
                    Button {
                        text: "📢 Broadcast";
                        height: 44px;
                        enabled: root.is-running;
                        clicked => {
                            root.broadcast-clicked(broadcast-input.text);
                            broadcast-input.text = "";
                        }
                    }
                    Button {
                        text: "🔄 Reload World";
                        height: 44px;
                        enabled: root.is-running;
                        clicked => {
                            root.reload-world-clicked();
                        }
                    }
                }

                Text {
                    text: root.command-feedback;
                    font-size: 18px;
                    color: #3498db;
                }
            }

            // 6. Who's Online Section (Clickable to select player)
            VerticalBox {
                spacing: 8px;
                vertical-stretch: 1;

                Text {
                    text: root.online-header;
                    font-size: 22px;
                    font-weight: 700;
                }

                Rectangle {
                    border-radius: 6px;
                    border-width: 1px;
                    border-color: #333333;
                    background: #111111;
                    min-height: 100px;
                    vertical-stretch: 1;

                    ListView {
                        for player in root.online-players : Rectangle {
                            height: 36px;
                            background: row-touch.has-hover && root.is-running && player != "No players online." && player != "Waiting for player list..." ? #1c1c1c : transparent;

                            row-touch := TouchArea {
                                mouse-cursor: (root.is-running && player != "No players online." && player != "Waiting for player list...") ? pointer : default;
                                clicked => {
                                    if (root.is-running && player != "No players online." && player != "Waiting for player list...") {
                                        user-input.text = player;
                                        delete-user-input.text = player;
                                        root.command-feedback = "Selected player '" + player + "'.";
                                    }
                                }
                            }

                            HorizontalBox {
                                padding-left: 8px;
                                padding-right: 8px;

                                Text {
                                    text: player;
                                    font-size: 21px;
                                    color: #ecf0f1;
                                    vertical-alignment: center;
                                }
                            }
                        }
                    }
                }
            }

            // 7. Collapsible Server Log Dropdown Section
            VerticalBox {
                spacing: 6px;

                HorizontalBox {
                    alignment: start;
                    Button {
                        text: root.show-logs ? "▲  Hide Server Log" : "▼  Show Server Log";
                        height: 38px;
                        clicked => {
                            root.show-logs = !root.show-logs;
                            if (root.show-logs) {
                                root.refresh-clicked();
                            }
                        }
                    }
                }

                if root.show-logs : Rectangle {
                    border-radius: 6px;
                    border-width: 1px;
                    border-color: #444444;
                    background: #0a0a0a;
                    height: 150px;

                    ListView {
                        for line in root.log-lines : Text {
                            text: line;
                            font-size: 15px;
                            color: #27ae60;
                        }
                    }
                }
            }

            // 8. Utility Footer
            HorizontalBox {
                alignment: space-between;
                Button {
                    text: "↻  Refresh Status";
                    height: 44px;
                    clicked => { root.refresh-clicked() }
                }
                Button {
                    text: "Exit Panel";
                    height: 44px;
                    clicked => { root.exit-clicked() }
                }
            }
        }
    }
}

fn silvermere_command(b: &Path) -> Command { let d = b.parent().unwrap_or_else(|| Path::new(".")); let mut c = new_command(b); c.current_dir(d); if let Ok(t) = fs::read_to_string(d.join("admin.token")) { let t = t.trim(); if !t.is_empty() { c.env("SILVERMERE_ADMIN_TOKEN", t); } } c }

fn get_server_paths() -> (PathBuf, PathBuf, PathBuf, PathBuf) {
    let base_dir = env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
        .unwrap_or_else(|| env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

    let binary_name = if cfg!(windows) {
        "silvermere.exe"
    } else {
        "silvermere"
    };

    (
        base_dir.join(binary_name),
        base_dir.join("silvermere.pid"),
        base_dir.join("server.log"),
        base_dir.join("online.txt"),
    )
}

fn is_process_running(pid_file: &Path) -> bool {
    let Ok(pid_str) = fs::read_to_string(pid_file) else {
        return false;
    };
    let Ok(pid) = pid_str.trim().parse::<u32>() else {
        let _ = fs::remove_file(pid_file);
        return false;
    };

    let alive = if cfg!(windows) {
        let target = format!("\"{}\"", pid);
        new_command("tasklist")
            .args(["/FI", &format!("PID eq {}", pid), "/FO", "CSV", "/NH"])
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).contains(&target))
            .unwrap_or(false)
    } else {
        new_command("kill")
            .args(["-0", &pid.to_string()])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    };

    if !alive {
        let _ = fs::remove_file(pid_file);
    }
    alive
}

fn is_port_listening(port: u16) -> bool {
    let addr = SocketAddr::from((Ipv4Addr::LOCALHOST, port));
    TcpStream::connect_timeout(&addr, Duration::from_millis(150)).is_ok()
}

fn read_server_version(base_dir: &Path) -> String {
    let mut current = Some(base_dir);
    while let Some(dir) = current {
        let candidate = dir.join("silvermere/Cargo.toml");
        if let Some(ver) = parse_cargo_version(&candidate) {
            return ver;
        }
        current = dir.parent();
    }

    if let Ok(home) = env::var("HOME") {
        let home_candidate = PathBuf::from(home).join("silvermere/Cargo.toml");
        if let Some(ver) = parse_cargo_version(&home_candidate) {
            return ver;
        }
    }

    "V ?".to_string()
}

fn parse_cargo_version(path: &Path) -> Option<String> {
    let content = fs::read_to_string(path).ok()?;
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("version") {
            if let Some((_, val)) = trimmed.split_once('=') {
                let clean = val.trim().trim_matches('"');
                return Some(format!("V {}", clean));
            }
        }
    }
    None
}

fn fetch_online_players(b: &Path, _: &Path) -> Option<Vec<String>> {
    let o = silvermere_command(b).args(["sysop", "who", "--json"]).output().ok()?;
    if !o.status.success() {
        return None;
    }
    let t = String::from_utf8_lossy(&o.stdout);
    let mut v = Vec::new();
    let mut r = t.as_ref();
    while let Some(i) = r.find("\"first_name\":\"") {
        let a = &r[i + 14..];
        let Some(e) = a.find('"') else { break };
        let fnm = &a[..e];
        r = &a[e + 1..];
        let lnm = r
            .find("\"last_name\":\"")
            .and_then(|j| {
                let b = &r[j + 13..];
                b.find('"').map(|k| &b[..k])
            })
            .unwrap_or("");
        v.push(format!("{} {}", fnm, lnm).trim().to_string());
    }
    Some(v)
}

fn update_ui_status(ui: &AppWindow) {
    let (binary, pid_file, log_file, online_file) = get_server_paths();
    let proc_alive = is_process_running(&pid_file);
    let port_open = is_port_listening(2323);
    let running = proc_alive && port_open;
    ui.set_is_running(running);

    if running {
        ui.set_status_text("Running".into());
        ui.set_status_color(slint::Color::from_rgb_u8(46, 204, 113).into());

        match fetch_online_players(&binary, &online_file) {
            Some(players) if players.is_empty() => {
                ui.set_online_header("Who's Online (0)".into());
                ui.set_online_players(ModelRc::new(VecModel::from(vec![
                    SharedString::from("No players online."),
                ])));
            }
            Some(players) => {
                let count = players.len();
                let shared: Vec<SharedString> = players.into_iter().map(SharedString::from).collect();
                ui.set_online_header(format!("Who's Online ({})", count).into());
                ui.set_online_players(ModelRc::new(VecModel::from(shared)));
            }
            None => {
                ui.set_online_header("Who's Online (0)".into());
                ui.set_online_players(ModelRc::new(VecModel::from(vec![
                    SharedString::from("Waiting for player list..."),
                ])));
            }
        }
    } else if proc_alive && !port_open {
        ui.set_status_text("Starting / Not Listening".into());
        ui.set_status_color(slint::Color::from_rgb_u8(243, 156, 18).into());
        ui.set_online_header("Who's Online (Port 2323 Closed)".into());
        ui.set_online_players(ModelRc::new(VecModel::from(vec![
            SharedString::from("Process running, waiting for port 2323..."),
        ])));
    } else {
        ui.set_status_text("Stopped".into());
        ui.set_status_color(slint::Color::from_rgb_u8(231, 76, 60).into());
        ui.set_online_header("Who's Online (Offline)".into());
        ui.set_online_players(ModelRc::new(VecModel::from(vec![
            SharedString::from("Server is stopped."),
        ])));
    }

    if ui.get_show_logs() {
        match fs::read_to_string(&log_file) {
            Ok(content) => {
                let all_lines: Vec<&str> = content.lines().collect();
                if all_lines.is_empty() {
                    ui.set_log_lines(ModelRc::new(VecModel::from(vec![
                        SharedString::from("server.log is currently empty."),
                    ])));
                } else {
                    let start = all_lines.len().saturating_sub(40);
                    let recent: Vec<SharedString> = all_lines[start..]
                        .iter()
                        .map(|&l| SharedString::from(l))
                        .collect();
                    ui.set_log_lines(ModelRc::new(VecModel::from(recent)));
                }
            }
            Err(_) => {
                ui.set_log_lines(ModelRc::new(VecModel::from(vec![
                    SharedString::from("No server.log file found yet."),
                ])));
            }
        }
    }
}

fn open_url(url: &str) -> std::io::Result<()> {
    #[cfg(target_os = "windows")]
    {
        new_command("cmd").args(["/C", "start", "", url]).spawn()?;
    }
    #[cfg(target_os = "macos")]
    {
        Command::new("open").arg(url).spawn()?;
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        Command::new("xdg-open").arg(url).spawn()?;
    }
    Ok(())
}

fn main() -> Result<(), slint::PlatformError> {
    let ui = AppWindow::new()?;
    update_ui_status(&ui);

    let (binary, _, _, _) = get_server_paths();
    let base_dir = binary.parent().unwrap_or_else(|| Path::new("."));
    ui.set_server_version(read_server_version(base_dir).into());

    let refresh_timer = Timer::default();
    let timer_ui = ui.as_weak();
    refresh_timer.start(TimerMode::Repeated, Duration::from_secs(2), move || {
        if let Some(ui) = timer_ui.upgrade() {
            update_ui_status(&ui);
        }
    });

    let ui_handle = ui.as_weak();
    ui.on_refresh_clicked(move || {
        if let Some(ui) = ui_handle.upgrade() {
            update_ui_status(&ui);

            let (binary, _, _, _) = get_server_paths();
            let base_dir = binary.parent().unwrap_or_else(|| Path::new("."));
            ui.set_server_version(read_server_version(base_dir).into());
        }
    });

    ui.on_exit_clicked(|| {
        let _ = slint::quit_event_loop();
    });

    let ui_handle = ui.as_weak();
    ui.on_webserver_clicked(move || {
        if let Err(e) = open_url("http://127.0.0.1:8088") {
            if let Some(ui) = ui_handle.upgrade() {
                ui.set_command_feedback(format!("Failed to open webserver: {}", e).into());
            }
        }
    });

    let ui_handle = ui.as_weak();
    ui.on_import_clicked(move || {
        let thread_ui = ui_handle.clone();
        thread::spawn(move || {
            let folder = rfd::FileDialog::new()
                .set_title("Select Data Folder to Import")
                .pick_folder();

            let Some(path) = folder else {
                let _ = slint::invoke_from_event_loop(move || {
                    if let Some(ui) = thread_ui.upgrade() {
                        ui.set_command_feedback("Folder import canceled.".into());
                    }
                });
                return;
            };

            let (binary, _, _, _) = get_server_paths();
            if !binary.exists() {
                let _ = slint::invoke_from_event_loop(move || {
                    if let Some(ui) = thread_ui.upgrade() {
                        ui.set_command_feedback("Error: silvermere binary not found.".into());
                    }
                });
                return;
            }

            let status_ui = thread_ui.clone();
            let path_display = path.display().to_string();
            let _ = slint::invoke_from_event_loop(move || {
                if let Some(ui) = status_ui.upgrade() {
                    ui.set_command_feedback(
                        format!("Importing from '{}'...", path_display).into(),
                    );
                }
            });

            let result = silvermere_command(&binary)
                .arg("import")
                .arg(&path)
                .output();

            let _ = slint::invoke_from_event_loop(move || {
                if let Some(ui) = thread_ui.upgrade() {
                    match result {
                        Ok(output) if output.status.success() => {
                            ui.set_command_feedback(
                                "Import complete! Assets placed in silvermere/data/assets.".into(),
                            );
                        }
                        Ok(output) => {
                            let err = String::from_utf8_lossy(&output.stderr);
                            let out = String::from_utf8_lossy(&output.stdout);
                            let details = if !err.trim().is_empty() {
                                err.trim()
                            } else {
                                out.trim()
                            };
                            ui.set_command_feedback(
                                format!("Import failed: {}", details).into(),
                            );
                        }
                        Err(e) => {
                            ui.set_command_feedback(
                                format!("Failed to run import: {}", e).into(),
                            );
                        }
                    }
                }
            });
        });
    });

    let ui_handle = ui.as_weak();
    ui.on_start_clicked(move || {
        let (binary, pid_file, log_file, _) = get_server_paths();
        if !binary.exists() {
            if let Some(ui) = ui_handle.upgrade() {
                ui.set_command_feedback(
                    format!("Error: {} not found.", binary.display()).into(),
                );
            }
            return;
        }
        if is_process_running(&pid_file) {
            return;
        }

        let Ok(stdout_log) = fs::File::create(&log_file) else { return };
        let stderr_log = stdout_log
            .try_clone()
            .map(Stdio::from)
            .unwrap_or_else(|_| Stdio::null());

        let base_dir = binary.parent().unwrap_or_else(|| Path::new("."));

        match silvermere_command(&binary)
            .current_dir(base_dir)
            .stdout(Stdio::from(stdout_log))
            .stderr(stderr_log)
            .spawn()
        {
            Ok(mut child) => {
                let _ = fs::write(&pid_file, child.id().to_string());

                let thread_ui = ui_handle.clone();
                thread::spawn(move || {
                    let _ = child.wait();
                    let _ = fs::remove_file(&pid_file);
                    let _ = slint::invoke_from_event_loop(move || {
                        if let Some(ui) = thread_ui.upgrade() {
                            update_ui_status(&ui);
                        }
                    });
                });
            }
            Err(e) => {
                if let Some(ui) = ui_handle.upgrade() {
                    ui.set_command_feedback(format!("Start failed: {}", e).into());
                }
            }
        }

        if let Some(ui) = ui_handle.upgrade() {
            update_ui_status(&ui);
        }
    });

    let ui_handle = ui.as_weak();
    ui.on_shutdown_clicked(move || {
        let (_, pid_file, _, online_file) = get_server_paths();
        if let Ok(pid_str) = fs::read_to_string(&pid_file) {
            if let Ok(pid) = pid_str.trim().parse::<u32>() {
                if cfg!(windows) {
                    let _ = new_command("taskkill")
                        .args(["/PID", &pid.to_string(), "/F"])
                        .status();
                } else {
                    let _ = new_command("kill").arg(pid.to_string()).status();
                }
                let _ = fs::remove_file(&pid_file);
                let _ = fs::remove_file(&online_file);
            }
        }

        if let Some(ui) = ui_handle.upgrade() {
            ui.set_command_feedback("Server shut down.".into());
            update_ui_status(&ui);
        }
    });

    let ui_handle = ui.as_weak();
    ui.on_grant_clicked(move |user, role| {
        let Some(ui) = ui_handle.upgrade() else { return };
        let user_trimmed = user.trim();
        let role_trimmed = role.trim();

        if user_trimmed.is_empty() || role_trimmed.is_empty() {
            ui.set_command_feedback("Please enter both a username and a role.".into());
            return;
        }

        let (binary, _, _, _) = get_server_paths();
        if !binary.exists() {
            ui.set_command_feedback("Error: silvermere binary not found.".into());
            return;
        }

        match silvermere_command(&binary)
            .args(["account", "grant", user_trimmed, role_trimmed])
            .output()
        {
            Ok(output) if output.status.success() => {
                let msg = format!("Granted '{}' to user '{}'.", role_trimmed, user_trimmed);
                ui.set_command_feedback(msg.into());
            }
            Ok(output) => {
                let err = String::from_utf8_lossy(&output.stderr);
                let msg = format!("Grant failed: {}", err.trim());
                ui.set_command_feedback(msg.into());
            }
            Err(e) => {
                ui.set_command_feedback(format!("Failed to run binary: {}", e).into());
            }
        }
    });

    let ui_handle = ui.as_weak();
    ui.on_revoke_clicked(move |user, role| {
        let Some(ui) = ui_handle.upgrade() else { return };
        let user_trimmed = user.trim();
        let role_trimmed = role.trim();

        if user_trimmed.is_empty() || role_trimmed.is_empty() {
            ui.set_command_feedback("Please enter both a username and a role to revoke.".into());
            return;
        }

        let (binary, _, _, _) = get_server_paths();
        if !binary.exists() {
            ui.set_command_feedback("Error: silvermere binary not found.".into());
            return;
        }

        match silvermere_command(&binary)
            .args(["account", "revoke", user_trimmed, role_trimmed])
            .output()
        {
            Ok(output) if output.status.success() => {
                let msg = format!("Revoked '{}' from user '{}'.", role_trimmed, user_trimmed);
                ui.set_command_feedback(msg.into());
            }
            Ok(output) => {
                let err = String::from_utf8_lossy(&output.stderr);
                let msg = format!("Revoke failed: {}", err.trim());
                ui.set_command_feedback(msg.into());
            }
            Err(e) => {
                ui.set_command_feedback(format!("Failed to run binary: {}", e).into());
            }
        }
    });

    let ui_handle = ui.as_weak();
    ui.on_delete_user_clicked(move |user| {
        let Some(ui) = ui_handle.upgrade() else { return };
        let user_trimmed = user.trim();

        if user_trimmed.is_empty() {
            ui.set_command_feedback("Please enter a username to delete.".into());
            return;
        }

        let (binary, _, _, _) = get_server_paths();
        if !binary.exists() {
            ui.set_command_feedback("Error: silvermere binary not found.".into());
            return;
        }

        match silvermere_command(&binary)
            .args(["account", "delete", user_trimmed, "--yes"])
            .output()
        {
            Ok(output) if output.status.success() => {
                let msg = format!("Deleted account '{}'.", user_trimmed);
                ui.set_command_feedback(msg.into());
            }
            Ok(output) => {
                let err = String::from_utf8_lossy(&output.stderr);
                let msg = format!("Delete failed: {}", err.trim());
                ui.set_command_feedback(msg.into());
            }
            Err(e) => {
                ui.set_command_feedback(format!("Failed to run binary: {}", e).into());
            }
        }
    });

    let ui_handle = ui.as_weak();
    ui.on_broadcast_clicked(move |msg| {
        let Some(ui) = ui_handle.upgrade() else { return };
        let msg_trimmed = msg.trim();

        if msg_trimmed.is_empty() {
            ui.set_command_feedback("Please enter a message to broadcast.".into());
            return;
        }

        let (binary, _, _, _) = get_server_paths();
        if !binary.exists() {
            ui.set_command_feedback("Error: silvermere binary not found.".into());
            return;
        }

        match silvermere_command(&binary)
            .args(["sysop", "broadcast", msg_trimmed])
            .output()
        {
            Ok(output) if output.status.success() => {
                ui.set_command_feedback(format!("Broadcast sent: \"{}\"", msg_trimmed).into());
            }
            Ok(output) => {
                let err = String::from_utf8_lossy(&output.stderr);
                ui.set_command_feedback(format!("Broadcast failed: {}", err.trim()).into());
            }
            Err(e) => {
                ui.set_command_feedback(format!("Failed to run binary: {}", e).into());
            }
        }
    });

    let ui_handle = ui.as_weak();
    ui.on_reload_world_clicked(move || {
        let Some(ui) = ui_handle.upgrade() else { return };
        let (binary, _, _, _) = get_server_paths();

        if !binary.exists() {
            ui.set_command_feedback("Error: silvermere binary not found.".into());
            return;
        }

        match silvermere_command(&binary)
            .args(["sysop", "reload-world"])
            .output()
        {
            Ok(output) if output.status.success() => {
                ui.set_command_feedback("World tables reloaded from database.".into());
            }
            Ok(output) => {
                let err = String::from_utf8_lossy(&output.stderr);
                ui.set_command_feedback(format!("Reload world failed: {}", err.trim()).into());
            }
            Err(e) => {
                ui.set_command_feedback(format!("Failed to run binary: {}", e).into());
            }
        }
    });

    ui.run()
}






