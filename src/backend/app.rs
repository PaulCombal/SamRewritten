use std::{env, io};
use std::io::{BufRead, BufReader, Write};
use interprocess::local_socket::{ListenerOptions, Stream};
use interprocess::local_socket::traits::ListenerExt;
use crate::backend::app_functions::{get_achievements, get_statistics};
use crate::backend::connected_steam::ConnectedSteam;
use crate::dev_println;
use crate::frontend::ipc_process::get_app_socket_path;
use crate::steam_client::types::AppId_t;
use crate::utils::ipc_types::{SteamCommand, SteamResponse};

fn handle_error(conn: io::Result<Stream>) -> Option<Stream> {
    match conn {
        Ok(c) => Some(c),
        Err(e) => {
            dev_println!("Incoming connection failed: {e}");
            None
        }
    }
}

pub fn app(app_id: AppId_t) -> i32 {
    let (socket_name_str, socket_name) = get_app_socket_path(app_id);

    // Unlink the socket if it exists from a previous run
    if std::fs::metadata(&socket_name_str).is_ok() {
        std::fs::remove_file(&socket_name_str).expect("Failed to remove app socket file");
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
        x => x.expect("Failed to create listener"),
    };

    unsafe {
        env::set_var("SteamAppId", app_id.to_string().as_str());
    }

    let connected_steam = match ConnectedSteam::new() {
        Ok(c) => Some(c),
        Err(e) => {
            dev_println!("Error connecting to Steam: {e}");
            None
        }
    };

    // Buffer should be large enough to hold a serialized SteamCommand
    let mut buffer = String::with_capacity(128);

    for conn in listener.incoming().filter_map(handle_error) {
        dev_println!("[APP SERVER] Incoming connection");

        let mut conn = BufReader::new(conn);
        conn.read_line(&mut buffer).expect("Failed to read line");

        print!("[APP SERVER] Received: {buffer}");

        if connected_steam.as_ref().is_none() {
            let response : SteamResponse<String> = SteamResponse::Error("Failed to connect to Steam".to_owned());
            let response = serde_json::to_string(&response).unwrap() + "\n";
            conn.get_mut().write_all(response.as_bytes()).expect("failed to write");
            conn.get_mut().flush().expect("failed to flush");
            continue;
        }
        else {
            let utils = &connected_steam.as_ref().unwrap().utils;
            match utils.get_app_id() { 
                Ok(id) => {
                    if id != app_id {
                        dev_println!("[APP SERVER] App ID mismatch: {id} != {app_id}");
                        let response : SteamResponse<String> = SteamResponse::Error("App ID mismatch".to_owned());
                        let response = serde_json::to_string(&response).unwrap() + "\n";
                        conn.get_mut().write_all(response.as_bytes()).expect("failed to write");
                        conn.get_mut().flush().expect("failed to flush");
                        continue;
                    }
                },
                Err(e) => {
                    dev_println!("[APP SERVER] Error getting app ID: {e}");
                    let response : SteamResponse<String> = SteamResponse::Error("Failed to get app ID".to_owned());
                    let response = serde_json::to_string(&response).unwrap() + "\n";
                    conn.get_mut().write_all(response.as_bytes()).expect("failed to write");
                    conn.get_mut().flush().expect("failed to flush");
                    continue;
                }
            }
        }

        let command: SteamCommand = match serde_json::from_str(&buffer) {
            Ok(c) => c,
            Err(e) => {
                dev_println!("[APP SERVER] Error deserializing command: {e}");
                buffer.clear();
                continue;
            }
        };

        match command {
            SteamCommand::Shutdown => {
                connected_steam.unwrap().shutdown();

                let response = SteamResponse::Success(true);
                let response = serde_json::to_string(&response).unwrap() + "\n";
                conn.get_mut().write_all(response.as_bytes()).expect("failed to write");
                break;
            }
            
            SteamCommand::GetAchievements(app_id_param) => {
                if app_id_param != app_id { 
                    dev_println!("[APP SERVER] App ID mismatch: {app_id_param} != {app_id}");
                    let response : SteamResponse<String> = SteamResponse::Error("App ID mismatch".to_owned());
                    let response = serde_json::to_string(&response).unwrap() + "\n";
                    conn.get_mut().write_all(response.as_bytes()).expect("failed to write");
                    conn.get_mut().flush().expect("failed to flush");
                    continue;
                }
                
                let achievements = get_achievements(app_id_param, connected_steam.as_ref().unwrap());
                let response = SteamResponse::Success(achievements);
                let response = serde_json::to_string(&response).unwrap() + "\n";
                dev_println!("Achievements: {response}");
                conn.get_mut().write_all(response.as_bytes()).expect("failed to write");
                conn.get_mut().flush().expect("failed to flush");
            }

            SteamCommand::GetStats(app_id_param) => {
                if app_id_param != app_id {
                    dev_println!("[APP SERVER] App ID mismatch: {app_id_param} != {app_id}");
                    let response : SteamResponse<String> = SteamResponse::Error("App ID mismatch".to_owned());
                    let response = serde_json::to_string(&response).unwrap() + "\n";
                    conn.get_mut().write_all(response.as_bytes()).expect("failed to write");
                    conn.get_mut().flush().expect("failed to flush");
                    continue;
                }

                let statistics = get_statistics(app_id_param, connected_steam.as_ref().unwrap());
                let response = SteamResponse::Success(statistics);
                let response = serde_json::to_string(&response).unwrap() + "\n";
                dev_println!("Statistics: {response}");
                conn.get_mut().write_all(response.as_bytes()).expect("failed to write");
                conn.get_mut().flush().expect("failed to flush");
            }
            
            _ => {
                dev_println!("[APP SERVER] Received unknown command");
                let response = SteamResponse::<()>::Error("Unknown command".to_owned());
                let response = serde_json::to_string(&response).unwrap() + "\n";
                conn.get_mut().write_all(response.as_bytes()).expect("failed to write");
                conn.get_mut().flush().expect("failed to flush");
            }
        }

        buffer.clear();
    }

    dev_println!("[APP SERVER] Exiting");

    0
}