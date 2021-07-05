#[macro_use]
extern crate log;

use structopt::StructOpt;
use tokio::runtime::Builder;
use tokio::signal;

#[derive(Debug, StructOpt)]
struct Opts {
    #[structopt(short, long, parse(from_occurrences))]
    verbose: u32,
    #[structopt(short, long = "db-path")]
    database_path: Option<String>,
    #[structopt(long)]
    dump_config: bool,
}

async fn run(opts: Opts) -> color_eyre::eyre::Result<()> {
    // Connect to database
    let mut db = hyperion::db::Db::try_default(opts.database_path.as_deref()).await?;

    // Load configuration
    let config = hyperion::models::Config::load(&mut db).await?;

    // Dump configuration if this was asked
    if opts.dump_config {
        serde_json::to_writer(&mut std::io::stdout(), &config)?;
        return Ok(());
    }

    // Create the global state object
    let global = hyperion::global::GlobalData::new(&config).wrap();

    // Initialize and spawn the devices
    for (_, inst) in &config.instances {
        // Create the instance
        let (inst, handle) = hyperion::instance::Instance::new(global.clone(), inst.clone()).await;
        // Register the instance globally using its handle
        global.register_instance(handle).await;
        // Run the instance futures
        tokio::spawn(async move {
            let result = inst.run().await;

            if let Err(error) = result {
                error!("Instance error: {:?}", error);
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

#[paw::main]
fn main(opts: Opts) -> color_eyre::eyre::Result<()> {
    color_eyre::install()?;

    env_logger::Builder::from_env(
        env_logger::Env::new()
            .filter_or(
                "HYPERION_LOG",
                match opts.verbose {
                    0 => "hyperion=warn,hyperiond=warn",
                    1 => "hyperion=info,hyperiond=info",
                    2 => "hyperion=debug,hyperiond=debug",
                    _ => "hyperion=trace,hyperiond=trace",
                },
            )
            .write_style("HYPERION_LOG_STYLE"),
    )
    .try_init()
    .ok();

    // Create tokio runtime
    let thd_count = num_cpus::get().min(4);
    let rt = Builder::new_multi_thread()
        .worker_threads(thd_count)
        .enable_all()
        .build()?;
    rt.block_on(run(opts))
}
