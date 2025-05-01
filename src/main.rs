mod backend;
mod steam_client;
mod utils;
mod frontend;

use gtk::prelude::*;
use gtk::{glib};
use crate::backend::app::app;
use crate::utils::arguments::parse_arguments;
use crate::backend::orchestrator::orchestrator;
use crate::frontend::build_app::build_app;

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

    // GUI
    let app = build_app();
    app.run()
}