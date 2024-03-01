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
fn test_vrf_verify_error_case() {
    let message = String::from_utf8(vec![72, 89, 105, 114, 109, 100, 85, 89, 97, 109, 86, 119, 100, 51, 106, 100, 118, 97, 98, 103, 102, 114, 119, 120, 76, 102, 76, 71, 100, 69, 75, 56, 112, 115, 111, 114, 104, 81, 98, 105, 65, 100, 78, 117]).unwrap();
    let pubkey = Pubkey::from([99, 153, 233, 89, 188, 223, 93, 217, 204, 207, 204, 114, 98, 76, 227, 98, 83, 252, 152, 64, 190, 88, 48, 163, 230, 133, 23, 0, 250, 175, 170, 119]);
    let proof: [u8; 80] = [10, 28, 164, 251, 139, 105, 133, 237, 140, 101, 234, 249, 231, 96, 250, 108, 182, 132, 153, 61, 138, 86, 220, 39, 37, 130, 80, 66, 150, 114, 45, 246, 102, 177, 150, 144, 116, 15, 52, 79, 246, 135, 144, 24, 54, 66, 130, 213, 143, 95, 179, 100, 119, 47, 44, 214, 144, 64, 174, 212, 74, 132, 54, 11, 25, 218, 118, 24, 94, 62, 28, 237, 216, 199, 98, 112, 241, 172, 126, 2];
    let hash = vrf_verify(&message, &pubkey, &proof);
    assert_eq!(hash, Err(-1));
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
