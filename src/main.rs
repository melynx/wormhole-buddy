use std::path::{PathBuf, Path};
use std::str::FromStr;
use std::io::Write;

use clap::{Parser, Subcommand, Args, ValueEnum};
use lazy_static::lazy_static;

mod common;
mod vaa;

use crate::common::{GUARDIAN_URL, EmitterType, CooChain, PayloadType, hextobytes, base58tobytes, base64tobytes, EMITTERS, PayloadResponse, resolve_emitter_address};
use crate::vaa::{query_guardian, parse_vaa, pretty_vaa, decode_wormhole_token, decode_wormhole_nft};

lazy_static! {
    static ref DEFAULT_APP_PATH: PathBuf = dirs::home_dir().unwrap().join(".coo");
    static ref DEFAULT_CONFIG_PATH: PathBuf = DEFAULT_APP_PATH.join("config");
    static ref DEFAULT_CACHE_PATH: PathBuf = DEFAULT_APP_PATH.join("cache");
}

#[derive(Debug, Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[clap(subcommand)]
    command: Option<CooCommand>,
    #[arg(long)]
    app_path: Option<PathBuf>,
}

#[derive(Debug, Subcommand)]
enum CooCommand {
    /// Your friendly all-in-one toolkit to view or manipulate Wormhole VAAs.
    Vaa(VaaArgs),
}

#[derive(Debug, Args)]
struct VaaArgs {
    #[clap(subcommand)]
    vaa_command: Option<VaaCommand>,
}

#[derive(Debug, Subcommand)]
enum VaaCommand {
    /// Performs a query to the Wormhole Guardian API to get the VAA.
    Query(VaaQueryArgs),
    /// Decodes a VAA.
    Decode(VaaDecodeArgs),
    /// List VAAs that have been queried.
    List,
}

#[derive(Debug, Args)]
struct VaaQueryArgs {
    #[arg(short, long, default_value = GUARDIAN_URL)]
    /// Wormhole Guardian RPC URL
    guardian_url_str: String,
    /// Chain ID of the emitter aka source chain (can be id or name)
    chain_id: CooChain,
    /// Emitter contract address or emitter type
    emitter: EmitterType,
    /// Sequence number of the VAA
    sequence: u64,
}

