use cosmwasm_std::{Uint128, HumanAddr, Binary};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{
    state::{MintTokenId, TokenAmount, Tx, Permission}, 
    vk::viewing_key::ViewingKey, 
    // expiration::Expiration
};


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
    GivePermission {
        /// address being granted/revoked permission
        address: HumanAddr,
        /// token id to apply approval/revocation to.
        /// Todo: if == None, perform action for all owner's `token_id`s
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
}

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum HandleAnswer {
    MintTokenIds { status: ResponseStatus },
    MintTokens { status: ResponseStatus },
    BurnTokens { status: ResponseStatus },
    Transfer { status: ResponseStatus },
    Send { status: ResponseStatus },
    GivePermission { status: ResponseStatus },
    RegisterReceive { status: ResponseStatus },
    CreateViewingKey { key: ViewingKey },
    SetViewingKey { status: ResponseStatus },
}



/////////////////////////////////////////////////////////////////////////////////
// Query messages
/////////////////////////////////////////////////////////////////////////////////

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    ContractInfo { },
    Balance {
        address: HumanAddr,
        key: String,
        token_id: String,
    },
    TransferHistory {
        address: HumanAddr,
        key: String,
        page: Option<u32>,
        page_size: u32,
    },
    Permission {
        owner: HumanAddr,
        perm_address: HumanAddr,
        key: String,
        token_id: String,
    },
}

impl QueryMsg {
    pub fn get_validation_params(&self) -> (Vec<&HumanAddr>, ViewingKey) {
        match self {
            Self::Balance { address, key, .. } => (vec![address], ViewingKey(key.clone())),
            Self::TransferHistory { address, key, .. } => (vec![address], ViewingKey(key.clone())),
            Self::Permission {
                owner,
                perm_address,
                key,
                ..
            } => (vec![owner, perm_address], ViewingKey(key.clone())),
            _ => panic!("This query type does not require authentication"),
        }
    }
}

#[derive(Serialize, Deserialize, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub enum QueryAnswer {
    ContractInfo {
        info: String,
    },
    Balance {
        amount: Uint128,
    },
    TransferHistory {
        txs: Vec<Tx>,
        total: Option<u64>,
    },
    Permission(Permission),
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
