use std::io::{BufRead, BufReader, Write};
use std::sync::{Mutex, OnceLock};
use interprocess::local_socket::prelude::LocalSocketStream;
use interprocess::local_socket::{GenericFilePath, Name, ToFsName};
use interprocess::local_socket::traits::Stream;
use serde::{Deserialize, Serialize};
use crate::steam_client::types::AppId_t;
use crate::utils::ipc_types::{SteamCommand, SteamResponse};

pub fn send_global_command<T: for<'a> Deserialize<'a> + Serialize>(command: SteamCommand) -> SteamResponse<T> {
    static BUFFER: OnceLock<Mutex<String>> = OnceLock::new();
    const INITIAL_CAPACITY: usize = 1024 * 1024;
    let buffer_mutex = BUFFER.get_or_init(|| Mutex::new(String::with_capacity(INITIAL_CAPACITY)));
    let mut buffer = buffer_mutex.lock().unwrap();
    buffer.clear();
    
    let (_, socket_name) = get_orchestrator_socket_path();

    let stream = LocalSocketStream::connect(socket_name).expect("Failed to connect to backend");

    let mut conn = BufReader::new(stream);

    let message = serde_json::to_string(&command).expect("Failed to serialize command") + "\n";
    conn.get_mut().write_all(message.as_bytes()).expect("Failed to send command from client");
    // conn.get_mut().write_all(b"Hello from client!\n").expect("Failed to send command hello");

    conn.read_line(&mut buffer).expect("Failed to read line from client");

    // print!("[CLIENT] Response: {buffer}");
    
    let deserialized: SteamResponse<T> = serde_json::from_str(&buffer).expect("Failed to deserialize response");
    
    deserialized
}

#[cfg(target_os = "linux")]
pub fn get_app_socket_path(app_id: AppId_t) -> (String, Name<'static>)
{
    let socket_name_str = format!("/tmp/sam2.app.{app_id}.sock");
    let socket_name = socket_name_str.clone().to_fs_name::<GenericFilePath>().expect("Name conversion failed");
    (socket_name_str, socket_name)
}

#[cfg(target_os = "windows")]
pub fn get_app_socket_path(app_id: AppId_t) -> (String, Name<'static>)
{
    let socket_name_str = format!("\\\\.\\pipe\\sam2.app.{app_id}.sock");
    let socket_name = socket_name_str.clone().to_fs_name::<GenericFilePath>().expect("Name conversion failed");
    (socket_name_str, socket_name)
}

#[cfg(target_os = "linux")]
pub fn get_orchestrator_socket_path() -> (String, Name<'static>)
{
    let socket_name_str = "/tmp/sam2.orchestrator.sock".to_string();
    let socket_name = socket_name_str.clone().to_fs_name::<GenericFilePath>().expect("Name conversion failed");
    (socket_name_str, socket_name)
}

#[cfg(target_os = "windows")]
pub fn get_orchestrator_socket_path() -> (String, Name<'static>)
{
    let socket_name_str = "\\\\.\\pipe\\sam2.orchestrator.sock".to_string();
    let socket_name = socket_name_str.clone().to_fs_name::<GenericFilePath>().expect("Name conversion failed");
    (socket_name_str, socket_name)
}