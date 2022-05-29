use std::{
    any::type_name,
    // collections::HashSet,
};

use schemars::JsonSchema;
use serde::{de::DeserializeOwned, Deserialize, Serialize};

use cosmwasm_std::{
    Storage, Api, Uint128, HumanAddr, CanonicalAddr, BlockInfo,  // Extern, Api, Querier,
    StdResult, StdError, //to_binary, 
    ReadonlyStorage, to_binary,  
};
use cosmwasm_storage::{
    PrefixedStorage, ReadonlyPrefixedStorage, 
    bucket, bucket_read, Bucket, ReadonlyBucket,
    singleton, singleton_read, ReadonlySingleton, Singleton, 
};

use secret_toolkit::{
    serialization::{Json, Serde}, //Bincode2, 
    storage::{AppendStore, AppendStoreMut},  
};

use crate::{
    metadata::{Metadata},
    vk::viewing_key::{ViewingKey},
    expiration::{Expiration},
};

#[cfg(test)]
use crate::{
    metadata::{Extension},
};

pub const RESPONSE_BLOCK_SIZE: usize = 256;

// namespaces
pub const CONTR_CONF: &[u8] = b"contrconfig";
pub const TKN_TOTAL_SUPPLY: &[u8] = b"totalsupply";
pub const BALANCES: &[u8] = b"balances";
pub const TKN_INFO: &[u8] = b"tokeninfo";
/// storage key for the BlockInfo when the last handle was executed
pub const BLOCK_KEY: &[u8] = b"blockinfo";

/// prefix for storage of transactions
pub const PREFIX_TXS: &[u8] = b"preftxs";
/// prefix for storage of tx ids
pub const PREFIX_TX_IDS: &[u8] = b"txids";
/// prefix for NFT ownership history
pub const PREFIX_NFT_OWNER: &[u8] = b"nftowner";
/// prefix for storing permissions
pub const PREFIX_PERMISSIONS: &[u8] = b"permissions";
/// prefix for storing permission identifier (ID) for a given address
pub const PREFIX_PERMISSION_ID: &[u8] = b"permid";
pub const PREFIX_VIEW_KEY: &[u8] = b"s1155viewkey";
pub const PREFIX_REVOKED_PERMITS: &str = "revokedperms";
pub const PREFIX_RECEIVERS: &[u8] = b"s1155receivers";



/////////////////////////////////////////////////////////////////////////////////
// Singletons
/////////////////////////////////////////////////////////////////////////////////


/// FtokenContr storage: stores information on this ftokens contract
pub fn contr_conf_w<S: Storage>(storage: &mut S) -> Singleton<S, ContrConf> {
    singleton(storage, CONTR_CONF)
}
pub fn contr_conf_r<S: Storage>(storage: &S) -> ReadonlySingleton<S, ContrConf> {
    singleton_read( storage, CONTR_CONF)
}


/////////////////////////////////////////////////////////////////////////////////
// Buckets
/////////////////////////////////////////////////////////////////////////////////

/// token_id configs. Key is `token_id.as_bytes()`
pub fn tkn_info_w<S: Storage>(storage: &mut S) -> Bucket<S, TknInfo> {
    bucket(TKN_INFO, storage)
}
pub fn tkn_info_r<S: Storage>(storage: &S) -> ReadonlyBucket<S, TknInfo> {
    bucket_read(TKN_INFO, storage)
}

/// total supply of a token_id. Key is `token_id.as_bytes()`
pub fn tkn_tot_supply_w<S: Storage>(storage: &mut S) -> Bucket<S, Uint128> {
    bucket(TKN_TOTAL_SUPPLY, storage)
}
pub fn tkn_tot_supply_r<S: Storage>(storage: &S) -> ReadonlyBucket<S, Uint128> {
    bucket_read(TKN_TOTAL_SUPPLY, storage)
}

/////////////////////////////////////////////////////////////////////////////////
// Multi-level Buckets
/////////////////////////////////////////////////////////////////////////////////

/// Multilevel bucket to store balances for each token_id & addr combination. Key intended to 
/// be [`token_id`, `owner`: to_binary(&HumanAddr)?.as_slice()]  
/// When using `balances_w` make sure to also check if need to change `current owner` of an nft and `total_supply` 
pub fn balances_w<'a, 'b, S: Storage>(
    storage: &'a mut S,
    token_id: &'b str
) -> Bucket<'a, S, Uint128> {
    Bucket::multilevel(&[BALANCES, token_id.as_bytes()], storage)
}
pub fn balances_r<'a, 'b, S: Storage>(
    storage: &'a S,
    token_id: &'b str
) -> ReadonlyBucket<'a, S, Uint128> {
    ReadonlyBucket::multilevel(&[BALANCES, token_id.as_bytes()], storage)
}

