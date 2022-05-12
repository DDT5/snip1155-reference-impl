use cosmwasm_std::{Uint128, HumanAddr, Binary};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{
    state::{MintTokenId, MintToken}
};


/////////////////////////////////////////////////////////////////////////////////
// Init messages
/////////////////////////////////////////////////////////////////////////////////

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InitMsg {
    pub has_admin: bool,
    pub admin: Option<HumanAddr>,
    pub minters: Vec<HumanAddr>,
    pub initial_tokens: Vec<MintTokenId>,
}

/////////////////////////////////////////////////////////////////////////////////
// Handle Messages
/////////////////////////////////////////////////////////////////////////////////

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum HandleMsg {
    MintTokenIds {
        initial_tokens: Vec<MintTokenId>,
        memo: Option<String>,
        padding: Option<String>,
    },
    MintTokens {
        mint_tokens: Vec<MintToken>,
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
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum HandleAnswer {
    NewTokenIds { status: ResponseStatus },
    Mint { status: ResponseStatus },
    Transfer { status: ResponseStatus },
    Send { status: ResponseStatus },
}



/////////////////////////////////////////////////////////////////////////////////
// Query messages
/////////////////////////////////////////////////////////////////////////////////

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    ContractInfo { },
}



/////////////////////////////////////////////////////////////////////////////////
// Structs and Enums
/////////////////////////////////////////////////////////////////////////////////

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub enum ResponseStatus {
    Success,
    Failure,
}


