extern crate rlp;

pub(crate) mod constants;
mod node;
pub(crate) mod types;

use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

use indy_blssignatures::{Bls, Generator, MultiSignature, VerKey};
use indy_data_types::merkle_tree::{MerkleTree, Positioned};
use rlp::UntrustedRlp;
use serde_json::Value as SJsonValue;
use sha2::{Digest, Sha256};

use crate::common::error::prelude::*;
use crate::pool::{ProtocolVersion, StateProofAssertions, StateProofResult, VerifierKeys};
use crate::utils::base58;
use crate::utils::base64;

use self::constants::{
    REQUESTS_FOR_MULTI_STATE_PROOFS, REQUESTS_FOR_STATE_PROOFS,
    REQUESTS_FOR_STATE_PROOFS_IN_THE_PAST,
};
use self::node::{Node, TrieDB};
use self::types::*;

pub use types::ParsedSP;

/// A `StateProofParser` appropriate for attaching to a `PreparedRequest`
pub struct BoxedSPParser(Box<dyn StateProofParser + Send + Sync>);

impl std::ops::Deref for BoxedSPParser {
    type Target = dyn StateProofParser;
    fn deref(&self) -> &(dyn StateProofParser + 'static) {
        &*self.0
    }
}

impl PartialEq for BoxedSPParser {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
impl Eq for BoxedSPParser {}

/// Construct a `StateProofParser` from a simple callback function
pub fn state_proof_parser_fn<F>(cb: F) -> impl StateProofParser
where
    F: Fn(&str, &str) -> Option<Vec<ParsedSP>> + Send,
{
    StateProofParserFn(cb)
}

/// Custom state proof parser implementation
pub trait StateProofParser {
    /// Construct a `BoxedSPParser` from this instance
    fn boxed(self) -> BoxedSPParser
    where
        Self: Send + Sync + Sized + 'static,
    {
        BoxedSPParser(Box::new(self))
    }

