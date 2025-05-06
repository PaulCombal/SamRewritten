use interprocess::local_socket::{GenericFilePath, Name, ToFsName};
use crate::steam_client::steamworks_types::AppId_t;

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