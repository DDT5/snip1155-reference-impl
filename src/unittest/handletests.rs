use std::any::Any;

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
    to_binary, 
};


/////////////////////////////////////////////////////////////////////////////////
// Helper functions
/////////////////////////////////////////////////////////////////////////////////

fn init_helper_default() -> (
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

fn mint_addtl_default<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: &Env,
) -> StdResult<()> {
    // init addtl addresses
    let addr1 = HumanAddr("addr1".to_string());
    let addr2 = HumanAddr("addr2".to_string());

    // mint fungible token_id
    let mut mint = MintTokenId::default();
    mint.token_info.token_id = "1".to_string();
    mint.token_info.name = "token1".to_string();
    mint.token_info.symbol = "TKN1".to_string();
    mint.balances[0].address = addr1.clone();
    mint.balances[0].amount = Uint128(500);
    let mut msg = HandleMsg::MintTokenIds{initial_tokens: vec![mint], memo: None, padding: None };
    handle(deps, env.clone(), msg)?;

    // mint NFT
    let mut mint = MintTokenId::default();
    mint.token_info.token_id = "2".to_string();
    mint.token_info.name = "token2".to_string();
    mint.token_info.symbol = "TKN2".to_string();
    mint.token_info.is_nft = true;
    mint.balances = vec![Balance { address: addr2.clone(), amount: Uint128(1) }];
    msg = HandleMsg::MintTokenIds{initial_tokens: vec![mint], memo: None, padding: None };
    handle(deps, env.clone(), msg)?;
    
    Ok(())
}

fn extract_error_msg<T: Any>(error: &StdResult<T>) -> String {
    match error {
        Ok(_response) => panic!("Expected error, but had Ok response"),
        Err(err) => match err {
            StdError::GenericErr { msg, .. } => msg.to_string(),
            _ => panic!("Unexpected error result {:?}", err),
        },
    }
}

fn _extract_log(resp: StdResult<HandleResponse>) -> String {
    match resp {
        Ok(response) => response.log[0].value.clone(),
        Err(_err) => "These are not the logs you are looking for".to_string(),
    }
}

/// checks token balance. Token_id input takes `&str` input, which converts to `String`  
fn chk_bal<S: Storage>(
    storage: &S,
    token_id_str: &str,
    address: &HumanAddr,
) -> Option<Uint128> {
    balances_r(storage, &token_id_str.to_string())
    .may_load(to_binary(&address).unwrap().as_slice()).unwrap()
}

/////////////////////////////////////////////////////////////////////////////////
// Tests
/////////////////////////////////////////////////////////////////////////////////

#[test]
fn init_sanity() -> StdResult<()> {
    // init addresses
    let addr0 = HumanAddr("addr0".to_string());

    // instantiate
    let (init_result, deps) = init_helper_default();
    assert_eq!(init_result.unwrap(), InitResponse::default());
    
    // check contract config
    let contr_conf = contr_conf_r(&deps.storage).load()?;
    assert_eq!(contr_conf.admin.unwrap(), addr0);
    assert_eq!(contr_conf.minters, vec![addr0.clone()]);
    // 1 minting could have happened, so tx_cnt should == 1:
    assert_eq!(contr_conf.tx_cnt, 1u64);
    let token_id = "0".to_string();
    
    // check initial balances
    let balance = balances_r(&deps.storage, &token_id).load(to_binary(&addr0)?.as_slice())?;
    assert_eq!(balance, Uint128(1000));
    Ok(())
}


