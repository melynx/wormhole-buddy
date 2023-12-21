use std::{collections::HashMap, fmt::Display};
use base64::{Engine, engine::general_purpose};
use clap::ValueEnum;
use comfy_table::Table;
use lazy_static::lazy_static;
use serde_wormhole::RawMessage;
use wormhole_sdk::{Chain, token::Message, nft::TokenId};

pub const GUARDIAN_URL: &str = "https://wormhole-v2-mainnet-api.certus.one/";

lazy_static! {
    pub static ref EMITTERS: HashMap<(CooChain, EmitterType), &'static str> = HashMap::from(
        [
            ((CooChain::Inner(Chain::Ethereum), EmitterType::CoreBridge), "98f3c9e6E3fAce36bAAd05FE09d375Ef1464288B"),
            ((CooChain::Inner(Chain::Ethereum), EmitterType::TokenBridge), "3ee18B2214AFF97000D974cf647E7C347E8fa585"),
            ((CooChain::Inner(Chain::Ethereum), EmitterType::NftBridge), "6FFd7EdE62328b3Af38FCD61461Bbfc52F5651fE"),
            ((CooChain::Inner(Chain::Avalanche), EmitterType::CoreBridge), "54a8e5f9c4CbA08F9943965859F6c34eAF03E26c"),
            ((CooChain::Inner(Chain::Avalanche), EmitterType::TokenBridge), "e082f06ff657d94310cb8ce8b0d9a04541d8052"),
            ((CooChain::Inner(Chain::Avalanche), EmitterType::NftBridge), "f7B6737Ca9c4e08aE573F75A97B73D7a813f5De5"),
        ]
    );
    pub static ref RPC_ENDPOINTS: HashMap<CooChain, &'static str> = HashMap::from(
        [
            (CooChain::Inner(Chain::Ethereum), "https://1rpc.io/eth"),
            (CooChain::Inner(Chain::Avalanche), "https://1rpc.io/avax"),
            (CooChain::Inner(Chain::Solana), "https://1rpc.io/sol"),
            (CooChain::Inner(Chain::Bsc), "https://1rpc.io/bnb"),
            (CooChain::Inner(Chain::Polygon), "https://1rpc.io/matic"),
        ]
    );
}


#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum EmitterType {
    Unset,
    CoreBridge,
    TokenBridge,
    NftBridge,
    Address([u8; 32]),
}

impl From<&str> for EmitterType {
    fn from(s: &str) -> Self {
        match s {
            "" => EmitterType::Unset,
            "core" => EmitterType::CoreBridge,
            "token" => EmitterType::TokenBridge,
            "nft" => EmitterType::NftBridge,
            _ => { 
                let mut emitter_address = [0u8; 32];
                let decoded = hextobytes(s).unwrap();
                let diff = 32 - decoded.len();
                emitter_address[diff..].copy_from_slice(&decoded);
                EmitterType::Address(emitter_address)
            },
        }
    }
}

impl Display for EmitterType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EmitterType::Unset => panic!("Unset emitter type"),
            EmitterType::CoreBridge => write!(f, "core"),
            EmitterType::TokenBridge => write!(f, "token"),
            EmitterType::NftBridge => write!(f, "nft"),
            EmitterType::Address(a) => write!(f, "{}", hex::encode(a)),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CooVaaAugment {
    digest: [[u8; 32]; 2],
    guardians_set: Vec<[u8; 32]>,
}


#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, ValueEnum)]
pub enum PayloadType {
    SmartInfer,
    RawBytes,
    WormholeTokenTransfer,
    WormholeTokenTransferPayload,
    WormholeNftTransfer,
    WormholeAssetMeta,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum PayloadResponse {
    RawBytes(Vec<u8>),
    WormholeTokenTransfer(wormhole_sdk::token::Message<Box<RawMessage>>),
    WormholeTokenTransferPayload(wormhole_sdk::token::Message<Box<RawMessage>>),
    WormholeAssetMeta(wormhole_sdk::token::Message<Box<RawMessage>>),
    WormholeNftTransfer(wormhole_sdk::nft::Message),
}

impl Display for PayloadResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PayloadResponse::RawBytes(b) => write!(f, "{}", hex::encode(b)),
            PayloadResponse::WormholeTokenTransfer(m) |
            PayloadResponse::WormholeTokenTransferPayload(m) |
            PayloadResponse::WormholeAssetMeta(m) => {
                let table = match m {
                    Message::Transfer { amount, token_address, token_chain, recipient, recipient_chain, fee } => {
                        let mut table = Table::new();
                        table
                            .set_header(["Payload"])
                            .add_row(["Payload Type", "Wormhole Token Transfer"])
                            .add_row(["Amount", &bytestohex(&amount.0)])
                            .add_row(["Token Address", &bytestohex(&token_address.0)])
                            .add_row(["Token Chain", &token_chain.to_string()])
                            .add_row(["Recipient", &bytestohex(&recipient.0)])
                            .add_row(["Recipient Chain", &recipient_chain.to_string()])
                            .add_row(["Fee", &bytestohex(&fee.0)]);
                        table
                    },
                    Message::AssetMeta { token_address, token_chain, name, symbol, decimals} => {
                        let mut table = Table::new();
                        table
                            .set_header(["Payload"])
                            .add_row(["Payload Type", "Wormhole Asset Meta"])
                            .add_row(["Token Address", &bytestohex(&token_address.0)])
                            .add_row(["Token Chain", &token_chain.to_string()])
                            .add_row(["Name", &name.to_string()])
                            .add_row(["Symbol", &symbol.to_string()])
                            .add_row(["Decimals", &decimals.to_string()]);
                        table
                    },
                    Message::TransferWithPayload { amount, token_address, token_chain, recipient, recipient_chain, sender_address, payload } => {
                        let mut table = Table::new();
                        table
                            .set_header(["Payload"])
                            .add_row(["Payload Type", "Wormhole Token Transfer with Payload"])
                            .add_row(["Amount", &bytestohex(&amount.0)])
                            .add_row(["Token Address", &bytestohex(&token_address.0)])
                            .add_row(["Token Chain", &token_chain.to_string()])
                            .add_row(["Recipient", &bytestohex(&recipient.0)])
                            .add_row(["Recipient Chain", &recipient_chain.to_string()])
                            .add_row(["Sender Address", &bytestohex(&sender_address.0)])
                            .add_row(["Payload", &payload.to_string()]);
                        table
                    },
                };
                write!(f, "{}", table)
            }
            PayloadResponse::WormholeNftTransfer(m) => write!(f, "{}", serde_json::to_string_pretty(m).unwrap()),
        }
    }
}