#[derive(Debug, Args)]
struct VaaDecodeArgs {
    #[arg(value_enum, short, long, default_value_t = VaaDataFormat::Base64)]
    /// VAA data format
    data_format: VaaDataFormat,
    #[arg(value_enum, short, long, default_value_t = PayloadType::SmartInfer)]
    /// Specifies the payload type for the VAA. If not specified, the payload type will be inferred from the VAA.
    payload_type: PayloadType,
    /// Input (VAA data or path)
    data: String,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum VaaDataFormat{
    Base64,
    Base58,
    Hex,
    Path,
}

fn main() {
    let cli = Cli::parse();

    let app_path = match cli.app_path {
        Some(v) => v,
        None => DEFAULT_APP_PATH.clone(),
    };

    create_config_dir(&app_path);

    match cli.command {
        Some(CooCommand::Vaa(vaa_args)) => {
            match vaa_args.vaa_command {
                Some(VaaCommand::Query(vaa_query_args)) => {
                    cli_vaa_query(vaa_query_args, &app_path);
                },
                Some(VaaCommand::Decode(vaa_decode_args)) => {
                    cli_vaa_decode(vaa_decode_args, &app_path);
                },
                Some(VaaCommand::List) => {
                    cli_vaa_list(&app_path);
                },
                None => {
                    println!("No VAA command specified");
                }
            }
        },
        None => {
            println!("No command specified");
        }
    }
}

fn create_config_dir(app_path: &Path) {
    let config_path = app_path.join("config");
    let cache_path = app_path.join("cache");
    std::fs::create_dir_all(app_path).unwrap();
    std::fs::create_dir_all(&config_path).unwrap();
    std::fs::create_dir_all(&cache_path).unwrap();
}

fn cli_vaa_list(app_path: &Path) {
    let cache_path = app_path.join("cache");
    let vaa_files = std::fs::read_dir(&cache_path).unwrap();
    let mut vaa_files: Vec<_> = vaa_files.map(|f| f.unwrap()).collect();
    vaa_files.sort_by(|a, b| b.path().cmp(&a.path()));
    for (index, vaa_file) in vaa_files.iter().enumerate() {
        let vaa_filename = vaa_file.path().file_stem().unwrap().to_string_lossy().to_string();
        let vaa_filename_parts: Vec<_> = vaa_filename.split("-").collect();
        let chain_id = vaa_filename_parts[0].parse::<u16>().unwrap();
        let emitter = vaa_filename_parts[1].to_string();
        let sequence = vaa_filename_parts[2].parse::<u64>().unwrap();
        println!("{: <3}: {} {} {}", index, chain_id, emitter, sequence);
    }
}

fn cli_vaa_query(vaa_query_args: VaaQueryArgs, app_path: &Path) {
    let guardian_url = url::Url::from_str(&vaa_query_args.guardian_url_str).unwrap();
    let sequence = vaa_query_args.sequence;
    let chain = vaa_query_args.chain_id;
    let emitter = vaa_query_args.emitter;

    let vaa_bytes = query_guardian(chain, emitter, sequence, guardian_url).unwrap();
    // save vaa_bytes to a file in cache
    let emitter_address = resolve_emitter_address(chain, emitter).unwrap();
    let vaa_filename = format!("{}-{}-{}.vaa", u16::from(chain), emitter_address, sequence);
    let cache_path = app_path.join("cache").join(vaa_filename);
    let mut file = std::fs::File::create(&cache_path).unwrap();    
    file.write_all(&vaa_bytes).unwrap();
    println!("saved {} bytes to {:?}", vaa_bytes.len(), cache_path);
    println!("vaa data: {}", hex::encode(&vaa_bytes));
}

fn cli_vaa_decode(vaa_decode_args: VaaDecodeArgs, app_path: &Path) {
    let data_format = vaa_decode_args.data_format;
    let data = vaa_decode_args.data;
    let vaa_bytes = match data_format {
        VaaDataFormat::Base64 => {
            base64tobytes(&data).unwrap()
        },
        VaaDataFormat::Base58 => {
            base58tobytes(&data).unwrap()
        },
        VaaDataFormat::Hex => {
            hextobytes(&data).unwrap()
        },
        VaaDataFormat::Path => {
            // checks if data is an absolute path
            let path = if Path::new(&data).is_absolute() {
                PathBuf::from(&data)
            } else {
                app_path.join("cache").join(&data)
            };
            std::fs::read(&path).unwrap()
        },
    };
    let vaa = parse_vaa(&vaa_bytes).unwrap();
    println!("{}", pretty_vaa(&vaa));
    // we'll deal with the payload here
    let payload = vaa.payload;

    // if its SmartInfer, we'll perform the inference first before doing the decoding
    let payload_type = match vaa_decode_args.payload_type {
        PayloadType::SmartInfer => {
            // we'll first check out what is the emitter address, and from there we will know if it is one of the known contracts
            // if it is, we'll decode the payload accordingly

            let emitter_address = vaa.emitter_address.to_string().to_lowercase();
            // emitter_address is a 0 left-padded hex string in lower case.
            // we'll perform the needed transformation from the map
            let key = EMITTERS.iter().find_map(|(k, v)| { 
                    let map_entry = format!("{:0>64}", v).to_lowercase();
                    if emitter_address == map_entry {
                        Some(k)
                    } else {
                        None
                    }
            });
            match key {
                Some((_, emitter)) => {
                    match emitter {
                        EmitterType::Unset => unreachable!("unset should not be in the map"),
                        EmitterType::Address(_) => unreachable!("address should not be in the map"),
                        EmitterType::TokenBridge => {
                            // we'll check the payload type from the first byte
                            let payload_type = payload[0];
                            match payload_type {
                                0x01 => PayloadType::WormholeTokenTransfer,
                                0x02 => PayloadType::WormholeAssetMeta,
                                0x03 => PayloadType::WormholeTokenTransferPayload,
                                // we're not really sure what this is, so raw bytes it shall be.
                                _ => PayloadType::RawBytes
                            }
                        },
                        EmitterType::NftBridge => {
                            // we'll check the payload type from the first byte
                            let payload_type = payload[0];
                            match payload_type {
                                0x01 => PayloadType::WormholeNftTransfer,
                                // we're not really sure what this is, so raw bytes it shall be.
                                _ => PayloadType::RawBytes,
                            }
                        }
                        // currently corebridge have governance stuff, so we'll just leave it as raw bytes
                        EmitterType::CoreBridge => PayloadType::RawBytes, 
                    }
                },
                // not one of the known emitters, so raw bytes it shall be.
                None => PayloadType::RawBytes,
            }
        },
        v => v,
    };

    let payload = match payload_type {
        PayloadType::SmartInfer => unreachable!("smart infer should have been handled above"),
        PayloadType::RawBytes => {
            PayloadResponse::RawBytes(payload.to_vec())
        },
        PayloadType::WormholeTokenTransfer => {
            let message = decode_wormhole_token(&vaa).unwrap();
            PayloadResponse::WormholeTokenTransfer(message)
        },
        PayloadType::WormholeAssetMeta => {
            let message = decode_wormhole_token(&vaa).unwrap();
            PayloadResponse::WormholeAssetMeta(message)
        },
        PayloadType::WormholeTokenTransferPayload => {
            let message = decode_wormhole_token(&vaa).unwrap();
            PayloadResponse::WormholeTokenTransferPayload(message)
        },
        PayloadType::WormholeNftTransfer => {
            let message = decode_wormhole_nft(&vaa).unwrap();
            PayloadResponse::WormholeNftTransfer(message)
        },
    };
    println!("{}", payload);
}
