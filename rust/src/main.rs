#![allow(unused)]
use bitcoincore_rpc::bitcoin::Amount;
use bitcoincore_rpc::{Auth, Client, RpcApi};
use serde::Deserialize;
use serde_json::json;
use std::fs::File;
use std::io::Write;

const RPC_BASE: &str = "http://127.0.0.1:18443";
const RPC_USER: &str = "alice";
const RPC_PASS: &str = "password";

fn main() -> bitcoincore_rpc::Result<()> {
    let auth = Auth::UserPass(RPC_USER.into(), RPC_PASS.into());

    // Clients for each wallet
    let miner_rpc = Client::new(&format!("{}/wallet/Miner", RPC_BASE), auth.clone())?;
    let trader_rpc = Client::new(&format!("{}/wallet/Trader", RPC_BASE), auth.clone())?;

    // Generate miner address and mine 101 blocks
    let miner_address = miner_rpc
        .get_new_address(Some("mining"), None)?
        .assume_checked();
    miner_rpc.generate_to_address(101, &miner_address)?;

    // Generate trader address
    let trader_address = trader_rpc
        .get_new_address(Some("receiving"), None)?
        .assume_checked();

    // Send 20 BTC from miner to trader
    let txid = miner_rpc.send_to_address(
        &trader_address,
        Amount::from_btc(20.0).unwrap(),
        None,
        None,
        None,
        None,
        None,
        None,
    )?;

    // Mine 1 block to confirm tx
    miner_rpc.generate_to_address(1, &miner_address)?;

    // Get transaction and decode
    let tx = miner_rpc.get_transaction(&txid, Some(true))?;
    let decoded = miner_rpc.decode_raw_transaction(&tx.hex, None)?;

    // Get input details
    let input = &decoded.vin[0];
    let input_txid = input.txid.expect("Missing input txid");
    let input_vout = input.vout.expect("Missing input vout");
    let input_tx = miner_rpc.get_raw_transaction_info(&input_txid, None)?;
    let input_vout_info = &input_tx.vout[input_vout as usize];
    let input_address = input_vout_info
        .script_pub_key
        .address
        .clone()
        .unwrap()
        .assume_checked();
    let input_amount = input_vout_info.value.to_btc();

    // Parse vouts for trader and change
    let mut trader_output_address = String::new();
    let mut trader_output_amount = 0.0;
    let mut change_address = String::new();
    let mut change_amount = 0.0;

    for vout in &decoded.vout {
        if let Some(addr) = &vout.script_pub_key.address {
            let addr_checked = addr.clone().assume_checked();
            let value = vout.value.to_btc();

            if addr_checked == trader_address {
                trader_output_address = addr_checked.to_string();
                trader_output_amount = value;
            } else {
                change_address = addr_checked.to_string();
                change_amount = value;
            }
        }
    }

    // Calculate fee
    let fee = input_amount - (trader_output_amount + change_amount);

    // Get block info
    let blockhash = tx.info.blockhash.expect("Missing blockhash");
    let block = miner_rpc.get_block_info(&blockhash)?; // Can use miner_rpc here safely
    let block_height = block.height;

    // Write to out.txt
    let mut file = File::create("out.txt")?;
    writeln!(file, "{txid}")?;
    writeln!(file, "{input_address}")?;
    writeln!(file, "{input_amount}")?;
    writeln!(file, "{trader_output_address}")?;
    writeln!(file, "{trader_output_amount}")?;
    writeln!(file, "{change_address}")?;
    writeln!(file, "{change_amount}")?;
    writeln!(file, "{fee:.8}")?;
    writeln!(file, "{block_height}")?;
    writeln!(file, "{blockhash}")?;

    println!("✅ out.txt written");
    Ok(())
}
