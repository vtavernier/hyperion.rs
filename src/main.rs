#[macro_use]
extern crate tracing;

use std::path::PathBuf;

use structopt::StructOpt;
use tokio::runtime::Builder;
use tokio::signal;

use hyperion::models::backend::ConfigExt;

#[derive(Debug, StructOpt)]
struct Opts {
    /// Log verbosity. Overrides logger level in config, but is overridden by HYPERION_LOG
    #[structopt(short, long, parse(from_occurrences))]
    verbose: u32,
    /// Path to the configuration database
    #[structopt(
        short,
        long = "db-path",
        default_value = "$ROOT/hyperion.db",
        env = "DATABASE_URL"
    )]
    database_path: PathBuf,
    /// Path to a TOML config file. Overrides the configuration database
    #[structopt(short, long = "config")]
    config_path: Option<PathBuf>,
    /// Dump the loaded configuration
    #[structopt(long)]
    dump_config: bool,
    /// Path to the user root folder. Defaults to .config/hyperion.rs (Linux) or
    /// %APPDATA%\hyperion.rs (Windows)
    #[structopt(long)]
    user_root: Option<PathBuf>,
}

async fn run(opts: Opts) -> color_eyre::eyre::Result<()> {
    // Path resolver
    let paths = hyperion::global::Paths::new(opts.user_root.clone())?;

    // Load configuration
    let mut backend: Box<dyn hyperion::models::backend::ConfigBackend> =
        if let Some(config_path) = opts.config_path.as_deref() {
            Box::new(hyperion::models::backend::FileBackend::new(&config_path))
        } else {
            // Connect to database
            let db = hyperion::db::Db::open(&paths.resolve_path(opts.database_path)).await?;
            Box::new(hyperion::models::backend::DbBackend::new(db))
        };

    let config = backend.load().await?;

    // Dump configuration if this was asked
    if opts.dump_config {
        print!("{}", config.to_string()?);
        return Ok(());
    }

    // Create the global state object
    let global = hyperion::global::GlobalData::new(&config).wrap();

    // Spawn the hook runner
    tokio::spawn(
        hyperion::global::HookRunner::new(
            config.global.hooks.clone(),
            global.subscribe_events().await,
        )
        .run(),
    );

    // Keep a list of all instances
    let mut instances = Vec::with_capacity(config.instances.len());

    // Initialize and spawn the devices
    for (&id, inst) in &config.instances {
        // Create the instance
        let (inst, handle) = hyperion::instance::Instance::new(global.clone(), inst.clone()).await;
        // Register the instance globally using its handle
        global.register_instance(handle.clone()).await;
        // Keep it around
        instances.push(handle);
        // Run the instance futures
        tokio::spawn({
            let global = global.clone();
            let event_tx = global.get_event_tx().await;

            async move {
                event_tx
                    .send(hyperion::global::Event::instance(
                        id,
                        hyperion::global::InstanceEventKind::Start,
                    ))
                    .map(|_| ())
                    .unwrap_or_else(|err| {
                        error!(error = %err, "event error");
                    });

                let result = inst.run().await;

                if let Err(error) = result {
                    error!(error = %error, "instance error");
                }

                global.unregister_instance(id).await;

                event_tx
                    .send(hyperion::global::Event::instance(
                        id,
                        hyperion::global::InstanceEventKind::Stop,
                    ))
                    .map(|_| ())
                    .unwrap_or_else(|err| {
                        error!(error = %err, "event error");
                    });
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

    // Start the webconfig server
    let _webconfig_server = tokio::task::spawn(
        hyperion::web::bind(global.clone(), &config.global.web_config, &paths).await?,
    );

    // Global event handle
    let event_tx = global.get_event_tx().await;

    // We have started
    event_tx.send(hyperion::global::Event::Start)?;

    // Should we continue running?
    let mut abort = false;

    while !abort {
        tokio::select! {
            _ = signal::ctrl_c() => {
                abort = true;
            }
        }
    }

    // Stop all instances
    for instance in instances.into_iter() {
        instance.stop().await.ok();
    }

    // We have finished running properly
    event_tx.send(hyperion::global::Event::Stop)?;

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
