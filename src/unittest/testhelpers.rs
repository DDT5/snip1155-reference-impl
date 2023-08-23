use std::any::Any;
use serde::de::DeserializeOwned;

use super::super::{
    handles::*,
    msg::*,
    state::*,
    state::{
        state_structs::*,
    },
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

pub struct Addrs {
    addrs: Vec<HumanAddr>,
    hashes: Vec<String>,
}

impl Addrs {
    pub fn a(&self) -> HumanAddr {
        self.addrs[0].clone()
    }
    pub fn b(&self) -> HumanAddr {
        self.addrs[1].clone()
    }
    pub fn c(&self) -> HumanAddr {
        self.addrs[2].clone()
    }
    pub fn d(&self) -> HumanAddr {
        self.addrs[3].clone()
    }
    pub fn all(&self) -> Vec<HumanAddr> {
        self.addrs.clone()
    }
    pub fn a_hash(&self) -> String {
        self.hashes[0].clone()
    }
    pub fn b_hash(&self) -> String {
        self.hashes[1].clone()
    }
    pub fn c_hash(&self) -> String {
        self.hashes[2].clone()
    }
    pub fn _d_hash(&self) -> String {
        self.hashes[3].clone()
    }
}

/// inits 3 addresses
pub fn init_addrs() -> Addrs {
    let addr_strs = vec!["addr0", "addr1", "addr2", "addr3"];
    let hashes = vec!["addr0_hash".to_string(), "addr1_hash".to_string(), "addr2_hash".to_string(), "addr3_hash".to_string()];
    let mut addrs: Vec<HumanAddr> = vec![];
    for addr in addr_strs {
        addrs.push(HumanAddr(addr.to_string()));
    }
    Addrs { addrs, hashes }
}

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
        curators: vec![env.message.sender.clone()],
        initial_tokens: vec![CurateTokenId::default()],
        entropy: "seedentropy".to_string(),
    };

    (init(&mut deps, env, init_msg), deps)
}

/// curate additional:
/// * 800 fungible token_id 0a to addr0,
/// * 500 fungible token_id 1 to addr1,
/// * 1 NFT token_id 2 to addr2
/// * 1 NFT token_id 2a to addr2
pub fn curate_addtl_default<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: &Env,
) -> StdResult<()> {
    // init addtl addresses
    let addr0 = HumanAddr("addr0".to_string());
    let addr1 = HumanAddr("addr1".to_string());
    let addr2 = HumanAddr("addr2".to_string());

    // fungible token_id "0a"
    let mut curate0a = CurateTokenId::default();
    curate0a.token_info.token_id = "0a".to_string();
    curate0a.token_info.name = "token0a".to_string();
    curate0a.token_info.symbol = "TKNO".to_string();
    curate0a.balances[0].address = addr0;
    curate0a.balances[0].amount = Uint128(800);

    // fungible token_id "1"
    let mut curate1 = CurateTokenId::default();
    curate1.token_info.token_id = "1".to_string();
    curate1.token_info.name = "token1".to_string();
    curate1.token_info.symbol = "TKNA".to_string();
    curate1.balances[0].address = addr1;
    curate1.balances[0].amount = Uint128(500);

    // NFT "2"
    let mut curate2 = CurateTokenId::default();
    curate2.token_info.token_id = "2".to_string();
    curate2.token_info.name = "token2".to_string();
    curate2.token_info.symbol = "TKNB".to_string();
    curate2.token_info.token_config = TknConfig::default_nft();
    curate2.balances = vec![TokenIdBalance { address: addr2.clone(), amount: Uint128(1) }];
    
    // NFT "2a"
    let mut curate2a = CurateTokenId::default();
    curate2a.token_info.token_id = "2a".to_string();
    curate2a.token_info.name = "token2a".to_string();
    curate2a.token_info.symbol = "TKNBA".to_string();
    curate2a.token_info.token_config = TknConfig::default_nft();
    curate2a.balances = vec![TokenIdBalance { address: addr2, amount: Uint128(1) }];

    // batch curate token_id "0a", "1", NFT "2" and NFT "3"
    let msg = HandleMsg::CurateTokenIds{initial_tokens: vec![curate0a, curate1, curate2, curate2a], memo: None, padding: None };
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
    balances_r(storage, token_id_str)
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
    let decoded_msg: U = from_binary(msg).unwrap();
    Ok((decoded_msg, receiver_addr, receiver_hash))
}

/// generates an array of viewing keys (as Strings)
pub fn generate_viewing_keys<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: &Env,
    addresses: Vec<HumanAddr>
) -> StdResult<Vks> {
    let mut vks: Vec<String> = vec![];
    let mut env = env.clone();
    for address in addresses {
        env.message.sender = address;
        let msg = HandleMsg::CreateViewingKey { entropy: "askdjlm".to_string(), padding: None };
        let response = handle(deps, env.clone(), msg)?;
        let vk = from_binary::<HandleAnswer>(&response.data.unwrap())?;
        if let HandleAnswer::CreateViewingKey { key } = vk {
            vks.push(key.to_string())
        } else { 
            return Err(StdError::generic_err("no viewing key generated"))
        }
    }

    for i in 0..vks.len() {
        if i == 0 { continue };
        assert_ne!(vks[i], vks[i-1], "viewing keys of two different addresses are similar");
    }

    Ok(Vks {vks})
}

pub struct Vks {
    vks: Vec<String>
}

impl Vks {
    pub fn a(&self) -> String {
        self.vks[0].clone()
    }
    pub fn b(&self) -> String {
        self.vks[1].clone()
    }
    pub fn c(&self) -> String {
        self.vks[2].clone()
    }
    // pub fn d(&self) -> String {
    //     self.vks[3].clone()
    // }
}
