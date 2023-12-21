
use reqwest::Url;
use comfy_table::{Table, Row};
use wormhole_sdk::Vaa;
use serde_wormhole::RawMessage;

// use ethers::providers::{Middleware, Provider, Http};

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

pub fn decode_wormhole_token<'a> (vaa: &Vaa<&'a RawMessage>) -> Result<wormhole_sdk::token::Message, CooError> {
    let message: wormhole_sdk::token::Message = serde_wormhole::from_slice(vaa.payload).unwrap();
    return Ok(message);
}

pub fn decode_wormhole_nft<'a> (vaa: &Vaa<&'a RawMessage>) -> Result<wormhole_sdk::nft::Message, CooError> {
    let message: wormhole_sdk::nft::Message = serde_wormhole::from_slice(vaa.payload).unwrap();
    return Ok(message);
}

pub fn pretty_token_payload(payload: &wormhole_sdk::token::Message) -> String {
    let mut table = Table::new();
    table.set_header(["Wormhole Token Payload Information"]);
    let rows:Vec<Row> = match payload {
        wormhole_sdk::token::Message::Transfer { amount, token_address, token_chain, recipient, recipient_chain, fee } => {
            vec![
                ["Payload Type", "Transfer"].into(),
                ["Amount", &amounttostring(amount)].into(),
                ["Token Address (Origin)", &token_address.to_string()].into(),
                ["Token Chain (Origin)", &token_chain.to_string()].into(),
                ["Token Recipient", &recipient.to_string()].into(),
                ["Token Recipient Chain", &recipient_chain.to_string()].into(),
                ["Relayer Fees", &amounttostring(fee)].into(),
            ]
        },
        wormhole_sdk::token::Message::AssetMeta { token_address, token_chain, decimals, symbol, name } => {
            vec![
                ["Payload Type", "AssetMeta"].into(),
                ["Token Address (Origin)", &token_address.to_string()].into(),
                ["Token Chain (Origin)", &token_chain.to_string()].into(),
                ["Token Decimals", &decimals.to_string()].into(),
                ["Token Symbol", &symbol.to_string()].into(),
                ["Token Name", &name.to_string()].into(),
            ]

        },
        wormhole_sdk::token::Message::TransferWithPayload { amount, token_address, token_chain, recipient, recipient_chain, sender_address, payload } => {
            vec![
                ["Payload Type", "TransferWithPayload"].into(),
                ["Amount", &amounttostring(amount)].into(),
                ["Token Address (Origin)", &token_address.to_string()].into(),
                ["Token Chain (Origin)", &token_chain.to_string()].into(),
                ["Token Recipient", &recipient.to_string()].into(),
                ["Token Recipient Chain", &recipient_chain.to_string()].into(),
                ["Sender Address", &sender_address.to_string()].into(),
                ["Payload", &payload.to_string()].into(),
            ]
        },
    };
    table.add_rows(rows);
    return format!("{table}");
}


pub fn pretty_nft_payload(payload: &wormhole_sdk::nft::Message) -> String {
    let mut table = Table::new();
    table.set_header(["Wormhole NFT Payload Information"]);
    let rows: Vec<Row> = match payload {
        wormhole_sdk::nft::Message::Transfer { nft_address, nft_chain, symbol, name, token_id, uri, to, to_chain } => {
            vec![
                ["Payload Type", "Transfer"].into(),
                ["NFT Address (Origin)", &nft_address.to_string()].into(),
                ["NFT Chain (Origin)", &nft_chain.to_string()].into(),
                ["NFT Symbol", &symbol.to_string()].into(),
                ["NFT Name", &name.to_string()].into(),
                ["Token ID", &tokenidtostring(&token_id)].into(),
                ["URI", &uri.to_string()].into(),
                ["Destination Address", &to.to_string()].into(),
                ["Destination Chain", &to_chain.to_string()].into(),
            ]
        }
    };
    table.add_rows(rows);
    return format!("{table}");
}

pub fn pretty_vaa<T>(vaa: &Vaa<T>) -> String {
    let multiline_signatures = vaa.signatures.iter().map(
        |s| format!("{: <2}: {}", s.index, hex::encode(s.signature))
    ).collect::<Vec<String>>().join("\n");
    let mut table = Table::new();
    table.set_header(["VAA Information"]);
    let rows:Vec<Row> = vec![
        ["Version", &vaa.version.to_string()].into(),
        ["Timestamp", &vaa.timestamp.to_string()].into(),
        ["Nonce", &vaa.nonce.to_string()].into(),
        ["Emitter Chain", &vaa.emitter_chain.to_string()].into(),
        ["Emitter Address", &vaa.emitter_address.to_string()].into(),
        ["Sequence", &vaa.sequence.to_string()].into(),
        ["Consistency Level", &vaa.consistency_level.to_string()].into(),
        ["Guardian Set", &vaa.guardian_set_index.to_string()].into(),
        ["Signatures", &multiline_signatures].into(),
    ];
    table.add_rows(rows);

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
        let chain = CooChain::Inner(wormhole_sdk::Chain::Avalanche);
        let vaa_bytes = query_guardian(chain, EmitterType::TokenBridge, 1, guardian_url).unwrap();
        assert_eq!(vaa_bytes.len(), 1015);
    }

    // write a test case for the get_query_url function
    #[test]
    fn test_get_query_url() {
        let guardian_url = Url::from_str(GUARDIAN_URL).unwrap();
        let chain = CooChain::Inner(wormhole_sdk::Chain::Avalanche);
        let query_url = get_query_url(chain, EmitterType::TokenBridge, 1, guardian_url).unwrap();
        assert_eq!(query_url.to_string(), "https://wormhole-v2-mainnet-api.certus.one/v1/signed_vaa/6/0000000000000000000000000e082f06ff657d94310cb8ce8b0d9a04541d8052/1")
    }
}