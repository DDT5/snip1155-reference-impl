use super::*;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{
    Storage, Api, Uint128, HumanAddr, CanonicalAddr, BlockInfo, 
    StdResult,
    ReadonlyStorage,
};

use cosmwasm_storage::{
    PrefixedStorage, ReadonlyPrefixedStorage, 
};

use secret_toolkit::{
    storage::{AppendStore, AppendStoreMut},  
};

use crate::state::save_load_functions::{json_save, json_load};


/////////////////////////////////////////////////////////////////////////////////
// Transaction history
/////////////////////////////////////////////////////////////////////////////////

/// Returns StdResult<(Vec<Tx>, u64)> of the txs to display and the total count of txs
///
/// # Arguments
///
/// * `api` - a reference to the Api used to convert human and canonical addresses
/// * `storage` - a reference to the contract's storage
/// * `address` - a reference to the address whose txs to display
/// * `page` - page to start displaying
/// * `page_size` - number of txs per page
pub fn get_txs<S: ReadonlyStorage, A: Api>( 
    api: &A,
    storage: &S,
    address: &CanonicalAddr,
    page: u32,
    page_size: u32,
) -> StdResult<(Vec<Tx>, u64)> {
    let id_store =
        ReadonlyPrefixedStorage::multilevel(&[PREFIX_TX_IDS, address.as_slice()], storage);

    // Try to access the storage of tx ids for the account.
    // If it doesn't exist yet, return an empty list of txs.
    let id_store = if let Some(result) = AppendStore::<u64, _>::attach(&id_store) {
        result?
    } else {
        return Ok((vec![], 0));
    };
    let count = id_store.len() as u64;
    // access tx storage
    let tx_store = ReadonlyPrefixedStorage::new(PREFIX_TXS, storage);
    // Take `page_size` txs starting from the latest tx, potentially skipping `page * page_size`
    // txs from the start.
    let txs: StdResult<Vec<Tx>> = id_store
        .iter()
        .rev()
        .skip((page * page_size) as usize)
        .take(page_size as usize)
        .map(|id| {
            id.map(|id| {
                json_load(&tx_store, &id.to_le_bytes())
                    .and_then(|tx: StoredTx| tx.into_humanized(api))
            })
            .and_then(|x| x)
        })
        .collect();

    txs.map(|t| (t, count))
}

#[allow(clippy::too_many_arguments)]
pub fn store_transfer<S: Storage>(
    storage: &mut S,
    config: &mut ContractConfig,
    block: &BlockInfo,
    token_id: &str,
    from: CanonicalAddr,
    sender: Option<CanonicalAddr>,
    recipient: CanonicalAddr,
    amount: Uint128,
    memo: Option<String>,
) -> StdResult<()> {
    let action = StoredTxAction::Transfer {
        from,
        sender,
        recipient,
        amount,
    };
    let tx = StoredTx {
        tx_id: config.tx_cnt,
        block_height: block.height,
        block_time: block.time,
        token_id: token_id.to_string(),
        action,
        memo,
    };
    let mut tx_store = PrefixedStorage::new(PREFIX_TXS, storage);
    json_save(&mut tx_store, &config.tx_cnt.to_le_bytes(), &tx)?;
    if let StoredTxAction::Transfer {
        from,
        sender,
        recipient,
        amount: _,
    } = tx.action
    {
        append_tx_for_addr(storage, config.tx_cnt, &from)?;
        append_tx_for_addr(storage, config.tx_cnt, &recipient)?;
        if let Some(sndr) = sender.as_ref() {
            if *sndr != recipient {
                append_tx_for_addr(storage, config.tx_cnt, sndr)?;
            }
        }
    }
    config.tx_cnt += 1;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub fn store_mint<S: Storage>(
    storage: &mut S,
    config: &mut ContractConfig,
    block: &BlockInfo,
    token_id: &str,
    minter: CanonicalAddr,
    recipient: CanonicalAddr,
    amount: Uint128,
    memo: Option<String>,
) -> StdResult<()> {
    let action = StoredTxAction::Mint { minter, recipient, amount };
    let tx = StoredTx {
        tx_id: config.tx_cnt,
        block_height: block.height,
        block_time: block.time,
        token_id: token_id.to_string(),
        action,
        memo,
    };
    let mut tx_store = PrefixedStorage::new(PREFIX_TXS, storage);
    json_save(&mut tx_store, &config.tx_cnt.to_le_bytes(), &tx)?;
    if let StoredTxAction::Mint { minter, recipient, amount: _ } = tx.action {
        append_tx_for_addr(storage, config.tx_cnt, &recipient)?;
        if recipient != minter {
            append_tx_for_addr(storage, config.tx_cnt, &minter)?;
        }
    }
    config.tx_cnt += 1;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub fn store_burn<S: Storage>(
    storage: &mut S,
    config: &mut ContractConfig,
    block: &BlockInfo,
    token_id: &str,
    burner: Option<CanonicalAddr>,
    owner: CanonicalAddr,
    amount: Uint128,
    memo: Option<String>,
) -> StdResult<()> {
    let action = StoredTxAction::Burn { burner, owner, amount };
    let tx = StoredTx {
        tx_id: config.tx_cnt,
        block_height: block.height,
        block_time: block.time,
        token_id: token_id.to_string(),
        action,
        memo,
    };
    let mut tx_store = PrefixedStorage::new(PREFIX_TXS, storage);
    json_save(&mut tx_store, &config.tx_cnt.to_le_bytes(), &tx)?;
    if let StoredTxAction::Burn { burner, owner, amount: _ } = tx.action {
        append_tx_for_addr(storage, config.tx_cnt, &owner)?;
        if let Some(bnr) = burner.as_ref() {
            if bnr != &owner {
                append_tx_for_addr(storage, config.tx_cnt, bnr)?;
            }
        }
    }
    config.tx_cnt += 1;
    Ok(())
}

/// Returns StdResult<()> after saving tx id
///
/// # Arguments
///
/// * `storage` - a mutable reference to the storage this item should go to
/// * `tx_id` - the tx id to store
/// * `address` - a reference to the address for which to store this tx id
fn append_tx_for_addr<S: Storage>(
    storage: &mut S,
    tx_id: u64,
    address: &CanonicalAddr,
) -> StdResult<()> {
    let mut store = PrefixedStorage::multilevel(&[PREFIX_TX_IDS, address.as_slice()], storage);
    let mut store = AppendStoreMut::attach_or_create(&mut store)?;
    store.push(&tx_id)
}


/// tx type and specifics for storage
#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum StoredTxAction {
    Mint {
        minter: CanonicalAddr,
        recipient: CanonicalAddr,
        amount: Uint128,
    },
    Burn {
        /// in the base specification, the burner MUST be the owner. In the additional
        /// specifications, it is OPTIONAL to allow other addresses to burn tokens.
        burner: Option<CanonicalAddr>,
        owner: CanonicalAddr,
        amount: Uint128,
    },
    /// `transfer` or `send` txs
    Transfer {
        /// previous owner
        from: CanonicalAddr,
        /// optional sender if not owner
        sender: Option<CanonicalAddr>,
        /// new owner
        recipient: CanonicalAddr,
        /// amount of tokens transferred
        amount: Uint128,
    },
}

/// tx in storage
#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct StoredTx {
    /// tx id
    pub tx_id: u64,
    /// the block containing this tx
    pub block_height: u64,
    /// the time (in seconds since 01/01/1970) of the block containing this tx
    pub block_time: u64,
    /// token id
    pub token_id: String,
    /// tx type and specifics
    pub action: StoredTxAction,
    /// optional memo
    pub memo: Option<String>,
}

