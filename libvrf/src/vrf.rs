use {
    domichain_sdk::{
        pubkey::Pubkey,
        signature::{Keypair},
    },
    std::{
        ffi::CString,
        os::raw::c_uchar,
    },
};

pub fn vrf_prove(message: &str, keypair: &Keypair) -> Vec<u8> {
    let c_message = CString::new(message).expect("CString::new failed");
    let c_message_bytes = c_message.into_bytes();
    let c_message_slice = c_message_bytes.as_slice();

    let proof_len = 80;
    let mut proof_vec : Vec<c_uchar> = vec![0; proof_len];
    let proof_vec_slice = proof_vec.as_mut_slice();

    // let skpk_len = 64 + 32;
    let skpk_vec = keypair.to_bytes().to_vec();
    let skpk_vec_slice = skpk_vec.as_slice();

    unsafe {
        // int vrf_prove(
        //     unsigned char proof[80],
        //     const unsigned char skpk[64],
        //     const unsigned char *msg,
        //     unsigned long long msglen,
        // )
        let proof_vec_ptr = proof_vec_slice.as_mut_ptr();
        let skpk_vec_ptr = skpk_vec_slice.as_ptr();
        let c_message_ptr = c_message_slice.as_ptr();
        let c_message_len = message.len().try_into().unwrap();

        let res = libvrf_sys::vrf_prove(
            proof_vec_ptr,
            skpk_vec_ptr,
            c_message_ptr,
            c_message_len,
        );

        println!("vrf_prove result code: {}", res);
    }

    proof_vec
}

pub fn vrf_verify(message: &str, pubkey: &Pubkey, proof: &[u8; 80]) -> Vec<u8> {
    let c_message = CString::new(message).expect("CString::new failed");
    let c_message_bytes = c_message.into_bytes();
    let c_message_slice = c_message_bytes.as_slice();

    let hash_len = 64;
    let mut hash_vec : Vec<c_uchar> = vec![0; hash_len];
    let hash_vec_slice = hash_vec.as_mut_slice();

    let pk_vec = pubkey.to_bytes().to_vec();
    let pk_vec_slice = pk_vec.as_slice();

    unsafe {
        // int vrf_verify(
        //     unsigned char output[64],
        //     const unsigned char pk[32],
        //     const unsigned char proof[80],
        //     const unsigned char *msg,
        //     unsigned long long msglen
        // );
        let hash_vec_ptr = hash_vec_slice.as_mut_ptr();
        let pk_vec_ptr = pk_vec_slice.as_ptr();
        let proof_vec_ptr = proof.as_ptr();
        let c_message_ptr = c_message_slice.as_ptr();
        let c_message_len = message.len().try_into().unwrap();

        let res = libvrf_sys::vrf_verify(
            hash_vec_ptr,
            pk_vec_ptr,
            proof_vec_ptr,
            c_message_ptr,
            c_message_len,
        );

        println!("vrf_verify result code: {}", res);
    }

    hash_vec
}

pub fn vrf_proof_to_hash(proof: &[u8; 80]) -> Vec<u8> {
    let hash_len = 64;
    let mut hash_vec : Vec<c_uchar> = vec![0; hash_len];
    let hash_vec_slice = hash_vec.as_mut_slice();
    
    unsafe {
        // int vrf_proof_to_hash(
        //     unsigned char hash[64],
        //     const unsigned char proof[80]
        // );
        let hash_vec_ptr = hash_vec_slice.as_mut_ptr();
        let proof_vec_ptr = proof.as_ptr();

        let res = libvrf_sys::vrf_proof_to_hash(
            hash_vec_ptr,
            proof_vec_ptr,
        );

        println!("vrf_proof_to_hash result code: {}", res);
    }

    hash_vec
}