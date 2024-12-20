use cosmwasm_storage::ReadonlyPrefixedStorage;
use secret_toolkit::viewing_key::{ViewingKey, ViewingKeyStore};
use serde::de::DeserializeOwned;
use std::any::Any;

use super::super::{handles::*, msg::*, state::state_structs::*, state::*};

use cosmwasm_std::{
    from_binary, testing::*, to_binary, Addr, CosmosMsg, Env, MessageInfo, OwnedDeps, Response,
    StdError, StdResult, Storage, Uint256, WasmMsg,
};

/////////////////////////////////////////////////////////////////////////////////
// Helper functions
/////////////////////////////////////////////////////////////////////////////////

pub struct Addrs {
    addrs: Vec<Addr>,
    hashes: Vec<String>,
}

impl Addrs {
    pub fn a(&self) -> Addr {
        self.addrs[0].clone()
    }
    pub fn b(&self) -> Addr {
        self.addrs[1].clone()
    }
    pub fn c(&self) -> Addr {
        self.addrs[2].clone()
    }
    pub fn d(&self) -> Addr {
        self.addrs[3].clone()
    }
    pub fn all(&self) -> Vec<Addr> {
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
    let hashes = vec![
        "addr0_hash".to_string(),
        "addr1_hash".to_string(),
        "addr2_hash".to_string(),
        "addr3_hash".to_string(),
    ];
    let mut addrs: Vec<Addr> = vec![];
    for addr in addr_strs {
        addrs.push(Addr::unchecked(addr.to_string()));
    }
    Addrs { addrs, hashes }
}

/// inits contract, with initial balances:
/// * 1000 token_id 0 to addr0
pub fn init_helper_default() -> (
    StdResult<Response>,
    OwnedDeps<MockStorage, MockApi, MockQuerier>,
) {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let info = mock_info("addr0", &[]);

    let init_msg = InstantiateMsg {
        has_admin: true,
        admin: None, // None -> sender defaults as admin
        curators: vec![info.sender.clone()],
        initial_tokens: vec![CurateTokenId::default()],
        entropy: "seedentropy".to_string(),
    };

    (instantiate(deps.as_mut(), env, info, init_msg), deps)
}

/// curate additional:
/// * 800 fungible token_id 0a to addr0,
/// * 500 fungible token_id 1 to addr1,
/// * 1 NFT token_id 2 to addr2
/// * 1 NFT token_id 2a to addr2
pub fn curate_addtl_default(
    deps: &mut OwnedDeps<MockStorage, MockApi, MockQuerier>,
    env: Env,
    info: MessageInfo,
) -> StdResult<()> {
    // init addtl addresses
    let addr0 = Addr::unchecked("addr0".to_string());
    let addr1 = Addr::unchecked("addr1".to_string());
    let addr2 = Addr::unchecked("addr2".to_string());

    // fungible token_id "0a"
    let mut curate0a = CurateTokenId::default();
    curate0a.token_info.token_id = "0a".to_string();
    curate0a.token_info.name = "token0a".to_string();
    curate0a.token_info.symbol = "TKNO".to_string();
    curate0a.balances[0].address = addr0;
    curate0a.balances[0].amount = Uint256::from(800u128);

    // fungible token_id "1"
    let mut curate1 = CurateTokenId::default();
    curate1.token_info.token_id = "1".to_string();
    curate1.token_info.name = "token1".to_string();
    curate1.token_info.symbol = "TKNA".to_string();
    curate1.balances[0].address = addr1;
    curate1.balances[0].amount = Uint256::from(500u128);

    // NFT "2"
    let mut curate2 = CurateTokenId::default();
    curate2.token_info.token_id = "2".to_string();
    curate2.token_info.name = "token2".to_string();
    curate2.token_info.symbol = "TKNB".to_string();
    curate2.token_info.token_config = TknConfig::default_nft();
    curate2.balances = vec![TokenIdBalance {
        address: addr2.clone(),
        amount: Uint256::from(1u128),
    }];

    // NFT "2a"
    let mut curate2a = CurateTokenId::default();
    curate2a.token_info.token_id = "2a".to_string();
    curate2a.token_info.name = "token2a".to_string();
    curate2a.token_info.symbol = "TKNBA".to_string();
    curate2a.token_info.token_config = TknConfig::default_nft();
    curate2a.balances = vec![TokenIdBalance {
        address: addr2,
        amount: Uint256::from(1u128),
    }];

    // batch curate token_id "0a", "1", NFT "2" and NFT "3"
    let msg = ExecuteMsg::CurateTokenIds {
        initial_tokens: vec![curate0a, curate1, curate2, curate2a],
        memo: None,
        padding: None,
    };
    execute(deps.as_mut(), env, info, msg)?;

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

pub fn _extract_log(resp: StdResult<Response>) -> String {
    match resp {
        Ok(response) => response.attributes[0].value.clone(),
        Err(_err) => "These are not the logs you are looking for".to_string(),
    }
}

/// checks token balance. Token_id input takes `&str` input, which converts to `String`
pub fn chk_bal(storage: &dyn Storage, token_id_str: &str, address: &Addr) -> Option<Uint256> {
    balances_r(storage, token_id_str)
        .may_load(to_binary(&address).unwrap().as_slice())
        .unwrap()
}

pub fn extract_cosmos_msg<U: DeserializeOwned>(
    message: &CosmosMsg,
) -> StdResult<(U, Option<Addr>, &String)> {
    let (receiver_addr, receiver_hash, msg) = match message {
        CosmosMsg::Wasm(i) => match i {
            WasmMsg::Execute {
                contract_addr,
                code_hash,
                msg,
                ..
            } => (Some(contract_addr), code_hash, msg),
            WasmMsg::Instantiate { code_hash, msg, .. } => (None, code_hash, msg),
            _ => {
                return Err(StdError::generic_err(
                    "unable to extract msg from CosmosMsg",
                ))
            }
        },
        _ => {
            return Err(StdError::generic_err(
                "unable to extract msg from CosmosMsg",
            ))
        }
    };
    let decoded_msg: U = from_binary(msg).unwrap();
    Ok((
        decoded_msg,
        receiver_addr.map(Addr::unchecked),
        receiver_hash,
    ))
}

/// generates an array of viewing keys (as Strings)
pub fn generate_viewing_keys(
    deps: &mut OwnedDeps<MockStorage, MockApi, MockQuerier>,
    env: Env,
    info: MessageInfo,
    addresses: Vec<Addr>,
) -> StdResult<Vks> {
    let mut vks: Vec<String> = vec![];
    let mut info = info;
    for address in addresses {
        info.sender = address;
        let msg = ExecuteMsg::CreateViewingKey {
            entropy: "askdjlm".to_string(),
            padding: None,
        };
        let response = execute(deps.as_mut(), env.clone(), info.to_owned(), msg)?;
        let vk = from_binary::<ExecuteAnswer>(&response.data.unwrap())?;
        if let ExecuteAnswer::CreateViewingKey { key } = vk {
            vks.push(key.to_string())
        } else {
            return Err(StdError::generic_err("no viewing key generated"));
        }
    }

    for i in 0..vks.len() {
        if i == 0 {
            continue;
        };
        assert_ne!(
            vks[i],
            vks[i - 1],
            "viewing keys of two different addresses are similar"
        );
    }

    Ok(Vks { vks })
}

/// Unfortunately only reads the sha_256 hash of the viewing key. Contract does not store viewing key
pub fn read_viewing_key_hash(store: &dyn Storage, owner: &str) -> Option<Vec<u8>> {
    let vk_store = ReadonlyPrefixedStorage::new(store, ViewingKey::STORAGE_KEY);
    vk_store.get(owner.as_bytes())
}

pub struct Vks {
    vks: Vec<String>,
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