    /// Parse a node message into a sequence of `ParsedSP` instances
    fn parse(&self, txn_type: &str, raw_msg: &str) -> Option<Vec<ParsedSP>>;
}

struct StateProofParserFn<F>(F)
where
    F: Fn(&str, &str) -> Option<Vec<ParsedSP>>;

impl<F> StateProofParser for StateProofParserFn<F>
where
    F: Fn(&str, &str) -> Option<Vec<ParsedSP>> + Send,
{
    fn parse(&self, txn_type: &str, raw_msg: &str) -> Option<Vec<ParsedSP>> {
        self.0(txn_type, raw_msg)
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn check_state_proof(
    msg_result: &SJsonValue,
    f: usize,
    gen: &Generator,
    bls_keys: &VerifierKeys,
    raw_msg: &str,
    sp_key: Option<&[u8]>,
    requested_timestamps: (Option<u64>, Option<u64>),
    last_write_time: u64,
    threshold: u64,
    custom_state_proof_parser: Option<&BoxedSPParser>,
) -> StateProofResult {
    trace!("process_reply: Try to verify proof and signature >>");

    let res = match parse_generic_reply_for_proof_checking(
        msg_result,
        raw_msg,
        sp_key,
        custom_state_proof_parser,
    ) {
        Some(parsed_sps) => {
            trace!("process_reply: Proof and signature are present");
            match verify_parsed_sp(parsed_sps, bls_keys, f, gen) {
                Ok((asserts, None)) => {
                    if check_freshness(msg_result, requested_timestamps, last_write_time, threshold)
                    {
                        StateProofResult::Verified(asserts)
                    } else {
                        StateProofResult::Expired(asserts)
                    }
                }
                Ok((asserts, Some(verify_err))) => {
                    StateProofResult::Invalid(verify_err, Some(asserts))
                }
                Err(err) => StateProofResult::Invalid(err, None),
            }
        }
        None => StateProofResult::Missing,
    };

    trace!(
        "process_reply: Try to verify proof and signature << {:?}",
        res
    );
    res
}

pub(crate) fn check_freshness(
    msg_result: &SJsonValue,
    requested_timestamps: (Option<u64>, Option<u64>),
    last_write_time: u64,
    threshold: u64,
) -> bool {
    trace!(
        "check_freshness: requested_timestamps: {:?} >>",
        requested_timestamps
    );

    let res = match requested_timestamps {
        (Some(from), Some(to)) => {
            let left_last_write_time = extract_left_last_write_time(msg_result).unwrap_or(0);
            trace!("Last last signed time: {}", left_last_write_time);
            trace!("Last right signed time: {}", last_write_time);

            let left_time_for_freshness_check = from;
            let right_time_for_freshness_check = to;

            trace!(
                "Left time for freshness check: {}",
                left_time_for_freshness_check
            );
            trace!(
                "Right time for freshness check: {}",
                right_time_for_freshness_check
            );

            left_time_for_freshness_check <= threshold + left_last_write_time
                && right_time_for_freshness_check <= threshold + last_write_time
        }
        (None, Some(to)) => {
            let time_for_freshness_check = to;

            trace!("Last signed time: {}", last_write_time);
            trace!("Time for freshness check: {}", time_for_freshness_check);

            time_for_freshness_check <= threshold + last_write_time
        }
        (Some(from), None) => {
            let left_last_write_time = extract_left_last_write_time(msg_result).unwrap_or(0);

            trace!("Last last signed time: {}", left_last_write_time);
            trace!("Last right signed time: {}", last_write_time);

            let left_time_for_freshness_check = from;
            let time_for_freshness_check = get_cur_time();

            trace!(
                "Left time for freshness check: {}",
                left_time_for_freshness_check
            );
            trace!("Time for freshness check: {}", time_for_freshness_check);

            left_time_for_freshness_check <= threshold + left_last_write_time
                && time_for_freshness_check <= threshold + last_write_time
        }
        (None, None) => {
            let time_for_freshness_check = get_cur_time();

            trace!("Last signed time: {}", last_write_time);
            trace!("Time for freshness check: {}", time_for_freshness_check);

            time_for_freshness_check <= threshold + last_write_time
        }
    };

    trace!("check_freshness << {:?} ", res);

    res
}

pub(crate) fn get_cur_time() -> u64 {
    let since_epoch = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time has gone backwards");
    let res = since_epoch.as_secs();
    trace!("Current time: {}", res);
    res
}

fn extract_left_last_write_time(msg_result: &SJsonValue) -> Option<u64> {
    let state_proof = &msg_result["data"]["stateProofFrom"]
        .as_object()
        .or_else(|| msg_result["state_proof"].as_object());
    match (msg_result["type"].as_str(), state_proof) {
        (Some(constants::GET_REVOC_REG_DELTA), Some(state_proof)) => {
            state_proof["multi_signature"]["value"]["timestamp"].as_u64()
        }
        _ => None,
    }
}

pub(crate) fn result_without_state_proof(result: &SJsonValue) -> SJsonValue {
    let mut result_without_proof = result.clone();
    result_without_proof
        .as_object_mut()
        .map(|obj| obj.remove("state_proof"));

    if result_without_proof["data"].is_object() {
        result_without_proof["data"]
            .as_object_mut()
            .map(|obj| obj.remove("stateProofFrom"));

        // Sort participant names to allow consensus matching
        if let Some(multi_sig) = result_without_proof["data"]["multi_signature"].as_object_mut() {
            if let Some(participants) = multi_sig["participants"].as_array() {
                let mut entries = participants
                    .iter()
                    .filter_map(SJsonValue::as_str)
                    .collect::<Vec<&str>>();
                entries.sort_unstable();
                multi_sig["participants"] = SJsonValue::from(entries);
            }
        }
    }

    result_without_proof
}

pub(crate) fn parse_generic_reply_for_proof_checking(
    json_msg: &SJsonValue,
    raw_msg: &str,
    sp_key: Option<&[u8]>,
    custom_state_proof_parser: Option<&BoxedSPParser>,
) -> Option<Vec<ParsedSP>> {
    let type_ = if let Some(type_) = json_msg["type"].as_str() {
        trace!("type_: {:?}", type_);
        type_
    } else {
        debug!("parse_generic_reply_for_proof_checking: <<< No type field");
        return None;
    };

    if REQUESTS_FOR_STATE_PROOFS.contains(&type_) {
        if let Some(sp_key) = sp_key {
            _parse_reply_for_builtin_sp(json_msg, type_, sp_key)
        } else {
            debug!("parse_generic_reply_for_proof_checking: no sp_key for built-in type");
            None
        }
    } else if let Some(custom_state_proof_parser_) = custom_state_proof_parser {
        custom_state_proof_parser_.parse(type_, raw_msg)
    } else {
        trace!("parse_generic_reply_for_proof_checking: <<< type not supported");
        None
    }
}

pub(crate) fn verify_parsed_sp(
    parsed_sps: Vec<ParsedSP>,
    nodes: &VerifierKeys,
    f: usize,
    gen: &Generator,
) -> Result<(StateProofAssertions, Option<String>), String> {
    let mut multi_sig: Option<SJsonValue> = None;

    for parsed_sp in parsed_sps {
        if parsed_sp.multi_signature["value"]["state_root_hash"]
            .as_str()
            .ne(&Some(&parsed_sp.root_hash))
            && parsed_sp.multi_signature["value"]["txn_root_hash"]
                .as_str()
                .ne(&Some(&parsed_sp.root_hash))
        {
            return Err("Given signature does not match state proof, aborting verification".into());
        }

        match multi_sig.as_ref() {
            Some(sig) => {
                if sig != &parsed_sp.multi_signature {
                    return Err("No consistency between proof multi signatures".into());
                }
            }
            None => {
                multi_sig.replace(parsed_sp.multi_signature);
            }
        }

        let Ok(proof_nodes) = base64::decode(&parsed_sp.proof_nodes) else {
            return Err("Error decoding proof nodes from state proof".into());
        };
        let Ok(root_hash) = base58::decode(parsed_sp.root_hash) else {
            return Err("Error decoding root hash from state proof".into());
        };
        match parsed_sp.kvs_to_verify {
            KeyValuesInSP::Simple(kvs) => match kvs.verification_type {
                KeyValueSimpleDataVerificationType::Simple => {
                    for (k, v) in kvs.kvs {
                        let Ok(key) = base64::decode(&k) else {
                            return Err("Error decoding proof key".into());
                        };
                        if !_verify_proof(
                            proof_nodes.as_slice(),
                            root_hash.as_slice(),
                            &key,
                            v.as_deref(),
                        ) {
                            return Err("Simple verification failed".into());
                        }
                    }
                }
                KeyValueSimpleDataVerificationType::NumericalSuffixAscendingNoGaps(data) => {
                    if !_verify_proof_range(
                        proof_nodes.as_slice(),
                        root_hash.as_slice(),
                        data.prefix.as_str(),
                        data.from,
                        data.next,
                        &kvs.kvs,
                    ) {
                        return Err("Range verification failed".into());
                    }
                }
                KeyValueSimpleDataVerificationType::MerkleTree(length) => {
                    if !_verify_merkle_tree(
                        proof_nodes.as_slice(),
                        root_hash.as_slice(),
                        &kvs.kvs,
                        length,
                    ) {
                        return Err("Merkle tree verification failed".into());
                    }
                }
            },
            //TODO IS-713 support KeyValuesInSP::SubTrie
            kvs => {
                return Err(format!(
                    "Unsupported parsed state proof format for key-values: {kvs:?}"
                ));
            }
        }
    }

    if let Some(multi_sig) = multi_sig.as_ref() {
        let Some((signature, participants, value)) = _parse_reply_for_proof_signature_checking(multi_sig) else {
            return Err("State proof parsing of reply failed".into());
        };
        let verify_err = match _verify_proof_signature(
            signature,
            participants.as_slice(),
            &value,
            nodes,
            f,
            gen,
        ) {
            Ok(_) => None,
            Err(err) => Some(format!("Proof signature verification failed: {}", err)),
        };
        let Ok(asserts) = serde_json::from_value(multi_sig["value"].clone()) else {
            return Err("Error parsing state proof assertions".into());
        };
        Ok((asserts, verify_err))
    } else {
        Err("Proof signature verification failed: no parsed state proofs".into())
    }
}

pub(crate) fn parse_key_from_request_for_builtin_sp(
    json_msg: &SJsonValue,
    protocol_version: ProtocolVersion,
) -> Option<Vec<u8>> {
    let is_node_1_3 = protocol_version == ProtocolVersion::Node1_3;
    let type_ = json_msg["operation"]["type"].as_str()?;
    let json_msg = &json_msg["operation"];
    let key_suffix: String = match type_ {
        constants::GET_ATTR => {
            if let Some(attr_name) = json_msg["raw"]
                .as_str()
                .or_else(|| json_msg["enc"].as_str())
                .or_else(|| json_msg["hash"].as_str())
            {
                trace!(
                    "parse_key_from_request_for_builtin_sp: GET_ATTR attr_name {:?}",
                    attr_name
                );

                let marker = if is_node_1_3 { '\x01' } else { '1' };
                let hash = Sha256::digest(attr_name.as_bytes());
                format!(":{}:{}", marker, hex::encode(hash))
            } else {
                trace!("parse_key_from_request_for_builtin_sp: <<< GET_ATTR No key suffix");
                return None;
            }
        }
        constants::GET_CRED_DEF => {
            if let (Some(sign_type), Some(sch_seq_no)) = (
                json_msg["signature_type"].as_str(),
                json_msg["ref"].as_u64(),
            ) {
                trace!(
                    "parse_key_from_request_for_builtin_sp: GET_CRED_DEF sign_type {:?}, sch_seq_no: {:?}",
                    sign_type,
                    sch_seq_no
                );
                let marker = if is_node_1_3 { '\x03' } else { '3' };
                let tag = if is_node_1_3 {
                    None
                } else {
                    json_msg["tag"].as_str()
                };
                let tag = tag
                    .map(|t| format!(":{}", t))
                    .unwrap_or_else(|| "".to_owned());
                format!(":{}:{}:{}{}", marker, sign_type, sch_seq_no, tag)
            } else {
                trace!("parse_key_from_request_for_builtin_sp: <<< GET_CRED_DEF No key suffix");
                return None;
            }
        }
        constants::GET_NYM | constants::GET_REVOC_REG_DEF => {
            trace!("parse_key_from_request_for_builtin_sp: GET_NYM");
            "".to_string()
        }
        constants::GET_SCHEMA => {
            if let (Some(name), Some(ver)) = (
                json_msg["data"]["name"].as_str(),
                json_msg["data"]["version"].as_str(),
            ) {
                trace!(
                    "parse_key_from_request_for_builtin_sp: GET_SCHEMA name {:?}, ver: {:?}",
                    name,
                    ver
                );
                let marker = if is_node_1_3 { '\x02' } else { '2' };
                format!(":{}:{}:{}", marker, name, ver)
            } else {
                trace!("parse_key_from_request_for_builtin_sp: <<< GET_SCHEMA No key suffix");
                return None;
            }
        }
        constants::GET_REVOC_REG => {
            //{MARKER}:{REVOC_REG_DEF_ID} MARKER = 6
            if let Some(revoc_reg_def_id) = json_msg["revocRegDefId"].as_str() {
                trace!(
                    "parse_key_from_request_for_builtin_sp: GET_REVOC_REG revoc_reg_def_id {:?}",
                    revoc_reg_def_id
                );
                let marker = if is_node_1_3 { '\x06' } else { '6' };
                format!("{}:{}", marker, revoc_reg_def_id)
            } else {
                trace!("parse_key_from_request_for_builtin_sp: <<< GET_REVOC_REG No key suffix");
                return None;
            }
        }
        constants::GET_AUTH_RULE => {
            if let (Some(auth_type), Some(auth_action), Some(field), new_value, old_value) = (
                json_msg["auth_type"].as_str(),
                json_msg["auth_action"].as_str(),
                json_msg["field"].as_str(),
                json_msg["new_value"].as_str(),
                json_msg["old_value"].as_str(),
            ) {
                trace!(
                    "parse_key_from_request_for_builtin_sp: GET_AUTH_RULE auth_type {:?}",
                    auth_type
                );
                let default_old_value = if auth_action == "ADD" { "*" } else { "" };
                format!(
                    "1:{}--{}--{}--{}--{}",
                    auth_type,
                    auth_action,
                    field,
                    old_value.unwrap_or(default_old_value),
                    new_value.unwrap_or("")
                )
            } else {
                debug!("parse_key_from_request_for_builtin_sp: <<< GET_AUTH_RULE No key suffix");
                return None;
            }
        }
        constants::GET_REVOC_REG_DELTA if json_msg["from"].is_null() => {
            //{MARKER}:{REVOC_REG_DEF_ID} MARKER = 5
            if let Some(revoc_reg_def_id) = json_msg["revocRegDefId"].as_str() {
                trace!(
                    "parse_key_from_request_for_builtin_sp: GET_REVOC_REG_DELTA revoc_reg_def_id {:?}",
                    revoc_reg_def_id
                );
                let marker = if is_node_1_3 { '\x05' } else { '5' };
                format!("{}:{}", marker, revoc_reg_def_id)
            } else {
                debug!(
                    "parse_key_from_request_for_builtin_sp: <<< GET_REVOC_REG_DELTA No key suffix"
                );
                return None;
            }
        }
        // TODO add external verification of indexes
        constants::GET_REVOC_REG_DELTA if !json_msg["from"].is_null() => {
            //{MARKER}:{REVOC_REG_DEF_ID} MARKER = 6 for both
            if let Some(revoc_reg_def_id) = json_msg["revocRegDefId"].as_str() {
                trace!(
                    "parse_key_from_request_for_builtin_sp: GET_REVOC_REG_DELTA revoc_reg_def_id {:?}",
                    revoc_reg_def_id
                );
                let marker = if is_node_1_3 { '\x06' } else { '6' };
                format!("{}:{}", marker, revoc_reg_def_id)
            } else {
                debug!(
                    "parse_key_from_request_for_builtin_sp: <<< GET_REVOC_REG_DELTA No key suffix"
                );
                return None;
            }
        }
        constants::GET_TXN_AUTHR_AGRMT => {
            match (
                json_msg["version"].as_str(),
                json_msg["digest"].as_str(),
                json_msg["timestamp"].as_u64(),
            ) {
                (None, None, _ts) => "2:latest".to_owned(),
                (None, Some(digest), None) => format!("2:d:{}", digest),
                (Some(version), None, None) => format!("2:v:{}", version),
                _ => {
                    debug!("parse_key_from_request_for_builtin_sp: <<< GET_TXN_AUTHR_AGRMT Unexpected combination of request parameters, skip StateProof logic");
                    return None;
                }
            }
        }
        constants::GET_TXN_AUTHR_AGRMT_AML => {
            if let Some(version) = json_msg["version"].as_str() {
                format!("3:v:{}", version)
            } else {
                "3:latest".to_owned()
            }
        }
        constants::GET_TXN => {
            if let Some(seq_no) = json_msg["data"].as_u64() {
                format!("{}", seq_no)
            } else {
                debug!("parse_key_from_request_for_builtin_sp: <<< GET_TXN has no seq_no, skip AuditProof logic");
                return None;
            }
        }
        _ => {
            trace!("parse_key_from_request_for_builtin_sp: <<< Unsupported transaction");
            return None;
        }
    };

    let dest = json_msg["dest"]
        .as_str()
        .or_else(|| json_msg["origin"].as_str());
    let key_prefix = match type_ {
        constants::GET_NYM => {
            if let Some(dest) = dest {
                Sha256::digest(dest.as_bytes()).to_vec()
            } else {
                debug!("parse_key_from_request_for_builtin_sp: <<< No dest");
                return None;
            }
        }
        constants::GET_REVOC_REG
        | constants::GET_REVOC_REG_DELTA
        | constants::GET_TXN_AUTHR_AGRMT
        | constants::GET_TXN_AUTHR_AGRMT_AML
        | constants::GET_AUTH_RULE => Vec::new(),
        constants::GET_REVOC_REG_DEF => {
            if let Some(id) = json_msg["id"].as_str() {
                //FIXME
                id.as_bytes().to_vec()
            } else {
                debug!("parse_key_from_request_for_builtin_sp: <<< No dest");
                return None;
            }
        }
        constants::GET_TXN => vec![],
        _ => {
            if let Some(dest) = dest {
                dest.as_bytes().to_vec()
            } else {
                debug!("parse_key_from_request_for_builtin_sp: <<< No dest");
                return None;
            }
        }
    };

    let mut key = key_prefix;
    key.extend_from_slice(key_suffix.as_bytes());

    Some(key)
}

pub(crate) fn parse_timestamp_from_req_for_builtin_sp(
    req: &SJsonValue,
    op: &str,
) -> (Option<u64>, Option<u64>) {
    if !REQUESTS_FOR_STATE_PROOFS_IN_THE_PAST.contains(&op) {
        return (None, None);
    }

    if op == constants::GET_TXN {
        return (None, Some(0));
    }

    match op {
        constants::GET_REVOC_REG
        | constants::GET_TXN_AUTHR_AGRMT
        | constants::GET_TXN_AUTHR_AGRMT_AML => (None, req["operation"]["timestamp"].as_u64()),
        constants::GET_REVOC_REG_DELTA => (
            req["operation"]["from"].as_u64(),
            req["operation"]["to"].as_u64(),
        ),
        _ => (None, None),
    }
}

fn _parse_reply_for_builtin_sp(
    json_msg: &SJsonValue,
    type_: &str,
    key: &[u8],
) -> Option<Vec<ParsedSP>> {
    trace!("parse_reply_for_builtin_sp: >>> json_msg: {:?}", json_msg);

    assert!(REQUESTS_FOR_STATE_PROOFS.contains(&type_));

    // TODO: FIXME: It is a workaround for Node's problem. Node returns some transactions as strings and some as objects.
    // If node returns marshaled json it can contain spaces and it can cause invalid hash.
    // So we have to save the original string too.
    // See https://jira.hyperledger.org/browse/INDY-699
    let (data, parsed_data): (Option<String>, SJsonValue) = match json_msg["data"] {
        SJsonValue::Null => {
            trace!("parse_reply_for_builtin_sp: Data is null");
            (None, SJsonValue::Null)
        }
        SJsonValue::String(ref strval) => {
            trace!("parse_reply_for_builtin_sp: Data is string");
            if let Ok(parsed_data) = serde_json::from_str(strval) {
                (Some(strval.to_owned()), parsed_data)
            } else {
                trace!("parse_reply_for_builtin_sp: <<< Data field is invalid json");
                return None;
            }
        }
        SJsonValue::Object(ref map) => {
            trace!("parse_reply_for_builtin_sp: Data is object");
            (
                Some(json_msg["data"].to_string()),
                SJsonValue::from(map.clone()),
            )
        }
        SJsonValue::Array(ref array) => {
            trace!("parse_reply_for_builtin_sp: Data is array");
            (
                Some(json_msg["data"].to_string()),
                SJsonValue::from(array.clone()),
            )
        }
        _ => {
            trace!("parse_reply_for_builtin_sp: <<< Data field is invalid type");
            return None;
        }
    };

    let mut state_proofs = vec![];

    match _parse_reply_for_sp(json_msg, data.as_deref(), &parsed_data, type_, key) {
        Ok(state_proof) => state_proofs.push(state_proof),
        Err(err) => {
            debug!("_parse_reply_for_sp: <<<  {}", err);
            return None;
        }
    }

    if REQUESTS_FOR_MULTI_STATE_PROOFS.contains(&type_) {
        match _parse_reply_for_multi_sp(json_msg, data.as_deref(), &parsed_data, type_, key) {
            Ok(Some(state_proof)) => {
                trace!("_parse_reply_for_multi_sp: proof: {:?}", state_proof);
                state_proofs.push(state_proof);
            }
            Ok(None) => {
                trace!("_parse_reply_for_multi_sp: <<<  No proof");
            }
            Err(err) => {
                debug!("_parse_reply_for_multi_sp: <<<  {}", err);
                return None;
            }
        }
    }

    Some(state_proofs)
}

fn _parse_reply_for_sp(
    json_msg: &SJsonValue,
    data: Option<&str>,
    parsed_data: &SJsonValue,
    xtype: &str,
    sp_key: &[u8],
) -> VdrResult<ParsedSP> {
    let (proof, root_hash, ver_type, multi_sig) = if xtype != constants::GET_TXN {
        let proof = if let Some(proof) = json_msg["state_proof"]["proof_nodes"].as_str() {
            trace!("_parse_reply_for_sp: proof: {:?}", proof);
            proof.to_string()
        } else {
            return Err(input_err("No proof"));
        };

        let root_hash = if let Some(root_hash) = json_msg["state_proof"]["root_hash"].as_str() {
            trace!("_parse_reply_for_sp: root_hash: {:?}", root_hash);
            root_hash
        } else {
            return Err(input_err("No root hash"));
        };

        (
            proof,
            root_hash,
            KeyValueSimpleDataVerificationType::Simple,
            json_msg["state_proof"]["multi_signature"].clone(),
        )
    } else {
        let proof = if let Some(path) = parsed_data["auditPath"].as_array() {
            let path_str = json!(path).to_string();
            trace!("parse_reply_for_builtin_sp: proof: {:?}", path);
            base64::encode(path_str)
        } else {
            return Err(input_err("No proof"));
        };

        let root_hash = if let Some(root_hash) = parsed_data["rootHash"].as_str() {
            trace!("_parse_reply_for_sp: root_hash: {:?}", root_hash);
            root_hash
        } else {
            return Err(input_err("No root hash"));
        };

        let len = if let Some(len) = parsed_data["ledgerSize"].as_u64() {
            trace!("Ledger length: {}", len);
            len
        } else {
            return Err(input_err("No ledger length for this proof"));
        };

        (
            proof,
            root_hash,
            KeyValueSimpleDataVerificationType::MerkleTree(len),
            json_msg["state_proof"]
                .get("multi_signature")
                .unwrap_or(&parsed_data["multi_signature"])
                .clone(),
        )
    };

    let value: Option<String> =
        _parse_reply_for_proof_value(json_msg, data, parsed_data, xtype, sp_key)?;
    trace!(
        "_parse_reply_for_sp: <<< proof {:?}, root_hash: {:?}, dest: {:?}, value: {:?}",
        proof,
        root_hash,
        sp_key,
        value
    );

    Ok(ParsedSP {
        root_hash: root_hash.to_owned(),
        proof_nodes: proof,
        multi_signature: multi_sig,
        kvs_to_verify: KeyValuesInSP::Simple(KeyValueSimpleData {
            kvs: vec![(base64::encode(sp_key), value)],
            verification_type: ver_type,
        }),
    })
}

fn _parse_reply_for_multi_sp(
    _json_msg: &SJsonValue,
    data: Option<&str>,
    parsed_data: &SJsonValue,
    xtype: &str,
    sp_key: &[u8],
) -> Result<Option<ParsedSP>, String> {
    trace!(
        "_parse_reply_for_multi_sp: data: {:?}, parsed_data: {:?}",
        data,
        parsed_data
    );
    let state_proof = &parsed_data["stateProofFrom"];
    let (proof_nodes, root_hash, multi_signature, value) = match xtype {
        constants::GET_REVOC_REG_DELTA if _if_rev_delta_multi_state_proof_expected(sp_key) => {
            let proof = if let Some(proof) = state_proof["proof_nodes"].as_str() {
                trace!("_parse_reply_for_multi_sp: proof: {:?}", proof);
                proof
            } else {
                return Err("No proof found".to_string());
            };

            let root_hash = if let Some(root_hash) = state_proof["root_hash"].as_str() {
                trace!("_parse_reply_for_multi_sp: root_hash: {:?}", root_hash);
                root_hash
            } else {
                return Err("No root hash".to_string());
            };

            let multi_signature = state_proof["multi_signature"].clone();

            let value_str = if !parsed_data["value"]["accum_from"].is_null() {
                Some(
                    json!({
                        "lsn": parsed_data["value"]["accum_from"]["seqNo"],
                        "lut": parsed_data["value"]["accum_from"]["txnTime"],
                        "val": parsed_data["value"]["accum_from"],
                    })
                    .to_string(),
                )
            } else {
                None
            };

            (
                proof.to_owned(),
                root_hash.to_owned(),
                multi_signature,
                value_str,
            )
        }
        constants::GET_REVOC_REG_DELTA => {
            trace!("_parse_reply_for_multi_sp: <<< proof not expected");
            return Ok(None);
        }
        _ => {
            return Err("Unsupported transaction".to_string());
        }
    };

    trace!(
        "_parse_reply_for_multi_sp: <<< proof {:?}, root_hash: {:?}, dest: {:?}, value: {:?}",
        proof_nodes,
        root_hash,
        sp_key,
        value
    );

    Ok(Some(ParsedSP {
        root_hash,
        proof_nodes,
        multi_signature,
        kvs_to_verify: KeyValuesInSP::Simple(KeyValueSimpleData {
            kvs: vec![(base64::encode(sp_key), value)],
            verification_type: KeyValueSimpleDataVerificationType::Simple,
        }),
    }))
}

fn _parse_reply_for_proof_signature_checking(
    json_msg: &SJsonValue,
) -> Option<(&str, Vec<String>, Vec<u8>)> {
    match (
        json_msg["signature"].as_str(),
        json_msg["participants"].as_array(),
        rmp_serde::to_vec_named(&json_msg["value"]),
    ) {
        (Some(signature), Some(participants), Ok(value)) => {
            let participants_unwrap = participants
                .iter()
                .flat_map(SJsonValue::as_str)
                .map(str::to_owned)
                .collect::<Vec<String>>();

            if participants.len() == participants_unwrap.len() {
                Some((signature, participants_unwrap, value))
            } else {
                debug!("Received non-string participant values");
                None
            }
        }
        (_, _, Err(err)) => {
            debug!("Error deserializing transaction: {}", err);
            None
        }
        (Some(_), None, _) => {
            debug!("Missing participants list");
            None
        }
        _ => {
            debug!("Missing signature");
            None
        }
    }
}

fn _verify_merkle_tree(
    proof_nodes: &[u8],
    root_hash: &[u8],
    kvs: &[(String, Option<String>)],
    length: u64,
) -> bool {
    let (key, value) = &kvs[0];
    let Some(value) = value.as_ref() else {
        debug!("No value for merkle tree hash");
        return false;
    };
    let seq_no = match base64::decode(key)
        .map_err_string()
        .and_then(|key| String::from_utf8(key).map_err_string())
        .and_then(|key| key.parse::<u64>().map_err_string())
    {
        Ok(seq_no) => seq_no,
        Err(err) => {
            debug!("Error while parsing merkle tree seq_no: {}", err);
            return false;
        }
    };

    let turns = _calculate_turns(length, seq_no - 1);

    let nodes = match std::str::from_utf8(proof_nodes) {
        Ok(res) => res,
        Err(err) => {
            debug!("Wrong state during mapping bytes to string: {:?}", err);
            return false;
        }
    };
    let hashes: Vec<String> = match serde_json::from_str(nodes) {
        Ok(vec) => vec,
        Err(err) => {
            debug!("Errors during deserialization: {:?}", err);
            return false;
        }
    };

    trace!("_verify_merkle_tree >> hashes: {:?}", hashes);

    trace!(
        "_verify_merkle_tree >> seq_no: {}, turns: {:?}",
        seq_no,
        turns
    );

    if hashes.len() != turns.len() {
        debug!("Different count of hashes and turns, unable to verify");
        return false;
    }

    let mut path = Vec::with_capacity(hashes.len());
    for (hash, t_right) in hashes.into_iter().zip(turns) {
        let Ok(hash) = base58::decode(hash) else {
            debug!("Error decoding hash as base58");
            return false;
        };
        path.push(if t_right {
            Positioned::Right(hash)
        } else {
            Positioned::Left(hash)
        });
    }

    let leaf_value = match serde_json::from_str::<serde_json::Value>(value)
        .map_err_string()
        .and_then(|val| rmp_serde::to_vec(&val).map_err_string())
    {
        Ok(val) => val,
        Err(err) => {
            debug!("Error while decoding merkle tree leaf: {:?}", err);
            return false;
        }
    };

    trace!("Leaf value: {}", base58::encode(&leaf_value));

    match MerkleTree::check_inclusion_proof(root_hash, &leaf_value, &path) {
        Ok(true) => {
            trace!("Matched root hash: {}", base58::encode(root_hash));
            true
        }
        Ok(false) => {
            debug!("Merkle tree hash mismatch");
            false
        }
        Err(err) => {
            trace!("Error checking merkle tree root hash: {}", err);
            false
        }
    }
}

// true is right
// false is left
fn _calculate_turns(length: u64, idx: u64) -> Vec<bool> {
    let mut idx = idx;
    let mut length = length;
    let mut result: Vec<bool> = vec![];
    while length != 1 {
        let middle = length.next_power_of_two() / 2;
        let right = idx < middle;
        result.push(right);
        idx = if right { idx } else { idx - middle };
        length = if right { middle } else { length - middle };
    }
    result.reverse();
    result
}

fn _verify_proof(
    proofs_rlp: &[u8],
    root_hash: &[u8],
    key: &[u8],
    expected_value: Option<&str>,
) -> bool {
    trace!(
        "verify_proof >> key {:?}, expected_value {:?}",
        key,
        expected_value
    );
    let nodes: Vec<Node> = UntrustedRlp::new(proofs_rlp).as_list().unwrap_or_default(); //default will cause error below
    let mut map: TrieDB = HashMap::with_capacity(nodes.len());
    for node in &nodes {
        map.insert(node.get_hash(), node);
    }
    map.get(root_hash)
        .map(|root| {
            root.get_str_value(&map, key)
                .map_err(map_err_log!(level: debug, "{}"))
                .map(|value| value.as_deref().eq(&expected_value))
                .unwrap_or(false)
        })
        .unwrap_or(false)
}

fn _verify_proof_range(
    proofs_rlp: &[u8],
    root_hash: &[u8],
    prefix: &str,
    from: Option<u64>,
    next: Option<u64>,
    kvs: &[(String, Option<String>)],
) -> bool {
    trace!(
        "verify_proof_range >> from {:?}, prefix {:?}, kvs {:?}",
        from,
        prefix,
        kvs
    );
    let nodes: Vec<Node> = UntrustedRlp::new(proofs_rlp).as_list().unwrap_or_default(); //default will cause error below
    let mut map: TrieDB = HashMap::with_capacity(nodes.len());
    for node in &nodes {
        map.insert(node.get_hash(), node);
    }
    map.get(root_hash).map(|root| {
        let res = root.get_all_values(&map, Some(prefix.as_bytes()));
        trace!("All values from trie: {:?}", res);
        let vals = if let Ok(vals) = res {
            vals
        } else {
            debug!("Errors occurred when collecting values from state proof");
            return false;
        };
        // Preparation of data for verification
        // Fetch numerical suffixes
        #[allow(clippy::type_complexity)]
        let vals_for_sort_check: Vec<Option<(u64, (String, Option<String>))>> = vals.into_iter()
            .filter(|(key, _)| key.starts_with(prefix))
            .map(|(key, value)| {
                let no = key.replacen(prefix, "", 1).parse::<u64>();
                no.ok().map(|a| (a, (key, Some(value))))
            }).collect();
        if !vals_for_sort_check.iter().all(|a| a.is_some()) {
            debug!("Some values in state proof are not correlating with state proof rule, aborting.");
            return false;
        }
        let mut vals_for_sort: Vec<(u64, (String, Option<String>))> = vals_for_sort_check.into_iter().flatten().collect();
        // Sort by numerical suffixes in ascending order
        vals_for_sort.sort_by_key(|&(a, _)| a);
        trace!("Sorted trie values: {:?}", vals_for_sort);
        // Shift on the left side by from
        let vals_with_from = if let Some(from_seqno) = from {
            match vals_for_sort.binary_search_by_key(&from_seqno, |&(a, _)| a) {
                Ok(idx) | Err(idx) => vals_for_sort[idx..].to_vec()
            }
        } else {
            vals_for_sort
        };
        // Verification
        // Check that all values from response match the trie
        trace!("Got values from trie: {:?}", vals_with_from);
        let vals_slice = if let Some(next_seqno) = next {
            match vals_with_from.binary_search_by_key(&next_seqno, |&(a, _)| a) {
                Ok(idx) => &vals_with_from[..idx],
                Err(_) => {
                    debug!("Next seqno is incorrect");
                    return false;
                }
            }
        } else {
            vals_with_from.as_slice()
        };
        let vals_prepared: Vec<(String, Option<String>)> = vals_slice.iter().map(|(_, ref pair)| pair.clone()).collect();
        vals_prepared[..] == kvs[..]
    }).unwrap_or(false)
}

fn _verify_proof_signature(
    signature: &str,
    participants: &[String],
    value: &[u8],
    nodes: &VerifierKeys,
    f: usize,
    gen: &Generator,
) -> Result<(), String> {
    let mut ver_keys: Vec<&VerKey> = Vec::with_capacity(nodes.len());

    for name in participants {
        if let Some(blskey) = nodes.get(name) {
            ver_keys.push(&blskey.inner)
        } else {
            return Err(format!("BLS key not found for node: {:?}", name));
        }
    }

    trace!(
        "verify_proof_signature: ver_keys.len(): {:?}",
        ver_keys.len()
    );

    if ver_keys.len() < (nodes.len() - f) {
        return Err("Insufficient participants in multi signature".into());
    }

    let signature = if let Some(signature) = base58::decode(signature)
        .ok()
        .and_then(|sig| MultiSignature::from_bytes(sig.as_slice()).ok())
    {
        signature
    } else {
        return Err("Error decoding multi signature".into());
    };

    if !Bls::verify_multi_sig(&signature, value, ver_keys.as_slice(), gen).unwrap_or(false) {
        return Err("Multi signature failed verification".into());
    }

    Ok(())
}

fn _parse_reply_for_proof_value(
    json_msg: &SJsonValue,
    data: Option<&str>,
    parsed_data: &SJsonValue,
    xtype: &str,
    sp_key: &[u8],
) -> VdrResult<Option<String>> {
    if let Some(data) = data {
        let mut value = json!({});

        let (seq_no, time) = (json_msg["seqNo"].clone(), json_msg["txnTime"].clone());

        match xtype {
            constants::GET_NYM => {
                value["seqNo"] = seq_no;
                value["txnTime"] = time;
            }
            constants::GET_AUTH_RULE => {}
            xtype
                if xtype.ne(constants::GET_TXN_AUTHR_AGRMT)
                    || _is_full_taa_state_value_expected(sp_key) =>
            {
                value["lsn"] = seq_no;
                value["lut"] = time;
            }
            _ => {}
        }

        match xtype {
            //TODO constants::GET_DDO => support DDO
            constants::GET_TXN => {
                value = json!({});
                if parsed_data["txn"].is_null()
                    && parsed_data["txnMetadata"].is_null()
                    && parsed_data["ver"].is_null()
                    && parsed_data["reqSignature"].is_null()
                {
                    return Ok(None);
                }
                if !parsed_data["txn"].is_null() {
                    value["txn"] = parsed_data["txn"].clone();
                }
                if !parsed_data["txnMetadata"].is_null() {
                    value["txnMetadata"] = parsed_data["txnMetadata"].clone();
                }
                if !parsed_data["ver"].is_null() {
                    value["ver"] = parsed_data["ver"].clone();
                }
                if !parsed_data["reqSignature"].is_null() {
                    value["reqSignature"] = parsed_data["reqSignature"].clone();
                }

                // Adjust attrib transaction to match stored state
                if value["txn"]["type"].as_str() == Some("100") {
                    if let Some(raw) = value["txn"]["data"]["raw"].as_str() {
                        if raw.is_empty() {
                            value["txn"]["data"]["raw"] = SJsonValue::from("");
                        } else {
                            value["txn"]["data"]["raw"] =
                                SJsonValue::from(hex::encode(Sha256::digest(raw.as_bytes())));
                        }
                    } else if let Some(enc) = value["txn"]["data"]["enc"].as_str() {
                        if enc.is_empty() {
                            value["txn"]["data"]["enc"] = SJsonValue::from("");
                        } else {
                            value["txn"]["data"]["enc"] =
                                SJsonValue::from(hex::encode(Sha256::digest(enc.as_bytes())));
                        }
                    }
                }
            }
            constants::GET_NYM => {
                value["identifier"] = parsed_data["identifier"].clone();
                value["role"] = parsed_data["role"].clone();
                value["verkey"] = parsed_data["verkey"].clone();
            }
            constants::GET_ATTR => {
                value["val"] = SJsonValue::String(hex::encode(Sha256::digest(data.as_bytes())));
            }
            constants::GET_CRED_DEF
            | constants::GET_REVOC_REG_DEF
            | constants::GET_REVOC_REG
            | constants::GET_TXN_AUTHR_AGRMT_AML => {
                value["val"] = parsed_data.clone();
            }
            constants::GET_AUTH_RULE => {
                let constraint = parsed_data
                    .as_array()
                    .and_then(|data| data.first())
                    .map(|auth_rule| auth_rule["constraint"].clone());
                match constraint {
                    Some(ref x) => value = x.clone(),
                    None => return Ok(None),
                };
            }
            constants::GET_SCHEMA => {
                if let Some(map) = parsed_data.as_object() {
                    let mut map = map.clone();
                    map.remove("name");
                    map.remove("version");
                    if map.is_empty() {
                        return Ok(None); // TODO FIXME remove after INDY-699 will be fixed
                    } else {
                        value["val"] = SJsonValue::from(map)
                    }
                } else {
                    return Err(input_err("Invalid data for GET_SCHEMA"));
                };
            }
            constants::GET_REVOC_REG_DELTA => {
                if !parsed_data["value"]["accum_to"].is_null() {
                    value["val"] = parsed_data["value"]["accum_to"].clone()
                } else {
                    return Ok(None);
                }
            }
            constants::GET_TXN_AUTHR_AGRMT => {
                if _is_full_taa_state_value_expected(sp_key) {
                    value["val"] = parsed_data.clone();
                } else {
                    value = SJsonValue::String(hex::encode(_calculate_taa_digest(parsed_data["text"].as_str().unwrap_or(""),
                                                                                 parsed_data["version"].as_str().unwrap_or(""))
                        .with_input_err("Can't calculate expected TAA digest to verify StateProof on the request")?));
                }
            }
            _ => {
                return Err(input_err("Unknown transaction"));
            }
        }

        let value_str = if let Some(value) = value.as_str() {
            value.to_owned()
        } else {
            value.to_string()
        };

        Ok(Some(value_str))
    } else {
        Ok(None)
    }
}

fn _calculate_taa_digest(text: &str, version: &str) -> VdrResult<Vec<u8>> {
    let content: String = version.to_string() + text;
    Ok(Sha256::digest(content.as_bytes()).to_vec())
}

fn _is_full_taa_state_value_expected(expected_state_key: &[u8]) -> bool {
    expected_state_key.starts_with(b"2:d:")
}

fn _if_rev_delta_multi_state_proof_expected(sp_key: &[u8]) -> bool {
    sp_key.starts_with(b"\x06:") || sp_key.starts_with(b"6:")
}

#[cfg(test)]
mod tests {
    use crate::{config::constants::DEFAULT_GENERATOR, pool::VerifierKey};

    use super::*;

    use hex::FromHex;
    // use libc::c_char;
    // use std::ffi::{CStr, CString};

    /// For audit proofs tree looks like this
    ///         12345
    ///         /  \
    ///      1234  5
    ///     /    \
    ///   12     34
    ///  /  \   /  \
    /// 1   2  3   4

    #[test]
    fn audit_proof_verify_works() {
        let nodes = json!([
            "Gf9aBhHCtBpTYbJXQWnt1DU8q33hwi6nN4f3NhnsBgMZ",
            "68TGAdRjeQ29eNcuFYhsX5uLakGQLgKMKp5wSyPzt9Nq",
            "25KLEkkyCEPSBj4qMFE3AcH87mFocyJEuPJ5xzPGwDgz"
        ])
        .to_string();
        let kvs = vec![(base64::encode("3"), Some(r#"{"3":"3"}"#.to_string()))];
        let node_bytes = &nodes;
        let root_hash = base58::decode("CrA5sqYe3ruf2uY7d8re7ePmyHqptHqANtMZcfZd4BvK").unwrap();
        assert!(_verify_merkle_tree(
            node_bytes.as_bytes(),
            root_hash.as_slice(),
            kvs.as_slice(),
            5
        ));
    }

    #[test]
    fn audit_proof_verify_works_for_invalid_proof() {
        let nodes = json!([
            "Gf9aBhHCtBpTYbJXQWnt1DU8q33hwi6nN4f3NhnsBgM3", //wrong hash here
            "68TGAdRjeQ29eNcuFYhsX5uLakGQLgKMKp5wSyPzt9Nq",
            "25KLEkkyCEPSBj4qMFE3AcH87mFocyJEuPJ5xzPGwDgz"
        ])
        .to_string();
        let kvs = vec![(base64::encode("3"), Some(r#"{"3":"3"}"#.to_string()))];
        let node_bytes = &nodes;
        let root_hash = base58::decode("CrA5sqYe3ruf2uY7d8re7ePmyHqptHqANtMZcfZd4BvK").unwrap();
        assert!(!_verify_merkle_tree(
            node_bytes.as_bytes(),
            root_hash.as_slice(),
            kvs.as_slice(),
            5
        ));
    }

    #[test]
    fn audit_proof_verify_works_for_invalid_root_hash() {
        let nodes = json!([
            "Gf9aBhHCtBpTYbJXQWnt1DU8q33hwi6nN4f3NhnsBgMZ",
            "68TGAdRjeQ29eNcuFYhsX5uLakGQLgKMKp5wSyPzt9Nq",
            "25KLEkkyCEPSBj4qMFE3AcH87mFocyJEuPJ5xzPGwDgz"
        ])
        .to_string();
        let kvs = vec![(base64::encode("3"), Some(r#"{"3":"3"}"#.to_string()))];
        let node_bytes = &nodes;
        let root_hash = base58::decode("G9QooEDKSmEtLGNyTwafQiPfGHMqw3A3Fjcj2eLRG4G1").unwrap();
        assert!(!_verify_merkle_tree(
            node_bytes.as_bytes(),
            root_hash.as_slice(),
            kvs.as_slice(),
            5
        ));
    }

    #[test]
    fn audit_proof_verify_works_for_invalid_ledger_length() {
        let nodes = json!([
            "Gf9aBhHCtBpTYbJXQWnt1DU8q33hwi6nN4f3NhnsBgMZ",
            "68TGAdRjeQ29eNcuFYhsX5uLakGQLgKMKp5wSyPzt9Nq",
            "25KLEkkyCEPSBj4qMFE3AcH87mFocyJEuPJ5xzPGwDgz"
        ])
        .to_string();
        let kvs = vec![(base64::encode("3"), Some(r#"{"3":"3"}"#.to_string()))];
        let node_bytes = &nodes;
        let root_hash = base58::decode("CrA5sqYe3ruf2uY7d8re7ePmyHqptHqANtMZcfZd4BvK").unwrap();
        assert!(!_verify_merkle_tree(
            node_bytes.as_bytes(),
            root_hash.as_slice(),
            kvs.as_slice(),
            9
        ));
    }

    #[test]
    fn audit_proof_verify_works_for_invalid_value() {
        let nodes = json!([
            "Gf9aBhHCtBpTYbJXQWnt1DU8q33hwi6nN4f3NhnsBgMZ",
            "68TGAdRjeQ29eNcuFYhsX5uLakGQLgKMKp5wSyPzt9Nq",
            "25KLEkkyCEPSBj4qMFE3AcH87mFocyJEuPJ5xzPGwDgz"
        ])
        .to_string();
        let kvs = vec![(base64::encode("3"), Some(r#"{"4":"4"}"#.to_string()))];
        let node_bytes = &nodes;
        let root_hash = base58::decode("CrA5sqYe3ruf2uY7d8re7ePmyHqptHqANtMZcfZd4BvK").unwrap();
        assert!(!_verify_merkle_tree(
            node_bytes.as_bytes(),
            root_hash.as_slice(),
            kvs.as_slice(),
            5
        ));
    }

    #[test]
    fn audit_proof_verify_works_for_invalid_seqno() {
        let nodes = json!([
            "Gf9aBhHCtBpTYbJXQWnt1DU8q33hwi6nN4f3NhnsBgMZ",
            "68TGAdRjeQ29eNcuFYhsX5uLakGQLgKMKp5wSyPzt9Nq",
            "25KLEkkyCEPSBj4qMFE3AcH87mFocyJEuPJ5xzPGwDgz"
        ])
        .to_string();
        let kvs = vec![(base64::encode("4"), Some(r#"{"3":"3"}"#.to_string()))];
        let node_bytes = &nodes;
        let root_hash = base58::decode("CrA5sqYe3ruf2uY7d8re7ePmyHqptHqANtMZcfZd4BvK").unwrap();
        assert!(!_verify_merkle_tree(
            node_bytes.as_bytes(),
            root_hash.as_slice(),
            kvs.as_slice(),
            5
        ));
    }

    #[test]
    fn state_proof_nodes_parse_and_get_works() {
        /*
            '33' -> 'v1'
            '34' -> 'v2'
            '3C' -> 'v3'
            '4'  -> 'v4'
            'D'  -> 'v5asdfasdf'
            'E'  -> 'v6fdsfdfs'
        */
        let str = "f8c0f7808080a0762fc4967c792ef3d22fefd3f43209e2185b25e9a97640f09bb4b61657f67cf3c62084c3827634808080808080808080808080f4808080dd808080c62084c3827631c62084c3827632808080808080808080808080c63384c3827633808080808080808080808080f851808080a0099d752f1d5a4b9f9f0034540153d2d2a7c14c11290f27e5d877b57c801848caa06267640081beb8c77f14f30c68f30688afc3e5d5a388194c6a42f699fe361b2f808080808080808080808080";
        let vec = Vec::from_hex(str).unwrap();
        let rlp = UntrustedRlp::new(vec.as_slice());
        let proofs: Vec<Node> = rlp.as_list().unwrap();
        info!("Input");
        for rlp in rlp.iter() {
            info!("{:?}", rlp.as_raw());
        }
        info!("parsed");
        let mut map: TrieDB = HashMap::with_capacity(proofs.len());
        for node in &proofs {
            info!("{:?}", node);
            let out = node.get_hash();
            info!("{:?}", out);
            map.insert(out, node);
        }
        for k in 33..35 {
            info!("Try get {}", k);
            let x = proofs[2]
                .get_str_value(&map, k.to_string().as_bytes())
                .unwrap()
                .unwrap();
            info!("{:?}", x);
            assert_eq!(x, format!("v{}", k - 32));
        }
    }

    #[test]
    fn state_proof_verify_proof_works_for_get_value_from_leaf() {
        /*
            '33' -> 'v1'
            '34' -> 'v2'
            '3C' -> 'v3'
            '4'  -> 'v4'
            'D'  -> 'v5asdfasdf'
            'E'  -> 'v6fdsfdfs'
        */
        let proofs = Vec::from_hex("f8c0f7808080a0762fc4967c792ef3d22fefd3f43209e2185b25e9a97640f09bb4b61657f67cf3c62084c3827634808080808080808080808080f4808080dd808080c62084c3827631c62084c3827632808080808080808080808080c63384c3827633808080808080808080808080f851808080a0099d752f1d5a4b9f9f0034540153d2d2a7c14c11290f27e5d877b57c801848caa06267640081beb8c77f14f30c68f30688afc3e5d5a388194c6a42f699fe361b2f808080808080808080808080").unwrap();
        let root_hash =
            Vec::from_hex("badc906111df306c6afac17b62f29792f0e523b67ba831651d6056529b6bf690")
                .unwrap();
        assert!(_verify_proof(
            proofs.as_slice(),
            root_hash.as_slice(),
            "33".as_bytes(),
            Some("v1")
        ));
        assert!(_verify_proof(
            proofs.as_slice(),
            root_hash.as_slice(),
            "34".as_bytes(),
            Some("v2")
        ));
        assert!(_verify_proof(
            proofs.as_slice(),
            root_hash.as_slice(),
            "3C".as_bytes(),
            Some("v3")
        ));
        assert!(_verify_proof(
            proofs.as_slice(),
            root_hash.as_slice(),
            "4".as_bytes(),
            Some("v4")
        ));
    }

    #[test]
    fn state_proof_verify_proof_works_for_get_value_from_leaf_in_range() {
        /*
            'abcdefgh1'     -> '3630'
            'abcdefgh4'     -> '3037'
            'abcdefgh10'    -> '4970'
            'abcdefgh11'    -> '4373'
            'abcdefgh24'    -> '4905'
            'abcdefgh99'    -> '4522'
            'abcdefgh100'   -> '3833'
        */
        let proofs = base64::decode("+QEO34CAgMgwhsWEMzgzM4CAgICAgICAgICAgIbFhDQ5NzD4TYCgWvV3JP22NK5fmfA2xp0DgkFi9rkBdw4ADHTeyez/RtzKgiA0hsWENDkwNYDIIIbFhDMwMzeAgICAyoIgOYbFhDQ1MjKAgICAgICA94CAgKCwvJK5hgh1xdoCVjFsZLAr2Ct5ADxnseuJtF+m80+y64CAgICAgICAgICAgIbFhDM2MzD4OaAfBo1nqEW9/DhdOYucHjHAgqpZsF3f96awYBKZkmR2i8gghsWENDM3M4CAgICAgICAgICAgICAgOuJFhYmNkZWZnaDoNDKeVFnNI85QpRhrd2t8hS4By3wpD4R5ZyUegAPUtga").unwrap();
        let root_hash = base58::decode("EA9zTfmf5Ex4ZUTPpMwpsQxQzTkevtwg9PADTqJczhSF").unwrap();
        assert!(_verify_proof_range(
            proofs.as_slice(),
            root_hash.as_slice(),
            "abcdefgh",
            Some(10),
            Some(99),
            &[
                ("abcdefgh10".to_string(), Some("4970".to_string())),
                ("abcdefgh11".to_string(), Some("4373".to_string())),
                ("abcdefgh24".to_string(), Some("4905".to_string())),
            ]
        ));
    }

    #[test]
    fn state_proof_verify_proof_works_for_get_value_from_leaf_in_range_empty_from() {
        /*
            'abcdefgh1'     -> '3630'
            'abcdefgh4'     -> '3037'
            'abcdefgh10'    -> '4970'
            'abcdefgh11'    -> '4373'
            'abcdefgh24'    -> '4905'
            'abcdefgh99'    -> '4522'
            'abcdefgh100'   -> '3833'
        */
        let proofs = base64::decode("+QEO34CAgMgwhsWEMzgzM4CAgICAgICAgICAgIbFhDQ5NzD4TYCgWvV3JP22NK5fmfA2xp0DgkFi9rkBdw4ADHTeyez/RtzKgiA0hsWENDkwNYDIIIbFhDMwMzeAgICAyoIgOYbFhDQ1MjKAgICAgICA94CAgKCwvJK5hgh1xdoCVjFsZLAr2Ct5ADxnseuJtF+m80+y64CAgICAgICAgICAgIbFhDM2MzD4OaAfBo1nqEW9/DhdOYucHjHAgqpZsF3f96awYBKZkmR2i8gghsWENDM3M4CAgICAgICAgICAgICAgOuJFhYmNkZWZnaDoNDKeVFnNI85QpRhrd2t8hS4By3wpD4R5ZyUegAPUtga").unwrap();
        let root_hash = base58::decode("EA9zTfmf5Ex4ZUTPpMwpsQxQzTkevtwg9PADTqJczhSF").unwrap();
        assert!(_verify_proof_range(
            proofs.as_slice(),
            root_hash.as_slice(),
            "abcdefgh",
            Some(101),
            None,
            &[]
        ));
    }

    #[test]
    fn state_proof_verify_proof_works_for_get_value_from_leaf_in_range_fails_missing_values() {
        /*
            'abcdefgh1'     -> '3630'
            'abcdefgh4'     -> '3037'
            'abcdefgh10'    -> '4970'
            'abcdefgh11'    -> '4373'
            'abcdefgh24'    -> '4905'
            'abcdefgh99'    -> '4522'
            'abcdefgh100'   -> '3833'
        */
        let proofs = base64::decode("+QEO34CAgMgwhsWEMzgzM4CAgICAgICAgICAgIbFhDQ5NzD4TYCgWvV3JP22NK5fmfA2xp0DgkFi9rkBdw4ADHTeyez/RtzKgiA0hsWENDkwNYDIIIbFhDMwMzeAgICAyoIgOYbFhDQ1MjKAgICAgICA94CAgKCwvJK5hgh1xdoCVjFsZLAr2Ct5ADxnseuJtF+m80+y64CAgICAgICAgICAgIbFhDM2MzD4OaAfBo1nqEW9/DhdOYucHjHAgqpZsF3f96awYBKZkmR2i8gghsWENDM3M4CAgICAgICAgICAgICAgOuJFhYmNkZWZnaDoNDKeVFnNI85QpRhrd2t8hS4By3wpD4R5ZyUegAPUtga").unwrap();
        let root_hash = base58::decode("EA9zTfmf5Ex4ZUTPpMwpsQxQzTkevtwg9PADTqJczhSF").unwrap();
        // no "abcdefgh11" value in kvs
        assert!(!_verify_proof_range(
            proofs.as_slice(),
            root_hash.as_slice(),
            "abcdefgh",
            Some(10),
            Some(99),
            &[
                ("abcdefgh10".to_string(), Some("4970".to_string())),
                ("abcdefgh24".to_string(), Some("4905".to_string())),
            ]
        ));
    }

    #[test]
    fn state_proof_verify_proof_works_for_get_value_from_leaf_in_range_fails_extra_values() {
        /*
            'abcdefgh1'     -> '3630'
            'abcdefgh4'     -> '3037'
            'abcdefgh10'    -> '4970'
            'abcdefgh11'    -> '4373'
            'abcdefgh24'    -> '4905'
            'abcdefgh99'    -> '4522'
            'abcdefgh100'   -> '3833'
        */
        let proofs = base64::decode("+QEO34CAgMgwhsWEMzgzM4CAgICAgICAgICAgIbFhDQ5NzD4TYCgWvV3JP22NK5fmfA2xp0DgkFi9rkBdw4ADHTeyez/RtzKgiA0hsWENDkwNYDIIIbFhDMwMzeAgICAyoIgOYbFhDQ1MjKAgICAgICA94CAgKCwvJK5hgh1xdoCVjFsZLAr2Ct5ADxnseuJtF+m80+y64CAgICAgICAgICAgIbFhDM2MzD4OaAfBo1nqEW9/DhdOYucHjHAgqpZsF3f96awYBKZkmR2i8gghsWENDM3M4CAgICAgICAgICAgICAgOuJFhYmNkZWZnaDoNDKeVFnNI85QpRhrd2t8hS4By3wpD4R5ZyUegAPUtga").unwrap();
        let root_hash = base58::decode("EA9zTfmf5Ex4ZUTPpMwpsQxQzTkevtwg9PADTqJczhSF").unwrap();
        // no "abcdefgh11" value in kvs
        assert!(!_verify_proof_range(
            proofs.as_slice(),
            root_hash.as_slice(),
            "abcdefgh",
            Some(10),
            Some(99),
            &[
                ("abcdefgh10".to_string(), Some("4970".to_string())),
                ("abcdefgh11".to_string(), Some("4373".to_string())),
                ("abcdefgh13".to_string(), Some("4234".to_string())),
                ("abcdefgh24".to_string(), Some("4905".to_string())),
            ]
        ));
    }

    #[test]
    fn state_proof_verify_proof_works_for_get_value_from_leaf_in_range_fails_changed_values() {
        /*
            'abcdefgh1'     -> '3630'
            'abcdefgh4'     -> '3037'
            'abcdefgh10'    -> '4970'
            'abcdefgh11'    -> '4373'
            'abcdefgh24'    -> '4905'
            'abcdefgh99'    -> '4522'
            'abcdefgh100'   -> '3833'
        */
        let proofs = base64::decode("+QEO34CAgMgwhsWEMzgzM4CAgICAgICAgICAgIbFhDQ5NzD4TYCgWvV3JP22NK5fmfA2xp0DgkFi9rkBdw4ADHTeyez/RtzKgiA0hsWENDkwNYDIIIbFhDMwMzeAgICAyoIgOYbFhDQ1MjKAgICAgICA94CAgKCwvJK5hgh1xdoCVjFsZLAr2Ct5ADxnseuJtF+m80+y64CAgICAgICAgICAgIbFhDM2MzD4OaAfBo1nqEW9/DhdOYucHjHAgqpZsF3f96awYBKZkmR2i8gghsWENDM3M4CAgICAgICAgICAgICAgOuJFhYmNkZWZnaDoNDKeVFnNI85QpRhrd2t8hS4By3wpD4R5ZyUegAPUtga").unwrap();
        let root_hash = base58::decode("EA9zTfmf5Ex4ZUTPpMwpsQxQzTkevtwg9PADTqJczhSF").unwrap();
        assert!(!_verify_proof_range(
            proofs.as_slice(),
            root_hash.as_slice(),
            "abcdefgh",
            Some(10),
            Some(99),
            &[
                ("abcdefgh10".to_string(), Some("4970".to_string())),
                ("abcdefgh12".to_string(), Some("4373".to_string())),
                ("abcdefgh24".to_string(), Some("4905".to_string())),
            ]
        ));
    }

    #[test]
    fn state_proof_verify_proof_works_for_get_value_from_leaf_in_range_fails_wrong_next() {
        /*
            'abcdefgh1'     -> '3630'
            'abcdefgh4'     -> '3037'
            'abcdefgh10'    -> '4970'
            'abcdefgh11'    -> '4373'
            'abcdefgh24'    -> '4905'
            'abcdefgh99'    -> '4522'
            'abcdefgh100'   -> '3833'
        */
        let proofs = base64::decode("+QEO34CAgMgwhsWEMzgzM4CAgICAgICAgICAgIbFhDQ5NzD4TYCgWvV3JP22NK5fmfA2xp0DgkFi9rkBdw4ADHTeyez/RtzKgiA0hsWENDkwNYDIIIbFhDMwMzeAgICAyoIgOYbFhDQ1MjKAgICAgICA94CAgKCwvJK5hgh1xdoCVjFsZLAr2Ct5ADxnseuJtF+m80+y64CAgICAgICAgICAgIbFhDM2MzD4OaAfBo1nqEW9/DhdOYucHjHAgqpZsF3f96awYBKZkmR2i8gghsWENDM3M4CAgICAgICAgICAgICAgOuJFhYmNkZWZnaDoNDKeVFnNI85QpRhrd2t8hS4By3wpD4R5ZyUegAPUtga").unwrap();
        let root_hash = base58::decode("EA9zTfmf5Ex4ZUTPpMwpsQxQzTkevtwg9PADTqJczhSF").unwrap();
        assert!(!_verify_proof_range(
            proofs.as_slice(),
            root_hash.as_slice(),
            "abcdefgh",
            Some(10),
            Some(100),
            &[
                ("abcdefgh10".to_string(), Some("4970".to_string())),
                ("abcdefgh11".to_string(), Some("4373".to_string())),
                ("abcdefgh24".to_string(), Some("4905".to_string())),
            ]
        ));
    }

    #[test]
    fn state_proof_verify_proof_works_for_get_value_from_leaf_in_range_no_next() {
        /*
            'abcdefgh1'     -> '3630'
            'abcdefgh4'     -> '3037'
            'abcdefgh10'    -> '4970'
            'abcdefgh11'    -> '4373'
            'abcdefgh24'    -> '4905'
            'abcdefgh99'    -> '4522'
            'abcdefgh100'   -> '3833'
        */
        let proofs = base64::decode("+QEO34CAgMgwhsWEMzgzM4CAgICAgICAgICAgIbFhDQ5NzD4TYCgWvV3JP22NK5fmfA2xp0DgkFi9rkBdw4ADHTeyez/RtzKgiA0hsWENDkwNYDIIIbFhDMwMzeAgICAyoIgOYbFhDQ1MjKAgICAgICA94CAgKCwvJK5hgh1xdoCVjFsZLAr2Ct5ADxnseuJtF+m80+y64CAgICAgICAgICAgIbFhDM2MzD4OaAfBo1nqEW9/DhdOYucHjHAgqpZsF3f96awYBKZkmR2i8gghsWENDM3M4CAgICAgICAgICAgICAgOuJFhYmNkZWZnaDoNDKeVFnNI85QpRhrd2t8hS4By3wpD4R5ZyUegAPUtga").unwrap();
        let root_hash = base58::decode("EA9zTfmf5Ex4ZUTPpMwpsQxQzTkevtwg9PADTqJczhSF").unwrap();
        assert!(_verify_proof_range(
            proofs.as_slice(),
            root_hash.as_slice(),
            "abcdefgh",
            Some(10),
            None,
            &[
                ("abcdefgh10".to_string(), Some("4970".to_string())),
                ("abcdefgh11".to_string(), Some("4373".to_string())),
                ("abcdefgh24".to_string(), Some("4905".to_string())),
                ("abcdefgh99".to_string(), Some("4522".to_string())),
                ("abcdefgh100".to_string(), Some("3833".to_string())),
            ],
        ));
    }

    #[test]
    fn state_proof_verify_proof_works_for_get_value_from_leaf_in_range_no_next_fails_missing_values(
    ) {
        /*
            'abcdefgh1'     -> '3630'
            'abcdefgh4'     -> '3037'
            'abcdefgh10'    -> '4970'
            'abcdefgh11'    -> '4373'
            'abcdefgh24'    -> '4905'
            'abcdefgh99'    -> '4522'
            'abcdefgh100'   -> '3833'
        */
        let proofs = base64::decode("+QEO34CAgMgwhsWEMzgzM4CAgICAgICAgICAgIbFhDQ5NzD4TYCgWvV3JP22NK5fmfA2xp0DgkFi9rkBdw4ADHTeyez/RtzKgiA0hsWENDkwNYDIIIbFhDMwMzeAgICAyoIgOYbFhDQ1MjKAgICAgICA94CAgKCwvJK5hgh1xdoCVjFsZLAr2Ct5ADxnseuJtF+m80+y64CAgICAgICAgICAgIbFhDM2MzD4OaAfBo1nqEW9/DhdOYucHjHAgqpZsF3f96awYBKZkmR2i8gghsWENDM3M4CAgICAgICAgICAgICAgOuJFhYmNkZWZnaDoNDKeVFnNI85QpRhrd2t8hS4By3wpD4R5ZyUegAPUtga").unwrap();
        let root_hash = base58::decode("EA9zTfmf5Ex4ZUTPpMwpsQxQzTkevtwg9PADTqJczhSF").unwrap();
        assert!(!_verify_proof_range(
            proofs.as_slice(),
            root_hash.as_slice(),
            "abcdefgh",
            Some(10),
            None,
            &[
                ("abcdefgh10".to_string(), Some("4970".to_string())),
                ("abcdefgh11".to_string(), Some("4373".to_string())),
                //                ("abcdefgh24".to_string(), Some("4905".to_string())),
                ("abcdefgh99".to_string(), Some("4522".to_string())),
                ("abcdefgh100".to_string(), Some("3833".to_string())),
            ]
        ));
    }

    #[test]
    fn state_proof_verify_proof_works_for_get_value_from_leaf_in_range_no_next_fails_extra_values()
    {
        /*
            'abcdefgh1'     -> '3630'
            'abcdefgh4'     -> '3037'
            'abcdefgh10'    -> '4970'
            'abcdefgh11'    -> '4373'
            'abcdefgh24'    -> '4905'
            'abcdefgh99'    -> '4522'
            'abcdefgh100'   -> '3833'
        */
        let proofs = base64::decode("+QEO34CAgMgwhsWEMzgzM4CAgICAgICAgICAgIbFhDQ5NzD4TYCgWvV3JP22NK5fmfA2xp0DgkFi9rkBdw4ADHTeyez/RtzKgiA0hsWENDkwNYDIIIbFhDMwMzeAgICAyoIgOYbFhDQ1MjKAgICAgICA94CAgKCwvJK5hgh1xdoCVjFsZLAr2Ct5ADxnseuJtF+m80+y64CAgICAgICAgICAgIbFhDM2MzD4OaAfBo1nqEW9/DhdOYucHjHAgqpZsF3f96awYBKZkmR2i8gghsWENDM3M4CAgICAgICAgICAgICAgOuJFhYmNkZWZnaDoNDKeVFnNI85QpRhrd2t8hS4By3wpD4R5ZyUegAPUtga").unwrap();
        let root_hash = base58::decode("EA9zTfmf5Ex4ZUTPpMwpsQxQzTkevtwg9PADTqJczhSF").unwrap();
        assert!(!_verify_proof_range(
            proofs.as_slice(),
            root_hash.as_slice(),
            "abcdefgh",
            Some(10),
            None,
            &[
                ("abcdefgh10".to_string(), Some("4970".to_string())),
                ("abcdefgh11".to_string(), Some("4373".to_string())),
                ("abcdefgh24".to_string(), Some("4905".to_string())),
                ("abcdefgh25".to_string(), Some("4905".to_string())),
                ("abcdefgh99".to_string(), Some("4522".to_string())),
                ("abcdefgh100".to_string(), Some("3833".to_string())),
            ],
        ));
    }

    #[test]
    fn state_proof_verify_proof_works_for_get_value_from_leaf_in_range_no_next_fails_changed_values(
    ) {
        /*
            'abcdefgh1'     -> '3630'
            'abcdefgh4'     -> '3037'
            'abcdefgh10'    -> '4970'
            'abcdefgh11'    -> '4373'
            'abcdefgh24'    -> '4905'
            'abcdefgh99'    -> '4522'
            'abcdefgh100'   -> '3833'
        */
        let proofs = base64::decode("+QEO34CAgMgwhsWEMzgzM4CAgICAgICAgICAgIbFhDQ5NzD4TYCgWvV3JP22NK5fmfA2xp0DgkFi9rkBdw4ADHTeyez/RtzKgiA0hsWENDkwNYDIIIbFhDMwMzeAgICAyoIgOYbFhDQ1MjKAgICAgICA94CAgKCwvJK5hgh1xdoCVjFsZLAr2Ct5ADxnseuJtF+m80+y64CAgICAgICAgICAgIbFhDM2MzD4OaAfBo1nqEW9/DhdOYucHjHAgqpZsF3f96awYBKZkmR2i8gghsWENDM3M4CAgICAgICAgICAgICAgOuJFhYmNkZWZnaDoNDKeVFnNI85QpRhrd2t8hS4By3wpD4R5ZyUegAPUtga").unwrap();
        let root_hash = base58::decode("EA9zTfmf5Ex4ZUTPpMwpsQxQzTkevtwg9PADTqJczhSF").unwrap();
        assert!(!_verify_proof_range(
            proofs.as_slice(),
            root_hash.as_slice(),
            "abcdefgh",
            Some(10),
            None,
            &[
                ("abcdefgh10".to_string(), Some("4970".to_string())),
                ("abcdefgh11".to_string(), Some("4373".to_string())),
                ("abcdefgh25".to_string(), Some("4905".to_string())),
                ("abcdefgh99".to_string(), Some("4522".to_string())),
                ("abcdefgh100".to_string(), Some("3833".to_string())),
            ]
        ));
    }

    #[test]
    fn state_proof_verify_proof_works_for_get_value_from_leaf_in_range_no_from() {
        /*
            'abcdefgh1'     -> '3630'
            'abcdefgh4'     -> '3037'
            'abcdefgh10'    -> '4970'
            'abcdefgh11'    -> '4373'
            'abcdefgh24'    -> '4905'
            'abcdefgh99'    -> '4522'
            'abcdefgh100'   -> '3833'
        */
        let proofs = base64::decode("+QEO34CAgMgwhsWEMzgzM4CAgICAgICAgICAgIbFhDQ5NzD4TYCgWvV3JP22NK5fmfA2xp0DgkFi9rkBdw4ADHTeyez/RtzKgiA0hsWENDkwNYDIIIbFhDMwMzeAgICAyoIgOYbFhDQ1MjKAgICAgICA94CAgKCwvJK5hgh1xdoCVjFsZLAr2Ct5ADxnseuJtF+m80+y64CAgICAgICAgICAgIbFhDM2MzD4OaAfBo1nqEW9/DhdOYucHjHAgqpZsF3f96awYBKZkmR2i8gghsWENDM3M4CAgICAgICAgICAgICAgOuJFhYmNkZWZnaDoNDKeVFnNI85QpRhrd2t8hS4By3wpD4R5ZyUegAPUtga").unwrap();
        let root_hash = base58::decode("EA9zTfmf5Ex4ZUTPpMwpsQxQzTkevtwg9PADTqJczhSF").unwrap();
        assert!(_verify_proof_range(
            proofs.as_slice(),
            root_hash.as_slice(),
            "abcdefgh",
            None,
            Some(24),
            &[
                ("abcdefgh1".to_string(), Some("3630".to_string())),
                ("abcdefgh4".to_string(), Some("3037".to_string())),
                ("abcdefgh10".to_string(), Some("4970".to_string())),
                ("abcdefgh11".to_string(), Some("4373".to_string())),
            ],
        ));
    }

    #[test]
    fn state_proof_verify_proof_works_for_get_value_from_leaf_in_range_no_from_fails_missing_values(
    ) {
        /*
            'abcdefgh1'     -> '3630'
            'abcdefgh4'     -> '3037'
            'abcdefgh10'    -> '4970'
            'abcdefgh11'    -> '4373'
            'abcdefgh24'    -> '4905'
            'abcdefgh99'    -> '4522'
            'abcdefgh100'   -> '3833'
        */
        let proofs = base64::decode("+QEO34CAgMgwhsWEMzgzM4CAgICAgICAgICAgIbFhDQ5NzD4TYCgWvV3JP22NK5fmfA2xp0DgkFi9rkBdw4ADHTeyez/RtzKgiA0hsWENDkwNYDIIIbFhDMwMzeAgICAyoIgOYbFhDQ1MjKAgICAgICA94CAgKCwvJK5hgh1xdoCVjFsZLAr2Ct5ADxnseuJtF+m80+y64CAgICAgICAgICAgIbFhDM2MzD4OaAfBo1nqEW9/DhdOYucHjHAgqpZsF3f96awYBKZkmR2i8gghsWENDM3M4CAgICAgICAgICAgICAgOuJFhYmNkZWZnaDoNDKeVFnNI85QpRhrd2t8hS4By3wpD4R5ZyUegAPUtga").unwrap();
        let root_hash = base58::decode("EA9zTfmf5Ex4ZUTPpMwpsQxQzTkevtwg9PADTqJczhSF").unwrap();
        assert!(!_verify_proof_range(
            proofs.as_slice(),
            root_hash.as_slice(),
            "abcdefgh",
            None,
            Some(24),
            &[
                ("abcdefgh1".to_string(), Some("3630".to_string())),
                ("abcdefgh4".to_string(), Some("3037".to_string())),
                //                ("abcdefgh10".to_string(), Some("4970".to_string())),
                ("abcdefgh11".to_string(), Some("4373".to_string())),
            ]
        ));
    }

    #[test]
    fn state_proof_verify_proof_works_for_get_value_from_leaf_in_range_no_from_fails_extra_values()
    {
        /*
            'abcdefgh1'     -> '3630'
            'abcdefgh4'     -> '3037'
            'abcdefgh10'    -> '4970'
            'abcdefgh11'    -> '4373'
            'abcdefgh24'    -> '4905'
            'abcdefgh99'    -> '4522'
            'abcdefgh100'   -> '3833'
        */
        let proofs = base64::decode("+QEO34CAgMgwhsWEMzgzM4CAgICAgICAgICAgIbFhDQ5NzD4TYCgWvV3JP22NK5fmfA2xp0DgkFi9rkBdw4ADHTeyez/RtzKgiA0hsWENDkwNYDIIIbFhDMwMzeAgICAyoIgOYbFhDQ1MjKAgICAgICA94CAgKCwvJK5hgh1xdoCVjFsZLAr2Ct5ADxnseuJtF+m80+y64CAgICAgICAgICAgIbFhDM2MzD4OaAfBo1nqEW9/DhdOYucHjHAgqpZsF3f96awYBKZkmR2i8gghsWENDM3M4CAgICAgICAgICAgICAgOuJFhYmNkZWZnaDoNDKeVFnNI85QpRhrd2t8hS4By3wpD4R5ZyUegAPUtga").unwrap();
        let root_hash = base58::decode("EA9zTfmf5Ex4ZUTPpMwpsQxQzTkevtwg9PADTqJczhSF").unwrap();
        assert!(!_verify_proof_range(
            proofs.as_slice(),
            root_hash.as_slice(),
            "abcdefgh",
            None,
            Some(24),
            &[
                ("abcdefgh1".to_string(), Some("3630".to_string())),
                ("abcdefgh4".to_string(), Some("3037".to_string())),
                ("abcdefgh10".to_string(), Some("4970".to_string())),
                ("abcdefgh11".to_string(), Some("4373".to_string())),
                ("abcdefgh12".to_string(), Some("4373".to_string())),
            ],
        ));
    }

    #[test]
    fn state_proof_verify_proof_works_for_get_value_from_leaf_in_range_no_from_fails_changed_values(
    ) {
        /*
            'abcdefgh1'     -> '3630'
            'abcdefgh4'     -> '3037'
            'abcdefgh10'    -> '4970'
            'abcdefgh11'    -> '4373'
            'abcdefgh24'    -> '4905'
            'abcdefgh99'    -> '4522'
            'abcdefgh100'   -> '3833'
        */
        let proofs = base64::decode("+QEO34CAgMgwhsWEMzgzM4CAgICAgICAgICAgIbFhDQ5NzD4TYCgWvV3JP22NK5fmfA2xp0DgkFi9rkBdw4ADHTeyez/RtzKgiA0hsWENDkwNYDIIIbFhDMwMzeAgICAyoIgOYbFhDQ1MjKAgICAgICA94CAgKCwvJK5hgh1xdoCVjFsZLAr2Ct5ADxnseuJtF+m80+y64CAgICAgICAgICAgIbFhDM2MzD4OaAfBo1nqEW9/DhdOYucHjHAgqpZsF3f96awYBKZkmR2i8gghsWENDM3M4CAgICAgICAgICAgICAgOuJFhYmNkZWZnaDoNDKeVFnNI85QpRhrd2t8hS4By3wpD4R5ZyUegAPUtga").unwrap();
        let root_hash = base58::decode("EA9zTfmf5Ex4ZUTPpMwpsQxQzTkevtwg9PADTqJczhSF").unwrap();
        assert!(!_verify_proof_range(
            proofs.as_slice(),
            root_hash.as_slice(),
            "abcdefgh",
            None,
            Some(24),
            &[
                ("abcdefgh1".to_string(), Some("3630".to_string())),
                ("abcdefgh4".to_string(), Some("3037".to_string())),
                ("abcdefgh10".to_string(), Some("4970".to_string())),
                ("abcdefgh12".to_string(), Some("4373".to_string())),
            ]
        ));
    }

    #[test]
    fn state_proof_verify_proof_works_for_get_value_from_leaf_in_range_no_from_fails_wrong_next() {
        /*
            'abcdefgh1'     -> '3630'
            'abcdefgh4'     -> '3037'
            'abcdefgh10'    -> '4970'
            'abcdefgh11'    -> '4373'
            'abcdefgh24'    -> '4905'
            'abcdefgh99'    -> '4522'
            'abcdefgh100'   -> '3833'
        */
        let proofs = base64::decode("+QEO34CAgMgwhsWEMzgzM4CAgICAgICAgICAgIbFhDQ5NzD4TYCgWvV3JP22NK5fmfA2xp0DgkFi9rkBdw4ADHTeyez/RtzKgiA0hsWENDkwNYDIIIbFhDMwMzeAgICAyoIgOYbFhDQ1MjKAgICAgICA94CAgKCwvJK5hgh1xdoCVjFsZLAr2Ct5ADxnseuJtF+m80+y64CAgICAgICAgICAgIbFhDM2MzD4OaAfBo1nqEW9/DhdOYucHjHAgqpZsF3f96awYBKZkmR2i8gghsWENDM3M4CAgICAgICAgICAgICAgOuJFhYmNkZWZnaDoNDKeVFnNI85QpRhrd2t8hS4By3wpD4R5ZyUegAPUtga").unwrap();
        let root_hash = base58::decode("EA9zTfmf5Ex4ZUTPpMwpsQxQzTkevtwg9PADTqJczhSF").unwrap();
        assert!(!_verify_proof_range(
            proofs.as_slice(),
            root_hash.as_slice(),
            "abcdefgh",
            None,
            Some(99),
            &[
                ("abcdefgh1".to_string(), Some("3630".to_string())),
                ("abcdefgh4".to_string(), Some("3037".to_string())),
                ("abcdefgh10".to_string(), Some("4970".to_string())),
                ("abcdefgh11".to_string(), Some("4373".to_string())),
            ]
        ));
    }

    #[test]
    fn state_proof_verify_proof_works_for_get_value_from_leaf_through_extension() {
        /*
            '33'  -> 'v1'
            'D'   -> 'v2'
            'E'   -> 'v3'
            '333' -> 'v4'
            '334' -> 'v5'
        */
        let proofs = Vec::from_hex("f8a8e4821333a05fff9765fa0c56a26b361c81b7883478da90259d0c469896e8da7edd6ad7c756f2808080dd808080c62084c3827634c62084c382763580808080808080808080808080808080808080808080808084c3827631f84e808080a06a4096e59e980d2f2745d0ed2d1779eb135a1831fd3763f010316d99fd2adbb3dd80808080c62084c3827632c62084c38276338080808080808080808080808080808080808080808080").unwrap();
        let root_hash =
            Vec::from_hex("d01bd87a6105a945c5eb83e328489390e2843a9b588f03d222ab1a51db7b9fab")
                .unwrap();
        assert!(_verify_proof(
            proofs.as_slice(),
            root_hash.as_slice(),
            "333".as_bytes(),
            Some("v4")
        ));
    }

    #[test]
    fn state_proof_verify_proof_works_for_get_value_from_full_node() {
        /*
            '33'  -> 'v1'
            'D'   -> 'v2'
            'E'   -> 'v3'
            '333' -> 'v4'
            '334' -> 'v5'
        */
        let proofs = Vec::from_hex("f8a8e4821333a05fff9765fa0c56a26b361c81b7883478da90259d0c469896e8da7edd6ad7c756f2808080dd808080c62084c3827634c62084c382763580808080808080808080808080808080808080808080808084c3827631f84e808080a06a4096e59e980d2f2745d0ed2d1779eb135a1831fd3763f010316d99fd2adbb3dd80808080c62084c3827632c62084c38276338080808080808080808080808080808080808080808080").unwrap();
        let root_hash =
            Vec::from_hex("d01bd87a6105a945c5eb83e328489390e2843a9b588f03d222ab1a51db7b9fab")
                .unwrap();
        assert!(_verify_proof(
            proofs.as_slice(),
            root_hash.as_slice(),
            "33".as_bytes(),
            Some("v1")
        ));
    }

    #[test]
    fn state_proof_verify_proof_works_for_corrupted_rlp_bytes_for_proofs() {
        let proofs = Vec::from_hex("f8c0f7798080a0792fc4967c792ef3d22fefd3f43209e2185b25e9a97640f09bb4b61657f67cf3c62084c3827634808080808080808080808080f4808080dd808080c62084c3827631c62084c3827632808080808080808080808080c63384c3827633808080808080808080808080f851808080a0099d752f1d5a4b9f9f0034540153d2d2a7c14c11290f27e5d877b57c801848caa06267640081beb8c77f14f30c68f30688afc3e5d5a388194c6a42f699fe361b2f808080808080808080808080").unwrap();
        assert!(!_verify_proof(
            proofs.as_slice(),
            &[0x00],
            "".as_bytes(),
            None
        ));
    }

    #[test]
    fn transaction_handler_parse_generic_reply_for_proof_checking_works_for_get_txn() {
        let json_msg = &json!({
            "type": constants::GET_TXN,
            "data": {
                "auditPath": ["1", "2"],
                "ledgerSize": 2,
                "rootHash": "123",
                "txn": {"test1": "test2", "seqNo": 2},
            },
            "state_proof": {
                "multi_signature": "ms"
            }
        });

        let nodes_str = base64::encode(json!(["1", "2"]).to_string());

        let mut parsed_sps =
            super::parse_generic_reply_for_proof_checking(json_msg, "", Some("2".as_bytes()), None)
                .unwrap();

        assert_eq!(parsed_sps.len(), 1);
        let parsed_sp = parsed_sps.remove(0);
        assert_eq!(parsed_sp.root_hash, "123");
        assert_eq!(parsed_sp.multi_signature, "ms");
        assert_eq!(parsed_sp.proof_nodes, nodes_str);
        assert_eq!(
            parsed_sp.kvs_to_verify,
            KeyValuesInSP::Simple(KeyValueSimpleData {
                kvs: vec![(
                    base64::encode("2"),
                    Some(json!({"txn":{"test1": "test2", "seqNo": 2}}).to_string())
                )],
                verification_type: KeyValueSimpleDataVerificationType::MerkleTree(2),
            })
        );
    }

    #[test]
    fn transaction_handler_parse_generic_reply_for_proof_checking_works_for_get_txn_no_multi_signature(
    ) {
        let json_msg = &json!({
                    "type": constants::GET_TXN,
                    "data": {
                        "auditPath": ["1", "2"],
                        "ledgerSize": 2,
                        "rootHash": "123",
                        "txn": {"test1": "test2", "seqNo": 2},
        //              "multi_signature": "ms"
                    }
                });

        let nodes_str = base64::encode(json!(["1", "2"]).to_string());

        let mut parsed_sps =
            super::parse_generic_reply_for_proof_checking(json_msg, "", Some("2".as_bytes()), None)
                .unwrap();

        assert_eq!(parsed_sps.len(), 1);
        let parsed_sp = parsed_sps.remove(0);
        assert_eq!(parsed_sp.root_hash, "123");
        assert!(parsed_sp.multi_signature.is_null());
        assert_eq!(parsed_sp.proof_nodes, nodes_str);
        assert_eq!(
            parsed_sp.kvs_to_verify,
            KeyValuesInSP::Simple(KeyValueSimpleData {
                kvs: vec![(
                    base64::encode("2"),
                    Some(json!({"txn":{"test1": "test2", "seqNo": 2}}).to_string())
                )],
                verification_type: KeyValueSimpleDataVerificationType::MerkleTree(2),
            })
        );
    }

    #[test]
    fn transaction_handler_parse_generic_reply_for_proof_checking_works_for_get_txn_no_ledger_length(
    ) {
        let json_msg = &json!({
                    "type": constants::GET_TXN,
                    "data": {
                        "auditPath": ["1", "2"],
        //                "ledgerSize": 2,
                        "rootHash": "123",
                        "txn": {"test1": "test2", "seqNo": 2},
                        "state_proof": {
                            "multi_signature": "ms"
                        }
                    }
                });

        assert!(super::parse_generic_reply_for_proof_checking(
            json_msg,
            "",
            Some("2".as_bytes()),
            None
        )
        .is_none());
    }

    #[test]
    fn transaction_handler_parse_generic_reply_for_proof_checking_works_for_get_txn_no_txn() {
        let json_msg = &json!({
                    "type": constants::GET_TXN,
                    "data": {
                        "auditPath": ["1", "2"],
                        "ledgerSize": 2,
                        "rootHash": "123",
        //                "txn": {"test1": "test2", "seqNo": 2},
                    },
                    "state_proof": {
                        "multi_signature": "ms"
                    }
                });

        let nodes_str = base64::encode(json!(["1", "2"]).to_string());

        let mut parsed_sps =
            super::parse_generic_reply_for_proof_checking(json_msg, "", Some("2".as_bytes()), None)
                .unwrap();

        assert_eq!(parsed_sps.len(), 1);
        let parsed_sp = parsed_sps.remove(0);
        assert_eq!(parsed_sp.root_hash, "123");
        assert_eq!(parsed_sp.multi_signature, "ms");
        assert_eq!(parsed_sp.proof_nodes, nodes_str);
        assert_eq!(
            parsed_sp.kvs_to_verify,
            KeyValuesInSP::Simple(KeyValueSimpleData {
                kvs: vec![(base64::encode("2"), None)],
                verification_type: KeyValueSimpleDataVerificationType::MerkleTree(2),
            })
        );
    }

    #[test]
    fn transaction_handler_parse_generic_reply_for_proof_checking_works_for_plugged() {
        let parsed_sp = json!([{
            "root_hash": "rh",
            "proof_nodes": "pns",
            "multi_signature": "ms",
            "kvs_to_verify": {
                "type": "Simple",
                "kvs": [],
            },
        }]);

        struct CustomSPParser {}
        impl StateProofParser for CustomSPParser {
            fn parse(&self, _txn_type: &str, reply_from_node: &str) -> Option<Vec<ParsedSP>> {
                Some(serde_json::from_str(reply_from_node).unwrap())
            }
        }

        let mut parsed_sps = super::parse_generic_reply_for_proof_checking(
            &json!({"type".to_owned(): "test"}),
            parsed_sp.to_string().as_str(),
            None,
            Some(CustomSPParser {}.boxed()).as_ref(),
        )
        .unwrap();

        assert_eq!(parsed_sps.len(), 1);
        let parsed_sp = parsed_sps.remove(0);
        assert_eq!(parsed_sp.root_hash, "rh");
        assert_eq!(parsed_sp.multi_signature, "ms");
        assert_eq!(parsed_sp.proof_nodes, "pns");
        assert_eq!(
            parsed_sp.kvs_to_verify,
            KeyValuesInSP::Simple(KeyValueSimpleData {
                kvs: Vec::new(),
                verification_type: KeyValueSimpleDataVerificationType::Simple,
            })
        );
    }

    #[test]
    fn transaction_handler_parse_generic_reply_for_proof_checking_works_for_plugged_range() {
        let parsed_sp = json!([{
            "root_hash": "rh",
            "proof_nodes": "pns",
            "multi_signature": "ms",
            "kvs_to_verify": {
                "type": "Simple",
                "kvs": [],
                "verification_type": {
                    "type": "NumericalSuffixAscendingNoGaps",
                    "from": 1,
                    "next": 2,
                    "prefix": "abc"
                }
            },
        }]);

        let custom_state_proofs_parser =
            state_proof_parser_fn(|_txn_type: &str, reply_from_node: &str| {
                Some(serde_json::from_str(reply_from_node).unwrap())
            });

        let mut parsed_sps = super::parse_generic_reply_for_proof_checking(
            &json!({"type".to_owned(): "test"}),
            parsed_sp.to_string().as_str(),
            None,
            Some(&custom_state_proofs_parser.boxed()),
        )
        .unwrap();

        assert_eq!(parsed_sps.len(), 1);
        let parsed_sp = parsed_sps.remove(0);
        assert_eq!(parsed_sp.root_hash, "rh");
        assert_eq!(parsed_sp.multi_signature, "ms");
        assert_eq!(parsed_sp.proof_nodes, "pns");
        assert_eq!(
            parsed_sp.kvs_to_verify,
            KeyValuesInSP::Simple(KeyValueSimpleData {
                kvs: Vec::new(),
                verification_type:
                    KeyValueSimpleDataVerificationType::NumericalSuffixAscendingNoGaps(
                        NumericalSuffixAscendingNoGapsData {
                            from: Some(1),
                            next: Some(2),
                            prefix: "abc".to_string(),
                        }
                    ),
            })
        );
    }

    #[test]
    fn transaction_handler_parse_generic_reply_for_proof_checking_works_for_plugged_range_nones() {
        let parsed_sp = json!([{
            "root_hash": "rh",
            "proof_nodes": "pns",
            "multi_signature": "ms",
            "kvs_to_verify": {
                "type": "Simple",
                "kvs": [],
                "verification_type": {
                    "type": "NumericalSuffixAscendingNoGaps",
                    "from": serde_json::Value::Null,
                    "next": serde_json::Value::Null,
                    "prefix": "abc"
                }
            },
        }]);

        let custom_state_proofs_parser =
            state_proof_parser_fn(|_txn_type: &str, reply_from_node: &str| {
                Some(serde_json::from_str(reply_from_node).unwrap())
            });

        let mut parsed_sps = super::parse_generic_reply_for_proof_checking(
            &json!({"type".to_owned(): "test"}),
            parsed_sp.to_string().as_str(),
            None,
            Some(&custom_state_proofs_parser.boxed()),
        )
        .unwrap();

        assert_eq!(parsed_sps.len(), 1);
        let parsed_sp = parsed_sps.remove(0);
        assert_eq!(parsed_sp.root_hash, "rh");
        assert_eq!(parsed_sp.multi_signature, "ms");
        assert_eq!(parsed_sp.proof_nodes, "pns");
        assert_eq!(
            parsed_sp.kvs_to_verify,
            KeyValuesInSP::Simple(KeyValueSimpleData {
                kvs: Vec::new(),
                verification_type:
                    KeyValueSimpleDataVerificationType::NumericalSuffixAscendingNoGaps(
                        NumericalSuffixAscendingNoGapsData {
                            from: None,
                            next: None,
                            prefix: "abc".to_string(),
                        }
                    ),
            })
        );
    }

    #[test]
    fn check_state_proof_valid() {
        let raw_msg = r#"{"op":"REPLY","result":{"identifier":"LibindyDid111111111111","reqId":1691520834828315000,"type":"3","data":{"reqSignature":{},"txn":{"data":{"dest":"V4SGRU86Z58d6TV7PBUe6f","role":"0","verkey":"~CoRER63DVYnWZtK8uAzNbx"},"metadata":{},"type":"1"},"txnMetadata":{"seqNo":1},"ver":"1","rootHash":"DxX9E3XxEPHbb3JjakcmSduPc2bBcWsFhZZGp5aa842q","auditPath":["3XtSyZ8CQPJUYbc5mFKvUendLZSt4ybG2Y4zRtJEewSL","96irBGYpWrTvrVATexGGvktPrT3WicixwT8BtoZTtkYX","HqXD3TkLbpRuRU7CrrvrBeZwKuNFVCfta1ez7X7jGjtF","3fsGMWtrpYdNiLZKRKGmhGUJTUkdC2yn2yNd8MPGjwdq","BwS8ttPxJXQ4yn5RDy6spyxrFRZkukr9dbs9bjfskz1U","3wvhiYWLX3fRwGp1SoLeMQas6xtRHK8n7a3WqLPiwyMc","8oJHS289uuhcmgrvrzVtXvRGFfoXRnTWZnHQRYopDtUG","B5yx8ExTWjkgaDHuYWbosaoPhuq15uBx1jmp6npp6cKa","41vHGCg6qKUEtLAveyeWLMNdhZoH89Ym6xymFvSj64ER","APznt6o24yBWCNs5tVF4fC6h6rMz1Joj9BYWQuXJH1V5","3EByMrinqTxqaC7VEnQj4bKn29Gg357MoaTJxhZJvAbv","CV3xU14oTyGxemt6ZzLGhcBoTEcQ9MivEgo4fREPJbax","9MvXyCYNaPnTWV5ZW6E8hkPnjEurmTGmzTTUJJ9sGZ3L","8T7istFjSSxgYzZoxcJLtBm1hW48kTpGXBqbXMigopZ5"],"ledgerSize":12713},"state_proof":{"multi_signature":{"signature":"RRM4P551uBWUUZrz1AnspaL2n4ar65WBLn1ANS2XUPWir8bEq5LWdowmdjYvp3scEHPEMxGgJTB5ffVevBsoMVgtyB2SUxr6ZTAAtmE73RETGVwRCQnz3k2gEGaYyAxVSon51RHW5Jg9hEgyMWR2j3aib5o7fFDZFhBy2oB1bS46go","participants":["Node3","Node2","Node1"],"value":{"ledger_id":1,"pool_state_root_hash":"7siDH8Qanh82UviK4zjBSfLXcoCvLaeGkrByi1ow9Tsm","state_root_hash":"8AasPY2KBtPLiVnvePAZhPZKAfRozAR9CBUYAXFBhdXo","timestamp":1691520806,"txn_root_hash":"DxX9E3XxEPHbb3JjakcmSduPc2bBcWsFhZZGp5aa842q"}}},"seqNo":1}}"#;
        let f = 1;
        let mut bls_keys = HashMap::new();
        bls_keys.insert("Node1".to_owned(), VerifierKey::from_bytes(&hex::decode("20e085f100560896f50ea75e681a780275e9e39d645fcf8a48bc771dd41e304d099f5a5c009f5ac95776c7534ac4ec2550a0fa0da8422aa4b28a5ab76b34ba16054995a826fceef2fc619732c6971e5ca39a49f41b117e33868551c8f3f481751e34851a6c913a6f4e8c1d5ae13ac5460b69378b7d94a07f46fa92445dc8eecd").unwrap()).unwrap());
        bls_keys.insert("Node2".to_owned(), VerifierKey::from_bytes(&hex::decode("14b2c1cb385e56510cc8f050317580bcaf792ba555f29f7a8454d4367d63ea8020e9a34506a173320a5d0a4dff36cdda7d1d7848495e8e0c2a420d55c5704efc0dd8cec3869e061728abc55ce9948085358c1661799a2e289ea2fda0d8d083640ade487d5787924a6ed0cd7cbe727b9296ea66e8acc7b47fa9e1254ac6ee2827").unwrap()).unwrap());
        bls_keys.insert("Node3".to_owned(), VerifierKey::from_bytes(&hex::decode("187945bb8673691a57fa719dbc93653c909f359da42281b22b2b2e2748abc4d71ff796348e496d6be919bc3710f1b11d04fe9c436fb3c80ac5da556e94a73ba617d9180856dd73c6c30b9716ec0546ccebda8a80cd9058c88af45079a45ad35921cb2e6488caab9c4f35dbef9efdc22ece8769c60f82b38c78d547f7ad866016").unwrap()).unwrap());
        bls_keys.insert("Node4".to_owned(), VerifierKey::from_bytes(&hex::decode("136feaf1ad5b81d70de5c5287b0ef24746b8db60dba8ec502aeb213ae5c9f1900b59ff8e6f38e00e5d4cf2a45fb3317a0ccfc710806d368acb2267e097ed696611cc9295d2bbca32d1e176f026a66f02f70a8851ec71f2f4321dc62f00b5cf071f32e6fc3a1f63278360c7dd8285224ed482ff59ab5063aee3117a111fc9ffd2").unwrap()).unwrap());
        let reply: serde_json::Value = serde_json::from_str(raw_msg).unwrap();
        let msg_result = &reply["result"];
        let asserts = StateProofAssertions {
            ledger_id: 1,
            pool_state_root_hash: "7siDH8Qanh82UviK4zjBSfLXcoCvLaeGkrByi1ow9Tsm".into(),
            state_root_hash: "8AasPY2KBtPLiVnvePAZhPZKAfRozAR9CBUYAXFBhdXo".into(),
            timestamp: 1691520806,
            txn_root_hash: "DxX9E3XxEPHbb3JjakcmSduPc2bBcWsFhZZGp5aa842q".into(),
        };
        assert_eq!(
            check_state_proof(
                msg_result,
                f,
                &DEFAULT_GENERATOR,
                &bls_keys,
                raw_msg,
                Some(&[49]),
                (None, Some(0)),
                1691520806,
                300,
                None,
            ),
            StateProofResult::Verified(asserts.clone())
        );

        assert_eq!(
            check_state_proof(
                msg_result,
                f,
                &DEFAULT_GENERATOR,
                &bls_keys,
                raw_msg,
                Some(&[49]),
                (None, Some(1691521806)),
                1691520806,
                300,
                None,
            ),
            StateProofResult::Expired(asserts)
        );
    }
}
