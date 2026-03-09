//! VProgs Wallet - Production-ready wallet for stealth addresses
//!
//! Usage:
//!   vprogs-wallet init                    # Initialize new wallet
//!   vprogs-wallet receive                 # Show meta-address
//!   vprogs-wallet balance                # Show balance
//!   vprogs-wallet transactions           # List transactions
//!   vprogs-wallet send <to> <amount>     # Send funds

use clap::{Parser, Subcommand};
use std::path::PathBuf;
use vprogs_node_transaction_submitter::WalletCLI;

#[derive(Parser)]
#[command(name = "vprogs-wallet")]
#[command(about = "VProgs Wallet - Private Kaspa transactions with stealth addresses")]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Wallet directory (default: ~/.vprogs-wallet)
    #[arg(short, long)]
    wallet_dir: Option<PathBuf>,

    /// Kaspa RPC URL (optional, uses default if not provided)
    #[arg(short, long)]
    rpc_url: Option<String>,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize a new wallet
    Init {
        /// Password for encrypting keys (optional, will prompt if not provided)
        #[arg(short, long)]
        password: Option<String>,
    },
    /// Show meta-address for receiving funds
    Receive {
        /// Password to unlock wallet (optional, will prompt if not provided)
        #[arg(short, long)]
        password: Option<String>,
    },
    /// Show balance
    Balance {
        /// Password to unlock wallet (optional, will prompt if not provided)
        #[arg(short, long)]
        password: Option<String>,
    },
    /// List transactions
    Transactions {
        /// Password to unlock wallet (optional, will prompt if not provided)
        #[arg(short, long)]
        password: Option<String>,
    },
    /// Create a backup of the wallet
    Backup {
        /// Password to unlock wallet (optional, will prompt if not provided)
        #[arg(short, long)]
        password: Option<String>,
    },
    /// Restore wallet from backup
    Restore {
        /// Path to backup file
        backup_path: PathBuf,
        /// Password to unlock wallet (optional, will prompt if not provided)
        #[arg(short, long)]
        password: Option<String>,
    },
    /// List available backups
    ListBackups,
    /// Send funds
    Send {
        /// Destination address
        to: String,
        /// Amount in sompi
        amount: u64,
        /// Source address
        from: String,
        /// Password to unlock wallet (optional, will prompt if not provided)
        #[arg(short, long)]
        password: Option<String>,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let cli = Cli::parse();

    // Determine wallet directory
    let wallet_dir = cli.wallet_dir.unwrap_or_else(|| {
        let mut dir = dirs::home_dir().expect("Failed to get home directory");
        dir.push(".vprogs-wallet");
        dir
    });

    // Create wallet CLI
    let wallet = WalletCLI::new(wallet_dir, cli.rpc_url.as_deref()).await?;

    // Helper to get password (prompt if not provided)
    let get_password = |provided: Option<String>,
                        prompt: &str|
     -> Result<String, Box<dyn std::error::Error>> {
        if let Some(pwd) = provided {
            Ok(pwd)
        } else {
            use rpassword::prompt_password;
            prompt_password(prompt).map_err(|e| format!("Failed to read password: {}", e).into())
        }
    };

    // Execute command
    match cli.command {
        Commands::Init { password } => {
            let pwd = get_password(password, "Enter password for wallet encryption: ")?;
            wallet.init(&pwd)?;
        }
        Commands::Receive { password } => {
            let pwd = get_password(password, "Enter wallet password: ")?;
            wallet.receive(&pwd)?;
        }
        Commands::Balance { password } => {
            let pwd = get_password(password, "Enter wallet password: ")?;
            wallet.balance(&pwd).await?;
        }
        Commands::Transactions { password } => {
            let pwd = get_password(password, "Enter wallet password: ")?;
            wallet.transactions(&pwd).await?;
        }
        Commands::Send { to, amount, from, password } => {
            let pwd = get_password(password, "Enter wallet password: ")?;
            wallet.send(&pwd, &to, amount, &from).await?;
        }
        Commands::Backup { password } => {
            let pwd = get_password(password, "Enter wallet password: ")?;
            wallet.backup(&pwd)?;
        }
        Commands::Restore { backup_path, password } => {
            let pwd = get_password(password, "Enter wallet password: ")?;
            wallet.restore(backup_path, &pwd)?;
        }
        Commands::ListBackups => {
            wallet.list_backups()?;
        }
    }

    Ok(())
}
