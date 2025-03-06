extern crate tokio;
//use aries_askar::{Error as AskarError, ErrorKind as AskarErrorKind};
//use futures::io::empty;
use serde_json::Value;
//use crate::tools::wallet::credentials;
//use crate::unwrap_or_return;
//use futures::future::ready;
//use futures::executor::block_on; // Make sure to include futures or tokio for block_on
//use crate::tools::wallet::Credentials;
use crate::utils::clierror::CliError;
use crate::{
    tools::wallet::{Credentials, WalletConfig}
    //tools::wallet::;
};
use crate::tools::wallet::Wallet;
//use super::Credentials;
//use crate::tools::wallet::WalletConfig;
//use tokio::runtime::Runtime;
#[allow(dead_code)]
//#[derive(Debug, Default, Serialize, Deserialize)]
pub struct WalletBinding {
    pub config: WalletConfig,
    pub credential: Credentials
}
#[allow(dead_code)]
impl WalletBinding {
    //pub(crate) fn new(config: WalletConfig, credential: Credentials) -> Self { Self { config, credential } }
    pub fn parse_config_creds(key_str: &str, wallet_name: &str) -> Result<WalletBinding, CliError> {
        let walletconfig = WalletConfig {
            id: String::from(wallet_name),
            storage_type: String::from("sqlite"),
           storage_config: Some(Value::String(String::from("")))
        };
        let credentials = Credentials {
            key: String::from(key_str),
            key_derivation_method: Some(String::from("argon2m")),
           // storage_credentials: Some(String::from()),
            ..Credentials::default()
        };
        trace!("Wallet::create_wallet try: config {:?}", walletconfig);
        Ok(WalletBinding {
            config: walletconfig,
            credential: credentials
        })
    }
    pub fn wallet_create_async_func(config: WalletConfig, credentials: Credentials) -> Result<(), CliError> {
        // Assuming Wallet::create returns a Result
        Wallet::create(&config, &credentials)
    }
    // pub fn credential(&self) -> &Credentials {
    //   &self.credential
    // }
}
// //use super::Credentials;
// //
// // Assume these functions exist and can convert a &str to WalletConfig and Credentials respectively
// impl WalletBinding {
//      fn parse_config_creds(key_str: &str, wallet_name: &str) -> Result<WalletBinding, CliError> {
//         let walletconfig = WalletConfig {
//             id: String::from(wallet_name),
//             storage_type: String::from("sqlite"),
//            storage_config: Some(Value::String(String::from("")))
//         };
//         let credentials = Credentials {
//             key: String::from(key_str),
//             key_derivation_method: Some(String::from("argon2m")),
//            // storage_credentials: Some(String::from()),
//             ..Credentials::default()
//         };
//         trace!("Wallet::create_wallet try: config {:?}", walletconfig);
//         Ok(WalletBinding {
//             config: walletconfig,
//             credential: credentials
//         })
//     }
    // pub fn parse_wallet_config(json_str: &str) -> Result<WalletBinding, CliError> {
    //     // Parse the JSON string into a serde_json::Value
    // let json_value: JsonValue = serde_json::from_str(json_str)?;
    // // Extract the fields from the JSON value
    // let id = json_value["id"].as_str().ok_or_else(|| CliError::InvalidInput("Missing id field".to_string()))?.to_string();
    // let storage_type = json_value["storage_type"].as_str().ok_or_else(|| CliError::InvalidInput("Missing storage_type field".to_string()))?.to_string();
    // let storage_config = json_value["storage_config"].clone().into();
    // // Create and return the WalletConfig struct
    // let config = WalletConfig {
    //     id,
    //     storage_type,
    //     storage_config,
    // };
    // let credential = WalletCredentials {
    //     key: Default::default(), // Provide default or blank values for each field
    //     key_method: Default::default(),
    //     rekey: Default::default(),
    //     rekey_method: Default::default(),
    // };
    // // Create and return the WalletBinding struct
    // Ok(WalletBinding {
    //     config,
    //     credential
    // })
    // }
    // pub fn parse_credentials(json_str: &str) -> Result<WalletCredentials, CliError> {
    //     // Parse the JSON string into a serde_json::Value
    //     let json_value: serde_json::Value = serde_json::from_str(json_str)?;
    //     // Extract the fields from the JSON value
    //     let key_str = json_value["key"].as_str().ok_or_else(|| CliError::InvalidInput("Missing key field".to_string()))?;
    //     let key = PassKey::from(key_str);
    //     let key_method_str = json_value["key_method"].as_str().ok_or_else(|| CliError::InvalidInput("Missing key_method field".to_string()))?;
    //     let key_method = StoreKeyMethod::parse_uri(key_method_str);
    //     let rekey_str = json_value["rekey"].as_str();
    //     let rekey = match rekey_str {
    //         Some(rekey_str) => Some(PassKey::from(rekey_str)),
    //         None => None,
    //     };
    //     let rekey_method_str = json_value["rekey_method"].as_str();
    //     let rekey_method: Option<StoreKeyMethod> = match rekey_method_str {
    //         Some(rekey_method_str) => Some(StoreKeyMethod::from_str(rekey_method_str)?),
    //         None => None,
    //     };
    //     // Create and return the WalletCredentials struct
    //     Ok(WalletCredentials {
    //         key,
    //         key_method,
    //         rekey,
    //         rekey_method,
    //     })
    // }
//}
// pub extern "C" fn wallet_create(config : *const c_char, credentials: *const c_char) -> c_int {
//     let config_str = unsafe { CStr::from_ptr(config).to_str().unwrap() };
//     let credentials_str = unsafe { CStr::from_ptr(credentials).to_str().unwrap() };
//     // Conver1t strings to WalletConfig and Credentials Rust types
//     let config = match parse_wallet_config(config_str) {
//         Ok(c) => c,
//         Err(_) =>
//          1, // Simplified error handling
//     };
//     let credentials = match parse_credentials(credentials_str) {
//         Ok(c) => c,
//         Err(_) => return 1, // Simplified error handling
//     };
//    let result = some_async_function(config, credentials);
//     // Call the original Wallet::create method
//    // let result = block_on(Wallet::create(&config, &credentials));
//     match result {
//         Ok(_) => 0, // Success
//         Err(_) => 1, // Error
//     }
// }
// #[no_mangle]
// pub extern "C" fn wallet_create(config: *const c_char, credentials: *const c_char) -> c_int {
//     println!("Rust function executing...");
//     let config_str = unsafe { CStr::from_ptr(config).to_str().unwrap() };
//     let credentials_str = unsafe { CStr::from_ptr(credentials).to_str().unwrap() };
//     let config = match parse_wallet_config(config_str) {
//         Ok(c) => c,
//         Err(_) => return 1, // Simplified error handling
//     };
//     let credentials = match parse_credentials(credentials_str) {
//         Ok(c) => c,
//         Err(_) => return 1, // Simplified error handling
//     };
//     // Create an async runtime (Tokio) to execute the async function
//     let result = tokio::runtime::Runtime::new().unwrap().block_on(async {
//         // Call the async function and await its result
//         match some_async_function(config, credentials).await {
//             Ok(_) => 0, // Success
//             Err(_) => 1, // Error
//         }
//     });
//     println!("Rust function executed..");
//     result
// }