/// private functions.
/// To store permission. key intended to be [`owner`, `token_id`, `allowed_addr`]
/// `allowed_addr` is `to_binary(&HumanAddr)?.as_slice()` 
fn permission_w<'a, S: Storage>(
    storage: &'a mut S,
    owner: &'a HumanAddr,
    token_id: &'a str,
) -> Bucket<'a, S, Permission> {
    let owner_bin = to_binary(owner).unwrap();
    Bucket::multilevel(&[PREFIX_PERMISSIONS, owner_bin.as_slice(), token_id.as_bytes()], storage)
}
fn permission_r<'a, S: Storage>(
    storage: &'a S,
    owner: &'a HumanAddr,
    token_id: &'a str,
) -> ReadonlyBucket<'a, S, Permission> {
    let owner_bin = to_binary(owner).unwrap();
    ReadonlyBucket::multilevel(&[PREFIX_PERMISSIONS, owner_bin.as_slice(), token_id.as_bytes()], storage)
}
#[cfg(test)]
pub fn perm_r<'a, S: Storage>(
    storage: &'a S,
    owner: &'a HumanAddr,
    token_id: &'a str,
) -> ReadonlyBucket<'a, S, Permission> {
    let owner_bin = to_binary(owner).unwrap();
    ReadonlyBucket::multilevel(&[PREFIX_PERMISSIONS, owner_bin.as_slice(), token_id.as_bytes()], storage)
}

/////////////////////////////////////////////////////////////////////////////////
// Contract and Token Id configs
/////////////////////////////////////////////////////////////////////////////////

/// contract configuration, spanning all `token_ids` generated by this contract
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ContrConf {
    pub admin: Option<HumanAddr>,
    pub minters: Vec<HumanAddr>,
    pub token_id_list: Vec<String>,
    pub tx_cnt: u64,
    pub prng_seed: Vec<u8>,
    pub contract_address: HumanAddr,
}

