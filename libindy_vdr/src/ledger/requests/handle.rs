use sha2::{Digest, Sha256};

use super::constants::{HANDLE, HANDLE_GET};
use super::did::ShortDidValue;
use super::{get_sp_key_marker, ProtocolVersion, RequestType};
use crate::common::error::VdrResult;

#[derive(Serialize, PartialEq, Debug)]
pub struct HandleOperation {
    #[serde(rename = "type")]
    pub _type: String,
    pub dest: ShortDidValue,
    pub handle: String,
}

impl HandleOperation {
    pub fn new(
        dest: ShortDidValue,
        handle: String,
    ) -> HandleOperation {
        HandleOperation {
            _type: Self::get_txn_type().to_string(),
            dest,
            handle,
        }
    }
}

impl RequestType for HandleOperation {
    fn get_txn_type<'a>() -> &'a str {
        HANDLE
    }
}

// #[derive(Serialize, PartialEq, Debug)]
// pub struct GetAttribOperation {
//     #[serde(rename = "type")]
//     pub _type: String,
//     pub dest: ShortDidValue,
//     #[serde(skip_serializing_if = "Option::is_none")]
//     pub raw: Option<String>,
//     #[serde(skip_serializing_if = "Option::is_none")]
//     pub hash: Option<String>,
//     #[serde(skip_serializing_if = "Option::is_none")]
//     pub enc: Option<String>,
//     #[serde(skip_serializing_if = "Option::is_none")]
//     pub seq_no: Option<i32>,
//     #[serde(skip_serializing_if = "Option::is_none")]
//     pub timestamp: Option<u64>,
// }

// impl GetAttribOperation {
//     pub fn new(
//         dest: ShortDidValue,
//         raw: Option<String>,
//         hash: Option<String>,
//         enc: Option<String>,
//         seq_no: Option<i32>,
//         timestamp: Option<u64>,
//     ) -> GetAttribOperation {
//         GetAttribOperation {
//             _type: Self::get_txn_type().to_string(),
//             dest,
//             raw,
//             hash,
//             enc,
//             seq_no,
//             timestamp,
//         }
//     }
// }

// impl RequestType for GetAttribOperation {
//     fn get_txn_type<'a>() -> &'a str {
//         GET_ATTR
//     }

//     fn get_sp_key(&self, protocol_version: ProtocolVersion) -> VdrResult<Option<Vec<u8>>> {
//         if let Some(attr_name) = self
//             .raw
//             .as_ref()
//             .or(self.enc.as_ref())
//             .or(self.hash.as_ref())
//         {
//             let marker = get_sp_key_marker(1, protocol_version);
//             let hash = Sha256::digest(attr_name.as_bytes());
//             return Ok(Some(
//                 format!("{}:{}:{}", &*self.dest, marker, hex::encode(hash))
//                     .as_bytes()
//                     .to_vec(),
//             ));
//         }
//         Ok(None)
//     }
// }
