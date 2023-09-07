// SPDX-License-Identifier: CC0-1.0

//! Sign a transaction that spends an p2wpkh unspent output.

use std::str::FromStr;
use hex;
use bitcoin::hashes::Hash;
use bitcoin::locktime::absolute;
use bitcoin::secp256k1::{rand, Message, Secp256k1, SecretKey, Signing};
use bitcoin::sighash::{EcdsaSighashType, SighashCache};
use bitcoin::{
    Address, Network, OutPoint, ScriptBuf, Sequence, Transaction, TxIn, TxOut, Txid, WPubkeyHash,
    Witness
};

const DUMMY_UTXO_AMOUNT: u64 = 20_000_000;
const SPEND_AMOUNT: u64 = 5_000_000;
const CHANGE_AMOUNT: u64 = 14_999_000; // 1000 sat fee.

fn main() {
    // We need a signing secp256k1 context, if you have not seen this before just pass it in when
    // needed and otherwise ignore it.
    let secp = Secp256k1::new();

    // Get a secret key we control and the pubkeyhash of the associated pubkey.
    // In a real application these would come from a stored secret.
    let (sk, wpkh) = senders_keys(&secp);

    // Get an address to send to.
    let address = receivers_address();

    // Get an unspent output that is locked to the key above that we control.
    // In a real application these would come from the chain.
    let (dummy_out_point, dummy_utxo) = dummy_unspent_transaction_output(&wpkh);

    // The script code required to spend a p2wpkh output.
    let script_code = dummy_utxo
        .script_pubkey
        .p2wpkh_script_code()
        .expect("p2wpkh script pubkey");

    // The input for the transaction we are constructing.
    let input = TxIn {
        previous_output: dummy_out_point,
        sequence: Sequence::max_value(),
        witness: Witness::default(),
        script_sig: ScriptBuf::new(),
    };

    // The spend output is locked to a key controlled by the receiver.
    let spend = TxOut {
        value: SPEND_AMOUNT,
        script_pubkey: address.script_pubkey(),
    };

    // The change output is locked to a key controlled by us.
    let change = TxOut {
        value: CHANGE_AMOUNT,
        script_pubkey: ScriptBuf::new_v0_p2wpkh(&wpkh),
    };

    // The transaction we want to sign and broadcast.
    let mut unsigned_tx = Transaction {
        version: 2,
        lock_time: absolute::LockTime::ZERO,
        input: vec![input],
        output: vec![spend, change],
    };

    //
    // TODO: Sign the unsigned transaction.
    //

    let sighash_type = EcdsaSighashType::All;

    let mut sighasher = SighashCache::new(&mut unsigned_tx);

    let input_index = 0;

    let sighash = sighasher
        .segwit_signature_hash(input_index, &script_code, SPEND_AMOUNT, sighash_type)
        .expect("failed to construct sighash");

    // Sign the sighash using the secp256k1 library (exported by rust-bitcoin).
    let msg = Message::from(sighash);
    let sig = secp.sign_ecdsa(&msg, &sk);

    // Update the witness stack.
    sighasher
        .witness_mut(input_index)
        .unwrap()
        .push_bitcoin_signature(&sig.serialize_der(), sighash_type);

    // Get the signed transaction.
    let tx = sighasher.into_transaction();

    // BOOM! Transaction signed and ready to broadcast.
    println!("{:#?}", tx);

    // Encode the transaction in hex and print it.
    let raw_tx_bytes = bitcoin::consensus::encode::serialize(&tx);
    let raw_tx_hex = hex::encode(raw_tx_bytes);
    println!("Raw Transaction (Hex): {}", raw_tx_hex);
    
}

/// An example of keys controlled by the transaction sender.
///
/// In a real application these would be actual secrets.
fn senders_keys<C: Signing>(secp: &Secp256k1<C>) -> (SecretKey, WPubkeyHash) {
    let sk = SecretKey::new(&mut rand::thread_rng());
    let pk = bitcoin::PublicKey::new(sk.public_key(secp));
    let wpkh = pk.wpubkey_hash().expect("key is compressed");

    (sk, wpkh)
}

/// A dummy address for the receiver.
///
/// We lock the spend output to the key associated with this address.
///
/// (FWIW this is an arbitrary mainnet address.)
fn receivers_address() -> Address {
    Address::from_str("bc1q7cyrfmck2ffu2ud3rn5l5a8yv6f0chkp0zpemf")
        .expect("a valid address")
        .require_network(Network::Bitcoin)
        .expect("valid address for mainnet")
}

/// Creates a p2wpkh output locked to the key associated with `wpkh`.
///
/// An utxo is described by the `OutPoint` (txid and index within the transaction that it was
/// created). Using the out point one can get the transaction by `txid` and using the `vout` get the
/// transaction value and script pubkey (`TxOut`) of the utxo.
///
/// This output is locked to keys that we control, in a real application this would be a valid
/// output taken from a transaction that appears in the chain.
fn dummy_unspent_transaction_output(wpkh: &WPubkeyHash) -> (OutPoint, TxOut) {
    let script_pubkey = ScriptBuf::new_v0_p2wpkh(wpkh);

    let out_point = OutPoint {
        txid: Txid::all_zeros(), // Obviously invalid.
        vout: 0,
    };

    let utxo = TxOut {
        value: DUMMY_UTXO_AMOUNT,
        script_pubkey,
    };

    (out_point, utxo)
}
