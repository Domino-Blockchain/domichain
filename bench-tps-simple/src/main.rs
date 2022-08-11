use std::{thread::{sleep, spawn}, time::Duration};

use domichain_bench_tps::cli::Config;
use domichain_sdk::{system_transaction, signer::Signer};

use {
    clap::{value_t, ArgMatches},
    log::*,
    domichain_bench_tps::{
        bench::{generate_keypairs, fund_keypairs},
        bench_tps_client::BenchTpsClient,
        cli,
    },
    domichain_client::{
        connection_cache::{ConnectionCache, UseQUIC},
        thin_client::ThinClient,
    },
    domichain_gossip::gossip_service::{discover_cluster, get_client, try_get_multi_client},
    domichain_sdk::signature::Keypair,
    domichain_streamer::socket::SocketAddrSpace,
    std::{process::exit, sync::Arc},
};

fn get_bench_client(cli_config: &cli::Config, matches: &ArgMatches) -> ThinClient {
    let cli::Config {
        use_quic,
        tpu_connection_pool_size,
        ..
    } = &cli_config;

    let use_quic = UseQUIC::new(*use_quic).expect("Failed to initialize QUIC flags");
    let connection_cache =
        Arc::new(ConnectionCache::new(use_quic, *tpu_connection_pool_size));

    let client = if let Ok(rpc_addr) = value_t!(matches, "rpc_addr", String) {
        let rpc = rpc_addr.parse().unwrap_or_else(|e| {
            eprintln!("RPC address should parse as socketaddr {:?}", e);
            exit(1);
        });
        let tpu = value_t!(matches, "tpu_addr", String)
            .unwrap()
            .parse()
            .unwrap_or_else(|e| {
                eprintln!("TPU address should parse to a socket: {:?}", e);
                exit(1);
            });

        ThinClient::new(rpc, tpu, connection_cache)
    } else {
        wait_client(cli_config, connection_cache)
    };

    client
}

fn wait_client(
    cli_config: &cli::Config,
    connection_cache: Arc<ConnectionCache>,
) -> ThinClient {
    for _ in 0..10 {
        if let Some(client) = discover_client(cli_config, connection_cache.clone()) {
            return client;
        }
        info!("Trying to find clent...");
        sleep(Duration::from_secs(1));
    }
    panic!("Cannot find client");
}

fn discover_client(
    cli_config: &cli::Config,
    connection_cache: Arc<ConnectionCache>,
) -> Option<ThinClient> {
    let cli::Config {
        entrypoint_addr,
        num_nodes,
        multi_client,
        target_node,
        ..
    } = &cli_config;

    let nodes =
        discover_cluster(entrypoint_addr, *num_nodes, SocketAddrSpace::Unspecified)
            .unwrap_or_else(|err| {
                eprintln!("Failed to discover {} nodes: {:?}", num_nodes, err);
                exit(1);
            });
    if *multi_client {
        let (client, num_clients) =
            try_get_multi_client(&nodes, &SocketAddrSpace::Unspecified, connection_cache).ok()?;
        if nodes.len() < num_clients {
            eprintln!(
                "Error: Insufficient nodes discovered.  Expecting {} or more",
                num_nodes
            );
            exit(1);
        }
        Some(client)
    } else if let Some(target_node) = target_node {
        info!("Searching for target_node: {:?}", target_node);
        let mut target_client = None;
        for node in nodes {
            if node.id == *target_node {
                target_client = Some(get_client(
                    &[node],
                    &SocketAddrSpace::Unspecified,
                    connection_cache,
                ));
                break;
            }
        }
        Some(target_client.unwrap_or_else(|| {
            eprintln!("Target node {} not found", target_node);
            exit(1);
        }))
    } else {
        Some(get_client(&nodes, &SocketAddrSpace::Unspecified, connection_cache))
    }
}

fn get_bench_keypairs<T>(
    client: Arc<T>,
    id: &Keypair,
    keypair_count: usize,
    num_lamports_per_account: u64,
) -> Vec<Keypair>
where
    T: 'static + BenchTpsClient + Send + Sync,
{
    let (mut keypairs, extra) = generate_keypairs(&Keypair::new(), keypair_count as u64);
    fund_keypairs(client, id, &keypairs, extra, num_lamports_per_account).unwrap();

    // 'generate_keypairs' generates extra keys to be able to have size-aligned funding batches for fund_keys.
    keypairs.truncate(keypair_count);

    keypairs
}

fn do_bench_tps_simple<T>(
    client: Arc<T>,
    config: Config,
    from: &Keypair,
    to: &Keypair,
)
where
    T: 'static + BenchTpsClient + Send + Sync,
{
    info!("From {:?} To {:?}", from.pubkey(), to.pubkey());
    for _ in 0..15 {
        let from_balance = client.get_balance(&from.pubkey()).unwrap_or(u64::MAX);
        let to_balance = client.get_balance(&to.pubkey()).unwrap_or(u64::MAX);
        info!("Token balance: From {} To {}", from_balance, to_balance);
    
        sleep(Duration::from_secs(1));
    }

    let n = config.tx_count;
    let mut txs = Vec::with_capacity(n);
    for i in 0..n {
        let blockhash = client.get_latest_blockhash().unwrap();
        let tx = system_transaction::transfer(
            from, &to.pubkey(),
            1_000_000 + i as u64,
            blockhash
        );
        txs.push(tx);
    }
    let mut threads = Vec::new();
    for chunk in txs.chunks(250) {
        let chunk = chunk.to_vec();
        let client = client.clone();
        threads.push(spawn(move || {
            client.send_batch(chunk).unwrap();
        }));
    }
    threads.into_iter().for_each(|t| t.join().unwrap());

    info!("Transactions sent");

    for _ in 0..config.duration.as_secs() {
        let from_balance = client.get_balance(&from.pubkey()).unwrap_or(u64::MAX);
        let to_balance = client.get_balance(&to.pubkey()).unwrap_or(u64::MAX);
        info!("Token balance: From {} To {}", from_balance, to_balance);

        sleep(Duration::from_secs(1));
    }
}

fn main() {
    domichain_logger::setup_with_default("domichain=info");
    domichain_metrics::set_panic_hook("bench-tps-simple", /*version:*/ None);

    let matches = cli::build_args(domichain_version::version!()).get_matches();
    let cli_config = cli::extract_args(&matches);

    let cli::Config {
        id,
        num_lamports_per_account,
        ..
    } = &cli_config;

    info!("cli_config={cli_config:#?}");

    let client = get_bench_client(&cli_config, &matches);
    let client = Arc::new(client);
    
    let keypairs = get_bench_keypairs(
        client.clone(),
        id,
        1,
        *num_lamports_per_account,
    );

    let keypairs2 = get_bench_keypairs(
        client.clone(),
        id,
        1,
        0,
    );

    // do_bench_tps(client, cli_config, keypairs);
    do_bench_tps_simple(client, cli_config, &keypairs[0], &keypairs2[0]);
}