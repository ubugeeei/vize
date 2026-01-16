//! Check server command - Start JSON-RPC server for type checking

use clap::Args;
use vize_canon::{ServerConfig, TsgoServer};

#[derive(Args)]
pub struct CheckServerArgs {
    /// Unix socket path (if not specified, uses stdin/stdout)
    #[arg(long, short)]
    pub socket: Option<String>,

    /// Path to tsgo executable
    #[arg(long)]
    pub tsgo_path: Option<String>,

    /// Working directory
    #[arg(long)]
    pub working_dir: Option<String>,
}

pub fn run(args: CheckServerArgs) {
    let config = ServerConfig {
        tsgo_path: args.tsgo_path,
        working_dir: args.working_dir,
    };

    let mut server = TsgoServer::with_config(config);

    if let Some(socket_path) = args.socket {
        // Unix socket mode
        eprintln!("vize check-server: Starting on Unix socket");
        eprintln!("Socket: {}", socket_path);
        eprintln!("Methods: check, shutdown");
        eprintln!();
        eprintln!("Connect with:");
        eprintln!(
            r#"  echo '{{"jsonrpc":"2.0","id":1,"method":"check","params":{{...}}}}' | nc -U {}"#,
            socket_path
        );

        if let Err(e) = server.run_socket(&socket_path) {
            eprintln!("Server error: {}", e);
            std::process::exit(1);
        }
    } else {
        // stdio mode
        eprintln!("vize check-server: JSON-RPC server started (stdio mode)");
        eprintln!("Protocol: one JSON object per line on stdin, responses on stdout");
        eprintln!("Methods: check, shutdown");
        eprintln!();
        eprintln!("Tip: Use --socket for Unix socket mode (faster for multiple requests)");
        eprintln!();
        eprintln!("Example request:");
        eprintln!(
            r#"  {{"jsonrpc":"2.0","id":1,"method":"check","params":{{"uri":"test.vue","content":"<script setup>...</script>"}}}}"#
        );

        if let Err(e) = server.run() {
            eprintln!("Server error: {}", e);
            std::process::exit(1);
        }
    }
}
