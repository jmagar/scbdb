use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "scbdb-cli")]
#[command(about = "SCBDB command line interface")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Debug, Subcommand)]
enum Commands {
    Collect,
    Regs,
    Report,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();
    match cli.command {
        Some(Commands::Collect) => println!("collect command scaffolded"),
        Some(Commands::Regs) => println!("regs command scaffolded"),
        Some(Commands::Report) => println!("report command scaffolded"),
        None => println!("scbdb-cli scaffold ready"),
    }

    Ok(())
}
