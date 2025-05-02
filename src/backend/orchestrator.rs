use std::collections::HashMap;
use std::io::{self, Error, ErrorKind};
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, Command};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};
use interprocess::local_socket::{ListenerOptions, Stream as StreamEnum};
use interprocess::local_socket::prelude::LocalSocketStream;
use interprocess::local_socket::traits::{ListenerExt, Stream};
use crate::backend::app_lister::AppLister;
use crate::backend::connected_steam::ConnectedSteam;
use crate::dev_println;
use crate::frontend::ipc_process::{get_app_socket_path, get_orchestrator_socket_path};
use crate::utils::ipc_types::{SteamCommand, SteamResponse};
use crate::utils::utils::get_executable_path;

fn handle_error(conn: io::Result<StreamEnum>) -> Option<StreamEnum> {
    match conn {
        Ok(c) => Some(c),
        Err(e) => {
            dev_println!("[ORCHESTRATOR] Incoming connection failed: {e}");
            None
        }
    }
}

fn is_app_running(app_id: u32) -> bool {
    let res_str = send_app_command(app_id, SteamCommand::Status);
    let deserialized: SteamResponse<bool> = serde_json::from_str(&res_str).expect("Failed to deserialize response");

    match deserialized {
        SteamResponse::Success(_) => true,
        _ => false
    }
}

fn send_app_command(app_id: u32, command: SteamCommand) -> String {
    static BUFFER: OnceLock<Mutex<String>> = OnceLock::new();
    const INITIAL_CAPACITY: usize = 1024 * 1024;
    let buffer_mutex = BUFFER.get_or_init(|| Mutex::new(String::with_capacity(INITIAL_CAPACITY)));
    let mut buffer = buffer_mutex.lock().unwrap();
    buffer.clear();

    let (_, socket_name) = get_app_socket_path(app_id);

    let stream = match LocalSocketStream::connect(socket_name) {
        Ok(s) => s,
        Err(e) => {
            dev_println!("[ORCHESTRATOR] Failed to connect to app socket {app_id}: {e}");
            let response : SteamResponse<()> = SteamResponse::Error(format!("Failed to connect to app: {e}"));
            return serde_json::to_string(&response).unwrap() + "\n";
        }
    };

    let mut conn = BufReader::new(stream);

    let message = serde_json::to_string(&command).expect("Failed to serialize command") + "\n";
    conn.get_mut().write_all(message.as_bytes()).expect("Failed to send command from client");
    conn.read_line(&mut buffer).expect("Failed to read line from client");
    buffer.to_owned()
}

