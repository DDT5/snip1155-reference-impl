use core::panic;
use std::ops::Add;
use serde_json::to_string;

use super::{
    testhelpers::*
};

use super::super::{
    handles::*,
    queries::*,
    msg::*,
    state::{
        permissions::*,
        expiration::*,
    },
};

use cosmwasm_std::{
    testing::*, 
    StdResult, 
    InitResponse, 
    HumanAddr, 
    from_binary, Uint128,
};

/////////////////////////////////////////////////////////////////////////////////
// Tests
/////////////////////////////////////////////////////////////////////////////////

#[test]
fn test_q_init() -> StdResult<()> {
    // init addresses
    let addr0 = HumanAddr("addr0".to_string());

    // instantiate
    let (init_result, mut deps) = init_helper_default();
    assert_eq!(init_result.unwrap(), InitResponse::default());

    // check contract info
    let msg = QueryMsg::ContractInfo {  };
    let q_result = query(&deps, msg);
    let q_answer = from_binary::<QueryAnswer>(&q_result?)?;
    match q_answer {
        QueryAnswer::ContractInfo { admin, curators, all_token_ids } => {
            assert_eq!(&admin.unwrap(), &addr0);
            assert_eq!(&curators, &vec![addr0.clone()]);
            assert_eq!(&all_token_ids, &vec!["0".to_string()]);
        }
        _ => panic!("query error")
    }

    // set_viewing_key
    let env = mock_env("addr0", &[]);
    let msg = HandleMsg::SetViewingKey { key: "vkey".to_string(), padding: None };
    handle(&mut deps, env, msg)?;

    // query balance
    let msg = QueryMsg::Balance { owner: addr0.clone(), viewer: addr0, key: "vkey".to_string(), token_id: "0".to_string() };
    let q_result = query(&deps, msg);
    let q_answer = from_binary::<QueryAnswer>(&q_result?)?;
    match q_answer {
        QueryAnswer::Balance { amount } => assert_eq!(amount, Uint128(1000)),
        _ => panic!("query error")
    }

    Ok(())
}

#[test]
fn test_q_permission() -> StdResult<()> {
    // init addresses
    let addr0 = HumanAddr("addr0".to_string());
    let addr1 = HumanAddr("addr1".to_string());

    // instantiate
    let (_init_result, mut deps) = init_helper_default();

    // give permission to transfer: addr0 grants addr1
    let mut env = mock_env("addr0", &[]);
    let msg0_perm_1 = HandleMsg::GivePermission { 
        allowed_address: addr1.clone(), 
        token_id: "0".to_string(), 
        view_balance: Some(true), view_balance_expiry: None,
        view_private_metadata: None, view_private_metadata_expiry: None,
        transfer: Some(Uint128(10)), transfer_expiry: None,
        padding: None, 
    };  
    handle(&mut deps, env.clone(), msg0_perm_1)?;

    // query permission fails: no viewing key
    let msg_q = QueryMsg::Permission { owner: addr0.clone(), allowed_address: addr1.clone(), key: "vkey".to_string(), token_id: "0".to_string() };
    let q_result = query(&deps, msg_q.clone());
    let q_answer = from_binary::<QueryAnswer>(&q_result?)?;
    match q_answer {
        QueryAnswer::ViewingKeyError { msg } => assert!(msg.contains("Wrong viewing key for this address or viewing key not set")),
        _ => panic!("query error")
    }

    // query permission succeeds with owner's viewing key
    // i) set_viewing_key
    env.message.sender = addr0.clone();
    let msg_vk = HandleMsg::SetViewingKey { key: "vkey".to_string(), padding: None };
    handle(&mut deps, env.clone(), msg_vk)?;
    // ii) query permissions
    let q_result = query(&deps, msg_q);
    let q_answer = from_binary::<QueryAnswer>(&q_result?)?;
    match q_answer {
        QueryAnswer::Permission(perm
        ) => assert_eq!(perm.unwrap_or_default(), 
                Permission { 
                    view_balance_perm: true, view_balance_exp: Expiration::default(), 
                    view_pr_metadata_perm: false, view_pr_metadata_exp: Expiration::default(),  
                    trfer_allowance_perm: Uint128(10), trfer_allowance_exp: Expiration::default(), 
                }
            ),
        _ => panic!("query error")
    }

    // query permission succeeds with perm_addr's viewing key
    // i) set_viewing_key
    env.message.sender = addr1.clone();
    let msg_vk2 = HandleMsg::SetViewingKey { key: "vkey2".to_string(), padding: None };
    handle(&mut deps, env, msg_vk2)?;
    // ii) query permissions
    let msg_q2 = QueryMsg::Permission { owner: addr0, allowed_address: addr1, key: "vkey2".to_string(), token_id: "0".to_string() };
    let q_result = query(&deps, msg_q2);
    let q_answer = from_binary::<QueryAnswer>(&q_result?)?;
    match q_answer {
        QueryAnswer::Permission(perm
        ) => assert_eq!(
                perm.unwrap_or_default(), 
                Permission { 
                    view_balance_perm: true, view_balance_exp: Expiration::default(), 
                    view_pr_metadata_perm: false, view_pr_metadata_exp: Expiration::default(),  
                    trfer_allowance_perm: Uint128(10), trfer_allowance_exp: Expiration::default(), 
                }
            ),
        _ => panic!("query error")
    }

    Ok(())
}

