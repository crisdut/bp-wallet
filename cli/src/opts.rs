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

use std::fmt::Debug;
use std::path::{Path, PathBuf};

use bpstd::{Network, XpubDerivable};
use clap::ValueHint;
use descriptors::{Descriptor, StdDescr, TrKey, Wpkh};
use strict_encoding::Ident;

pub const DATA_DIR_ENV: &str = "LNPBP_DATA_DIR";
#[cfg(any(target_os = "linux"))]
pub const DATA_DIR: &str = "~/.lnp-bp";
#[cfg(any(target_os = "freebsd", target_os = "openbsd", target_os = "netbsd"))]
pub const DATA_DIR: &str = "~/.lnp-bp";
#[cfg(target_os = "macos")]
pub const DATA_DIR: &str = "~/Library/Application Support/LNP-BP Suite";
#[cfg(target_os = "windows")]
pub const DATA_DIR: &str = "~\\AppData\\Local\\LNP-BP Suite";
#[cfg(target_os = "ios")]
pub const DATA_DIR: &str = "~/Documents";
#[cfg(target_os = "android")]
pub const DATA_DIR: &str = ".";

pub const DEFAULT_ELECTRUM: &str = "example.com:50001";
pub const DEFAULT_ESPLORA: &str = "https://blockstream.info/testnet/api";

#[derive(Args, Clone, PartialEq, Eq, Debug)]
pub struct ResolverOpt {
    /// Electrum server to use.
    #[arg(
        conflicts_with = "esplora",
        long,
        global = true,
        default_value = DEFAULT_ELECTRUM,
        env = "ELECRTUM_SERVER",
        value_hint = ValueHint::Url,
        value_name = "URL"
    )]
    pub electrum: String,

    /// Esplora server to use.
    #[arg(
        conflicts_with = "electrum",
        long,
        global = true,
        default_value = DEFAULT_ESPLORA,
        env = "ESPLORA_SERVER",
        value_hint = ValueHint::Url,
        value_name = "URL"
    )]
    pub esplora: String,

    #[clap(long, global = true)]
    pub sync: bool,
}

pub trait DescriptorOpts: clap::Args + Clone + Eq + Debug {
    type Descr: Descriptor + serde::Serialize + for<'de> serde::Deserialize<'de>;
    fn is_some(&self) -> bool;
    fn descriptor(&self) -> Option<Self::Descr>;
}

#[derive(Args, Clone, PartialEq, Eq, Debug)]
#[group(multiple = false)]
pub struct DescrStdOpts {
    /// Use wpkh(KEY) descriptor as wallet
    #[arg(long, global = true)]
    pub wpkh: Option<XpubDerivable>,

    /// Use tr(KEY) descriptor as wallet
    #[arg(long, global = true)]
    pub tr_key_only: Option<XpubDerivable>,
}

impl DescriptorOpts for DescrStdOpts {
    type Descr = StdDescr;

    fn is_some(&self) -> bool { self.tr_key_only.is_some() | self.wpkh.is_some() }
    fn descriptor(&self) -> Option<Self::Descr> {
        if let Some(ref x) = self.tr_key_only {
            Some(TrKey::from(x.clone()).into())
        } else if let Some(ref x) = self.wpkh {
            Some(Wpkh::from(x.clone()).into())
        } else {
            None
        }
    }
}

#[derive(Args, Clone, PartialEq, Eq, Debug)]
#[group(multiple = false)]
pub struct WalletOpts<O: DescriptorOpts = DescrStdOpts> {
    #[arg(short = 'w', long = "wallet", global = true)]
    pub name: Option<Ident>,

    /// Path to wallet directory.
    #[arg(
        short = 'W',
        long,
        global = true,
        value_hint = ValueHint::DirPath,
    )]
    pub wallet_path: Option<PathBuf>,

    #[clap(flatten)]
    pub descriptor_opts: O,
}

#[derive(Args, Clone, PartialEq, Eq, Debug)]
pub struct GeneralOpts {
    /// Data directory path.
    ///
    /// Path to the directory that contains RGB stored data.
    #[arg(
        short,
        long,
        global = true,
        default_value = DATA_DIR,
        env = DATA_DIR_ENV,
        value_hint = ValueHint::DirPath
    )]
    pub data_dir: PathBuf,

    /// Network to use.
    #[arg(short, long, global = true, default_value = "testnet", env = "LNPBP_NETWORK")]
    pub network: Network,
}

impl GeneralOpts {
    pub fn process(&mut self) {
        self.data_dir =
            PathBuf::from(shellexpand::tilde(&self.data_dir.display().to_string()).to_string());
    }

    pub fn base_dir(&self) -> PathBuf {
        let mut dir = self.data_dir.clone();
        dir.push(self.network.to_string());
        dir
    }

    pub fn wallet_dir(&self, wallet_name: impl AsRef<Path>) -> PathBuf {
        let mut dir = self.base_dir();
        dir.push(wallet_name);
        dir
    }
}
