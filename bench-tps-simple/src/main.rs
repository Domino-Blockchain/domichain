use domichain_client::nonce_utils;
use domichain_sdk::{nonce::State, system_instruction, transaction::Transaction};

use {
    clap::{value_t, ArgMatches},
    domichain_bench_tps::{
        bench::{fund_keypairs, generate_keypairs},
        bench_tps_client::BenchTpsClient,
        cli,
    },
    domichain_client::{
        connection_cache::{ConnectionCache, UseQUIC},
        thin_client::ThinClient,
    },
    domichain_gossip::gossip_service::{discover_cluster, get_client, try_get_multi_client},
    domichain_sdk::{signature::Keypair, signer::Signer, system_transaction},
    domichain_streamer::socket::SocketAddrSpace,
    log::*,
    std::{
        process::exit,
        sync::Arc,
        thread::{sleep, spawn},
        time::Duration,
    },
};

fn get_bench_client(cli_config: &cli::Config, matches: &ArgMatches) -> ThinClient {
    let cli::Config {
        use_quic,
        tpu_connection_pool_size,
        ..
    } = &cli_config;

    let use_quic = UseQUIC::new(*use_quic).expect("Failed to initialize QUIC flags");
    let connection_cache = Arc::new(ConnectionCache::new(use_quic, *tpu_connection_pool_size));

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

fn wait_client(cli_config: &cli::Config, connection_cache: Arc<ConnectionCache>) -> ThinClient {
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

    let nodes = discover_cluster(entrypoint_addr, *num_nodes, SocketAddrSpace::Unspecified)
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
        Some(get_client(
            &nodes,
            &SocketAddrSpace::Unspecified,
            connection_cache,
        ))
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

// See: sdk/program/src/system_instruction.rs `create_nonce_account`
fn submit_create_nonce_account_tx(client: Arc<ThinClient>, payer: &Keypair) -> Keypair {
    let nonce_account = Keypair::new();

    let nonce_rent = client
        .get_minimum_balance_for_rent_exemption(State::size())
        .expect("Cannot get rent balance. Please restart benchmark");
    let instr = system_instruction::create_nonce_account(
        &payer.pubkey(),
        &nonce_account.pubkey(),
        &payer.pubkey(), // Make the fee payer the nonce account authority
        nonce_rent,
    );

    let mut tx = Transaction::new_with_payer(&instr, Some(&payer.pubkey()));

    let blockhash = client.get_latest_blockhash().unwrap();
    tx.try_sign(&[&nonce_account, payer], blockhash).unwrap();

    client
        .send_and_confirm_transaction(&[&nonce_account, payer], &mut tx, 5, 0)
        .unwrap();

    nonce_account
}

fn do_bench_tps_simple(
    client: ThinClient,
    config: &cli::Config,
    from: &Keypair,
    to: &Keypair,
    nonce_kps: &[Keypair],
) {
    let nonce_pks: Vec<_> = nonce_kps.iter().map(|nonce| nonce.pubkey()).collect();
    info!(
        "From {:?} To {:?} Nonce {:?}",
        from.pubkey(),
        to.pubkey(),
        nonce_pks.len()
    );
    loop {
        let from_balance = client.get_balance(&from.pubkey());
        let to_balance = client.get_balance(&to.pubkey());
        let nonce_balances: Vec<_> = nonce_pks
            .iter()
            .map(|nonce| client.get_balance(nonce).unwrap())
            .collect();
        let done = nonce_balances.iter().filter(|b| **b > 0).count();
        let sum: u64 = nonce_balances.iter().sum();
        info!(
            "Token balance: From {from_balance:?} To {to_balance:?} Nonce {done}/{} sum={sum}",
            nonce_pks.len()
        );

        if done == nonce_pks.len() {
            break;
        }

        sleep(Duration::from_secs(1));
    }

    info!("Sending transaction");

    let n = config.tx_count;
    let mut txs = Vec::with_capacity(n);
    for i in 0..n {
        // Sign the tx with nonce_account's `blockhash` instead of the
        // network's latest blockhash.
        let nonce = nonce_kps[i % nonce_kps.len()].pubkey();
        let nonce_account = client.get_account(&nonce).unwrap();
        let nonce_data = nonce_utils::data_from_account(&nonce_account).unwrap();
        let blockhash = nonce_data.blockhash();

        // let blockhash = client.get_latest_blockhash().unwrap();
        // TODO: use nonce instead https://docs.solana.com/offline-signing/durable-nonce
        let tx = system_transaction::nonced_transfer(
            from,
            &to.pubkey(),
            1_000_000 + i as u64,
            &nonce,
            from,
            blockhash,
        );

        txs.push(tx);
    }
    let mut threads = Vec::new();
    let client = Arc::new(client);
    for chunk in txs.chunks(250) {
        let chunk = chunk.to_vec();
        let client = client.clone();
        threads.push(spawn(move || {
            client.send_batch(chunk).unwrap();
        }));
    }
    threads.into_iter().for_each(|t| t.join().unwrap());

    info!("Transactions sent");

    let client = Arc::try_unwrap(client).map_err(|_| "").unwrap();

    for _ in 0..config.duration.as_secs() {
        let from_balance = client.get_balance(&from.pubkey());
        let to_balance = client.get_balance(&to.pubkey());
        let nonce_balances: u64 = nonce_pks
            .iter()
            .map(|nonce| client.get_balance(nonce).unwrap())
            .sum();

        // let new_blockhashes: Vec<_> = nonce_pks.iter().map(|nonce| {
        //     let nonce_account = {
        //         use domichain_sdk::client::SyncClient;
        //         SyncClient::get_account_with_commitment(&client, nonce, CommitmentConfig::processed()).unwrap().unwrap()
        //     };
        //     let nonce_data = nonce_utils::data_from_account(&nonce_account).unwrap();
        //     let new_blockhash = nonce_data.blockhash();
        //     new_blockhash
        // }).collect();

        info!("Token balance: From {from_balance:?} To {to_balance:?} Nonce {nonce_balances:?}");

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
        tx_count,
        ..
    } = &cli_config;

    info!("cli_config={cli_config:#?}");

    let client = get_bench_client(&cli_config, &matches);

    let client = Arc::new(client);

    let nonce_rent = client
        .get_minimum_balance_for_rent_exemption(State::size())
        .expect("Cannot get rent balance. Please restart benchmark");
    info!("Nonce rent: {nonce_rent}");

    let n = *tx_count;

    let from = get_bench_keypairs(
        client.clone(),
        id,
        1,
        *num_lamports_per_account + nonce_rent * n as u64,
    )
    .into_iter()
    .nth(0)
    .unwrap();

    let to = get_bench_keypairs(client.clone(), id, 1, 0)
        .into_iter()
        .nth(0)
        .unwrap();

    let nonce_kps: Vec<_> = (0..n)
        .map(|i| {
            let client = client.clone();
            let from = Keypair::from_bytes(&from.to_bytes()).unwrap();
            spawn(move || {
                let kp = submit_create_nonce_account_tx(client, &from);
                info!("Done {i}/{n} nonce account creation");
                kp
            })
        })
        .collect();
    let nonce_kps: Vec<_> = nonce_kps.into_iter().map(|t| t.join().unwrap()).collect();

    let client = Arc::try_unwrap(client).map_err(|_| "").unwrap();

    do_bench_tps_simple(client, &cli_config, &from, &to, &nonce_kps);
}
