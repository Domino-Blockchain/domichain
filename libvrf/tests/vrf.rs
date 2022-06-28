use {
    libvrf::vrf::*,
    domichain_sdk::{
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

fn read_file(filename: &str) -> Vec<u8> {
    let mut file = File::open(&filename).expect("no file found");

    let metadata = metadata(&filename).expect("unable to read metadata");
    let mut buffer = vec![0; metadata.len() as usize];
    file.read(&mut buffer).expect("buffer overflow");

    buffer
}
