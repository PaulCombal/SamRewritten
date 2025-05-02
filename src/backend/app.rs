use std::{env, io};
use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use interprocess::local_socket::{Listener, ListenerOptions, Stream};
use interprocess::local_socket::traits::ListenerExt;
use serde::Serialize;
use crate::backend::app_functions::{get_achievements, get_statistics, set_achievement};
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

fn send_response<T: Serialize>(conn: &mut BufReader<Stream>, response_data: SteamResponse<T>) -> io::Result<()> {
    let response_str = serde_json::to_string(&response_data)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("Serialization error: {e}")))?;
    conn.get_mut().write_all((response_str + "\n").as_bytes())?;
    conn.get_mut().flush()
}

fn initialize_listener(app_id: AppId_t) -> Result<Listener, i32> {
    let (socket_name_str, socket_name) = get_app_socket_path(app_id);
    let socket_path = Path::new(&socket_name_str);

    if socket_path.exists() {
        if let Err(e) = std::fs::remove_file(socket_path) {
            eprintln!("Warning: Failed to remove existing app socket file '{}': {}", socket_name_str, e);
            return Err(1);
        }
    }

    let opts = ListenerOptions::new().name(socket_name);
    match opts.create_sync() {
        Ok(listener) => Ok(listener),
        Err(e) if e.kind() == io::ErrorKind::AddrInUse => {
            eprintln!(
                "Error: Could not start server because the socket file is occupied. Please check \
                 if {} is in use by another process and try again.",
                socket_name_str
            );
            Err(1)
        }
        Err(e) => {
            eprintln!("Error: Failed to create listener for socket '{}': {}", socket_name_str, e);
            Err(1)
        }
    }
}

fn initialize_steam(app_id: AppId_t) -> Option<ConnectedSteam<'static>> {
    unsafe {
        env::set_var("SteamAppId", app_id.to_string());
    }
    
    match ConnectedSteam::new() {
        Ok(c) => Some(c),
        Err(e) => {
            dev_println!("Error connecting to Steam: {e}");
            None
        }
    }
}

pub fn app(app_id: AppId_t) -> i32 {
    let listener = initialize_listener(app_id).expect("Failed to create listener");
    let connected_steam = initialize_steam(app_id);
    let mut buffer = String::with_capacity(128); // Buffer should be large enough to hold a serialized SteamCommand

    for conn in listener.incoming().filter_map(handle_error) {
        dev_println!("[APP SERVER] Incoming connection");

        let mut conn = BufReader::new(conn);
        conn.read_line(&mut buffer).expect("Failed to read line");

        print!("[APP SERVER] Received: {buffer}");

        let command: SteamCommand = match serde_json::from_str(&buffer) {
            Ok(c) => {
                buffer.clear();
                c
            },
            Err(e) => {
                if buffer.len() == 0 {
                    dev_println!("[APP SERVER] Received empty message, ignoring");
                    continue;
                }
                
                buffer.clear();
                dev_println!("[APP SERVER] Error deserializing command: {e}");
                let response : SteamResponse<String> = SteamResponse::Error("Failed deserialize command".to_owned());
                send_response(&mut conn, response).expect("Failed to send response");
                continue;
            }
        };

        if connected_steam.as_ref().is_none() {
            let response : SteamResponse<String> = SteamResponse::Error("Failed to connect to Steam".to_owned());
            send_response(&mut conn, response).expect("Failed to send response");
            continue;
        }


        let utils = &connected_steam.as_ref().unwrap().utils;
        match utils.get_app_id() {
            Ok(id) => {
                if id != app_id {
                    dev_println!("[APP SERVER] App ID mismatch: {id} != {app_id}");
                    let response : SteamResponse<String> = SteamResponse::Error("App ID mismatch".to_owned());
                    send_response(&mut conn, response).expect("Failed to send response");
                    continue;
                }
            },
            Err(e) => {
                dev_println!("[APP SERVER] Error getting app ID: {e}");
                let response : SteamResponse<String> = SteamResponse::Error("Failed to get app ID".to_owned());
                send_response(&mut conn, response).expect("Failed to send response");
                continue;
            }
        }

        match command {
            SteamCommand::Status => {
                let response = SteamResponse::Success(true);
                send_response(&mut conn, response).expect("Failed to send response");
            }
            
            SteamCommand::Shutdown => {
                connected_steam.unwrap().shutdown();

                let response = SteamResponse::Success(true);
                send_response(&mut conn, response).expect("Failed to send response");
                break;
            }
            
            SteamCommand::GetAchievements(app_id_param) => {
                if app_id_param != app_id { 
                    dev_println!("[APP SERVER] App ID mismatch: {app_id_param} != {app_id}");
                    let response : SteamResponse<String> = SteamResponse::Error("App ID mismatch".to_owned());
                    send_response(&mut conn, response).expect("Failed to send response");
                    continue;
                }
                
                let achievements = get_achievements(app_id_param, connected_steam.as_ref().unwrap());
                let response = SteamResponse::Success(achievements);

                #[cfg(debug_assertions)]
                let response_str = serde_json::to_string(&response).unwrap();

                send_response(&mut conn, response).expect("Failed to send response");

                #[cfg(debug_assertions)]
                dev_println!("Achievements: {response_str}");
            }

            SteamCommand::GetStats(app_id_param) => {
                if app_id_param != app_id {
                    dev_println!("[APP SERVER] App ID mismatch: {app_id_param} != {app_id}");
                    let response : SteamResponse<String> = SteamResponse::Error("App ID mismatch".to_owned());
                    send_response(&mut conn, response).expect("Failed to send response");
                    continue;
                }

                let statistics = get_statistics(app_id_param, connected_steam.as_ref().unwrap());
                let response = SteamResponse::Success(statistics);

                #[cfg(debug_assertions)]
                let response_str = serde_json::to_string(&response).unwrap() + "\n";

                send_response(&mut conn, response).expect("Failed to send response");

                #[cfg(debug_assertions)]
                dev_println!("Statistics: {response_str}");
            }

            SteamCommand::SetAchievement(app_id_param, unlocked, achievement_id) => {
                if app_id_param != app_id {
                    dev_println!("[APP SERVER] App ID mismatch: {app_id_param} != {app_id}");
                    let response : SteamResponse<String> = SteamResponse::Error("App ID mismatch".to_owned());
                    send_response(&mut conn, response).expect("Failed to send response");
                    continue;
                }

                match set_achievement(connected_steam.as_ref().unwrap(), &achievement_id, unlocked) {
                    Ok(_) => {
                        let response = SteamResponse::Success(true);
                        send_response(&mut conn, response).expect("Failed to send response");
                    }
                    Err(e) => {
                        dev_println!("[APP SERVER] Error setting achievement: {e}");
                        let response = SteamResponse::Error::<bool>(e.to_string());
                        send_response(&mut conn, response).expect("Failed to send response");
                    }
                }
            }
            
            _ => {
                dev_println!("[APP SERVER] Received unknown command {command:?}");
                let response = SteamResponse::<()>::Error("Unknown command".to_owned());
                send_response(&mut conn, response).expect("Failed to send response");
            }
        }
    }

    dev_println!("[APP SERVER] Exiting");

    0
}