pub fn orchestrator() -> i32 {
    let (socket_name_str, socket_name) = get_orchestrator_socket_path();

    // Unlink the socket if it exists from a previous run
    if std::fs::metadata(&socket_name_str).is_ok() {
        std::fs::remove_file(&socket_name_str).expect("Failed to remove socket file");
    }

    let opts = ListenerOptions::new().name(socket_name);

    // ...then create it.
    let listener = match opts.create_sync() {
        Err(e) if e.kind() == io::ErrorKind::AddrInUse => {
            eprintln!(
                "Error: could not start server because the socket file is occupied. Please check
                if {socket_name_str} is in use by another process and try again."
            );

            return 1;
        }
        x => x.expect("Failed to creare"),
    };

    let connected_steam = match ConnectedSteam::new() {
        Ok(c) => Some(c),
        Err(e) => {
            dev_println!("[ORCHESTRATOR] Error connecting to Steam: {e}");
            None
        }
    };

    let mut children_processes : HashMap<u32, Child> = HashMap::new();

    // Buffer should be large enough to hold a serialized SteamCommand
    let mut buffer = String::with_capacity(128);

    for conn in listener.incoming().filter_map(handle_error) {
        dev_println!("[ORCHESTRATOR] Incoming connection");

        let mut conn = BufReader::new(conn);
        conn.read_line(&mut buffer).expect("Failed to read line");

        print!("[ORCHESTRATOR] Received: {buffer}");

        if connected_steam.as_ref().is_none() {
            let response : SteamResponse<String> = SteamResponse::Error("Failed to connect to Steam".to_owned());
            let response = serde_json::to_string(&response).unwrap() + "\n";
            conn.get_mut().write_all(response.as_bytes()).expect("failed to write");
            conn.get_mut().flush().expect("failed to flush");
            continue;
        }

        let command: SteamCommand = match serde_json::from_str(&buffer) {
            Ok(c) => c,
            Err(e) => {
                dev_println!("[ORCHESTRATOR] Error deserializing command: {e}");
                buffer.clear();
                continue;
            }
        };

        match command {
            SteamCommand::GetOwnedAppList => {
                dev_println!("[ORCHESTRATOR] Received GetOwnedAppList");
                let cs = connected_steam.as_ref().unwrap();
                let apps_001 = &cs.apps_001;
                let apps = &cs.apps;
                let app_lister = AppLister::new(apps_001, apps);

                match app_lister.get_owned_apps() {
                    Ok(apps) => {
                        let response = SteamResponse::Success(apps);
                        let response = serde_json::to_string(&response).unwrap() + "\n";
                        conn.get_mut().write_all(response.as_bytes()).expect("failed to write");
                    }
                    Err(e) => {
                        dev_println!("[ORCHESTRATOR] Error getting owned apps: {e}");
                        let response = SteamResponse::<()>::Error("Error".to_owned());
                        let response = serde_json::to_string(&response).unwrap() + "\n";
                        conn.get_mut().write_all(response.as_bytes()).expect("failed to write");
                        conn.get_mut().flush().expect("failed to flush");
                    }
                };
            }
            
            SteamCommand::LaunchApp(app_id) => {
                dev_println!("[ORCHESTRATOR] LaunchApp {}", app_id);

                // 1. Check if we own a process for this app
                if children_processes.contains_key(&app_id) {
                    dev_println!("[ORCHESTRATOR] App {} is already running", app_id);
                    let response: SteamResponse<()> = SteamResponse::Error("App is already running".to_owned());
                    let response = serde_json::to_string(&response).unwrap() + "\n";
                    conn.get_mut().write_all(response.as_bytes()).expect("failed to write");
                    continue;
                }

                // 2. Check if a process is running.
                if is_app_running(app_id) {
                    dev_println!("[ORCHESTRATOR] App {} is already running", app_id);
                    let response: SteamResponse<()> = SteamResponse::Error("App is already running in another process".to_owned());
                    let response = serde_json::to_string(&response).unwrap() + "\n";
                    conn.get_mut().write_all(response.as_bytes()).expect("failed to write");
                    continue;
                }

                // 3. Launch the process
                let current_exe = get_executable_path();
                let mut child = Command::new(current_exe)
                    .arg(format!("--app={app_id}"))
                    .spawn()
                    .expect("Failed to spawn sam2 orchestrator process");
                
                // 4. Wait for the socket to allow connections 
                let (.., app_socket_name) = get_app_socket_path(app_id);
                let start = Instant::now();

                loop {
                    match LocalSocketStream::connect(app_socket_name.clone()) {
                        Ok(..) => break,
                        Err(..) if start.elapsed() < Duration::from_secs(2) => {
                            std::thread::sleep(Duration::from_millis(500));
                        },
                        Err(error) => {
                            dev_println!("[ORCHESTRATOR] Failed to connect to socket for app {app_id}: {error}");
                            child.kill().expect("Failed to kill child process");
                            child.wait().expect("Failed to wait for child process");
                            let response: SteamResponse<()> = SteamResponse::Error("Failed to connect to socket".to_owned());
                            let response = serde_json::to_string(&response).unwrap() + "\n";
                            conn.get_mut().write_all(response.as_bytes()).expect("failed to write");
                            continue;
                        }
                    }
                }

                // All good!
                children_processes.insert(app_id, child);
                let response = SteamResponse::Success(true);
                let response = serde_json::to_string(&response).unwrap() + "\n";
                conn.get_mut().write_all(response.as_bytes()).expect("failed to write");
            }
            
            SteamCommand::StopApp(app_id) => {
                dev_println!("[ORCHESTRATOR] StopApp {}", app_id);
                if !children_processes.contains_key(&app_id) {  
                    dev_println!("[ORCHESTRATOR] App {} is not running", app_id);
                    let response: SteamResponse<()> = SteamResponse::Error("App is not running".to_owned());
                    let response = serde_json::to_string(&response).unwrap() + "\n";
                    conn.get_mut().write_all(response.as_bytes()).expect("failed to write");
                    continue;
                }
                
                children_processes.remove(&app_id);
                let response= send_app_command(app_id, SteamCommand::Shutdown);
                conn.get_mut().write_all(response.as_bytes()).expect("failed to write");
            }

            SteamCommand::StopApps => {
                dev_println!("[ORCHESTRATOR] StopApps");
                
                for (app_id, child) in children_processes.iter_mut() {
                    let response= send_app_command(*app_id, SteamCommand::Shutdown);
                    dev_println!("[ORCHESTRATOR] Sending shutdown command to app {app_id}: {response}");
                    child.wait().expect("failed to wait");
                }

                children_processes.clear();

                let response = SteamResponse::Success(true);
                let response = serde_json::to_string(&response).unwrap() + "\n";
                conn.get_mut().write_all(response.as_bytes()).expect("failed to write");
            }
            
            SteamCommand::Shutdown => {
                for (app_id, child) in children_processes.iter_mut() {
                    let response= send_app_command(*app_id, SteamCommand::Shutdown);
                    dev_println!("[ORCHESTRATOR] Sending shutdown command to app {app_id}: {response}");
                    child.wait().expect("failed to wait");
                }
                
                connected_steam.unwrap().shutdown();

                let response = SteamResponse::Success(true);
                let response = serde_json::to_string(&response).unwrap() + "\n";
                conn.get_mut().write_all(response.as_bytes()).expect("failed to write");
                break;
            }
            
            SteamCommand::Status => {
                let response = SteamResponse::Success(true);
                let response = serde_json::to_string(&response).unwrap() + "\n";
                conn.get_mut().write_all(response.as_bytes()).expect("failed to write");
            }
            
            SteamCommand::GetAchievements(app_id) => {
                let response= send_app_command(app_id, SteamCommand::GetAchievements(app_id));
                conn.get_mut().write_all(response.as_bytes()).expect("failed to write");
            }

            SteamCommand::GetStats(app_id) => {
                let response= send_app_command(app_id, SteamCommand::GetStats(app_id));
                conn.get_mut().write_all(response.as_bytes()).expect("failed to write");
            }
        }

        buffer.clear();
    }

    dev_println!("[ORCHESTRATOR] Exiting");

    0
}