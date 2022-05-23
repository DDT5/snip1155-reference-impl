use core::panic;
// use serde_json;

use crate::state::Permission;

use super::{
    testhelpers::*
};

use super::super::{
    contract::*,
    msg::*,
    // state::*,
    expiration::Expiration,
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
        QueryAnswer::ContractInfo { admin, minters, all_token_ids } => {
            assert_eq!(&admin.unwrap(), &addr0);
            assert_eq!(&minters, &vec![addr0.clone()]);
            assert_eq!(&all_token_ids, &vec!["0".to_string()]);
        }
        _ => panic!("query error")
    }

    // set_viewing_key
    let env = mock_env("addr0", &[]);
    let msg = HandleMsg::SetViewingKey { key: "vkey".to_string(), padding: None };
    handle(&mut deps, env.clone(), msg)?;

    // query balance
    let msg = QueryMsg::Balance { address: addr0, key: "vkey".to_string(), token_id: "0".to_string() };
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

    // give permission to transfer
    let mut env = mock_env("addr0", &[]);
    let msg0_perm_1 = HandleMsg::GivePermission { 
        allowed_address: addr1.clone(), 
        token_id: "0".to_string(), 
        view_owner: Some(true), view_private_metadata: None, 
        transfer: Some(Uint128(10)), 
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
        ) => assert_eq!(perm, 
                Permission { 
                    view_owner_perm: true, view_owner_exp: Expiration::default(), 
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
    handle(&mut deps, env.clone(), msg_vk2)?;
    // ii) query permissions
    let msg_q2 = QueryMsg::Permission { owner: addr0.clone(), allowed_address: addr1, key: "vkey2".to_string(), token_id: "0".to_string() };
    let q_result = query(&deps, msg_q2);
    let q_answer = from_binary::<QueryAnswer>(&q_result?)?;
    match q_answer {
        QueryAnswer::Permission(ref perm
        ) => assert_eq!(
                perm, 
                &Permission { 
                    view_owner_perm: true, view_owner_exp: Expiration::default(), 
                    view_pr_metadata_perm: false, view_pr_metadata_exp: Expiration::default(),  
                    trfer_allowance_perm: Uint128(10), trfer_allowance_exp: Expiration::default(), 
                }
            ),
        _ => panic!("query error")
    }

    Ok(())
}
