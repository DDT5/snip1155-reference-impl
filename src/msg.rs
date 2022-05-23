use cosmwasm_std::{Uint128, HumanAddr, Binary};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{
    state::{MintTokenId, TokenAmount, Tx, PermissionKey, Permission, TknInfo, }, 
    vk::viewing_key::ViewingKey, 
    // expiration::Expiration
};

use secret_toolkit::permit::Permit;


/////////////////////////////////////////////////////////////////////////////////
// Init messages
/////////////////////////////////////////////////////////////////////////////////

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)] //PartialEq
pub struct InitMsg {
    pub has_admin: bool,
    pub admin: Option<HumanAddr>,
    pub minters: Vec<HumanAddr>,
    pub initial_tokens: Vec<MintTokenId>,
    pub entropy: String,
}

/////////////////////////////////////////////////////////////////////////////////
// Handle Messages
/////////////////////////////////////////////////////////////////////////////////

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum HandleMsg {
    MintTokenIds {
        initial_tokens: Vec<MintTokenId>,
        memo: Option<String>,
        padding: Option<String>,
    },
    MintTokens {
        mint_tokens: Vec<TokenAmount>,
        memo: Option<String>,
        padding: Option<String>,
    },
    BurnTokens {
        burn_tokens: Vec<TokenAmount>,
        memo: Option<String>,
        padding: Option<String>,
    },
    Transfer {
        token_id: String,
        // equivalent to `owner` in SNIP20. Tokens are sent from this address. 
        from: HumanAddr,
        recipient: HumanAddr,
        amount: Uint128,
        memo: Option<String>,
        padding: Option<String>,
    },
    BatchTransfer {
        actions: Vec<TransferAction>,
        padding: Option<String>,
    },
    Send {
        token_id: String,
        // equivalent to `owner` in SNIP20. Tokens are sent from this address. 
        from: HumanAddr,
        recipient: HumanAddr,
        recipient_code_hash: Option<String>,
        amount: Uint128,
        msg: Option<Binary>,
        memo: Option<String>,
        padding: Option<String>,
    },
    BatchSend {
        actions: Vec<SendAction>,
        padding: Option<String>,
    },
    GivePermission {
        /// address being granted/revoked permission
        allowed_address: HumanAddr,
        /// token id to apply approval/revocation to.
        /// Additional Spec feature: if == None, perform action for all owner's `token_id`s
        token_id: String,
        /// optional permission level for viewing the owner. If ignored, leaves current permission settings
        view_owner: Option<bool>,
        /// optional permission level for viewing private metadata. If ignored, leaves current permission settings
        view_private_metadata: Option<bool>,
        /// set allowance by for transfer approvals. If ignored, leaves current permission settings
        transfer: Option<Uint128>,
        /// optional message length padding
        padding: Option<String>,
    },
    /// Removes all permissions that a specific owner has granted to a specific address, for a specific token_id 
    RevokePermission {
        token_id: String,
        /// token owner
        owner: HumanAddr,
        /// address which has permission
        allowed_address: HumanAddr,
        padding: Option<String>,
    },
    RegisterReceive {
        code_hash: String,
        padding: Option<String>,
    },
    CreateViewingKey {
        entropy: String,
        padding: Option<String>,
    },
    SetViewingKey {
        key: String,
        padding: Option<String>,
    },
    AddMinters {
        add_minters: Vec<HumanAddr>,
        padding: Option<String>,
    },
    RemoveMinters {
        remove_minters: Vec<HumanAddr>,
        padding: Option<String>,
    },
    ChangeAdmin {
        new_admin: HumanAddr,
        padding: Option<String>,
    },
    /// Permanently breaks admin keys for this contract. No admin function can be called after this
    /// action. Any existing minters will remain as minters; no new minters can be added and no current
    /// minter can be removed. 
    /// 
    /// Requires caller to input current admin address and contract address. These inputs are not strictly 
    /// necessary, but as a safety precaution to reduce the chances of accidentally calling this function.
    BreakAdminKey {
        current_admin: HumanAddr,
        contract_address: HumanAddr,
        padding: Option<String>,
    },
    /// disallow the use of a permit
    RevokePermit {
        permit_name: String,
        padding: Option<String>,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum HandleAnswer {
    MintTokenIds { status: ResponseStatus },
    MintTokens { status: ResponseStatus },
    BurnTokens { status: ResponseStatus },
    Transfer { status: ResponseStatus },
    BatchTransfer { status: ResponseStatus },
    Send { status: ResponseStatus },
    BatchSend { status: ResponseStatus },
    GivePermission { status: ResponseStatus },
    RevokePermission { status: ResponseStatus },
    RegisterReceive { status: ResponseStatus },
    CreateViewingKey { key: ViewingKey },
    SetViewingKey { status: ResponseStatus },
    AddMinters { status: ResponseStatus },
    RemoveMinters { status: ResponseStatus },
    ChangeAdmin { status: ResponseStatus },
    BreakAdminKey { status: ResponseStatus },
    RevokePermit { status: ResponseStatus },
}



/////////////////////////////////////////////////////////////////////////////////
// Query messages
/////////////////////////////////////////////////////////////////////////////////

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    ContractInfo {  },
    Balance {
        address: HumanAddr,
        key: String,
        token_id: String,
    },
    TransactionHistory {
        address: HumanAddr,
        key: String,
        page: Option<u32>,
        page_size: u32,
    },
    Permission {
        owner: HumanAddr,
        allowed_address: HumanAddr,
        key: String,
        token_id: String,
    },
    /// displays all permissions that a given address has granted
    AllPermissions {
        /// address that has granted permissions to others
        address: HumanAddr,
        key: String,
        page: Option<u32>,
        page_size: u32,
    },
    TokenIdPublicInfo { token_id: String },
    TokenIdPrivateInfo { 
        address: HumanAddr,
        key: String,
        token_id: String,
    },
    RegisteredCodeHash {
        contract: HumanAddr
    },
    WithPermit {
        permit: Permit,
        query: QueryWithPermit,
    }
}

impl QueryMsg {
    pub fn get_validation_params(&self) -> (Vec<&HumanAddr>, ViewingKey) {
        match self {
            Self::Balance { address, key, .. } => (vec![address], ViewingKey(key.clone())),
            Self::TransactionHistory { address, key, .. } => (vec![address], ViewingKey(key.clone())),
            Self::Permission {
                owner,
                allowed_address,
                key,
                ..
            } => (vec![owner, allowed_address], ViewingKey(key.clone())),
            Self::AllPermissions { address, key, .. } => (vec![address], ViewingKey(key.clone())),
            Self::TokenIdPrivateInfo { address, key, .. } => (vec![address], ViewingKey(key.clone())),
            _ => panic!("This query type does not require authentication"),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryWithPermit {
    Balance { token_id: String },
    TransactionHistory {
        page: Option<u32>,
        page_size: u32,
    },
    Permission {
        owner: HumanAddr,
        allowed_address: HumanAddr,
        token_id: String,
    },
    AllPermissions {
        page: Option<u32>,
        page_size: u32,
    },
    TokenIdPrivateInfo { 
        token_id: String,
    },
}

#[derive(Serialize, Deserialize, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub enum QueryAnswer {
    ContractInfo {
        admin: Option<HumanAddr>,
        minters: Vec<HumanAddr>,
        all_token_ids: Vec<String>,
    },
    Balance {
        amount: Uint128,
    },
    TransactionHistory {
        txs: Vec<Tx>,
        total: Option<u64>,
    },
    Permission(Permission),
    AllPermissions{
        permission_keys: Vec<PermissionKey>,
        permissions: Vec<Permission>,
        total: u64,
    },
    TokenIdPublicInfo {
        /// token_id_info.private_metadata will = None
        token_id_info: TknInfo,
        /// if public_total_supply == false, total_supply = None
        total_supply: Option<Uint128>,
    },
    TokenIdPrivateInfo {
        token_id_info: TknInfo,
        /// if public_total_supply == false, total_supply = None
        total_supply: Option<Uint128>,
        owner: Option<HumanAddr>
    },
    /// returns None if contract has not registered with SNIP1155 contract
    RegisteredCodeHash {
        code_hash: Option<String>,
    },
    ViewingKeyError {
        msg: String,
    },
}

/////////////////////////////////////////////////////////////////////////////////
// Structs, Enums and other functions
/////////////////////////////////////////////////////////////////////////////////

#[derive(Serialize, Deserialize, Clone, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub enum ResponseStatus {
    Success,
    Failure,
}

#[derive(Serialize, Deserialize, Clone, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub struct TransferAction {
    pub token_id: String,
    // equivalent to `owner` in SNIP20. Tokens are sent from this address. 
    pub from: HumanAddr,
    pub recipient: HumanAddr,
    pub amount: Uint128,
    pub memo: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub struct SendAction {
    pub token_id: String,
    // equivalent to `owner` in SNIP20. Tokens are sent from this address. 
    pub from: HumanAddr,
    pub recipient: HumanAddr,
    pub recipient_code_hash: Option<String>,
    pub amount: Uint128,
    pub msg: Option<Binary>,
    pub memo: Option<String>,
}

// Take a Vec<u8> and pad it up to a multiple of `block_size`, using spaces at the end.
pub fn space_pad(block_size: usize, message: &mut Vec<u8>) -> &mut Vec<u8> {
    let len = message.len();
    let surplus = len % block_size;
    if surplus == 0 {
        return message;
    }

    let missing = block_size - surplus;
    message.reserve(missing);
    message.extend(std::iter::repeat(b' ').take(missing));
    message
}
