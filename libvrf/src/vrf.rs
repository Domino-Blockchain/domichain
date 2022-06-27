use {
    domichain_sdk::{
        signature::{Keypair},
    },
    std::{
        ffi::CString,
        io::Result,
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

    let skpk_len = 64 + 32;
    let skpk_vec : Vec<c_uchar> = vec![0; skpk_len];
    let skpk_vec_slice = skpk_vec.as_slice();

    let pubkey = keypair.public.to_bytes();
    let secret = keypair.secret().to_bytes();
    skpk_vec[0..secret.len()].copy_from_slice(&secret);
    skpk_vec[secret.len()..].copy_from_slice(&pubkey);

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

        let res = libvrf_sys::vrf_prove(proof_vec_ptr, skpk_vec_ptr, c_message_ptr, message.len().try_into().unwrap());

        println!("vrf_prove result code: {}", res);
    }

    proof_vec
}
