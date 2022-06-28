use {
    libvrf::vrf::*,
    domichain_sdk::{
        pubkey::Pubkey,
        signature::{Keypair},
    },
    std::{
        fs::File,
        fs::metadata,
        io::Read,
    },
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

    assert_eq!(proof, expected_proof);
}

#[test]
fn test_vrf_verify() {
    let pk = read_file("./tests/pk");
    assert_eq!(pk.len(), 32);
    let pubkey = Pubkey::new(&pk);

    let proof = read_file("./tests/pi");
    assert_eq!(proof.len(), 80);

    let alpha = read_file("./tests/alpha");
    let message = String::from_utf8_lossy(&alpha[..1]);
    assert_eq!(message.len(), 1);

    let expected_hash = read_file("./tests/beta");

    let hash = vrf_verify(&message, &pubkey, proof.as_slice().try_into().unwrap());

    assert_eq!(hash, expected_hash);
}

#[test]
fn test_vrf_proof_to_hash() {
    let proof = read_file("./tests/pi");
    assert_eq!(proof.len(), 80);

    let expected_hash = read_file("./tests/beta");

    let hash = vrf_proof_to_hash(proof.as_slice().try_into().unwrap());

    assert_eq!(hash, expected_hash);
}

fn read_file(filename: &str) -> Vec<u8> {
    let mut file = File::open(&filename).expect("no file found");

    let metadata = metadata(&filename).expect("unable to read metadata");
    let mut buffer = vec![0; metadata.len() as usize];
    file.read(&mut buffer).expect("buffer overflow");

    buffer
}
