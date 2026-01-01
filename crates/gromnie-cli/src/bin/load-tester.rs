use clap::Parser;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tracing::error;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

use gromnie_runner::{
    AutoLoginConsumer, ClientConfig, ClientNaming, CompositeConsumer, FnConsumerBuilder,
    MultiClientStats, RunConfig, RunResult, StatsConsumer,
};

#[derive(Parser)]
#[command(name = "load-tester")]
#[command(about = "Load testing tool for AC server")]
pub struct Args {
    #[command(subcommand)]
    command: Option<Command>,

    #[command(flatten)]
    run_args: RunArgs,
}

#[derive(Parser, Debug)]
pub enum Command {
    /// Generate naming info for a specific client ID
    Naming {
        /// Client ID to generate naming for
        client_id: u32,
    },
    /// Run the load tester
    Run(RunArgs),
}

#[derive(Parser, Debug, Clone, Default)]
pub struct RunArgs {
    /// Number of clients to spawn
    #[arg(short, long, default_value = "5")]
    clients: u32,

    /// Server host address
    #[arg(long, default_value = "localhost")]
    host: String,

    /// Server port
    #[arg(short, long, default_value = "9000")]
    port: u16,

    /// Delay between client connections in milliseconds
    #[arg(short, long, default_value = "1000")]
    rate_limit: u64,

    /// Enable verbose per-client logging
    #[arg(short, long)]
    verbose: bool,

    /// Stats display interval in seconds
    #[arg(long, default_value = "5")]
    stats_interval: u64,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    // Handle naming subcommand early (doesn't need tracing setup)
    if let Some(Command::Naming { client_id }) = args.command {
        let naming = ClientNaming::new(client_id);
        println!("Client ID: {}", client_id);
        println!("Account: {}", naming.account_name());
        println!("Password: {}", naming.password());
        println!("Character: {}", naming.character_name());
        return;
    }

    // Set up file appender for load_tester.log
    let file_appender = tracing_appender::rolling::never(".", "load_tester.log");
    let (non_blocking_file, _guard) = tracing_appender::non_blocking(file_appender);

    // Create the env filter
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    // Set up layered subscriber that writes to both stdout and file
    tracing_subscriber::registry()
        .with(env_filter)
        .with(
            tracing_subscriber::fmt::layer()
                .with_writer(std::io::stdout)
                .with_ansi(true),
        )
        .with(
            tracing_subscriber::fmt::layer()
                .with_writer(non_blocking_file)
                .with_ansi(false),
        )
        .init();

    let run_args = if let Some(Command::Run(args)) = args.command {
        args
    } else {
        args.run_args
    };

    // Validate rate_limit is non-zero
    if run_args.rate_limit == 0 {
        error!("Rate limit must be a non-zero value (milliseconds)");
        std::process::exit(1);
    }

    // Create the run configuration
    let server_address = format!("{}:{}", run_args.host, run_args.port);
    let config = RunConfig::multi(
        server_address,
        run_args.clients,
        run_args.rate_limit,
        false, // shared_event_bus
    );

    // Create stats for tracking
    let stats = Arc::new(MultiClientStats::default());

    // Create the consumer builder
    let stats_for_builder = stats.clone();
    let verbose = run_args.verbose;
    let consumer_builder = FnConsumerBuilder::new(move |client_id, client_config, action_tx| {
        // Generate character name from account name
        let character_name = format!("{}-A", client_config.account_name);

        // Create composite consumer with both stats and auto-login
        let stats_consumer = Box::new(
            StatsConsumer::new(client_id, stats_for_builder.clone()).with_verbose(verbose),
        );
        let auto_login_consumer = Box::new(
            AutoLoginConsumer::new(client_id, character_name, action_tx).with_verbose(verbose),
        );

        Box::new(CompositeConsumer::new(vec![
            stats_consumer,
            auto_login_consumer,
        ]))
    });

    // Start time for stats
    let start_time = Instant::now();

    // Spawn stats display task
    let stats_for_task = stats.clone();
    let stats_interval = run_args.stats_interval;
    let stats_handle = tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(stats_interval)).await;
            stats_for_task.print(start_time.elapsed().as_secs());
        }
    });

    // Run the multi-client test
    let host = run_args.host.clone();
    let port = run_args.port;
    let result = gromnie_runner::run(
        config,
        consumer_builder,
        Some(move |client_id| {
            let naming = ClientNaming::new(client_id);
            ClientConfig::new(
                client_id,
                format!("{}:{}", host, port),
                naming.account_name(),
                naming.password(),
            )
        }),
        None, // No external shutdown
    )
    .await;

    // Abort stats task
    stats_handle.abort();

    // Display final stats
    if let RunResult::Multi(final_stats) = result {
        final_stats.print_final(start_time.elapsed());
    }
}
