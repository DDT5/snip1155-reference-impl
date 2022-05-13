use std::any::type_name;
use schemars::JsonSchema;
use serde::{de::DeserializeOwned, Deserialize, Serialize};

use cosmwasm_std::{
    Storage, Uint128, HumanAddr, CanonicalAddr, BlockInfo, 
    StdResult, StdError, //to_binary, 
    ReadonlyStorage, 
};
use cosmwasm_storage::{
    PrefixedStorage, ReadonlyPrefixedStorage, 
    bucket, bucket_read, Bucket, ReadonlyBucket,
    singleton, singleton_read, ReadonlySingleton, Singleton, 
};

use secret_toolkit::{
    serialization::{Json, Serde}, //Bincode2, 
    storage::{AppendStoreMut}, //AppendStore, 
};

use crate::{
    token::{Metadata},
    vk::viewing_key::{ViewingKey},
    expiration::{Expiration},
};

// U256
// use uint::{construct_uint};
// construct_uint! { pub struct U256(4); }


pub const RESPONSE_BLOCK_SIZE: usize = 256;


// namespaces
pub const CONTR_CONF: &[u8] = b"contrconfig";
pub const BALANCES: &[u8] = b"balances";
pub const TKN_INFO: &[u8] = b"tokeninfo";

/// prefix for storage of transactions
pub const PREFIX_TXS: &[u8] = b"preftxs";
/// prefix for storage of tx ids
pub const PREFIX_TX_IDS: &[u8] = b"txids";
pub const PREFIX_VIEW_KEY: &[u8] = b"s1155viewkey";
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

/// token_id configs
pub fn tkn_info_w<S: Storage>(storage: &mut S) -> Bucket<S, TknInfo> {
    bucket(TKN_INFO, storage)
}
pub fn tkn_info_r<S: Storage>(storage: &S) -> ReadonlyBucket<S, TknInfo> {
    bucket_read(TKN_INFO, storage)
}


/////////////////////////////////////////////////////////////////////////////////
// Multi-level Buckets
/////////////////////////////////////////////////////////////////////////////////

/// Multilevel bucket to store balances for each token_id & addr combination. Key intended to be [`token_id`, HumanAddr]  
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



/////////////////////////////////////////////////////////////////////////////////
// Appendstore
/////////////////////////////////////////////////////////////////////////////////

// // bids: Appendstore + bucket combo
// pub fn add_tkn_id<S: Storage>(
//     store: &mut S,
//     balance: &Balance,
// ) -> StdResult<()> {
//     // appendstore: adds info with u32 key 
//     let mut append_store = PrefixedStorage::new(BALANCES, store);
//     let mut append_store = AppendStoreMut::attach_or_create(&mut append_store)?;
//     append_store.push(balance)?;
//     Ok(())
// }

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


/////////////////////////////////////////////////////////////////////////////////
// Transaction history
/////////////////////////////////////////////////////////////////////////////////

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
pub struct ContrConf {
    pub admin: Option<HumanAddr>,
    pub minters: Vec<HumanAddr>,
    pub tx_cnt: u64,
    pub prng_seed: Vec<u8>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct TknInfo {
    pub token_id: String,
    pub name: String,
    pub symbol: String,
    pub is_nft: bool, 
    pub token_config: TknConf,
    pub public_metadata: Option<Metadata>,
    pub private_metadata: Option<Metadata>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct TknConf {
    pub enable_burn: bool,
}

impl TknConf {
    pub fn default() -> Self {
        Self {
            enable_burn: false, 
        }
    }
}

// /// code hash and address of a contract
// #[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema, Default)]
// pub struct ContractInfo {
//     /// Contract's code hash string
//     pub code_hash: String,
//     /// Contract's address in HumanAddr
//     pub address: HumanAddr,
// }


/// tx type and specifics
#[derive(Serialize, Deserialize, Clone, Debug)]
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
#[derive(Serialize, Deserialize, Clone, Debug)]
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
                symbol: "TKN0".to_string(), 
                is_nft: false, 
                token_config: TknConf {
                    // note that default is normally `false`. Default to `true` is for unit testing purposes
                    enable_burn: true, 
                },
                public_metadata: None, 
                private_metadata: None 
            }, 
            balances: vec![Balance { 
                address: HumanAddr("addr0".to_string()), 
                amount: Uint128(1000) 
            }],
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct TokenAmount {
    pub token_id: String,
    pub balances: Vec<Balance>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Balance {
    pub address: HumanAddr,
    pub amount: Uint128,
}

/// permission to view token info/transfer tokens
#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct Permission {
    /// permitted address
    pub address: CanonicalAddr,
    /// list of permission expirations for this address
    pub expirations: [Option<Expiration>; 3],
}

/// permission types
#[derive(Serialize, Deserialize, Debug)]
pub enum PermissionType {
    ViewOwner,
    ViewMetadata,
    Transfer,
}

impl PermissionType {
    /// Returns usize representation of the enum variant
    pub fn to_usize(&self) -> usize {
        match self {
            PermissionType::ViewOwner => 0,
            PermissionType::ViewMetadata => 1,
            PermissionType::Transfer => 2,
        }
    }

    /// returns the number of permission types
    pub fn num_types(&self) -> usize {
        3
    }
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