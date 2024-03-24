// Modern, minimalistic & standard-compliant cold wallet library.
//
// SPDX-License-Identifier: Apache-2.0
//
// Written in 2020-2023 by
//     Dr Maxim Orlovsky <orlovsky@lnp-bp.org>
//
// Copyright (C) 2020-2023 LNP/BP Standards Association. All rights reserved.
// Copyright (C) 2020-2023 Dr Maxim Orlovsky. All rights reserved.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::collections::BTreeMap;
use std::num::NonZeroU32;

use bpstd::{Address, LockTime, Outpoint, SeqNo, Witness};
use descriptors::Descriptor;
use esplora::{BlockingClient, Error};

use super::BATCH_SIZE;
use crate::{
    Indexer, Layer2, MayError, MiningInfo, Party, TxCredit, TxDebit, TxStatus, WalletAddr,
    WalletCache, WalletDescr, WalletTx,
};

impl From<esplora::TxStatus> for TxStatus {
    fn from(status: esplora::TxStatus) -> Self {
        if let esplora::TxStatus {
            confirmed: true,
            block_height: Some(height),
            block_hash: Some(hash),
            block_time: Some(ts),
        } = status
        {
            TxStatus::Mined(MiningInfo {
                height: NonZeroU32::try_from(height).unwrap_or(NonZeroU32::MIN),
                time: ts,
                block_hash: hash,
            })
        } else {
            TxStatus::Mempool
        }
    }
}

impl From<esplora::PrevOut> for Party {
    fn from(prevout: esplora::PrevOut) -> Self { Party::Unknown(prevout.scriptpubkey) }
}

impl From<esplora::Vin> for TxCredit {
    fn from(vin: esplora::Vin) -> Self {
        TxCredit {
            outpoint: Outpoint::new(vin.txid, vin.vout),
            sequence: SeqNo::from_consensus_u32(vin.sequence),
            coinbase: vin.is_coinbase,
            script_sig: vin.scriptsig,
            witness: Witness::from_consensus_stack(vin.witness),
            value: vin.prevout.as_ref().map(|prevout| prevout.value).unwrap_or_default().into(),
            payer: vin.prevout.map(Party::from).unwrap_or(Party::Subsidy),
        }
    }
}

impl From<esplora::Tx> for WalletTx {
    fn from(tx: esplora::Tx) -> Self {
        WalletTx {
            txid: tx.txid,
            status: tx.status.into(),
            inputs: tx.vin.into_iter().map(TxCredit::from).collect(),
            outputs: tx
                .vout
                .into_iter()
                .enumerate()
                .map(|(n, vout)| TxDebit {
                    outpoint: Outpoint::new(tx.txid, n as u32),
                    beneficiary: Party::from(vout.scriptpubkey),
                    value: vout.value.into(),
                    spent: None,
                })
                .collect(),
            fee: tx.fee.into(),
            size: tx.size,
            weight: tx.weight,
            version: tx.version,
            locktime: LockTime::from_consensus_u32(tx.locktime),
        }
    }
}

impl Indexer for BlockingClient {
    type Error = Error;

    fn create<K, D: Descriptor<K>, L2: Layer2>(
        &self,
        descriptor: &WalletDescr<K, D, L2::Descr>,
    ) -> MayError<WalletCache<L2::Cache>, Vec<Self::Error>> {
        let mut cache = WalletCache::new();
        let mut errors = vec![];

        let mut address_index = BTreeMap::new();
        for keychain in descriptor.keychains() {
            let mut empty_count = 0usize;
            eprint!(" keychain {keychain} ");
            for derive in descriptor.addresses(keychain) {
                let script = derive.addr.script_pubkey();

                eprint!(".");
                let mut txids = Vec::new();
                match self.scripthash_txs(&script, None) {
                    Err(err) => {
                        errors.push(err);
                        break;
                    }
                    Ok(txes) if txes.is_empty() => {
                        empty_count += 1;
                        if empty_count >= BATCH_SIZE as usize {
                            break;
                        }
                    }
                    Ok(txes) => {
                        empty_count = 0;
                        txids = txes.iter().map(|tx| tx.txid).collect();
                        cache
                            .tx
                            .extend(txes.into_iter().map(WalletTx::from).map(|tx| (tx.txid, tx)));
                    }
                }

                let wallet_addr = WalletAddr::<i64>::from(derive);
                address_index.insert(script, (wallet_addr, txids));
            }
        }

        // TODO: Update headers & tip

        for (script, (wallet_addr, txids)) in &mut address_index {
            for txid in txids {
                let mut tx = cache.tx.remove(txid).expect("broken logic");
                for debit in &mut tx.outputs {
                    let Some(s) = debit.beneficiary.script_pubkey() else {
                        continue;
                    };
                    if &s == script {
                        cache.utxo.insert(debit.outpoint);
                        debit.beneficiary = Party::from_wallet_addr(wallet_addr);
                        wallet_addr.used = wallet_addr.used.saturating_add(1);
                        wallet_addr.volume.saturating_add_assign(debit.value);
                        wallet_addr.balance = wallet_addr
                            .balance
                            .saturating_add(debit.value.sats().try_into().expect("sats overflow"));
                    } else if debit.beneficiary.is_unknown() {
                        Address::with(&s, descriptor.network())
                            .map(|addr| {
                                debit.beneficiary = Party::Counterparty(addr);
                            })
                            .ok();
                    }
                }
                cache.tx.insert(tx.txid, tx);
            }
        }

        for (script, (wallet_addr, txids)) in &mut address_index {
            for txid in txids {
                let mut tx = cache.tx.remove(txid).expect("broken logic");
                for credit in &mut tx.inputs {
                    let Some(s) = credit.payer.script_pubkey() else {
                        continue;
                    };
                    if &s == script {
                        credit.payer = Party::from_wallet_addr(wallet_addr);
                        wallet_addr.balance = wallet_addr
                            .balance
                            .saturating_sub(credit.value.sats().try_into().expect("sats overflow"));
                    } else if credit.payer.is_unknown() {
                        Address::with(&s, descriptor.network())
                            .map(|addr| {
                                credit.payer = Party::Counterparty(addr);
                            })
                            .ok();
                    }
                    if let Some(prev_tx) = cache.tx.get_mut(&credit.outpoint.txid) {
                        if let Some(txout) =
                            prev_tx.outputs.get_mut(credit.outpoint.vout_u32() as usize)
                        {
                            let outpoint = txout.outpoint;
                            cache.utxo.remove(&outpoint);
                            txout.spent = Some(credit.outpoint.into())
                        };
                    }
                }
                cache.tx.insert(tx.txid, tx);
            }
            cache
                .addr
                .entry(wallet_addr.terminal.keychain)
                .or_default()
                .insert(wallet_addr.expect_transmute());
        }

        if errors.is_empty() {
            MayError::ok(cache)
        } else {
            MayError::err(cache, errors)
        }
    }

    fn update<K, D: Descriptor<K>, L2: Layer2>(
        &self,
        _descr: &WalletDescr<K, D, L2::Descr>,
        _cache: &mut WalletCache<L2::Cache>,
    ) -> MayError<usize, Vec<Self::Error>> {
        todo!()
    }
}
