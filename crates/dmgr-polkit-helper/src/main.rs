use dmgr_core::{control, properties};
use std::env;
use std::process;

fn usage() {
    eprintln!("dmgr-polkit-helper - Privileged device manager operations");
    eprintln!();
    eprintln!("Usage:");
    eprintln!("  dmgr-polkit-helper bind <device_path> <driver>");
    eprintln!("  dmgr-polkit-helper unbind <device_path>");
    eprintln!("  dmgr-polkit-helper set <device_path> <property> <value>");
}

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        usage();
        process::exit(1);
    }

    let result = match args[1].as_str() {
        "bind" => {
            if args.len() != 4 {
                eprintln!("Usage: dmgr-polkit-helper bind <device_path> <driver>");
                process::exit(1);
            }
            control::bind_driver(&args[2], &args[3])
        }
        "unbind" => {
            if args.len() != 3 {
                eprintln!("Usage: dmgr-polkit-helper unbind <device_path>");
                process::exit(1);
            }
            control::unbind_driver(&args[2])
        }
        "set" => {
            if args.len() != 5 {
                eprintln!("Usage: dmgr-polkit-helper set <device_path> <property> <value>");
                process::exit(1);
            }
            properties::set_property(&args[2], &args[3], &args[4])
        }
        _ => {
            eprintln!("Unknown command: {}", args[1]);
            usage();
            process::exit(1);
        }
    };

    match result {
        Ok(()) => {
            println!("OK");
            process::exit(0);
        }
        Err(e) => {
            eprintln!("Error: {}", e);
            process::exit(1);
        }
    }
}
