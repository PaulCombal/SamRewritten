use std::{io};
use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use interprocess::local_socket::{Listener, ListenerOptions, Stream};
use interprocess::local_socket::traits::ListenerExt;
use serde::Serialize;
use crate::backend::app_manager::AppManager;
use crate::backend::stat_definitions::{AchievementInfo, StatInfo};
use crate::dev_println;
use crate::frontend::ipc_process::get_app_socket_path;
use crate::steam_client::steamworks_types::AppId_t;
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

pub fn app(app_id: AppId_t) -> i32 {
    let listener = initialize_listener(app_id).expect("Failed to create listener");
    let mut app_manager = AppManager::new_connected(app_id);

    #[cfg(debug_assertions)]
    if app_manager.as_ref().is_err() {
        dev_println!("[APP SERVER] Failed to connect to Steam");
    }

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

        if app_manager.as_ref().is_err() {
            let response : SteamResponse<String> = SteamResponse::Error("Failed to connect to Steam".to_owned());
            send_response(&mut conn, response).expect("Failed to send response");
            continue;
        }
        
        let app_manager = app_manager.as_mut().unwrap();

        match command {
            SteamCommand::Status => {
                let response = SteamResponse::Success(true);
                send_response(&mut conn, response).expect("Failed to send response");
            }
            
            SteamCommand::Shutdown => {
                app_manager.disconnect();

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
                
                let response = match app_manager.get_achievements() {
                    Ok(achievements) => {
                        SteamResponse::Success(achievements)
                    }
                    Err(e) => {
                        SteamResponse::Error::<Vec<AchievementInfo>>(e.to_string())
                    }
                };
                
                send_response(&mut conn, response).expect("Failed to send response");
            }

            SteamCommand::GetStats(app_id_param) => {
                if app_id_param != app_id {
                    dev_println!("[APP SERVER] App ID mismatch: {app_id_param} != {app_id}");
                    let response : SteamResponse<String> = SteamResponse::Error("App ID mismatch".to_owned());
                    send_response(&mut conn, response).expect("Failed to send response");
                    continue;
                }

                let response = match app_manager.get_statistics() {
                    Ok(achievements) => {
                        SteamResponse::Success(achievements)
                    }
                    Err(e) => {
                        SteamResponse::Error::<Vec<StatInfo>>(e.to_string())
                    }
                };

                #[cfg(debug_assertions)]
                let response_str = serde_json::to_string(&response).unwrap();
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

                let response = match app_manager.set_achievement(&achievement_id, unlocked) {
                    Ok(_) => {
                        SteamResponse::Success(true)
                    }
                    Err(e) => {
                        dev_println!("[APP SERVER] Error setting achievement: {e}");
                        SteamResponse::Error::<bool>(e.to_string())
                    }
                };

                send_response(&mut conn, response).expect("Failed to send response");
            }
            
            SteamCommand::SetIntStat(app_id_param, stat_id, value) => {
                if app_id_param != app_id {
                    dev_println!("[APP SERVER] App ID mismatch: {app_id_param} != {app_id}");
                    let response : SteamResponse<String> = SteamResponse::Error("App ID mismatch".to_owned());
                    send_response(&mut conn, response).expect("Failed to send response");
                    continue;
                }
                
                let response = match app_manager.set_stat_i32(&stat_id, value) {
                    Ok(result) => {
                        SteamResponse::Success(result)
                    }
                    Err(e) => {
                        dev_println!("[APP SERVER] Error setting int stat: {e}");
                        SteamResponse::Error::<bool>(e.to_string())
                    }
                };

                send_response(&mut conn, response).expect("Failed to send response");
            }

            SteamCommand::SetFloatStat(app_id_param, stat_id, value) => {
                if app_id_param != app_id {
                    dev_println!("[APP SERVER] App ID mismatch: {app_id_param} != {app_id}");
                    let response : SteamResponse<String> = SteamResponse::Error("App ID mismatch".to_owned());
                    send_response(&mut conn, response).expect("Failed to send response");
                    continue;
                }

                let response = match app_manager.set_stat_f32(&stat_id, value) {
                    Ok(result) => {
                        SteamResponse::Success(result)
                    }
                    Err(e) => {
                        dev_println!("[APP SERVER] Error setting float stat: {e}");
                        SteamResponse::Error::<bool>(e.to_string())
                    }
                };

                send_response(&mut conn, response).expect("Failed to send response");
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