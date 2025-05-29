// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2025 Paul <abonnementspaul (at) gmail.com>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, version 3.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

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
