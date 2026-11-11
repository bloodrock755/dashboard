use std::process::{Command, Stdio};
use std::io::{self, Write};
use std::fs;
use std::path::PathBuf;
use std::env;

fn get_server_paths() -> (PathBuf, PathBuf, PathBuf) {
    let mut base_dir = env::current_dir().expect("Failed to determine current directory");
    
    // If running via `cargo run`, it executes from the project root anyway,
    // but this ensures we stay anchored to the folder containing 'silvermere'
    let binary = base_dir.join("silvermere");
    let pid_file = base_dir.join("silvermere.pid");
    let log_file = base_dir.join("server.log");
    
    (binary, pid_file, log_file)
}

fn main() {
    loop {
        println!("\n=== Silvermere Control Panel ===");
        println!("1. Start Server");
        println!("2. Shutdown Server");
        println!("3. Exit");
        print!("Choose an option: ");
        io::stdout().flush().unwrap();

        let mut choice = String::new();
        io::stdin().read_line(&mut choice).unwrap();

        match choice.trim() {
            "1" => start_server(),
            "2" => shutdown_server(),
            "3" => break,
            _ => println!("Invalid option, please try again."),
        }
    }
}

fn start_server() {
    let (binary, pid_file, log_file) = get_server_paths();

    if !binary.exists() {
        println!("❌ Error: Could not find the './silvermere' executable in this folder!");
        return;
    }

    if pid_file.exists() {
        println!("⚠️ Silvermere appears to already be running (PID file exists).");
        return;
    }

    println!("🚀 Starting Silvermere server...");

    let log = fs::File::create(log_file).expect("Failed to create log file");

    // Setting the current_dir on the command ensures the server knows where its own assets are
    match Command::new(&binary)
        .current_dir(env::current_dir().unwrap())
        .stdout(Stdio::from(log))
        .stderr(Stdio::null()) 
        .spawn() 
    {
        Ok(child) => {
            let pid = child.id();
            fs::write(pid_file, pid.to_string()).expect("Failed to write PID file");
            println!("✅ Server started successfully in background (PID: {}).", pid);
        }
        Err(e) => {
            println!("❌ Failed to start server: {}", e);
        }
    }
}

fn shutdown_server() {
    let (_, pid_file, _) = get_server_paths();

    if !pid_file.exists() {
        println!("⚠️ No running server detected (missing PID file).");
        return;
    }

    let pid_str = fs::read_to_string(&pid_file).expect("Failed to read PID file");
    let pid: u32 = match pid_str.trim().parse() {
        Ok(num) => num,
        Err(_) => {
            println!("❌ Invalid PID found in file.");
            return;
        }
    };

    println!("🛑 Shutting down Silvermere server (PID {})...", pid);

    match Command::new("kill")
        .arg(pid.to_string())
        .status() 
    {
        Ok(status) if status.success() => {
            println!("✅ Shutdown signal sent.");
            let _ = fs::remove_file(pid_file);
        }
        _ => {
            println!("❌ Failed to shut down process cleanly.");
        }
    }
}
