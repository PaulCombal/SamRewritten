#[cfg(test)]
mod tests {
    use std::io::{BufRead, BufReader, Write};
    use std::sync::{Mutex, OnceLock};
    use interprocess::local_socket::prelude::LocalSocketStream;
    use interprocess::local_socket::traits::Stream;
    use crate::backend::app_manager::AppManager;
    use crate::frontend::ipc_process::get_orchestrator_socket_path;
    use crate::utils::ipc_types::{SteamCommand};

    pub fn send_global_command(command: SteamCommand) -> String {
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

        conn.read_line(&mut buffer).expect("Failed to read line from client");

        format!("{buffer}")
    }

    #[test]
    fn get_owned_apps() {
        let res = send_global_command(SteamCommand::GetOwnedAppList);
        println!("Owned apps: {res}");
    }

    #[test]
    fn start_app() {
        let res = send_global_command(SteamCommand::LaunchApp(480));
        println!("App launched: {res}");
    }

    #[test]
    fn stop_apps() {
        let res = send_global_command(SteamCommand::StopApps);
        println!("Apps stopped: {res}");
    }
    
    #[test]
    fn get_achievements() {
        let res = send_global_command(SteamCommand::GetAchievements(480));
        println!("Achievements: {res}");
    }
    
    #[test]
    fn shutdown() {
        let res = send_global_command(SteamCommand::Shutdown);
        println!("Shutdown: {res}");
    }
    
    #[test]
    fn get_achievements_with_callback() {
        // let connected_steam = ConnectedSteam::new().expect("Failed to create connected steam");
        let mut app_manager = AppManager::new_connected(206690).expect("Failed to create app manager");
        let achievements = app_manager.get_achievements().expect("Failed to get achievements");
        println!("{achievements:?}")
    }
}