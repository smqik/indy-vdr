
use std::ffi::c_char;
//use crate::tools::wallet::Wallet;

use libc::c_int;
use std::ffi::CStr;
//use tokio::runtime::Runtime;
//use tools::wallet::wallet_config::WalletConfig;
use crate::tools::wallet::{wallet_binding::WalletBinding, Wallet};
// use core::ffi::c_char;
// use std::os::raw::c_char;
// use std::ffi::CString;
// use std::ffi::c_char;
// //use crate::tools::wallet::Wallet;

// use libc::c_int;
// use std::ffi::CStr;
// //use tokio::runtime::Runtime;
// //use tools::wallet::wallet_config::WalletConfig;
// use crate::tools::wallet::wallet_binding::WalletBinding;
//use core::ffi::c_char;
//use std::os::raw::c_char;
//use std::ffi::CString;


#[no_mangle]
pub extern "C" fn wallet_create(key_ptr: *const c_char, wallet_name_ptr: *const c_char) -> c_int {
    println!("Wallet function execution success...");
   // Calculate length of key string
    let mut key_len = 0;
    unsafe {
        while *key_ptr.offset(key_len) != 0 {
            key_len += 1;
        }
    }
    // Calculate length of wallet_name string
    let mut wallet_name_len = 0;
    unsafe {
        while *wallet_name_ptr.offset(wallet_name_len) != 0 {
            wallet_name_len += 1;
        }
    }
    // Convert raw pointers to CStr and then to &str
    let key = unsafe { CStr::from_ptr(key_ptr).to_str().expect("Invalid UTF-8 sequence") };
    let wallet_name = unsafe { CStr::from_ptr(wallet_name_ptr).to_str().expect("Invalid UTF-8 sequence") };
    println!("Rust function executing...");
    // Define result variable in a scope that encompasses its usage
    let result = match WalletBinding::parse_config_creds(key, wallet_name) {
        Ok(wallet_binding) => {
            // Access fields of WalletBinding
            let config = wallet_binding.config;
            let credential = wallet_binding.credential;
            // Now you can use config and credential as needed
            println!("Config: {:?}", config);
            println!("Credential: {:?}", credential);
            // Create a Tokio  
        //    let rt = Runtime::new().unwrap();
            // Call the async function and await its result
            WalletBinding::wallet_create_async_func(config, credential)
            
        }
        Err(cli_error) => {
            // Handle error
            println!("Error: {:?}", cli_error);
            // Return an error value if needed
            return -1; // For example, returning -1 to indicate an error
        }
    };
    // Create an async runtime (Tokio) to execute the async function
    println!("Rust function executed..");
    // Use the result
    match result {
        Ok(_) => println!("Wallet creation succeeded"),
        Err(e) => println!("Wallet creation failed: {:?}", e),
    }
    // Return value indicating success
    0
}



#[no_mangle]
pub extern "C" fn wallet_open(key_ptr: *const c_char, wallet_name_ptr: *const c_char) -> Wallet {
    println!("Wallet open function execution in progress...");
   // Calculate length of key string
    let mut key_len = 0;
    unsafe {
        while *key_ptr.offset(key_len) != 0 {
            key_len += 1;
        }
    }
    // Calculate length of wallet_name string
    let mut wallet_name_len = 0;
    unsafe {
        while *wallet_name_ptr.offset(wallet_name_len) != 0 {
            wallet_name_len += 1;
        }
    }
    // Convert raw pointers to CStr and then to &str
    let key = unsafe { CStr::from_ptr(key_ptr).to_str().expect("Invalid UTF-8 sequence") };
    let wallet_name = unsafe { CStr::from_ptr(wallet_name_ptr).to_str().expect("Invalid UTF-8 sequence") };
    println!("Open Rust function executing...");
    // Define result variable in a scope that encompasses its usage
    let result = match WalletBinding::parse_config_creds(key, wallet_name) {
        Ok(wallet_binding) => {
            let config = wallet_binding.config;
            let credential = wallet_binding.credential;
            
            // Open the wallet asynchronously
            let wallet_result = WalletBinding::wallet_open_async_func(config, credential);
            match wallet_result {
                Ok(wallet) => Some(wallet), // Return the wallet
                Err(error) => {
                    // Handle the error if needed
                    println!("Error opening wallet: {:?}", error);
                    None // Return None if an error occurs
                }
            }
        }
        Err(cli_error) => {
            // Handle the error if needed
            println!("Error parsing configuration: {:?}", cli_error);
            None // Return None if an error occurs
        },
    };
    
    // Return the wallet or handle the error
    match result {
        Some(wallet) => wallet, // Return the wallet if it exists
        None => {
            // You may return a default wallet or panic here, depending on your requirements
            // For example:
            panic!("Failed to open wallet");
        }
    }    // Use the result
    // match result {
    //     Ok(_) => println!("Wallet creation succeeded"),
    //     Err(e) => println!("Wallet creation failed: {:?}", e),
    // }
    // Return value indicating success
}