#[test]
fn test_query_balance() -> StdResult<()> {
    // init addresses
    let addr = init_addrs();

    // instantiate + curate more tokens
    let (_init_result, mut deps) = init_helper_default();
    let mut env = mock_env("addr0", &[]);
    curate_addtl_default(&mut deps, &env)?;

    // cannot view balance without viewing keys
    let msg0_q_bal0_novk = QueryMsg::Balance { owner: addr.a(), viewer: addr.a(), key: "vkeya".to_string(), token_id: "0".to_string() };
    let q_answer = from_binary::<QueryAnswer>(&query(&deps, msg0_q_bal0_novk)?)?;
    match q_answer {
        QueryAnswer::ViewingKeyError { msg } => assert!(msg.contains("Wrong viewing key for this address or viewing key not set")),
        _ => panic!("query error")
    }

    // owner can view balance with viewing keys
    // i) generate all viewing keys
    let vks = generate_viewing_keys(&mut deps, &env, addr.all())?; 

    // ii) query
    let msg0_q_bal0 = QueryMsg::Balance { owner: addr.a(), viewer: addr.a(), key: vks.a(), token_id: "0".to_string() };
    let q_answer = from_binary::<QueryAnswer>(&query(&deps, msg0_q_bal0.clone())?)?;
    match q_answer {
        QueryAnswer::Balance { amount } => assert_eq!(amount, Uint128(1000)),
        _ => panic!("query error")
    }

    // addr1 cannot view a's balance with b's viewing keys
    let msg1_q_bal0 = QueryMsg::Balance { owner: addr.a(), viewer: addr.b(), key: vks.a(), token_id: "0".to_string() };
    let mut q_result = query(&deps, msg1_q_bal0.clone());
    assert!(extract_error_msg(&q_result).contains("you do have have permission to view balance"));

    // `b` cannot view `a`'s balance using `b` viewing keys, if `a` gives wrong permission
    let msg_perm_1_wrong = HandleMsg::GivePermission { 
        allowed_address: addr.b(), 
        token_id: "0".to_string(), 
        view_balance: None, view_balance_expiry: None,
        view_private_metadata: Some(true), view_private_metadata_expiry: None,
        transfer: Some(Uint128(1000)), transfer_expiry: None,
        padding: None, 
    };  
    handle(&mut deps, env.clone(), msg_perm_1_wrong)?;
    q_result = query(&deps, msg1_q_bal0.clone());
    assert!(extract_error_msg(&q_result).contains("you do have have permission to view balance"));

    // `b` can view `a`'s balance using `b` viewing keys, once `a` gives correct permission
    env.message.sender = addr.a();
    let msg_perm_1 = HandleMsg::GivePermission { 
        allowed_address: addr.b(), 
        token_id: "0".to_string(), 
        view_balance: Some(true), view_balance_expiry: Some(Expiration::AtHeight(env.block.height.add(1))),
        view_private_metadata: None, view_private_metadata_expiry: None,
        transfer: None, transfer_expiry: None,
        padding: None, 
    };
    handle(&mut deps, env.clone(), msg_perm_1)?;
    let q_answer = from_binary::<QueryAnswer>(&query(&deps, msg1_q_bal0.clone())?)?;
    match q_answer {
        QueryAnswer::Balance { amount } => assert_eq!(amount, Uint128(1000)),
        _ => panic!("query error")
    }

    // `b` cannot view `a`'s token_id "0a" balance, because only got permission for token_id "0"...
    let msg1_q_bal0_0a = QueryMsg::Balance { owner: addr.a(), viewer: addr.b(), key: vks.b(), token_id: "0a".to_string() };
    q_result = query(&deps, msg1_q_bal0_0a);
    assert!(extract_error_msg(&q_result).contains("you do have have permission to view balance"));

    // ... but `a` can still view its own tokens
    let msg0_q_bal0_0a = QueryMsg::Balance { owner: addr.a(), viewer: addr.a(), key: vks.a(), token_id: "0a".to_string() };
    let q_answer = from_binary::<QueryAnswer>(&query(&deps, msg0_q_bal0_0a)?)?;
    match q_answer {
        QueryAnswer::Balance { amount } => assert_eq!(amount, Uint128(800)),
        _ => panic!("query error")
    }

    // `c` cannot view `a`'s balance, because `a` gave permission only to `b`
    let msg2_q_bal0 = QueryMsg::Balance { owner: addr.a(), viewer: addr.c(), key: vks.c(), token_id: "0a".to_string() };
    q_result = query(&deps, msg2_q_bal0);
    assert!(extract_error_msg(&q_result).contains("you do have have permission to view balance"));

    // `b` cannot view `a`'s balance using `b` viewing keys, because [correct] permission expired
    // i) add block height
    env.block.height += 2;
    q_result = query(&deps, msg1_q_bal0.clone());
    assert!(q_result.is_ok());
    // ii) a handle must happen in order to trigger the block height change (won't be required once upgraded to CosmWasm v1.0)
    let random_msg = HandleMsg::AddCurators { add_curators: vec![], padding: None };
    handle(&mut deps, env, random_msg)?;
    // iii) query now
    q_result = query(&deps, msg1_q_bal0);
    assert!(extract_error_msg(&q_result).contains("you do have have permission to view balance"));

    // `a` can still view owns own balance, even after permission given to `b` has expired
    let q_answer = from_binary::<QueryAnswer>(&query(&deps, msg0_q_bal0)?)?;
    match q_answer {
        QueryAnswer::Balance { amount } => assert_eq!(amount, Uint128(1000)),
        _ => panic!("query error")
    }

    Ok(())
}

#[test]
fn test_query_tokenid_private_info() -> StdResult<()> {
    // init addresses
    let addr = init_addrs();

    // instantiate
    let (_init_result, mut deps) = init_helper_default();

    // generate viewing keys
    let env = mock_env("addr0", &[]);
    let vks = generate_viewing_keys(&mut deps, &env, vec![addr.a()])?;

    // view private info of fungible token
    let msg = QueryMsg::TokenIdPrivateInfo { address: addr.a(), key: vks.a(), token_id: "0".to_string() };
    let q_result = query(&deps, msg);
    let q_answer = from_binary::<QueryAnswer>(&q_result?)?;
    match q_answer {
        QueryAnswer::TokenIdPrivateInfo { token_id_info, total_supply, owner 
        } => {
            assert!(to_string(&token_id_info).unwrap().contains("\"public_metadata\":{\"token_uri\":\"public uri\""));
            assert!(to_string(&token_id_info).unwrap().contains("\"private_metadata\":{\"token_uri\":\"private uri\""));
            assert_eq!(total_supply, Some(Uint128(1000)));
            assert!(owner.is_none());
        },
        _ => panic!("query error"),
    }

    Ok(())
}