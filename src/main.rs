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
    let global = hyperion::global::GlobalData::new().wrap();

    // Initialize and spawn the devices
    let mut initialized_devices = 0;
    let mut max_id = 0;

    for (id, inst) in &config.instances {
        max_id = max_id.max(*id);

        match hyperion::instance::Instance::new(global.clone(), inst.clone()).await {
            Ok(inst) => {
                initialized_devices += 1;

                // TODO: Move this code to a function
                tokio::spawn(async move {
                    let result = inst.run().await;

                    if let Err(error) = result {
                        error!("Instance error: {:?}", error);
                    }
                });
            }
            Err(error) => {
                error!(
                    "Initializing instance {} `{}` failed: {}",
                    id, inst.instance.friendly_name, error
                );
            }
        }
    }

    // Add a dummy device for debugging if needed
    if initialized_devices == 0 {
        warn!("no devices were initialized, adding a dummy device");
        let inst = hyperion::instance::Instance::new(
            global.clone(),
            hyperion::models::InstanceConfig::new_dummy(max_id + 1),
        )
        .await?;

        // TODO: Move this code to a function
        tokio::spawn(async move {
            let result = inst.run().await;

            if let Err(error) = result {
                error!("Instance error: {:?}", error);
            }
        });
    }

    // Start the flatbuffers servers
    if config.global.flatbuffers_server.enable {
        tokio::spawn({
            let global = global.clone();
            let config = config.global.flatbuffers_server.clone();

            async move {
                let result =
                    hyperion::servers::bind(config, global, hyperion::servers::flat::handle_client)
                        .await;

                if let Err(error) = result {
                    error!("Flatbuffers server terminated: {:?}", error);
                }
            }
        });
    }

    // Start the JSON server
    tokio::spawn({
        let global = global.clone();
        let config = config.global.json_server.clone();

        async move {
            let result =
                hyperion::servers::bind(config, global, hyperion::servers::json::handle_client)
                    .await;

            if let Err(error) = result {
                error!("JSON server terminated: {:?}", error);
            }
        }
    });

    // Start the Protobuf server
    if config.global.proto_server.enable {
        tokio::spawn({
            let global = global.clone();
            let config = config.global.proto_server.clone();

            async move {
                let result = hyperion::servers::bind(
                    config,
                    global,
                    hyperion::servers::proto::handle_client,
                )
                .await;

                if let Err(error) = result {
                    error!("Protobuf server terminated: {:?}", error);
                }
            }
        });
    }

    // Spawn priority muxer
    let muxer = hyperion::muxer::PriorityMuxer::new(global.clone()).await;
    tokio::spawn(muxer.run());

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
    if thd_count > 1 {
        let rt = Builder::new_multi_thread()
            .worker_threads(thd_count)
            .enable_all()
            .build()?;
        rt.block_on(run(opts))
    } else {
        let rt = Builder::new_current_thread().enable_all().build()?;
        rt.block_on(run(opts))
    }
}
