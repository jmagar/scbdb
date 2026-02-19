mod collect;

use clap::{Parser, Subcommand};
use tracing_subscriber::EnvFilter;

#[derive(Debug, Parser)]
#[command(name = "scbdb-cli")]
#[command(about = "SCBDB command line interface")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Debug, Subcommand)]
pub enum CollectCommands {
    /// Collect full product catalog and variant data from all active brands
    Products {
        /// Restrict collection to a specific brand (by slug)
        #[arg(long)]
        brand: Option<String>,

        /// Preview what would be collected without writing to the database
        #[arg(long)]
        dry_run: bool,
    },
    /// Capture pricing snapshots for products already in the database
    Pricing {
        /// Restrict snapshots to a specific brand (by slug)
        #[arg(long)]
        brand: Option<String>,
    },
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Collect product and pricing data from Shopify storefronts
    Collect {
        #[command(subcommand)]
        command: CollectCommands,
    },
    /// Track regulatory filings and legislative activity (Phase 3)
    Regs,
    /// Generate reports and exports (Phase 5)
    Report,
    /// Database management commands
    Db {
        #[command(subcommand)]
        command: DbCommands,
    },
}

#[derive(Debug, Subcommand)]
enum DbCommands {
    /// Test the database connection
    Ping,
    /// Run pending migrations
    Migrate,
    /// Seed brands from config/brands.yaml
    Seed,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    let env_filter = EnvFilter::try_from_default_env().or_else(|_| {
        let level = std::env::var("SCBDB_LOG_LEVEL").unwrap_or_else(|_| "info".to_string());
        EnvFilter::try_new(level)
    })?;
    tracing_subscriber::fmt().with_env_filter(env_filter).init();

    let cli = Cli::parse();
    match cli.command {
        Some(Commands::Collect { command }) => match command {
            CollectCommands::Products { brand, dry_run } => {
                let config = load_config_or_exit();
                let pool = connect_or_exit().await;
                collect::run_collect_products(&pool, &config, brand.as_deref(), dry_run).await?;
            }
            CollectCommands::Pricing { brand } => {
                let config = load_config_or_exit();
                let pool = connect_or_exit().await;
                collect::run_collect_pricing(&pool, &config, brand.as_deref()).await?;
            }
        },
        Some(Commands::Regs) => {
            eprintln!("error: `regs` command is not yet implemented (Phase 3)");
            std::process::exit(1);
        }
        Some(Commands::Report) => {
            eprintln!("error: `report` command is not yet implemented (Phase 5)");
            std::process::exit(1);
        }
        Some(Commands::Db { command }) => match command {
            DbCommands::Ping => run_db_health_check().await?,
            DbCommands::Migrate => run_db_migrate().await?,
            DbCommands::Seed => run_db_seed().await?,
        },
        None => println!("scbdb-cli scaffold ready"),
    }

    Ok(())
}

async fn run_db_health_check() -> anyhow::Result<()> {
    let pool = connect_or_exit().await;
    scbdb_db::health_check(&pool).await?;
    println!("database is healthy");
    Ok(())
}

async fn run_db_migrate() -> anyhow::Result<()> {
    let pool = connect_or_exit().await;
    let applied = scbdb_db::run_migrations(&pool).await?;
    if applied == 0 {
        println!("0 pending migrations — database is up to date");
    } else {
        println!("applied {applied} migration(s) successfully");
    }
    Ok(())
}

async fn run_db_seed() -> anyhow::Result<()> {
    let config = load_config_or_exit();
    let brands_file = scbdb_core::load_brands(&config.brands_path).unwrap_or_else(|e| {
        eprintln!("error: failed to load brands config: {e}");
        std::process::exit(1);
    });
    let pool_config = scbdb_db::PoolConfig::from_app_config(&config);
    let pool = scbdb_db::connect_pool(&config.database_url, pool_config)
        .await
        .unwrap_or_else(|e| {
            eprintln!("error: failed to connect to database: {e}");
            eprintln!("hint: ensure postgres is running (just db-up)");
            std::process::exit(1);
        });
    let count = scbdb_db::seed::seed_brands(&pool, &brands_file.brands).await?;
    println!("seeded {count} brands");
    Ok(())
}

fn load_config_or_exit() -> scbdb_core::AppConfig {
    scbdb_core::load_app_config().unwrap_or_else(|e| {
        eprintln!("error: invalid configuration: {e}");
        std::process::exit(1);
    })
}

