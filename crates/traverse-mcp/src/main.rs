mod stdio_server;

use std::process::ExitCode;

fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    let Some(command) = args.next() else {
        eprintln!("Usage: traverse-mcp stdio [--simulate-startup-failure]");
        return ExitCode::from(1);
    };

    if command != "stdio" {
        eprintln!("Unsupported command: {command}");
        return ExitCode::from(1);
    }

    let simulate_startup_failure = args.any(|argument| argument == "--simulate-startup-failure");
    match stdio_server::run_stdio_server(simulate_startup_failure) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("traverse-mcp stdio server failed: {error:?}");
            ExitCode::from(1)
        }
    }
}
