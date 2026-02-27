use super::*;

#[test]
fn parses_db_ping_command() {
    let cli = Cli::try_parse_from(["scbdb-cli", "db", "ping"]).expect("expected valid cli args");

    assert!(matches!(
        cli.command,
        Some(Commands::Db {
            command: DbCommands::Ping
        })
    ));
}

#[test]
fn parses_db_migrate_command() {
    let cli = Cli::try_parse_from(["scbdb-cli", "db", "migrate"]).expect("expected valid cli args");

    assert!(matches!(
        cli.command,
        Some(Commands::Db {
            command: DbCommands::Migrate
        })
    ));
}

#[test]
fn parses_db_seed_command() {
    let cli = Cli::try_parse_from(["scbdb-cli", "db", "seed"]).expect("expected valid cli args");

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

#[test]
fn test_collect_verify_images_defaults() {
    let cli = Cli::try_parse_from(["scbdb", "collect", "verify-images"]).unwrap();
    assert!(matches!(
        cli.command,
        Some(Commands::Collect {
            command: CollectCommands::VerifyImages {
                brand: None,
                concurrency: 12
            }
        })
    ));
}

#[test]
fn test_collect_verify_images_with_brand_and_concurrency() {
    let cli = Cli::try_parse_from([
        "scbdb",
        "collect",
        "verify-images",
        "--brand",
        "wynk",
        "--concurrency",
        "4",
    ])
    .unwrap();
    assert!(matches!(
        cli.command,
        Some(Commands::Collect {
            command: CollectCommands::VerifyImages {
                brand: Some(ref b),
                concurrency: 4
            }
        }) if b == "wynk"
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

#[test]
fn parses_regs_ingest_defaults() {
    let cli = Cli::try_parse_from(["scbdb-cli", "regs", "ingest"]).unwrap();
    assert!(matches!(
        cli.command,
        Some(Commands::Regs {
            command: RegsCommands::Ingest {
                dry_run: false,
                max_pages: 3,
                max_requests: 5000,
                ..
            }
        })
    ));
    if let Some(Commands::Regs {
        command: RegsCommands::Ingest { ref state, .. },
    }) = cli.command
    {
        assert_eq!(state, &["SC"]);
    }
}

#[test]
fn parses_regs_ingest_with_state_and_keyword() {
    let cli = Cli::try_parse_from([
        "scbdb-cli",
        "regs",
        "ingest",
        "--state",
        "TX",
        "--keyword",
        "cannabis",
    ])
    .unwrap();
    if let Some(Commands::Regs {
        command:
            RegsCommands::Ingest {
                ref state,
                ref keyword,
                ..
            },
    }) = cli.command
    {
        assert_eq!(state, &["TX"]);
        assert_eq!(keyword, &["cannabis"]);
    } else {
        panic!("unexpected command variant");
    }
}

#[test]
fn parses_regs_ingest_dry_run() {
    let cli = Cli::try_parse_from(["scbdb-cli", "regs", "ingest", "--dry-run"]).unwrap();
    assert!(matches!(
        cli.command,
        Some(Commands::Regs {
            command: RegsCommands::Ingest { dry_run: true, .. }
        })
    ));
}

#[test]
fn parses_regs_status_no_args() {
    let cli = Cli::try_parse_from(["scbdb-cli", "regs", "status"]).unwrap();
    assert!(matches!(
        cli.command,
        Some(Commands::Regs {
            command: RegsCommands::Status {
                state: None,
                limit: 20,
            }
        })
    ));
}

#[test]
fn parses_regs_status_with_state() {
    let cli = Cli::try_parse_from(["scbdb-cli", "regs", "status", "--state", "SC"]).unwrap();
    assert!(matches!(
        cli.command,
        Some(Commands::Regs {
            command: RegsCommands::Status {
                state: Some(ref s),
                ..
            }
        }) if s == "SC"
    ));
}

#[test]
fn parses_regs_timeline() {
    let cli = Cli::try_parse_from([
        "scbdb-cli",
        "regs",
        "timeline",
        "--state",
        "SC",
        "--bill",
        "HB1234",
    ])
    .unwrap();
    assert!(matches!(
        cli.command,
        Some(Commands::Regs {
            command: RegsCommands::Timeline {
                ref state,
                ref bill,
            }
        }) if state == "SC" && bill == "HB1234"
    ));
}

#[test]
fn parses_regs_report_no_args() {
    let cli = Cli::try_parse_from(["scbdb-cli", "regs", "report"]).unwrap();
    assert!(matches!(
        cli.command,
        Some(Commands::Regs {
            command: RegsCommands::Report { state: None }
        })
    ));
}