#[test]
fn mint_token_id_sanity() -> StdResult<()> {
    // init addresses
    let addr0 = HumanAddr("addr0".to_string());
    let addr1 = HumanAddr("addr1".to_string());
    let addr2 = HumanAddr("addr2".to_string());

    // instantiate
    let (_init_result, mut deps) = init_helper_default();

    // mint additional token_ids
    let env = mock_env("addr0", &[]);
    mint_addtl_default(&mut deps, &env)?;
    
    // check balances
    assert_eq!(chk_bal(&deps.storage, "0", &addr0).unwrap(), Uint128(1000));
    assert_eq!(chk_bal(&deps.storage, "1", &addr1).unwrap(), Uint128(500));
    assert_eq!(chk_bal(&deps.storage, "2", &addr2).unwrap(), Uint128(1));
    // 1 initial balance, 2 mint_token_id 
    assert_eq!(contr_conf_r(&deps.storage).load()?.tx_cnt, 3u64);

    // initial balance comprehensive check 
    assert_eq!(chk_bal(&deps.storage, "0", &addr0).unwrap(), Uint128(1000));
    assert_eq!(chk_bal(&deps.storage, "0", &addr1), None); 
    assert_eq!(chk_bal(&deps.storage, "0", &addr2), None);
    assert_eq!(chk_bal(&deps.storage, "1", &addr0), None);
    assert_eq!(chk_bal(&deps.storage, "1", &addr1).unwrap(), Uint128(500));
    assert_eq!(chk_bal(&deps.storage, "1", &addr2), None);
    assert_eq!(chk_bal(&deps.storage, "2", &addr1), None);
    assert_eq!(chk_bal(&deps.storage, "2", &addr1), None);
    assert_eq!(chk_bal(&deps.storage, "2", &addr2).unwrap(), Uint128(1));

    Ok(())
}

#[test]
fn test_mint_token_id() -> StdResult<()> {
    // init addresses
    let addr0 = HumanAddr("addr0".to_string());
    let addr1 = HumanAddr("addr1".to_string());
    let addr2 = HumanAddr("addr2".to_string());

    // instantiate
    let (_init_result, mut deps) = init_helper_default();

    // mint additional token_ids
    let mut env = mock_env("addr0", &[]);
    mint_addtl_default(&mut deps, &env)?;
    
    // cannot mint more than 1 nft; address != 1
    let mut mint = MintTokenId::default();
    mint.token_info.token_id = "3".to_string();
    mint.token_info.is_nft = true;
    mint.balances = vec![
        Balance { address: addr0.clone(), amount: Uint128(1) },
        Balance { address: addr1.clone(), amount: Uint128(1) },
        ];
    let mut msg = HandleMsg::MintTokenIds{initial_tokens: vec![mint], memo: None, padding: None };
    let mut result = handle(&mut deps, env.clone(), msg);
    assert!(extract_error_msg(&result).contains("is an NFT; there can only be one NFT. Balances should only have one address"));

    // cannot mint more than 1 nft; amount != 1
    let mut mint = MintTokenId::default();
    mint.token_info.token_id = "4".to_string();
    mint.token_info.is_nft = true;
    mint.balances[0].amount = Uint128(2);
    msg = HandleMsg::MintTokenIds{initial_tokens: vec![mint], memo: None, padding: None };
    result = handle(&mut deps, env.clone(), msg);
    assert!(extract_error_msg(&result).contains("is an NFT; there can only be one NFT. Balances.amount must == 1"));

    // non-minter cannot mint
    env.message.sender = addr1.clone();
    let mut mint = MintTokenId::default();
    mint.token_info.token_id = "5".to_string();
    msg = HandleMsg::MintTokenIds{initial_tokens: vec![mint], memo: None, padding: None };
    result = handle(&mut deps, env, msg);
    assert!(extract_error_msg(&result).contains("Only minters are allowed to mint"));

    // check balances
    assert_eq!(chk_bal(&deps.storage, "0", &addr0).unwrap(), Uint128(1000));
    assert_eq!(chk_bal(&deps.storage, "1", &addr1).unwrap(), Uint128(500));
    assert_eq!(chk_bal(&deps.storage, "2", &addr2).unwrap(), Uint128(1));
    assert_eq!(chk_bal(&deps.storage, "3", &addr0), None); assert_eq!(chk_bal(&deps.storage, "3", &addr1), None);
    assert_eq!(chk_bal(&deps.storage, "4", &addr0), None);
    assert_eq!(chk_bal(&deps.storage, "5", &addr0), None);
    // 1 initial balance, 2 mint_token_id, 0 additional
    assert_eq!(contr_conf_r(&deps.storage).load()?.tx_cnt, 3u64);

    Ok(())
}

