use {
    domichain_faucet::faucet::{request_airdrop_transaction, run_local_faucet},
    domichain_sdk::{
        hash::Hash,
        message::Message,
        signature::{Keypair, Signer},
        system_instruction,
        transaction::Transaction,
    },
};

#[test]
fn test_local_faucet() {
    let keypair = Keypair::new();
    let to = domichain_sdk::pubkey::new_rand();
    let satomis = 50;
    let blockhash = Hash::new(to.as_ref());
    let create_instruction = system_instruction::transfer(&keypair.pubkey(), &to, satomis);
    let message = Message::new(&[create_instruction], Some(&keypair.pubkey()));
    let expected_tx = Transaction::new(&[&keypair], message, blockhash);

    let faucet_addr = run_local_faucet(keypair, None);

    let result = request_airdrop_transaction(&faucet_addr, &to, satomis, blockhash);
    assert_eq!(expected_tx, result.unwrap());
}