/// information for a specific `token_id`
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct TknInfo {
    pub token_id: String,
    pub name: String,
    pub symbol: String,
    pub token_config: TknConf,
    pub public_metadata: Option<Metadata>,
    /// private metadata can only be set for nfts in the base specification. It is OPTIONAL in 
    /// additional specifications to allow fungible tokens to have private metadata
    pub private_metadata: Option<Metadata>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum TknConf {
    /// no `owner_may_update_metadata`because there can be multiple owners
    Fungible {
        /// applications should ignore decimals if `is_nft` == true. Decimals play no part in the
        /// contract logic of the base specification of SNIP1155, as there are no `deposit` and 
        /// `redeem` features as seen in SNIP20
        decimals: u8,
        public_total_supply: bool,
        enable_mint: bool,
        enable_burn: bool,
        minter_may_update_metadata: bool,
    },
    /// no `enable_mint` option because NFT can be minted only once using `MintTokenIds`
    Nft {
        /// total supply can be zero if the token has been burnt
        public_total_supply: bool,
        owner_is_public: bool,
        enable_burn: bool,
        minter_may_update_metadata: bool,
        owner_may_update_metadata: bool,
    }
}

impl TknConf {
    /// Combines variables in the TknConf enum into a single struct for easier handling in contract logic.
    pub fn flatten(&self) -> TknConfFlat {
        match self {
            TknConf::Fungible { 
                decimals, 
                public_total_supply, 
                enable_mint, 
                enable_burn, 
                minter_may_update_metadata 
            } => {
                TknConfFlat {
                    is_nft: false,
                    decimals: *decimals,
                    public_total_supply: *public_total_supply,
                    owner_is_public: false,
                    enable_mint: *enable_mint,
                    enable_burn: *enable_burn,
                    minter_may_update_metadata: *minter_may_update_metadata,
                    /// there can be multiple owners, so owners cannot update metadata
                    owner_may_update_metadata: false,
                }
            },
            TknConf::Nft { 
                public_total_supply, 
                owner_is_public, 
                enable_burn, 
                minter_may_update_metadata, 
                owner_may_update_metadata 
            } => {
                TknConfFlat {
                    is_nft: true,
                    decimals: 0_u8,
                    public_total_supply: *public_total_supply,
                    owner_is_public: *owner_is_public,
                    /// NFT can be minted only once using `MintTokenIds`
                    enable_mint: false,
                    enable_burn: *enable_burn,
                    minter_may_update_metadata: *minter_may_update_metadata,
                    owner_may_update_metadata: *owner_may_update_metadata,
                }
            },
        } 
    }

    // note that default is normally `false`. These default to `true` is for unit testing purposes
    #[cfg(test)]
    pub fn default_fungible() -> Self {
        TknConf::Fungible { 
            decimals: 6_u8,
            public_total_supply: true, 
            enable_mint: true,
            enable_burn: true, 
            minter_may_update_metadata: true, 
        }
    }

    #[cfg(test)]
    pub fn default_nft() -> Self {
        TknConf::Nft { 
            public_total_supply: true, 
            owner_is_public: true,
            enable_burn: true, 
            minter_may_update_metadata: true, 
            owner_may_update_metadata: true, 
        }
    }
}

/// Constructed from input enum `TknConf`. Flattened for easier handling in contract logic  
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct TknConfFlat {
    pub is_nft: bool,
    pub decimals: u8,
    pub public_total_supply: bool,
    pub owner_is_public: bool,
    pub enable_mint: bool,
    pub enable_burn: bool,
    pub minter_may_update_metadata: bool,
    pub owner_may_update_metadata: bool,
}

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
    config: &mut ContrConf,
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
    config: &mut ContrConf,
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
    config: &mut ContrConf,
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


/////////////////////////////////////////////////////////////////////////////////
// Permissions
/////////////////////////////////////////////////////////////////////////////////

/// saves new permission entry and adds identifier to the list of permissions the owner address has
pub fn new_permission<S: Storage>(
    storage: &mut S,
    owner: &HumanAddr,
    token_id: &str,
    allowed_addr: &HumanAddr,
    // permission_key: &PermissionKey,
    permission: &Permission,
) -> StdResult<()> {
    // store permission
    permission_w(storage, owner, token_id).save(
        to_binary(allowed_addr)?.as_slice(),
        permission
    )?;

    // add permission to list of permissions for a given owner
    append_permission_for_addr(storage, owner, token_id, allowed_addr)?;

    Ok(())
}

// /// updates an existing permission entry. Does not check that existing entry exists, so 
// /// riskier to use this. But saves gas from potentially loading permission twice
// pub fn update_permission_unchecked<S: Storage>(
//     storage: &mut S,
//     owner: &HumanAddr,
//     token_id: &str,
//     allowed_addr: &HumanAddr,
//     permission: &Permission,
// ) -> StdResult<()> {
//     permission_w(storage, owner, token_id).save(
//         to_binary(allowed_addr)?.as_slice(),
//         permission
//     )?;

//     Ok(())
// }

/// updates an existing permission entry. Returns error if permission entry does not aleady exist
pub fn update_permission<S> (
    storage: &mut S,
    owner: &HumanAddr,
    token_id: &str,
    allowed_addr: &HumanAddr,
    permission: &Permission
    // update_action: A,
) -> StdResult<()> 
    where
    S: Storage, 
    // A: FnOnce(Option<Permission>) -> StdResult<Permission> 
    {

    let update_action = |perm: Option<Permission>| -> StdResult<Permission> {
        match perm {
            Some(_) => Ok(permission.clone()),
            None => Err(StdError::generic_err("cannot update or revoke a non-existent permission entry"))
        }
    };

    permission_w(storage, owner, token_id).update(
        to_binary(allowed_addr)?.as_slice(),
        update_action
    )?;

    Ok(())
}

/// returns StdResult<Option<Permission>> for a given [`owner`, `token_id`, `allowed_addr`] combination.
/// Returns "dormant" permissions we well, ie: where owner doesn't currently own tokens.
/// If permission does not exist -> returns StdResult<None> 
pub fn may_load_any_permission<S: Storage>(
    storage: &S,
    owner: &HumanAddr,
    token_id: &str,
    allowed_addr: &HumanAddr,
) -> StdResult<Option<Permission>> {
    permission_r(storage, owner, token_id).may_load(to_binary(allowed_addr)?.as_slice())
}

// /// returns StdResult<Option<Permission>> for a given [`owner`, `token_id`, `allowed_addr`] combination.
// /// If (permission does not exist) || (owner no longer owns tokens) () -> returns StdResult<None>
// pub fn may_load_active_permission<S: Storage>(
//     storage: &S,
//     owner: &HumanAddr,
//     token_id: &str,
//     allowed_addr: &HumanAddr,
// ) -> StdResult<Option<Permission>> {
//     let permission = permission_r(storage, owner, token_id).may_load(to_binary(allowed_addr)?.as_slice())?;
//     let owner_amount = balances_r(storage, token_id).may_load(to_binary(owner)?.as_slice())?;
//     match owner_amount {
//         None =>  return Ok(None),
//         Some(i) if i == Uint128(0) => return Ok(None),
//         Some(i) if i > Uint128(0) => return Ok(permission),
//         Some(_) => unreachable!("may_load_permission: this should be unreachable")
//     }
// }

/// Return (Vec<`PermissionKey { token_id, allowed_addr }`>, u64)
/// returns a list and total number of PermissionKeys for a given owner. The PermissionKeys represents (part of) 
/// the keys to retrieve all permissions an `owner` has currently granted
pub fn list_owner_permission_keys<S: Storage>(
    storage: &S,
    owner: &HumanAddr,
    page: u32,
    page_size: u32,
) -> StdResult<(Vec<PermissionKey>, u64)> {
    let store = ReadonlyPrefixedStorage::multilevel(&[PREFIX_PERMISSION_ID, to_binary(owner)?.as_slice()], storage);

    // Try to access the storage of PermissionKeys for the account.
    // If it doesn't exist yet, return an empty list of transfers.
    let store = AppendStore::<PermissionKey, _, _>::attach(&store);
    let store = if let Some(result) = store {
        result?
    } else {
        return Ok((vec![], 0));
    };

    // Take `page_size` starting from the latest entry, potentially skipping `page * page_size`
    // entries from the start.
    let pkeys_iter = store
        .iter()
        .rev()
        .skip((page * page_size) as _)
        .take(page_size as _);

    // Transform iterator to a `Vec<PermissionKey>`
    let pkeys: StdResult<Vec<PermissionKey>> = pkeys_iter
        // .map(|pkey| pkey)
        .collect();
    // return `(Vec<PermissionKey> , total_permission)`
    pkeys.map(|pkeys| (pkeys, store.len() as u64))
}

/// stores a `PermissionKey {token_id: String, allowed_addr: String]` for a given `owner`. Note that 
/// permission key is [`owner`, `token_id`, `allowed_addr`]. This function does not enforce that the 
/// list of PermissionKey stored is unique; while this doesn't really matter, the ref implementation's 
/// functions aim to ensure each entry is unique, for storage efficiency.
fn append_permission_for_addr<S: Storage>(
    storage: &mut S,
    owner: &HumanAddr,
    token_id: &str,
    allowed_addr: &HumanAddr,
) -> StdResult<()> {
    let permission_key = PermissionKey {
        token_id: token_id.to_string(),
        allowed_addr: allowed_addr.clone(),
    };
    let mut store = PrefixedStorage::multilevel(&[PREFIX_PERMISSION_ID, to_binary(owner)?.as_slice()], storage);
    let mut store = AppendStoreMut::attach_or_create(&mut store)?;
    store.push(&permission_key)
}

/// struct to store permission for a `[token_id, owner, allowed_addr]` combination
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema, Default)]
pub struct Permission {
    pub view_balance_perm: bool,
    pub view_balance_exp: Expiration,
    pub view_pr_metadata_perm: bool,
    pub view_pr_metadata_exp: Expiration,
    pub trfer_allowance_perm: Uint128, 
    pub trfer_allowance_exp: Expiration, 
}