#[test]
fn test_mint_tokens() -> StdResult<()> {
    // init addresses
    let addr0 = HumanAddr("addr0".to_string());
    let addr1 = HumanAddr("addr1".to_string());

    // instantiate
    let (_init_result, mut deps) = init_helper_default();

    // mint additional token_ids
    let mut env = mock_env("addr0", &[]);
    mint_addtl_default(&mut deps, &env)?;

    // mint more fungible tokens to multiple addresses
    let mint = TokenAmount { 
        token_id: "0".to_string(), 
        balances: vec![
            Balance { address: addr0.clone(), amount: Uint128(10) },
            Balance { address: addr1.clone(), amount: Uint128(10) }
        ],
    };
    let msg = HandleMsg::MintTokens{ mint_tokens: vec![mint], memo: None, padding: None };
    handle(&mut deps, env.clone(), msg.clone())?;
    assert_eq!(chk_bal(&deps.storage, "0", &addr0).unwrap(), Uint128(1010));
    assert_eq!(chk_bal(&deps.storage, "0", &addr1).unwrap(), Uint128(10));
    // 1 initial balance, 2 mint_token_id, 2 mint_token 
    assert_eq!(contr_conf_r(&deps.storage).load()?.tx_cnt, 5u64);

    // non-minter cannot mint
    env.message.sender = addr1;
    let result = handle(&mut deps, env.clone(), msg);
    assert!(extract_error_msg(&result).contains("Only minters are allowed to mint"));

    // cannot mint additional nfts
    env.message.sender = addr0.clone();
    let mint = TokenAmount { 
        token_id: "2".to_string(), 
        balances: vec![Balance { address: addr0.clone(), amount: Uint128(1) }],
    };
    let msg = HandleMsg::MintTokens{ mint_tokens: vec![mint], memo: None, padding: None };
    let result = handle(&mut deps, env, msg);
    assert!(extract_error_msg(&result).contains("NFTs can only be minted once using `mint_token_ids`"));
    assert_eq!(chk_bal(&deps.storage, "0", &addr0).unwrap(), Uint128(1010));
    // 1 initial balance, 2 mint_token_id, 2 mint_token 
    assert_eq!(contr_conf_r(&deps.storage).load()?.tx_cnt, 5u64);
    
    Ok(())
}

#[test]
fn test_burn() -> StdResult<()> {
    // init addresses
    let addr0 = HumanAddr("addr0".to_string());
    let addr1 = HumanAddr("addr1".to_string());
    let addr2 = HumanAddr("addr2".to_string());

    // instantiate
    let (_init_result, mut deps) = init_helper_default();

    // mint additional token_ids
    let mut env = mock_env("addr0", &[]);
    mint_addtl_default(&mut deps, &env)?;

    // initial balance check 
    assert_eq!(chk_bal(&deps.storage, "0", &addr0).unwrap(), Uint128(1000));
    assert_eq!(chk_bal(&deps.storage, "1", &addr1).unwrap(), Uint128(500));
    assert_eq!(chk_bal(&deps.storage, "2", &addr2).unwrap(), Uint128(1));

    // burn tokens of another address => should fail
    let burn = TokenAmount { 
        token_id: "1".to_string(), 
        balances: vec![
            Balance { address: addr1.clone(), amount: Uint128(501) }
        ],
    };
    let msg = HandleMsg::BurnTokens{ burn_tokens: vec![burn], memo: None, padding: None };
    let mut result = handle(&mut deps, env.clone(), msg.clone());
    assert!(extract_error_msg(&result).contains("you do not have permission to burn "));

    // burn more tokens than available => should fail
    env.message.sender = addr1.clone();
    result = handle(&mut deps, env.clone(), msg.clone());
    assert!(extract_error_msg(&result).contains("insufficient funds"));

    // burn fungible tokens should work
    let burn = TokenAmount { 
        token_id: "1".to_string(), 
        balances: vec![
            Balance { address: addr1.clone(), amount: Uint128(300) }
        ],
    };
    let msg = HandleMsg::BurnTokens{ burn_tokens: vec![burn], memo: None, padding: None };
    handle(&mut deps, env.clone(), msg.clone())?;
    assert_eq!(chk_bal(&deps.storage, "1", &addr1).unwrap(), Uint128(200));
    
    // burn nft should work
    env.message.sender = addr2.clone();
    let burn = TokenAmount { 
        token_id: "2".to_string(), 
        balances: vec![
            Balance { address: addr2.clone(), amount: Uint128(1) }
        ],
    };
    let msg = HandleMsg::BurnTokens{ burn_tokens: vec![burn], memo: None, padding: None };
    handle(&mut deps, env.clone(), msg.clone())?;
    assert_eq!(chk_bal(&deps.storage, "2", &addr2).unwrap(), Uint128(0));

    // 1 initial balance, 2 mint_token_id, 2 burns 
    assert_eq!(contr_conf_r(&deps.storage).load()?.tx_cnt, 5u64);

    Ok(())
}

