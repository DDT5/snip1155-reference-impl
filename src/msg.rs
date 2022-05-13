use cosmwasm_std::{Uint128, HumanAddr, Binary};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{
    state::{MintTokenId, TokenAmount}, 
    vk::viewing_key::ViewingKey
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
    NewTokenIds { status: ResponseStatus },
    MintTokens { status: ResponseStatus },
    BurnTokens { status: ResponseStatus },
    Transfer { status: ResponseStatus },
    Send { status: ResponseStatus },
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
