use std::path::PathBuf;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    let matches = clap::Command::new("Proof Submitter")
        .about("Submit proofs from calldata directory to Starknet")
        .arg(
            clap::Arg::new("calldata_dir")
                .long("calldata_dir")
                .value_name("PATH")
                .required(true),
        )
        .arg(
            clap::Arg::new("job_id")
                .long("job_id")
                .value_name("U64")
                .required(true)
                .value_parser(clap::value_parser!(u64)),
        )
        .arg(
            clap::Arg::new("layout")
                .long("layout")
                .value_name("LAYOUT")
                .default_value("recursive_with_poseidon"),
        )
        .arg(
            clap::Arg::new("hasher")
                .long("hasher")
                .value_name("HASHER")
                .default_value("keccak_160_lsb"),
        )
        .arg(
            clap::Arg::new("stone_version")
                .long("stone_version")
                .value_name("VERSION")
                .default_value("stone6"),
        )
        .arg(
            clap::Arg::new("memory_verification")
                .long("memory_verification")
                .value_name("BOOL")
                .default_value("true")
                .value_parser(clap::value_parser!(bool)),
        )
        .arg(
            clap::Arg::new("config")
                .long("config")
                .value_name("FILE")
                .default_value("config.toml"),
        )
        .get_matches();

    let calldata_dir = PathBuf::from(matches.get_one::<String>("calldata_dir").unwrap());
    let job_id = *matches.get_one::<u64>("job_id").unwrap();
    let layout = matches.get_one::<String>("layout").unwrap().clone();
    let hasher = matches.get_one::<String>("hasher").unwrap().clone();
    let stone_version = matches.get_one::<String>("stone_version").unwrap().clone();
    let memory_verification = *matches.get_one::<bool>("memory_verification").unwrap();
    let config = PathBuf::from(matches.get_one::<String>("config").unwrap());

    // Prefer a single thin entrypoint in the lib:
    // sequencer_lib::proof_submitter::run(calldata_dir, job_id, layout, hasher, stone_version, memory_verification, config).await
    //     .map_err(|e| anyhow::anyhow!(e))?;
    //
    // If the lib exposes a client instead:
    // let cfg = sequencer_lib::config::load(Some(&config))?;
    // let pool = sequencer_lib::db::get_pool(&cfg.database.get_db_url()).await?;
    // let client = sequencer_lib::relayer::ProofSubmissionClient::new(pool, cfg).await?;
    // client.submit_proof_from_calldata(calldata_dir, job_id, layout, hasher, stone_version, memory_verification).await?;
    Ok(())
}
