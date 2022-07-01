use {
    libvrf::vrf::*,
    domichain_sdk::{
        pubkey::Pubkey,
        signature::{Keypair},
    },
    std::fs::read,
};

#[test]
fn test_vrf_prove() {
    let sk = read_file("./tests/sk");
    assert_eq!(sk.len(), 32);
    let pk = read_file("./tests/pk");
    assert_eq!(pk.len(), 32);
    let keypair = Keypair::from_bytes(&[sk, pk].concat()).unwrap();

    let expected_proof = read_file("./tests/pi");
    assert_eq!(expected_proof.len(), 80);

    let alpha = read_file("./tests/alpha");
    let message = String::from_utf8_lossy(&alpha[..1]);
    assert_eq!(message.len(), 1);

    let proof = vrf_prove(&message, &keypair);

    assert_eq!(proof, Ok(expected_proof));
}

#[test]
fn test_vrf_verify() {
    let pk = read_file("./tests/pk");
    assert_eq!(pk.len(), 32);
    let pubkey = Pubkey::new(&pk);

    let proof = read_file("./tests/pi");
    assert_eq!(proof.len(), 80);
    let proof = proof.as_slice().try_into().unwrap();

    let alpha = read_file("./tests/alpha");
    let message = String::from_utf8_lossy(&alpha[..1]);
    assert_eq!(message.len(), 1);

    let expected_hash = read_file("./tests/beta");

    let hash = vrf_verify(&message, &pubkey, proof);

    assert_eq!(hash, Ok(expected_hash));
}

#[test]
fn test_vrf_proof_to_hash() {
    let proof = read_file("./tests/pi");
    assert_eq!(proof.len(), 80);
    let proof = proof.as_slice().try_into().unwrap();

    let expected_hash = read_file("./tests/beta");

    let hash = vrf_proof_to_hash(proof);

    assert_eq!(hash, Ok(expected_hash));
}

fn read_file(filename: &str) -> Vec<u8> {
    read(filename).expect("unable to read file")
}