impl Permission {
    pub fn check_view_balance_perm(&self, blockinfo: &BlockInfo) -> bool {
        self.view_balance_perm && !self.view_balance_exp.is_expired(blockinfo)
    }
    pub fn check_view_pr_metadata_perm(&self, blockinfo: &BlockInfo) -> bool {
        self.view_pr_metadata_perm && !self.view_pr_metadata_exp.is_expired(blockinfo)
    }
}

/// to store all keys to access all permissions for a given `owner`
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct PermissionKey {
    pub token_id: String,
    pub allowed_addr: HumanAddr,
}

/////////////////////////////////////////////////////////////////////////////////
// Viewing Keys
/////////////////////////////////////////////////////////////////////////////////

pub fn write_viewing_key<S: Storage>(store: &mut S, owner: &CanonicalAddr, key: &ViewingKey) {
    let mut balance_store = PrefixedStorage::new(PREFIX_VIEW_KEY, store);
    balance_store.set(owner.as_slice(), &key.to_hashed());
}

pub fn read_viewing_key<S: Storage>(store: &S, owner: &CanonicalAddr) -> Option<Vec<u8>> {
    let balance_store = ReadonlyPrefixedStorage::new(PREFIX_VIEW_KEY, store);
    balance_store.get(owner.as_slice())
}


