use std::ffi::c_char;
//use crate::tools::wallet::Wallet;

//use libc::c_int;
use std::ffi::CStr;
//use futures_util::future::ok;

use crate::tools::did::Did;
use crate::ffi::wallet;
use crate::tools::did::DidInfo;

#[no_mangle]
pub extern "C" fn did_create(key_ptr: *const c_char, wallet_name_ptr: *const c_char, seed_ptr: *const c_char, method_ptr: *const c_char, metadata_ptr: *const c_char) -> Result<(), ()> {
    println!("Starting DID creation...");
    let seed = unsafe { CStr::from_ptr(seed_ptr).to_str().expect("Invalid UTF-8 sequence") };
    let method = unsafe { CStr::from_ptr(method_ptr).to_str().expect("Invalid UTF-8 sequence") };
    let metadata = unsafe { CStr::from_ptr(metadata_ptr).to_str().expect("Invalid UTF-8 sequence") };
    let openwallet = wallet::wallet_open(key_ptr, wallet_name_ptr);
    let (did, vk) = Did::create(&openwallet,Some(seed),Some(metadata), Some(method))
    .map_err(|err| println_err!("{}", err.message(None)))?;
    let abbreviate_verkey = Did::abbreviate_verkey(&did, &vk);
    let vk = abbreviate_verkey.unwrap_or(vk);
    println_succ!("Did \"{}\" has been created with \"{}\" verkey", did, vk);
    Ok(())
}

#[no_mangle]
pub extern "C" fn did_list(key_ptr: *const c_char, wallet_name_ptr: *const c_char) -> Result<Vec<DidInfo>, ()> {
    println!("Listing DIDs...");
    let openwallet = wallet::wallet_open(key_ptr, wallet_name_ptr);
    let mut dids: Vec<crate::tools::did::DidInfo> = Did::list(&openwallet).map_err(|err| println_err!("{}", err.message(None)))?;
        for did_info in dids.iter_mut() {
            did_info.verkey = Did::abbreviate_verkey(&did_info.did, &did_info.verkey)
                .unwrap_or_else(|_| did_info.verkey.clone());
        }
    let vec = dids;
    Ok(vec)
}

#[no_mangle]
pub extern "C" fn did_use() {
    
}