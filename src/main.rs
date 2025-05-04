mod backend;
mod steam_client;
mod utils;
mod frontend;

use std::process::Command;

use gtk::glib;
use frontend::build_app::main_ui;
use utils::utils::get_executable_path;
use crate::backend::app::app;
use crate::utils::arguments::parse_arguments;
use crate::backend::orchestrator::orchestrator;

const APP_ID: &str = "org.paul_combal.sam_rewritten";

fn main() -> glib::ExitCode {
    let arguments = parse_arguments();

    if arguments.is_orchestrator {
        let exit_code = orchestrator();
        return glib::ExitCode::from(exit_code);
    }
    
    if arguments.is_app > 0 {
        let exit_code = app(arguments.is_app);
        return glib::ExitCode::from(exit_code);
    }

    let current_exe = get_executable_path();
    let orchestrator = Command::new(current_exe)
        .arg("--orchestrator")
        .spawn()
        .expect("Failed to spawn sam2 orchestrator process");

    main_ui(orchestrator)
}