/////////////////////////////////////////////////////////////////////////////////
// Receiver Interface
/////////////////////////////////////////////////////////////////////////////////

pub fn get_receiver_hash<S: ReadonlyStorage>(
    store: &S,
    account: &HumanAddr,
) -> Option<StdResult<String>> {
    let store = ReadonlyPrefixedStorage::new(PREFIX_RECEIVERS, store);
    store.get(account.as_str().as_bytes()).map(|data| {
        String::from_utf8(data)
            .map_err(|_err| StdError::invalid_utf8("stored code hash was not a valid String"))
    })
}

pub fn set_receiver_hash<S: Storage>(store: &mut S, account: &HumanAddr, code_hash: String) {
    let mut store = PrefixedStorage::new(PREFIX_RECEIVERS, store);
    store.set(account.as_str().as_bytes(), code_hash.as_bytes());
}


/////////////////////////////////////////////////////////////////////////////////
// Other structs, enums and functions
/////////////////////////////////////////////////////////////////////////////////


#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct MintTokenId {
    pub token_info: TknInfo,
    pub balances: Vec<Balance>,
}

#[cfg(test)]
impl Default for MintTokenId {
    fn default() -> Self {
        Self { 
            token_info: TknInfo { 
                token_id: "0".to_string(), 
                name: "token0".to_string(), 
                symbol: "TKN".to_string(), 
                token_config: TknConf::default_fungible(),
                public_metadata: Some(Metadata {
                    token_uri: Some("public uri".to_string()),
                    extension: Some(Extension::default()),
                }), 
                private_metadata: Some(Metadata {
                    token_uri: Some("private uri".to_string()),
                    extension: Some(Extension::default()),
                }),  
            }, 
            balances: vec![Balance { 
                address: HumanAddr("addr0".to_string()), 
                amount: Uint128(1000) 
            }],
        }
    }
}

/// used for MintToken and BurnToken in the base specifications
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct TokenAmount {
    pub token_id: String,
    /// For BurnToken, only `Balance.amount` is relevant. `Balance.address` need to be the 
    /// owner's address. This design decision is to allow `BurnToken` to apply to other addresses, 
    /// possible in the additional specifications
    pub balances: Vec<Balance>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Balance {
    /// For BurnToken, `address` needs to be the owner's address. This design decision is 
    /// to allow `BurnToken` to apply to other addresses, possible in the additional 
    /// specifications
    pub address: HumanAddr,
    pub amount: Uint128,
}

/// Returns StdResult<()> resulting from saving an item to storage using Json (de)serialization
/// because bincode2 annoyingly uses a float op when deserializing an enum
///
/// # Arguments
///
/// * `storage` - a mutable reference to the storage this item should go to
/// * `key` - a byte slice representing the key to access the stored item
/// * `value` - a reference to the item to store
pub fn json_save<T: Serialize, S: Storage>(
    storage: &mut S,
    key: &[u8],
    value: &T,
) -> StdResult<()> {
    storage.set(key, &Json::serialize(value)?);
    Ok(())
}

/// Returns StdResult<T> from retrieving the item with the specified key using Json
/// (de)serialization because bincode2 annoyingly uses a float op when deserializing an enum.  
/// Returns a StdError::NotFound if there is no item with that key
///
/// # Arguments
///
/// * `storage` - a reference to the storage this item is in
/// * `key` - a byte slice representing the key that accesses the stored item
pub fn json_load<T: DeserializeOwned, S: ReadonlyStorage>(storage: &S, key: &[u8]) -> StdResult<T> {
    Json::deserialize(
        &storage
            .get(key)
            .ok_or_else(|| StdError::not_found(type_name::<T>()))?,
    )
}

/// Returns StdResult<Option<T>> from retrieving the item with the specified key using Json
/// (de)serialization because bincode2 annoyingly uses a float op when deserializing an enum.
/// Returns Ok(None) if there is no item with that key
///
/// # Arguments
///
/// * `storage` - a reference to the storage this item is in
/// * `key` - a byte slice representing the key that accesses the stored item
pub fn json_may_load<T: DeserializeOwned, S: ReadonlyStorage>(
    storage: &S,
    key: &[u8],
) -> StdResult<Option<T>> {
    match storage.get(key) {
        Some(value) => Json::deserialize(&value).map(Some),
        None => Ok(None),
    }
}