#[test]
fn test_transfer() -> StdResult<()> {
    // init addresses
    let addr0 = HumanAddr("addr0".to_string());
    let addr1 = HumanAddr("addr1".to_string());
    let addr2 = HumanAddr("addr2".to_string());

    // instantiate
    let (_init_result, mut deps) = init_helper_default();

    // mint additional token_ids
    let mut env = mock_env("addr0", &[]);
    mint_addtl_default(&mut deps, &env)?;

    // initial balance check 
    assert_eq!(chk_bal(&deps.storage, "2", &addr2).unwrap(), Uint128(1));
    assert_eq!(chk_bal(&deps.storage, "2", &addr1), None);
    assert_eq!(chk_bal(&deps.storage, "0", &addr0).unwrap(), Uint128(1000));
    assert_eq!(chk_bal(&deps.storage, "0", &addr1), None);

    // transfer fungible token "tkn0"
    let msg = HandleMsg::Transfer { 
        token_id: "0".to_string(), 
        from: addr0.clone(), 
        recipient: addr1.clone(), 
        amount: Uint128(800),
        memo: None,
        padding: None, 
    };
    handle(&mut deps, env.clone(), msg.clone())?;
    assert_eq!(chk_bal(&deps.storage, "0", &addr0).unwrap(), Uint128(200));
    assert_eq!(chk_bal(&deps.storage, "0", &addr1).unwrap(), Uint128(800));

    // cannot transfer if not owner
    env.message.sender = addr2.clone();
    let result = handle(&mut deps, env.clone(), msg);
    assert!(extract_error_msg(&result).contains("you need to either be the owner of or have permission to transfer the tokens"));

    // transfer NFT "tkn2"; amount != 1
    env.message.sender = addr2.clone();
    let msg = HandleMsg::Transfer { 
        token_id: "2".to_string(), 
        from: addr2.clone(), 
        recipient: addr1.clone(), 
        amount: Uint128(0),
        memo: None,
        padding: None, 
    };
    let result = handle(&mut deps, env.clone(), msg);
    assert!(extract_error_msg(&result).contains("NFT amount must == 1"));

    // transfer NFT "tkn2"; should succeed
    let msg = HandleMsg::Transfer { 
        token_id: "2".to_string(), 
        from: addr2.clone(), 
        recipient: addr1.clone(), 
        amount: Uint128(1),
        memo: None,
        padding: None, 
    };
    handle(&mut deps, env.clone(), msg)?;

    // final balance check 
    assert_eq!(chk_bal(&deps.storage, "2", &addr2).unwrap(), Uint128(0));
    assert_eq!(chk_bal(&deps.storage, "2", &addr1).unwrap(), Uint128(1));
    assert_eq!(chk_bal(&deps.storage, "0", &addr0).unwrap(), Uint128(200));
    assert_eq!(chk_bal(&deps.storage, "0", &addr1).unwrap(), Uint128(800));
    // 1 initial balance, 2 mint_token_id, 2 transfers 
    assert_eq!(contr_conf_r(&deps.storage).load()?.tx_cnt, 5u64);

    Ok(())
}
