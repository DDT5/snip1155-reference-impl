pub mod state_structs;
pub mod permissions;
pub mod txhistory;
pub mod metadata;
pub mod expiration;
mod save_load_functions;

use cosmwasm_std::{
    Storage, BlockInfo, Uint128, HumanAddr, CanonicalAddr, 
    StdResult, StdError,
    ReadonlyStorage, to_binary, 
};

use cosmwasm_storage::{
    PrefixedStorage, ReadonlyPrefixedStorage, 
    bucket, bucket_read, Bucket, ReadonlyBucket,
    singleton, singleton_read, ReadonlySingleton, Singleton, 
};

use crate::{
    vk::viewing_key::{ViewingKey},
};

use self::{
    state_structs::{ContractConfig, StoredTokenInfo},
    permissions::Permission,
    expiration::{Expiration},
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


/// Contract configuration: stores information on this contract
pub fn contr_conf_w<S: Storage>(storage: &mut S) -> Singleton<S, ContractConfig> {
    singleton(storage, CONTR_CONF)
}
/// Contract configuration: reads information on this contract
pub fn contr_conf_r<S: Storage>(storage: &S) -> ReadonlySingleton<S, ContractConfig> {
    singleton_read( storage, CONTR_CONF)
}

/// Saves BlockInfo of latest tx. Should not be necessary after env becomes available to queries
pub fn blockinfo_w<S: Storage>(storage: &mut S) -> Singleton<S, BlockInfo> {
    singleton(storage, BLOCK_KEY)
}
/// Reads BlockInfo of latest tx. Should not be necessary after env becomes available to queries
pub fn blockinfo_r<S: Storage>(storage: &S) -> ReadonlySingleton<S, BlockInfo> {
    singleton_read(storage, BLOCK_KEY)
}

/////////////////////////////////////////////////////////////////////////////////
// Buckets
/////////////////////////////////////////////////////////////////////////////////

/// token_id configs. Key is `token_id.as_bytes()`
pub fn tkn_info_w<S: Storage>(storage: &mut S) -> Bucket<S, StoredTokenInfo> {
    bucket(TKN_INFO, storage)
}
/// token_id configs. Key is `token_id.as_bytes()`
pub fn tkn_info_r<S: Storage>(storage: &S) -> ReadonlyBucket<S, StoredTokenInfo> {
    bucket_read(TKN_INFO, storage)
}

/// total supply of a token_id. Key is `token_id.as_bytes()`
pub fn tkn_tot_supply_w<S: Storage>(storage: &mut S) -> Bucket<S, Uint128> {
    bucket(TKN_TOTAL_SUPPLY, storage)
}
/// total supply of a token_id. Key is `token_id.as_bytes()`
pub fn tkn_tot_supply_r<S: Storage>(storage: &S) -> ReadonlyBucket<S, Uint128> {
    bucket_read(TKN_TOTAL_SUPPLY, storage)
}

/////////////////////////////////////////////////////////////////////////////////
// Multi-level Buckets
/////////////////////////////////////////////////////////////////////////////////

/// Multilevel bucket to store balances for each token_id & addr combination. Key is to 
/// be [`token_id`, `owner`: to_binary(&HumanAddr)?.as_slice()]  
/// When using `balances_w` make sure to also check if need to change `current owner` of an nft and `total_supply` 
pub fn balances_w<'a, 'b, S: Storage>(
    storage: &'a mut S,
    token_id: &'b str
) -> Bucket<'a, S, Uint128> {
    Bucket::multilevel(&[BALANCES, token_id.as_bytes()], storage)
}
/// Multilevel bucket to store balances for each token_id & addr combination. Key is to 
/// be [`token_id`, `owner`: to_binary(&HumanAddr)?.as_slice()]  
pub fn balances_r<'a, 'b, S: Storage>(
    storage: &'a S,
    token_id: &'b str
) -> ReadonlyBucket<'a, S, Uint128> {
    ReadonlyBucket::multilevel(&[BALANCES, token_id.as_bytes()], storage)
}

/// private functions.
/// To store permission. key is to be [`owner`, `token_id`, `allowed_addr`]
/// `allowed_addr` is `to_binary(&HumanAddr)?.as_slice()` 
fn permission_w<'a, S: Storage>(
    storage: &'a mut S,
    owner: &'a HumanAddr,
    token_id: &'a str,
) -> Bucket<'a, S, Permission> {
    let owner_bin = to_binary(owner).unwrap();
    Bucket::multilevel(&[PREFIX_PERMISSIONS, owner_bin.as_slice(), token_id.as_bytes()], storage)
}
/// private functions.
/// To read permission. key is to be [`owner`, `token_id`, `allowed_addr`]
/// `allowed_addr` is `to_binary(&HumanAddr)?.as_slice()` 
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
// Viewing Keys
/////////////////////////////////////////////////////////////////////////////////

pub fn write_viewing_key<S: Storage>(store: &mut S, owner: &CanonicalAddr, key: &ViewingKey) {
    let mut vk_store = PrefixedStorage::new(PREFIX_VIEW_KEY, store);
    vk_store.set(owner.as_slice(), &key.to_hashed());
}

pub fn read_viewing_key<S: Storage>(store: &S, owner: &CanonicalAddr) -> Option<Vec<u8>> {
    let vk_store = ReadonlyPrefixedStorage::new(PREFIX_VIEW_KEY, store);
    vk_store.get(owner.as_slice())
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


