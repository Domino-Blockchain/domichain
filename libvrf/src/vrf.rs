use {
    domichain_sdk::{
        pubkey::Pubkey,
        signature::{Keypair},
    },
    log::*,
    std::{
        ffi::CString,
        os::raw::c_uchar,
    },
};

pub const PROOF_LEN: usize = 80;
pub const HASH_LEN: usize = 64;

pub fn vrf_prove(message: &str, keypair: &Keypair) -> Result<Vec<u8>, i32> {
    let c_message = CString::new(message).expect("CString::new failed");
    let c_message_bytes = c_message.into_bytes();
    let c_message_ptr = c_message_bytes.as_ptr();
    let c_message_len = c_message_bytes.len() as u64;

    let mut proof_vec : Vec<c_uchar> = vec![0; PROOF_LEN];
    let proof_vec_ptr = proof_vec.as_mut_ptr();
    let skpk_vec_ptr = keypair.to_bytes().as_ptr();

    let res = unsafe {
        // int vrf_prove(
        //     unsigned char proof[80],
        //     const unsigned char skpk[64],
        //     const unsigned char *msg,
        //     unsigned long long msglen,
        // )
        libvrf_sys::vrf_prove(
            proof_vec_ptr,
            skpk_vec_ptr,
            c_message_ptr,
            c_message_len,
        )
    };
    debug!("vrf_prove result code: {}", res);

    match res {
        0 => Ok(proof_vec),
        err => Err(err),
    }
}

pub fn vrf_verify(message: &str, pubkey: &Pubkey, proof: &[u8; PROOF_LEN]) -> Result<Vec<u8>, i32> {
    let c_message = CString::new(message).expect("CString::new failed");
    let c_message_bytes = c_message.into_bytes();
    let c_message_ptr = c_message_bytes.as_ptr();
    let c_message_len = c_message_bytes.len() as u64;

    let mut hash_vec : Vec<c_uchar> = vec![0; HASH_LEN];
    let hash_vec_ptr = hash_vec.as_mut_ptr();

    let pk_vec_ptr = pubkey.to_bytes().as_ptr();
    let proof_vec_ptr = proof.as_ptr();

    let res = unsafe {
        // int vrf_verify(
        //     unsigned char output[64],
        //     const unsigned char pk[32],
        //     const unsigned char proof[80],
        //     const unsigned char *msg,
        //     unsigned long long msglen
        // );
        libvrf_sys::vrf_verify(
            hash_vec_ptr,
            pk_vec_ptr,
            proof_vec_ptr,
            c_message_ptr,
            c_message_len,
        )
    };
    debug!("vrf_verify result code: {}", res);

    match res {
        0 => Ok(hash_vec),
        err => Err(err),
    }
}

pub fn vrf_proof_to_hash(proof: &[u8; PROOF_LEN]) -> Result<Vec<u8>, i32> {
    let mut hash_vec : Vec<c_uchar> = vec![0; HASH_LEN];
    let hash_vec_ptr = hash_vec.as_mut_ptr();
    
    let proof_vec_ptr = proof.as_ptr();

    let res = unsafe {
        // int vrf_proof_to_hash(
        //     unsigned char hash[64],
        //     const unsigned char proof[80]
        // );
        libvrf_sys::vrf_proof_to_hash(
            hash_vec_ptr,
            proof_vec_ptr,
        )
    };
    debug!("vrf_proof_to_hash result code: {}", res);

    match res {
        0 => Ok(hash_vec),
        err => Err(err),
    }
}
