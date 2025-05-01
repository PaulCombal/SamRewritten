use std::env;
use crate::dev_println;

#[derive(Debug)]
pub struct Arguments {
    pub is_orchestrator: bool,
    pub is_app: u32,
}
pub fn parse_arguments() -> Arguments {
    let mut args = Arguments {
        is_orchestrator: false,
        is_app: 0,
    };

    for (index, arg) in env::args().enumerate() {
        if index == 0 {
            // Self binary name
            continue;
        }

        match arg.as_str() {
            "--orchestrator" => {
                args.is_orchestrator = true;
                continue;
            }
            _ => {
                let split: Vec<&str> = arg.split("=").collect();
                if split.len() != 2 {
                    continue;
                }

                let key = split[0];
                let value = split[1];

                if value.len() == 0 {
                    continue;
                }

                if key == "--app" {
                    args.is_app = value.parse::<u32>().unwrap();
                }
            }
        }
    }

    dev_println!("New process launched with arguments: {:?}", args);

    args
}
