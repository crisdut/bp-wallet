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

#[macro_use]
extern crate amplify;
#[macro_use]
extern crate serde_crate as serde;

mod indexers;
mod runtime;
mod util;
mod chain;
mod wallet;

pub use chain::{
    AddrInfo, BlockHeight, BlockInfo, MiningInfo, TxInInfo, TxInfo, TxOutInfo, TxStatus, UtxoInfo,
};
pub use indexers::Indexer;
pub use runtime::{LoadError, Runtime};
pub use util::MayError;
pub use wallet::{Wallet, WalletCache, WalletData, WalletDescr};