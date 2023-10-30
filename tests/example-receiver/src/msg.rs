use cosmwasm_std::{Binary, HumanAddr, Uint128};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InitMsg {
    pub count: i32,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum HandleMsg {
    Increment {},
    Reset {
        count: i32,
    },
    Register {
        reg_addr: HumanAddr,
        reg_hash: String,
    },
    Snip1155Receive {
        sender: HumanAddr,
        token_id: String,
        from: HumanAddr,
        amount: Uint128,
        memo: Option<String>,
        msg: Binary,
    },
    Fail {},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    // GetCount returns the current count as a json-encoded number
    GetCount {},
}

// We define a custom struct for each query response
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct CountResponse {
    pub count: i32,
}

// Messages sent to SNIP-20 contracts
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Snip1155Msg {
    RegisterReceive {
        code_hash: String,
        padding: Option<String>,
    },
}

impl Snip1155Msg {
    pub fn register_receive(code_hash: String) -> Self {
        Snip1155Msg::RegisterReceive {
            code_hash,
            padding: None,
        }
    }
}