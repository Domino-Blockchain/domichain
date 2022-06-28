use {
    domichain_sdk::{
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
