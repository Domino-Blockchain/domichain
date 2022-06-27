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
    let keypair = Keypair::new();
    let expected_proof = read_file("./tests/pi");
    let message = "message";

    let proof = vrf_prove(message, &keypair);

    assert_eq!(proof, expected_proof);
}

fn read_file(filename: &str) -> Vec<u8> {
    let mut file = File::open(&filename).expect("no file found");

    let metadata = metadata(&filename).expect("unable to read metadata");
    let mut buffer = vec![0; metadata.len() as usize];
    file.read(&mut buffer).expect("buffer overflow");

    buffer
}