// wrapper chainid type such that we can implement FromStr
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CooChain {
    Inner(Chain),
}

impl From<Chain> for CooChain {
    fn from(c: Chain) -> Self {
        CooChain::Inner(c)
    }
}

impl From<&str> for CooChain {
    fn from(s: &str) -> Self {
        // tries to convert s into chain
        if let Ok(c) = s.parse::<Chain>() {
            return CooChain::Inner(c);
        }
        let c = s.parse::<u16>().unwrap();
        CooChain::from(c)
    }
}

impl From<u16> for CooChain {
    fn from(c: u16) -> Self {
        let c = Chain::from(c);
        CooChain::Inner(c)
    }
}

impl From<CooChain> for u16 {
    fn from(c: CooChain) -> Self {
        match c {
            CooChain::Inner(c) => u16::from(c),
        }
    }
}

#[derive(Debug)]
pub enum CooError {
    ReqwestError(reqwest::Error),
    SerdeJsonError(serde_json::Error),
    Base64Error(base64::DecodeError),
    Base58Error(bs58::decode::Error),
    HexError(hex::FromHexError),
    SerdeWormholeError(serde_wormhole::Error),
    ParseError(String),
}

impl From<reqwest::Error> for CooError {
    fn from(e: reqwest::Error) -> Self {
        CooError::ReqwestError(e)
    }
}

impl From<serde_json::Error> for CooError {
    fn from(e: serde_json::Error) -> Self {
        CooError::SerdeJsonError(e)
    }
}

impl From<base64::DecodeError> for CooError {
    fn from(e: base64::DecodeError) -> Self {
        CooError::Base64Error(e)
    }
}

impl From<url::ParseError> for CooError {
    fn from(e: url::ParseError) -> Self {
        CooError::ParseError(e.to_string())
    }
}

impl From<serde_wormhole::Error> for CooError {
    fn from(e: serde_wormhole::Error) -> Self {
        CooError::SerdeWormholeError(e)
    }
}

impl From<bs58::decode::Error> for CooError {
    fn from(e: bs58::decode::Error) -> Self {
        CooError::Base58Error(e)
    }
}

impl From<hex::FromHexError> for CooError {
    fn from(e: hex::FromHexError) -> Self {
        CooError::HexError(e)
    }
}

pub fn base64tobytes(s: &str) -> Result<Vec<u8>, CooError> {
    let s = general_purpose::STANDARD.decode(&s)?;
    Ok(s)
}

pub fn base58tobytes(s: &str) -> Result<Vec<u8>, CooError> {
    let s = bs58::decode(s).into_vec()?;
    Ok(s)
}

pub fn hextobytes(s: &str) -> Result<Vec<u8>, CooError> {
    // if string has 0x, remove it
    let s = if s.starts_with("0x") {
        &s[2..]
    } else {
        s
    };
    let s = hex::decode(s)?;
    Ok(s)
}

pub fn bytestohex(s: &[u8]) -> String {
    // find the number of leading 0s
    let mut leading_zeros = 0;
    for i in 0..s.len() {
        if s[i] == 0 {
            leading_zeros += 1;
        } else {
            break;
        }
    }
    format!("0x{}", hex::encode(&s[leading_zeros..]))
}

pub fn resolve_emitter_address(chain: CooChain, emitter: EmitterType) -> Result<String, CooError> {
    match emitter {
        EmitterType::Unset => Err(CooError::ParseError("Unset emitter type".to_string())),
        EmitterType::CoreBridge | EmitterType::TokenBridge | EmitterType::NftBridge  =>  {
            let contract_string = EMITTERS[&(chain, emitter)];
            let contract_address= hextobytes(contract_string)?;
            let wormhole_padded = format!("{:0>64}", hex::encode(contract_address));
            Ok(wormhole_padded)
        },
        EmitterType::Address(a) => Ok(hex::encode(a)),
    }
}

pub fn tokenidtostring(tokenid: &TokenId) -> String {
    bytestohex(&tokenid.0)
}

pub fn amounttostring(amount: &wormhole_sdk::Amount) -> String {
    bytestohex(&amount.0)
}