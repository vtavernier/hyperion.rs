#[macro_use]
extern crate tracing;

use std::path::PathBuf;

use structopt::StructOpt;
use tokio::runtime::Builder;
use tokio::signal;

#[derive(Debug, StructOpt)]
struct Opts {
    #[structopt(short, long, parse(from_occurrences))]
    verbose: u32,
    #[structopt(short, long = "db-path")]
    database_path: Option<String>,
    #[structopt(short, long = "config")]
    config_path: Option<PathBuf>,
    #[structopt(long)]
    dump_config: bool,
}

async fn run(opts: Opts) -> color_eyre::eyre::Result<()> {
    // Load configuration
    let config = {
        if let Some(config_path) = opts.config_path.as_deref() {
            hyperion::models::Config::load_file(config_path).await?
        } else {
            // Connect to database
            let mut db = hyperion::db::Db::try_default(opts.database_path.as_deref()).await?;
            hyperion::models::Config::load(&mut db).await?
        }
    };

    // Dump configuration if this was asked
    if opts.dump_config {
        print!("{}", config.to_string()?);
        return Ok(());
    }

    // Create the global state object
    let global = hyperion::global::GlobalData::new(&config).wrap();

    // Initialize and spawn the devices
    for (&id, inst) in &config.instances {
        // Create the instance
        let (inst, handle) = hyperion::instance::Instance::new(global.clone(), inst.clone()).await;
        // Register the instance globally using its handle
        global.register_instance(handle).await;
        // Run the instance futures
        tokio::spawn({
            let global = global.clone();

            async move {
                let result = inst.run().await;

                if let Err(error) = result {
                    error!(error = %error, "instance error");
                }

                global.unregister_instance(id).await;
            }
        });
    }

    // Start the Flatbuffers servers
    let _flatbuffers_server = if config.global.flatbuffers_server.enable {
        Some(
            hyperion::servers::bind(
                "Flatbuffers",
                config.global.flatbuffers_server.clone(),
                global.clone(),
                hyperion::servers::flat::handle_client,
            )
            .await?,
        )
    } else {
        None
    };

    // Start the JSON server
    let _json_server = hyperion::servers::bind(
        "JSON",
        config.global.json_server.clone(),
        global.clone(),
        hyperion::servers::json::handle_client,
    )
    .await?;

    // Start the Protobuf server
    let _proto_server = if config.global.proto_server.enable {
        Some(
            hyperion::servers::bind(
                "Protobuf",
                config.global.proto_server.clone(),
                global.clone(),
                hyperion::servers::proto::handle_client,
            )
            .await?,
        )
    } else {
        None
    };

    // Should we continue running?
    let mut abort = false;

    while !abort {
        tokio::select! {
            _ = signal::ctrl_c() => {
                abort = true;
            }
        }
    }

    Ok(())
}

fn install_tracing(opts: &Opts) -> Result<(), tracing_subscriber::util::TryInitError> {
    use tracing_error::ErrorLayer;
    use tracing_subscriber::{fmt, prelude::*, EnvFilter};

    let fmt_layer = fmt::layer();

    let filter_layer = EnvFilter::try_from_env("HYPERION_LOG").unwrap_or_else(|_| {
        EnvFilter::new(match opts.verbose {
            0 => "hyperion=warn,hyperiond=warn",
            1 => "hyperion=info,hyperiond=info",
            2 => "hyperion=debug,hyperiond=debug",
            _ => "hyperion=trace,hyperiond=trace",
        })
    });

    tracing_subscriber::registry()
        .with(filter_layer)
        .with(fmt_layer)
        .with(ErrorLayer::default())
        .try_init()
}

#[paw::main]
fn main(opts: Opts) -> color_eyre::eyre::Result<()> {
    color_eyre::install()?;
    install_tracing(&opts)?;

    // Create tokio runtime
    let thd_count = match num_cpus::get() {
        1 => 2,
        other => other.min(4),
    };

    let rt = Builder::new_multi_thread()
        .worker_threads(thd_count)
        .enable_all()
        .build()?;
    rt.block_on(run(opts))
}
