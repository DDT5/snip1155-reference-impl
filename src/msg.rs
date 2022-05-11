use cosmwasm_std::{Uint128, HumanAddr};
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
    MintTokenIds(Vec<MintTokenId>),
    MintTokens(Vec<MintToken>),
    Transfer {
        token_id: String,
        sender: HumanAddr,
        recipient: HumanAddr,
        amount: Uint128
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum HandleAnswer {
    NewTokenIds { status: ResponseStatus },
    Mint { status: ResponseStatus },
    Transfer { status: ResponseStatus },
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