async fn connect_or_exit() -> sqlx::PgPool {
    scbdb_db::connect_pool_from_env().await.unwrap_or_else(|e| {
        match &e {
            scbdb_db::DbError::MissingDatabaseUrl => {
                eprintln!("error: DATABASE_URL is not set");
                eprintln!("hint: copy .env.example to .env and set DATABASE_URL");
            }
            scbdb_db::DbError::Sqlx(sql_err) => {
                eprintln!("error: failed to connect to database: {sql_err}");
                eprintln!("hint: ensure postgres is running (just db-up)");
            }
            // TODO: DbError::Migration and DbError::NotFound are never returned by
            // connect_pool_from_env; move them to dedicated helpers when needed.
            scbdb_db::DbError::Migration(ref mig_err) => {
                eprintln!("error: unexpected migration error during connect: {mig_err}");
            }
            scbdb_db::DbError::NotFound => {
                eprintln!("error: unexpected not-found during connect");
            }
            scbdb_db::DbError::Config(cfg_err) => {
                eprintln!("error: invalid configuration: {cfg_err}");
                eprintln!("hint: copy .env.example to .env and fill required values");
            }
            scbdb_db::DbError::InvalidCollectionRunTransition { id, expected_status } => {
                eprintln!(
                    "error: unexpected collection run state for id {id}: expected '{expected_status}'"
                );
            }
        }
        std::process::exit(1);
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_db_ping_command() {
        let cli =
            Cli::try_parse_from(["scbdb-cli", "db", "ping"]).expect("expected valid cli args");

        assert!(matches!(
            cli.command,
            Some(Commands::Db {
                command: DbCommands::Ping
            })
        ));
    }

    #[test]
    fn parses_db_migrate_command() {
        let cli =
            Cli::try_parse_from(["scbdb-cli", "db", "migrate"]).expect("expected valid cli args");

        assert!(matches!(
            cli.command,
            Some(Commands::Db {
                command: DbCommands::Migrate
            })
        ));
    }

    #[test]
    fn parses_db_seed_command() {
        let cli =
            Cli::try_parse_from(["scbdb-cli", "db", "seed"]).expect("expected valid cli args");

        assert!(matches!(
            cli.command,
            Some(Commands::Db {
                command: DbCommands::Seed
            })
        ));
    }

    #[test]
    fn no_command_is_none() {
        let cli = Cli::try_parse_from(["scbdb-cli"]).expect("expected valid cli args");
        assert!(cli.command.is_none());
    }

    #[test]
    fn test_collect_products_no_filter_defaults_to_all_brands() {
        let cli = Cli::try_parse_from(["scbdb", "collect", "products"]).unwrap();
        assert!(matches!(
            cli.command,
            Some(Commands::Collect {
                command: CollectCommands::Products {
                    brand: None,
                    dry_run: false
                }
            })
        ));
    }

    #[test]
    fn test_collect_products_with_brand_filter() {
        let cli =
            Cli::try_parse_from(["scbdb", "collect", "products", "--brand", "high-rise"]).unwrap();
        assert!(matches!(
            cli.command,
            Some(Commands::Collect {
                command: CollectCommands::Products {
                    brand: Some(ref b),
                    dry_run: false
                }
            }) if b == "high-rise"
        ));
    }

    #[test]
    fn test_collect_products_dry_run() {
        let cli = Cli::try_parse_from(["scbdb", "collect", "products", "--dry-run"]).unwrap();
        assert!(matches!(
            cli.command,
            Some(Commands::Collect {
                command: CollectCommands::Products { dry_run: true, .. }
            })
        ));
    }

    #[test]
    fn test_collect_pricing_no_filter() {
        let cli = Cli::try_parse_from(["scbdb", "collect", "pricing"]).unwrap();
        assert!(matches!(
            cli.command,
            Some(Commands::Collect {
                command: CollectCommands::Pricing { brand: None }
            })
        ));
    }

    #[test]
    fn test_collect_pricing_with_brand() {
        let cli = Cli::try_parse_from(["scbdb", "collect", "pricing", "--brand", "cann"]).unwrap();
        assert!(matches!(
            cli.command,
            Some(Commands::Collect {
                command: CollectCommands::Pricing { brand: Some(ref b) }
            }) if b == "cann"
        ));
    }

    /// Verifies that brand + dry-run flags combine correctly when both are present.
    #[test]
    fn collect_products_brand_and_dry_run_together() {
        let cli = Cli::try_parse_from([
            "scbdb",
            "collect",
            "products",
            "--brand",
            "cann",
            "--dry-run",
        ])
        .unwrap();
        assert!(matches!(
            cli.command,
            Some(Commands::Collect {
                command: CollectCommands::Products {
                    brand: Some(ref b),
                    dry_run: true,
                }
            }) if b == "cann"
        ));
    }
}
