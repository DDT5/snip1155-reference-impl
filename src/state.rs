use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{
    Storage, Uint128, HumanAddr, CanonicalAddr, 
    // StdResult, to_binary, 
};
use cosmwasm_storage::{
    // PrefixedStorage, ReadonlyPrefixedStorage, 
    bucket, bucket_read, Bucket, ReadonlyBucket,
    singleton, singleton_read, ReadonlySingleton, Singleton, 
};

// use secret_toolkit::{
//     storage::{AppendStore, AppendStoreMut},
// };

use crate::{
    token::{Metadata},
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

/////////////////////////////////////////////////////////////////////////////////
// Composite functions
/////////////////////////////////////////////////////////////////////////////////



/////////////////////////////////////////////////////////////////////////////////
// Structs and enums
/////////////////////////////////////////////////////////////////////////////////

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
    // todo!()
}


#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ContrConf {
    pub admin: Option<HumanAddr>,
    pub minters: Vec<HumanAddr>,
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
                token_config: TknConf {  }, 
                public_metadata: None, 
                private_metadata: None 
            }, 
            balances: vec![Balance { 
                address: HumanAddr("addr0".to_string()), 
                amount: Uint128(1000) 
            }]
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct MintToken {
    pub token_id: String,
    pub add_balances: Vec<Balance>,
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
