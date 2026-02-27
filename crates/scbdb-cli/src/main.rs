mod collect;
mod regs;
mod sentiment;

use clap::{Parser, Subcommand};
use tracing_subscriber::EnvFilter;

use collect::CollectCommands;
use regs::RegsCommands;
use sentiment::SentimentCommands;

#[derive(Debug, Parser)]
#[command(name = "scbdb-cli")]
#[command(about = "SCBDB command line interface")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Collect product and pricing data from Shopify storefronts
    Collect {
        #[command(subcommand)]
        command: CollectCommands,
    },
    /// Track regulatory filings and legislative activity
    Regs {
        #[command(subcommand)]
        command: RegsCommands,
    },
    /// Collect and query brand sentiment signals
    Sentiment {
        #[command(subcommand)]
        command: SentimentCommands,
    },
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
            CollectCommands::VerifyImages { brand, concurrency } => {
                let pool = connect_or_exit().await;
                collect::run_collect_verify_images(&pool, brand.as_deref(), concurrency).await?;
            }
            CollectCommands::Locations { brand, dry_run } => {
                let config = load_config_or_exit();
                let pool = connect_or_exit().await;
                collect::run_collect_locations(&pool, &config, brand.as_deref(), dry_run).await?;
            }
        },
        Some(Commands::Regs { command }) => match command {
            RegsCommands::Ingest {
                state,
                keyword,
                max_pages,
                max_requests,
                all_sessions,
                dry_run,
            } => {
                let config = load_config_or_exit();
                let pool = connect_or_exit().await;
                regs::run_regs_ingest(
                    &pool,
                    &config,
                    &state,
                    &keyword,
                    max_pages,
                    max_requests,
                    all_sessions,
                    dry_run,
                )
                .await?;
            }
            RegsCommands::Status { state, limit } => {
                let pool = connect_or_exit().await;
                regs::run_regs_status(&pool, state.as_deref(), i64::from(limit)).await?;
            }
            RegsCommands::Timeline { state, bill } => {
                let pool = connect_or_exit().await;
                regs::run_regs_timeline(&pool, &state, &bill).await?;
            }
            RegsCommands::Report { state } => {
                let pool = connect_or_exit().await;
                regs::run_regs_report(&pool, state.as_deref()).await?;
            }
        },
        Some(Commands::Sentiment { command }) => match command {
            SentimentCommands::Collect { brand, dry_run } => {
                let pool = connect_or_exit().await;
                sentiment::run_sentiment_collect(&pool, brand.as_deref(), dry_run).await?;
            }
            SentimentCommands::Status { brand } => {
                let pool = connect_or_exit().await;
                sentiment::run_sentiment_status(&pool, brand.as_deref()).await?;
            }
            SentimentCommands::Report { brand } => {
                let pool = connect_or_exit().await;
                sentiment::run_sentiment_report(&pool, brand.as_deref()).await?;
            }
        },
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
            scbdb_db::DbError::Config(cfg_err) => {
                eprintln!("error: invalid configuration: {cfg_err}");
                eprintln!("hint: copy .env.example to .env and fill required values");
            }
            other => {
                eprintln!("error: unexpected database error during connect: {other}");
            }
        }
        std::process::exit(1);
    })
}

/// Attempt to mark a collection run as failed, logging any secondary error.
///
/// This is a best-effort operation — if the run is not in the expected state
/// (e.g., it was never transitioned to `running`), `fail_collection_run` will
/// itself return an error, which is logged and swallowed here.
pub(crate) async fn fail_run_best_effort(
    pool: &sqlx::PgPool,
    run_id: i64,
    context: &'static str,
    message: String,
) {
    if let Err(mark_err) = scbdb_db::fail_collection_run(pool, run_id, &message).await {
        tracing::error!(
            run_id,
            error = %mark_err,
            "failed to mark {context} run as failed"
        );
    }
}

#[cfg(test)]
mod tests;
