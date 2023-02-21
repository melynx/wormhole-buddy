
use reqwest::Url;
use comfy_table::Table;
use wormhole::{Vaa, vaa::Body};
use wormhole::Chain;
use serde_wormhole::RawMessage;

use ethers::providers::{Middleware, Provider, Http};

use crate::common::*;

pub fn query_guardian(chain: CooChain, emitter: EmitterType, sequence: u64, guardian_url: Url) -> Result<Vec<u8>, CooError> {
    let query_url = get_query_url(chain, emitter, sequence, guardian_url)?;
    println!("querying guardian at {}", query_url);
    // make a http request to the guardian
    let result = reqwest::blocking::get(query_url)?;
    let body = result.text()?;
    // deserialize body as a json object with a field "vaaBytes"
    let guardian_response: serde_json::Value = serde_json::from_str(&body)?;
    // ensure that vaaBytes exists, if not return the entire json object as an error
    let vaa_bytes_base64 = match guardian_response["vaaBytes"].as_str() {
        Some(v) => {
            v
        },
        None => {
            return Err(CooError::ParseError(format!("vaaBytes not found in response: {}", body)));
        }
    };
    let vaa_bytes = base64tobytes(vaa_bytes_base64)?;
    return Ok(vaa_bytes);
}

pub fn get_query_url(chain: CooChain, emitter: EmitterType, sequence: u64, guardian_url: Url) -> Result<Url, CooError> {
    let emitter_contract = resolve_emitter_address(chain, emitter)?;
    let query_path = format!("v1/signed_vaa/{}/{}/{}", u16::from(chain), emitter_contract, sequence);
    let query_url = guardian_url.join(&query_path)?;
    return Ok(query_url);
}

pub fn parse_vaa<'a> (vaa_bytes: &'a [u8]) -> Result<Vaa<&'a RawMessage>, CooError> {
    let vaa = serde_wormhole::from_slice(vaa_bytes)?;
    return Ok(vaa);
}

pub fn _augment_vaa<'a> (vaa: Vaa<&'a RawMessage>) -> Result<Body<&'a RawMessage>, CooError> {
    let (head, body): (wormhole::vaa::Header, wormhole::vaa::Body<&'a RawMessage>) = vaa.into();
    Ok(body)
}

pub fn decode_wormhole_token<'a> (vaa: &Vaa<&'a RawMessage>) -> Result<wormhole::token::Message, CooError> {
    let message: wormhole::token::Message = serde_wormhole::from_slice(vaa.payload).unwrap();
    return Ok(message);
}

pub fn decode_wormhole_nft<'a> (vaa: &Vaa<&'a RawMessage>) -> Result<wormhole::nft::Message, CooError> {
    let message: wormhole::nft::Message = serde_wormhole::from_slice(vaa.payload).unwrap();
    return Ok(message);
}

pub fn pretty_vaa<T>(vaa: &Vaa<T>) -> String {
    let multiline_signatures = vaa.signatures.iter().map(
        |s| format!("{: <2}: {}", s.index, hex::encode(s.signature))
    ).collect::<Vec<String>>().join("\n");
    let mut table = Table::new();
    table
        .set_header(["VAA Information"])
        .add_row(["Version", &vaa.version.to_string()])
        .add_row(["Timestamp", &vaa.timestamp.to_string()])
        .add_row(["Nonce", &vaa.nonce.to_string()])
        .add_row(["Emitter Chain", &vaa.emitter_chain.to_string()])
        .add_row(["Emitter Address", &vaa.emitter_address.to_string()])
        .add_row(["Sequence", &vaa.sequence.to_string()])
        .add_row(["Consistency Level", &vaa.consistency_level.to_string()])
        .add_row(["Guardian Set", &vaa.guardian_set_index.to_string()])
        .add_row(["Signatures", &multiline_signatures]);
    return format!("{table}");
}

// write some test cases for the query_guardian function
#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn test_query_guardian() {
        let guardian_url = Url::from_str(GUARDIAN_URL).unwrap();
        let chain = CooChain::Inner(Chain::Avalanche);
        let vaa_bytes = query_guardian(chain, EmitterType::TokenBridge, 1, guardian_url).unwrap();
        assert_eq!(vaa_bytes.len(), 1015);
    }

    // write a test case for the get_query_url function
    #[test]
    fn test_get_query_url() {
        let guardian_url = Url::from_str(GUARDIAN_URL).unwrap();
        let chain = CooChain::Inner(Chain::Avalanche);
        let query_url = get_query_url(chain, EmitterType::TokenBridge, 1, guardian_url).unwrap();
        assert_eq!(query_url.to_string(), "https://wormhole-v2-mainnet-api.certus.one/v1/signed_vaa/6/0000000000000000000000000e082f06ff657d94310cb8ce8b0d9a04541d8052/1")
    }
}