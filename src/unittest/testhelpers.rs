use std::any::Any;
use serde::de::DeserializeOwned;

use super::super::{
    contract::*,
    msg::*,
    state::*,
};

use cosmwasm_std::{
    testing::*, 
    StdResult, StdError,
    InitResponse, HandleResponse,
    Extern, Storage, Api, Querier, Env,
    HumanAddr, Uint128, 
    to_binary, from_binary, CosmosMsg, WasmMsg, 
};



/////////////////////////////////////////////////////////////////////////////////
// Helper functions
/////////////////////////////////////////////////////////////////////////////////

/// inits contract, with initial balances:
/// * 1000 token_id 0 to addr0
pub fn init_helper_default() -> (
    StdResult<InitResponse>,
    Extern<MockStorage, MockApi, MockQuerier>,
) {
    let mut deps = mock_dependencies(20, &[]);
    let env = mock_env("addr0", &[]);

    let init_msg = InitMsg {
        has_admin: true,
        admin: None, // None -> sender defaults as admin
        minters: vec![env.message.sender.clone()],
        initial_tokens: vec![MintTokenId::default()],
        entropy: "seedentropy".to_string(),
    };

    (init(&mut deps, env, init_msg), deps)
}

/// mints 
/// * 500 fungible token_id 1 to addr1,
/// * 1 NFT token_id 2 to addr2
/// * 1 NFT token_id 3 to addr2
pub fn mint_addtl_default<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: &Env,
) -> StdResult<()> {
    // init addtl addresses
    let addr1 = HumanAddr("addr1".to_string());
    let addr2 = HumanAddr("addr2".to_string());

    // fungible token_id "1"
    let mut mint1 = MintTokenId::default();
    mint1.token_info.token_id = "1".to_string();
    mint1.token_info.name = "token1".to_string();
    mint1.token_info.symbol = "TKNA".to_string();
    mint1.balances[0].address = addr1.clone();
    mint1.balances[0].amount = Uint128(500);

    // NFT "2"
    let mut mint2 = MintTokenId::default();
    mint2.token_info.token_id = "2".to_string();
    mint2.token_info.name = "token2".to_string();
    mint2.token_info.symbol = "TKNB".to_string();
    mint2.token_info.is_nft = true;
    mint2.balances = vec![Balance { address: addr2.clone(), amount: Uint128(1) }];
    
    // NFT "3"
    let mut mint3 = MintTokenId::default();
    mint3.token_info.token_id = "3".to_string();
    mint3.token_info.name = "token3".to_string();
    mint3.token_info.symbol = "TKNC".to_string();
    mint3.token_info.is_nft = true;
    mint3.balances = vec![Balance { address: addr2.clone(), amount: Uint128(1) }];

    // batch mint token_id "1", NFT "2" and NFT "3"
    let msg = HandleMsg::MintTokenIds{initial_tokens: vec![mint1, mint2, mint3], memo: None, padding: None };
    handle(deps, env.to_owned(), msg)?;
    
    Ok(())
}

pub fn extract_error_msg<T: Any>(error: &StdResult<T>) -> String {
    match error {
        Ok(_response) => panic!("Expected error, but had Ok response"),
        Err(err) => match err {
            StdError::GenericErr { msg, .. } => msg.to_string(),
            _ => panic!("Unexpected error result {:?}", err),
        },
    }
}

pub fn _extract_log(resp: StdResult<HandleResponse>) -> String {
    match resp {
        Ok(response) => response.log[0].value.clone(),
        Err(_err) => "These are not the logs you are looking for".to_string(),
    }
}

/// checks token balance. Token_id input takes `&str` input, which converts to `String`  
pub fn chk_bal<S: Storage>(
    storage: &S,
    token_id_str: &str,
    address: &HumanAddr,
) -> Option<Uint128> {
    balances_r(storage, &token_id_str.to_string())
    .may_load(to_binary(&address).unwrap().as_slice()).unwrap()
}

pub fn extract_cosmos_msg<U: DeserializeOwned>(message: &CosmosMsg) -> StdResult<(U, Option<&HumanAddr>, &String)> {
    let (receiver_addr, receiver_hash, msg) = match message {
        CosmosMsg::Wasm(i) => match i {
            WasmMsg::Execute{contract_addr, callback_code_hash, msg, ..
            } => (Some(contract_addr), callback_code_hash, msg),
            WasmMsg::Instantiate { callback_code_hash, msg, .. } => (None, callback_code_hash, msg),
        },
        _ => return Err(StdError::generic_err("unable to extract msg from CosmosMsg"))
    };
    let decoded_msg: U = from_binary(&msg).unwrap();
    Ok((decoded_msg, receiver_addr, receiver_hash))
}