impl StoredTx {
    pub fn into_humanized<A: Api>(self, api: &A) -> StdResult<Tx> {
        let action = match self.action {
            StoredTxAction::Mint { minter, recipient, amount } => {
                TxAction::Mint {
                    minter: api.human_address(&minter)?,
                    recipient: api.human_address(&recipient)?,
                    amount,
                }
            },
            StoredTxAction::Burn { burner, owner, amount } => {
                let bnr = if let Some(b) = burner { 
                    Some(api.human_address(&b)?) 
                } else { None };
                TxAction::Burn { 
                    burner: bnr, 
                    owner: api.human_address(&owner)?, 
                    amount, 
                }
            },
            StoredTxAction::Transfer { from, sender, recipient, amount } => {
                let sdr = if let Some(s) = sender { 
                    Some(api.human_address(&s)?) 
                } else { None };
                TxAction::Transfer { 
                    from: api.human_address(&from)?, 
                    sender: sdr, 
                    recipient: api.human_address(&recipient)?,  
                    amount 
                }
            },
        };
        let tx = Tx {
            tx_id: self.tx_id,
            block_height: self.block_height,
            block_time: self.block_time,
            token_id: self.token_id,
            action,
            memo: self.memo,
        };

        Ok(tx)
    }
}

/// tx type and specifics for storage with HumanAddr
#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum TxAction {
    Mint {
        minter: HumanAddr,
        recipient: HumanAddr,
        amount: Uint128,
    },
    Burn {
        /// in the base specification, the burner MUST be the owner. In the additional
        /// specifications, it is OPTIONAL to allow other addresses to burn tokens.
        burner: Option<HumanAddr>,
        owner: HumanAddr,
        amount: Uint128,
    },
    /// `transfer` or `send` txs
    Transfer {
        /// previous owner
        from: HumanAddr,
        /// optional sender if not owner
        sender: Option<HumanAddr>,
        /// new owner
        recipient: HumanAddr,
        /// amount of tokens transferred
        amount: Uint128,
    },
}

/// tx in storage
#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct Tx {
    /// tx id
    pub tx_id: u64,
    /// the block containing this tx
    pub block_height: u64,
    /// the time (in seconds since 01/01/1970) of the block containing this tx
    pub block_time: u64,
    /// token id
    pub token_id: String,
    /// tx type and specifics
    pub action: TxAction,
    /// optional memo
    pub memo: Option<String>,
}


/////////////////////////////////////////////////////////////////////////////////
// Token transfer history (for NFTs only)
/////////////////////////////////////////////////////////////////////////////////

/// stores ownership history for a given token_id. Meant to be used for nfts.
/// In base specification, only the latest (ie: current) owner is relevant. But  
/// this design pattern is used to allow viewing a token_id's ownership history, 
/// which is allowed in the additional specifications
pub fn append_new_owner<S: Storage>(
    storage: &mut S,
    token_id: &str,
    address: &HumanAddr,
) -> StdResult<()> {
    let mut store = PrefixedStorage::multilevel(&[PREFIX_NFT_OWNER, token_id.as_bytes()], storage);
    let mut store = AppendStoreMut::attach_or_create(&mut store)?;
    store.push(address)
}

pub fn may_get_current_owner<S: Storage>(
    storage: &S,
    token_id: &str,
) -> StdResult<Option<HumanAddr>> {
    let store_op = ReadonlyPrefixedStorage::multilevel(&[PREFIX_NFT_OWNER, token_id.as_bytes()], storage);
    let store_op = AppendStore::<HumanAddr, _, _>::attach(&store_op);
    let store = match store_op {
        Some(i) => i?,
        None => return Ok(None),
    };
    let pos = store.len().saturating_sub(1);
    let current_owner = store.get_at(pos)?;
    Ok(Some(current_owner))
}
