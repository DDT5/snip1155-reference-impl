use super::{
    testhelpers::*
};

use super::super::{
    contract::*,
    msg::*,
    state::*,
    receiver::{Snip1155ReceiveMsg, ReceiverHandleMsg},
    expiration::Expiration,
    metadata::{Metadata, Extension},
};

use cosmwasm_std::{
    testing::*, 
    StdResult, 
    InitResponse, 
    HumanAddr, Uint128, 
    to_binary, from_binary, 
};

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
    assert_eq!(contr_conf.curators, vec![addr0.clone()]);
    // 1 minting could have happened, so tx_cnt should == 1:
    assert_eq!(contr_conf.tx_cnt, 1u64);
    let token_id = "0".to_string();
    
    // check initial balances
    let balance = balances_r(&deps.storage, &token_id).load(to_binary(&addr0)?.as_slice())?;
    assert_eq!(balance, Uint128(1000));
    
    Ok(())
}


#[test]
fn curate_token_id_sanity() -> StdResult<()> {
    // init addresses
    let addr0 = HumanAddr("addr0".to_string());
    let addr1 = HumanAddr("addr1".to_string());
    let addr2 = HumanAddr("addr2".to_string());

    // instantiate
    let (_init_result, mut deps) = init_helper_default();

    // curate additional token_ids
    let env = mock_env("addr0", &[]);
    curate_addtl_default(&mut deps, &env)?;
    
    // check balances
    assert_eq!(chk_bal(&deps.storage, "0", &addr0).unwrap(), Uint128(1000));
    assert_eq!(chk_bal(&deps.storage, "0a", &addr0).unwrap(), Uint128(800));
    assert_eq!(chk_bal(&deps.storage, "1", &addr1).unwrap(), Uint128(500));
    assert_eq!(chk_bal(&deps.storage, "2", &addr2).unwrap(), Uint128(1));
    assert_eq!(chk_bal(&deps.storage, "2a", &addr2).unwrap(), Uint128(1));
    // 1 initial balance, 4 curate_token_id 
    assert_eq!(contr_conf_r(&deps.storage).load()?.tx_cnt, 5u64);

    // initial balance comprehensive check 
    assert_eq!(chk_bal(&deps.storage, "0", &addr0).unwrap(), Uint128(1000));
    assert_eq!(chk_bal(&deps.storage, "0", &addr1), None); 
    assert_eq!(chk_bal(&deps.storage, "0", &addr2), None);
    assert_eq!(chk_bal(&deps.storage, "0a", &addr0).unwrap(), Uint128(800));
    assert_eq!(chk_bal(&deps.storage, "0a", &addr1), None); 
    assert_eq!(chk_bal(&deps.storage, "0a", &addr2), None);
    assert_eq!(chk_bal(&deps.storage, "1", &addr0), None);
    assert_eq!(chk_bal(&deps.storage, "1", &addr1).unwrap(), Uint128(500));
    assert_eq!(chk_bal(&deps.storage, "1", &addr2), None);
    assert_eq!(chk_bal(&deps.storage, "2", &addr1), None);
    assert_eq!(chk_bal(&deps.storage, "2", &addr1), None);
    assert_eq!(chk_bal(&deps.storage, "2", &addr2).unwrap(), Uint128(1));
    assert_eq!(chk_bal(&deps.storage, "2a", &addr1), None);
    assert_eq!(chk_bal(&deps.storage, "2a", &addr1), None);
    assert_eq!(chk_bal(&deps.storage, "2a", &addr2).unwrap(), Uint128(1));

    Ok(())
}

#[test]
fn test_curate_token_id() -> StdResult<()> {
    // init addresses
    let addr0 = HumanAddr("addr0".to_string());
    let addr1 = HumanAddr("addr1".to_string());
    let addr2 = HumanAddr("addr2".to_string());

    // instantiate
    let (_init_result, mut deps) = init_helper_default();

    // curate additional token_ids
    let mut env = mock_env("addr0", &[]);
    curate_addtl_default(&mut deps, &env)?;
    
    // cannot mint more than 1 nft; address != 1
    let mut curate = CurateTokenId::default();
    curate.token_info.token_id = "testa".to_string();
    curate.token_info.token_config = TknConfig::default_nft();
    curate.balances = vec![
        Balance { address: addr0.clone(), amount: Uint128(1) },
        Balance { address: addr1.clone(), amount: Uint128(1) },
        ];
    let mut msg = HandleMsg::CurateTokenIds{initial_tokens: vec![curate], memo: None, padding: None };
    let mut result = handle(&mut deps, env.clone(), msg);
    assert!(extract_error_msg(&result).contains("is an NFT; there can only be one NFT. Balances should only have one address"));

    // cannot mint more than 1 nft; amount != 1
    let mut curate = CurateTokenId::default();
    curate.token_info.token_id = "testb".to_string();
    curate.token_info.token_config = TknConfig::default_nft();
    curate.balances[0].amount = Uint128(2);
    msg = HandleMsg::CurateTokenIds{initial_tokens: vec![curate], memo: None, padding: None };
    result = handle(&mut deps, env.clone(), msg);
    assert!(extract_error_msg(&result).contains("is an NFT; there can only be one NFT. Balances.amount must == 1"));

    // non-curator cannot curate
    env.message.sender = addr1.clone();
    let mut curate = CurateTokenId::default();
    curate.token_info.token_id = "testc".to_string();
    msg = HandleMsg::CurateTokenIds{initial_tokens: vec![curate], memo: None, padding: None };
    result = handle(&mut deps, env, msg);
    assert!(extract_error_msg(&result).contains("Only curators are allowed to curate"));

    // check balances
    assert_eq!(chk_bal(&deps.storage, "0", &addr0).unwrap(), Uint128(1000));
    assert_eq!(chk_bal(&deps.storage, "1", &addr1).unwrap(), Uint128(500));
    assert_eq!(chk_bal(&deps.storage, "2", &addr2).unwrap(), Uint128(1));
    assert_eq!(chk_bal(&deps.storage, "2a", &addr2).unwrap(), Uint128(1));
    assert_eq!(chk_bal(&deps.storage, "testa", &addr0), None); assert_eq!(chk_bal(&deps.storage, "4", &addr1), None);
    assert_eq!(chk_bal(&deps.storage, "testb", &addr0), None);
    assert_eq!(chk_bal(&deps.storage, "testc", &addr0), None);
    // 1 initial balance, 4 curate_token_id, 0 additional
    assert_eq!(contr_conf_r(&deps.storage).load()?.tx_cnt, 5u64);

    Ok(())
}

#[test]
fn test_mint_tokens() -> StdResult<()> {
    // init addresses
    let addr0 = HumanAddr("addr0".to_string());
    let addr1 = HumanAddr("addr1".to_string());

    // instantiate
    let (_init_result, mut deps) = init_helper_default();

    // curate additional token_ids
    let mut env = mock_env("addr0", &[]);
    curate_addtl_default(&mut deps, &env)?;

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
    // 1 initial balance, 4 curate_token_id, 2 mint_token 
    assert_eq!(contr_conf_r(&deps.storage).load()?.tx_cnt, 7u64);

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
    assert!(extract_error_msg(&result).contains("minting is not enabled for this token_id"));
    assert_eq!(chk_bal(&deps.storage, "0", &addr0).unwrap(), Uint128(1010));
    // 1 initial balance, 4 curate_token_id, 2 mint_token 
    assert_eq!(contr_conf_r(&deps.storage).load()?.tx_cnt, 7u64);
    
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
    curate_addtl_default(&mut deps, &env)?;

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
    result = handle(&mut deps, env.clone(), msg);
    assert!(extract_error_msg(&result).contains("insufficient funds"));

    // burn fungible tokens should work
    let burn = TokenAmount { 
        token_id: "1".to_string(), 
        balances: vec![
            Balance { address: addr1.clone(), amount: Uint128(300) }
        ],
    };
    let msg = HandleMsg::BurnTokens{ burn_tokens: vec![burn], memo: None, padding: None };
    handle(&mut deps, env.clone(), msg)?;
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
    handle(&mut deps, env.clone(), msg)?;
    assert_eq!(chk_bal(&deps.storage, "2", &addr2).unwrap(), Uint128(0));

    // 1 initial balance, 4 curate_token_id, 2 burns 
    assert_eq!(contr_conf_r(&deps.storage).load()?.tx_cnt, 7u64);

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
    curate_addtl_default(&mut deps, &env)?;

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
        memo: None, padding: None, 
    };
    handle(&mut deps, env.clone(), msg.clone())?;
    assert_eq!(chk_bal(&deps.storage, "0", &addr0).unwrap(), Uint128(200));
    assert_eq!(chk_bal(&deps.storage, "0", &addr1).unwrap(), Uint128(800));

    // cannot transfer if not owner
    env.message.sender = addr2.clone();
    let result = handle(&mut deps, env.clone(), msg);
    assert!(extract_error_msg(&result).contains("These tokens do not exist or you have no permission to transfer"));

    // transfer NFT "tkn2"; amount != 1
    env.message.sender = addr2.clone();
    let msg = HandleMsg::Transfer { 
        token_id: "2".to_string(), 
        from: addr2.clone(), 
        recipient: addr1.clone(), 
        amount: Uint128(0),
        memo: None, padding: None, 
    };
    let result = handle(&mut deps, env.clone(), msg);
    assert!(extract_error_msg(&result).contains("NFT amount must == 1"));

    // transfer NFT "tkn2"; should succeed
    let msg = HandleMsg::Transfer { 
        token_id: "2".to_string(), 
        from: addr2.clone(), 
        recipient: addr1.clone(), 
        amount: Uint128(1),
        memo: None, padding: None, 
    };
    handle(&mut deps, env.clone(), msg)?;

    // final balance check 
    assert_eq!(chk_bal(&deps.storage, "2", &addr2).unwrap(), Uint128(0));
    assert_eq!(chk_bal(&deps.storage, "2", &addr1).unwrap(), Uint128(1));
    assert_eq!(chk_bal(&deps.storage, "0", &addr0).unwrap(), Uint128(200));
    assert_eq!(chk_bal(&deps.storage, "0", &addr1).unwrap(), Uint128(800));
    // 1 initial balance, 4 curate_token_id, 2 transfers 
    assert_eq!(contr_conf_r(&deps.storage).load()?.tx_cnt, 7u64);

    Ok(())
}

#[test]
fn test_send() -> StdResult<()> {
    // init addresses
    let addr0 = HumanAddr("addr0".to_string());
    let addr1 = HumanAddr("addr1".to_string()); let addr1_h = "addr1_hash".to_string();

    // instantiate
    let (_init_result, mut deps) = init_helper_default();

    // initial balance check 
    assert_eq!(chk_bal(&deps.storage, "0", &addr0).unwrap(), Uint128(1000));

    // send "tkn0" with msg
    let env = mock_env("addr0", &[]);
    let msg = HandleMsg::Send { 
        token_id: "0".to_string(), 
        from: addr0.clone(), 
        recipient: addr1.clone(), 
        recipient_code_hash: Some(addr1_h.clone()),
        amount: Uint128(800),
        msg: Some(to_binary(&"msg_str")?), 
        memo: None, padding: None,
    };
    let response = handle(&mut deps, env, msg)?;
    assert_eq!(chk_bal(&deps.storage, "0", &addr0).unwrap(), Uint128(200));
    assert_eq!(chk_bal(&deps.storage, "0", &addr1).unwrap(), Uint128(800));
    let (receiver_msg, receiver_addr, receiver_hash) = extract_cosmos_msg::<ReceiverHandleMsg>(&response.messages[0])?; 
    assert_eq!(receiver_addr, Some(&addr1)); assert_eq!(receiver_hash, &addr1_h);
    let exp_receive_msg = Snip1155ReceiveMsg {
        sender: addr0.clone(),
        token_id: "0".to_string(),
        from: addr0,
        amount: Uint128(800),
        memo: None,
        msg: Some(to_binary(&"msg_str")?), 
    };
    match receiver_msg {
        ReceiverHandleMsg::Snip1155Receive(i) => assert_eq!(i, exp_receive_msg),
    }

    Ok(())
}

#[test]
fn test_transfer_permissions_fungible() -> StdResult<()> {
    // init addresses
    let addr0 = HumanAddr("addr0".to_string()); let addr0_bin = to_binary(&addr0)?; let _addr0_u8 = addr0_bin.as_slice();
    let addr1 = HumanAddr("addr1".to_string()); let addr1_bin = to_binary(&addr1)?; let addr1_u8 = addr1_bin.as_slice();
    let addr2 = HumanAddr("addr2".to_string()); let addr2_bin = to_binary(&addr2)?;let addr2_u8 = addr2_bin.as_slice();

    // instantiate
    let (_init_result, mut deps) = init_helper_default();

    // initial balance check 
    assert_eq!(chk_bal(&deps.storage, "0", &addr0).unwrap(), Uint128(1000));

    // cannot transfer without allowance
    let mut env = mock_env("addr1", &[]);
    let msg_trnsf_0 = HandleMsg::Transfer {
        token_id: "0".to_string(),
        from: addr0.clone(),
        recipient: addr1.clone(),
        amount: Uint128(10),
        memo: None, padding: None,
    }; 
    let mut result = handle(&mut deps, env.clone(), msg_trnsf_0.clone());
    assert!(extract_error_msg(&result).contains("These tokens do not exist or you have no permission to transfer"));

    // cannot transfer with insufficient allowance
    env.message.sender = addr0.clone();
    let msg0_perm_1 = HandleMsg::GivePermission { 
        allowed_address: addr1.clone(), 
        token_id: "0".to_string(), 
        view_balance: None, view_balance_expiry: None,
        view_private_metadata: None, view_private_metadata_expiry: None,
        transfer: Some(Uint128(9)), transfer_expiry: None,
        padding: None,
    };  
    handle(&mut deps, env.clone(), msg0_perm_1)?;

    env.message.sender = addr1.clone();
    result = handle(&mut deps, env.clone(), msg_trnsf_0.clone());
    assert!(extract_error_msg(&result).contains("Insufficient transfer allowance: 9"));

    // cannot transfer with wrong allowances: wrong spender address: addr2 has the transfer permission
    env.message.sender = addr0.clone();
    let msg0_perm_2 = HandleMsg::GivePermission { 
        allowed_address: addr2.clone(), 
        token_id: "0".to_string(), 
        view_balance: None, view_balance_expiry: None,
        view_private_metadata: None, view_private_metadata_expiry: None,
        transfer: Some(Uint128(15)), transfer_expiry: None,
        padding: None, 
    };  
    handle(&mut deps, env.clone(), msg0_perm_2)?;

    env.message.sender = addr1.clone();
    result = handle(&mut deps, env.clone(), msg_trnsf_0.clone());
    assert!(extract_error_msg(&result).contains("Insufficient transfer allowance: "));

    // cannot transfer with wrong allowances: wrong owner address: addr1 giving permission
    env.message.sender = addr1.clone();
    let msg1_perm_1 = HandleMsg::GivePermission { 
        allowed_address: addr1.clone(), 
        token_id: "0".to_string(), 
        view_balance: None, view_balance_expiry: None,
        view_private_metadata: None, view_private_metadata_expiry: None,
        transfer: Some(Uint128(10)), transfer_expiry: None,
        padding: None, 
    };  
    handle(&mut deps, env.clone(), msg1_perm_1)?;
    result = handle(&mut deps, env.clone(), msg_trnsf_0.clone());
    assert!(extract_error_msg(&result).contains("Insufficient transfer allowance: "));

    // can transfer with enough allowance: addr2 has enough allowance
    env.message.sender = addr2;
    handle(&mut deps, env.clone(), msg_trnsf_0.clone())?;
    assert_eq!(chk_bal(&deps.storage, "0", &addr0).unwrap(), Uint128(990));
    assert_eq!(chk_bal(&deps.storage, "0", &addr1).unwrap(), Uint128(10));

    // allowance gets consumed: cannot exceed allowance with a second tx 
    result = handle(&mut deps, env.clone(), msg_trnsf_0.clone());
    assert!(extract_error_msg(&result).contains("Insufficient transfer allowance: 5"));

    // allowance for different address does not get consumed
    assert_eq!(perm_r(&deps.storage, &addr0, "0").load(addr1_u8)?.trfer_allowance_perm, Uint128(9));
    assert_eq!(perm_r(&deps.storage, &addr0, "0").load(addr2_u8)?.trfer_allowance_perm, Uint128(5));
    assert_eq!(perm_r(&deps.storage, &addr1, "0").load(addr1_u8)?.trfer_allowance_perm, Uint128(10));

    // owner can transfer regardless of allowance
    env.message.sender = addr0.clone();
    handle(&mut deps, env.clone(), msg_trnsf_0.clone())?; handle(&mut deps, env, msg_trnsf_0)?;
    assert_eq!(chk_bal(&deps.storage, "0", &addr0).unwrap(), Uint128(970));
    assert_eq!(chk_bal(&deps.storage, "0", &addr1).unwrap(), Uint128(30));
    // 1 initial balance, 3 transfers 
    assert_eq!(contr_conf_r(&deps.storage).load()?.tx_cnt, 4u64);

    Ok(())
}

#[test]
fn test_transfer_permissions_nft() -> StdResult<()> {
    // init addresses
    let addr0 = HumanAddr("addr0".to_string()); let addr0_bin = to_binary(&addr0)?; let _addr0_u8 = addr0_bin.as_slice();
    let addr1 = HumanAddr("addr1".to_string()); let addr1_bin = to_binary(&addr1)?; let addr1_u8 = addr1_bin.as_slice();
    let addr2 = HumanAddr("addr2".to_string()); let addr2_bin = to_binary(&addr2)?;let _addr2_u8 = addr2_bin.as_slice();

    // instantiate
    let (_init_result, mut deps) = init_helper_default();

    // curate additional token_ids
    let mut env = mock_env("addr0", &[]);
    curate_addtl_default(&mut deps, &env)?;

    // cannot transfer: no permission 
    env.message.sender = addr1.clone();
    let msg1_trnsf_0 = HandleMsg::Transfer { 
        token_id: "2".to_string(), 
        from: addr2.clone(), 
        recipient: addr0.clone(), 
        amount: Uint128(1),
        memo: None, 
        padding: None, 
    };
    let mut result = handle(&mut deps, env.clone(), msg1_trnsf_0.clone());
    assert!(extract_error_msg(&result).contains("These tokens do not exist or you have no permission to transfer"));
    assert_eq!(chk_bal(&deps.storage, "2", &addr2).unwrap(), Uint128(1));
    assert_eq!(chk_bal(&deps.storage, "2", &addr0), None);
    
    // give permission to transfer
    env.message.sender = addr2.clone();
    let msg2_perm_1 = HandleMsg::GivePermission { 
        allowed_address: addr1.clone(), 
        token_id: "2".to_string(), 
        view_balance: None, view_balance_expiry: None,
        view_private_metadata: None, view_private_metadata_expiry: None,
        transfer: Some(Uint128(10)), transfer_expiry: None,
        padding: None, 
    };  
    handle(&mut deps, env.clone(), msg2_perm_1)?;

    // addr1 can now transfer addr2's nft to addr0
    env.message.sender = addr1.clone();
    handle(&mut deps, env.clone(), msg1_trnsf_0.clone())?;
    assert_eq!(chk_bal(&deps.storage, "2", &addr2).unwrap(), Uint128(0));
    assert_eq!(chk_bal(&deps.storage, "2", &addr0).unwrap(), Uint128(1));

    // cannot transfer again: insufficient funds
    result = handle(&mut deps, env.clone(), msg1_trnsf_0);
    assert!(extract_error_msg(&result).contains("insufficient funds"));
    // balance is unchanged
    assert_eq!(chk_bal(&deps.storage, "2", &addr2).unwrap(), Uint128(0));
    assert_eq!(chk_bal(&deps.storage, "2", &addr0).unwrap(), Uint128(1));

    // give permission to transfern token 2a
    env.message.sender = addr2.clone();
    let msg2_perm_1 = HandleMsg::GivePermission { 
        allowed_address: addr1.clone(), 
        token_id: "2a".to_string(), 
        view_balance: None, view_balance_expiry: None,
        view_private_metadata: None, view_private_metadata_expiry: None,
        transfer: Some(Uint128(1)), transfer_expiry: None,
        padding: None, 
    };  
    handle(&mut deps, env.clone(), msg2_perm_1)?;
    // double check that addr1 has permission to transfer token 2a
    assert_eq!(
        perm_r(&deps.storage, &addr2, "2a").load(addr1_u8)?, 
        Permission { 
            view_balance_perm: false, view_balance_exp: Expiration::default(), 
            view_pr_metadata_perm: false, view_pr_metadata_exp: Expiration::default(),  
            trfer_allowance_perm: Uint128(1), trfer_allowance_exp: Expiration::default(), 
        } 
    );
    
    // addr2 transfers away token 2a
    env.message.sender = addr2.clone();
    let msg = HandleMsg::Transfer {
        token_id: "2a".to_string(),
        from: addr2.clone(),
        recipient: addr0.clone(),
        amount: Uint128(1),
        memo: None, padding: None,
    };  
    handle(&mut deps, env.clone(), msg)?;
    assert_eq!(chk_bal(&deps.storage, "2a", &addr2).unwrap(), Uint128(0));
    assert_eq!(chk_bal(&deps.storage, "2a", &addr0).unwrap(), Uint128(1));

    // user1 cannot transfer nft (now owned by user0), even though it previously had allowance 
    // when it was owned by user2
    env.message.sender = addr1.clone();
    let msg = HandleMsg::Transfer {
        token_id: "2a".to_string(),
        from: addr0.clone(),
        recipient: addr1.clone(),
        amount: Uint128(1),
        memo: None, padding: None,
    };  
    result = handle(&mut deps, env, msg);
    assert!(extract_error_msg(&result).contains("These tokens do not exist or you have no permission to transfer"));
    assert_eq!(chk_bal(&deps.storage, "2a", &addr1), None);
    assert_eq!(chk_bal(&deps.storage, "2a", &addr0).unwrap(), Uint128(1));

    Ok(())
}

#[test]
fn test_add_remove_curators() -> StdResult<()> {
    // init addresses
    let addr = init_addrs();

    // instantiate
    let (_init_result, mut deps) = init_helper_default();
    
    // non-curator cannot curate new token_ids
    let mut env = mock_env("addr1", &[]);
    let mut curate0 = CurateTokenId::default();
    curate0.token_info.token_id = "test0".to_string();
    let msg_curate = HandleMsg::CurateTokenIds { 
        initial_tokens: vec![curate0],
        memo: None,
        padding: None 
    };
    let mut result = handle(&mut deps, env.clone(), msg_curate.clone());
    assert!(extract_error_msg(&result).contains("Only curators are allowed to curate token_ids"));

    // admin adds 2 curators...
    env.message.sender = addr.a();
    let msg_add_curators = HandleMsg::AddCurators { add_curators: vec![addr.b(), addr.c()], padding: None };
    handle(&mut deps, env.clone(), msg_add_curators.clone())?;
    assert_eq!(chk_bal(&deps.storage, "test0", &addr.a()), None);

    // ...then new curator addr.b can curate new token_id
    env.message.sender = addr.b();
    handle(&mut deps, env.clone(), msg_curate)?;
    assert_eq!(chk_bal(&deps.storage, "test0", &addr.a()), Some(Uint128(1000)));

    // addr.b is curator, but because not admin => cannot add curators
    env.message.sender = addr.b();
    result = handle(&mut deps, env.clone(), msg_add_curators.clone());
    assert!(extract_error_msg(&result).contains("This is an admin function"));

    // admin can remove curator addr.b with just one operation, even though addr.b was added as curator multiple times
    // admin can also remove itself as curator
    // i) add addr.b (and addr.c) as curator a few more times
    env.message.sender = addr.a();
    for _ in 0..2 {
        handle(&mut deps, env.clone(), msg_add_curators.clone())?;
    }
    let q_answer = from_binary::<QueryAnswer>(&query(&deps, QueryMsg::ContractInfo {  })?)?;
    match q_answer {
        QueryAnswer::ContractInfo { curators, .. } => {
            assert_eq!(curators, vec![addr.a(), addr.b(), addr.c(), addr.b(), addr.c(), addr.b(), addr.c()])
        }
        _ => panic!("query error")
    }

    // ii) remove addr.a and addr.b as curators
    let msg_remove_curators = HandleMsg::RemoveCurators { remove_curators: vec![addr.a(), addr.b()], padding: None };
    env.message.sender = addr.a();
    handle(&mut deps, env.clone(), msg_remove_curators)?;
    let q_answer = from_binary::<QueryAnswer>(&query(&deps, QueryMsg::ContractInfo {  })?)?;
    match q_answer {
        QueryAnswer::ContractInfo { curators, .. } => {
            assert_eq!(curators, vec![addr.c(), addr.c(), addr.c()])
        }
        _ => panic!("query error")
    }
    
    // now curator addr.b cannot curate new tokens anymore
    let mut curate1 = CurateTokenId::default();
    curate1.token_info.token_id = "test1".to_string();
    let msg_curate_1 = HandleMsg::CurateTokenIds { 
        initial_tokens: vec![curate1],
        memo: None,
        padding: None 
    };
    env.message.sender = addr.b();
    result = handle(&mut deps, env.clone(), msg_curate_1.clone());
    assert!(extract_error_msg(&result).contains("Only curators are allowed to curate token_ids"));
    assert_eq!(chk_bal(&deps.storage, "test1", &addr.a()), None);

    // addr.a (which is admin) cannot curate new tokens either, since it is no longer a curator
    env.message.sender = addr.a();
    result = handle(&mut deps, env.clone(), msg_curate_1.clone());
    assert!(extract_error_msg(&result).contains("Only curators are allowed to curate token_ids"));
    assert_eq!(chk_bal(&deps.storage, "test1", &addr.a()), None);

    // addr.c (still a curator), can still curate new tokens
    env.message.sender = addr.c();
    handle(&mut deps, env, msg_curate_1)?;
    assert_eq!(chk_bal(&deps.storage, "test1", &addr.a()), Some(Uint128(1000)));
    
    Ok(())
}

#[test]
fn test_add_remove_minters() -> StdResult<()> {
    // init addresses
    let addr = init_addrs();

    // instantiate
    let (_init_result, mut deps) = init_helper_default();
    
    // admin adds 2 curators, addr.b and addr.c ...
    let mut env = mock_env("addr0", &[]);
    let msg_add_curators = HandleMsg::AddCurators { add_curators: vec![addr.b(), addr.c()], padding: None };
    handle(&mut deps, env.clone(), msg_add_curators)?;
    
    // ...then new curator (addr.b) curates new token_id
    let mut curate0 = CurateTokenId::default();
    curate0.token_info.token_id = "test0".to_string();
    let msg_curate = HandleMsg::CurateTokenIds { 
        initial_tokens: vec![curate0],
        memo: None,
        padding: None 
    };
    env.message.sender = addr.b();
    handle(&mut deps, env.clone(), msg_curate)?;
    assert_eq!(chk_bal(&deps.storage, "test0", &addr.a()), Some(Uint128(1000)));
    
    // addr.b cannot mint new tokens because it is not a minter despite creating the token_id
    let msg_mint = HandleMsg::MintTokens { 
        mint_tokens: vec![TokenAmount { 
            token_id: "test0".to_string(), 
            balances: vec![Balance { address: addr.a(), amount: Uint128(100) }] 
        }], 
        memo: None, padding: None 
    };
    let mut result = handle(&mut deps, env.clone(), msg_mint.clone());
    assert!(extract_error_msg(&result).contains("Only minters are allowed to mint additional tokens for token_id test0"));

    // addr.c, is curator, but not token_id "test0"'s curator, so cannot add minters
    let msg_add_minter_c = HandleMsg::AddMinters { 
        token_id: "test0".to_string(), 
        add_minters: vec![addr.c()], 
        padding: None
    };
    env.message.sender = addr.c();
    result = handle(&mut deps, env.clone(), msg_add_minter_c.clone());
    assert!(extract_error_msg(&result).contains("You need to be either the admin or address that created token_id test0 to perform this function"));

    // addr.b, as token_id's curator, can add minter addr.c
    env.message.sender = addr.b();
    handle(&mut deps, env.clone(), msg_add_minter_c)?;
    let q_answer = from_binary::<QueryAnswer>(&query(&deps, QueryMsg::TokenIdPublicInfo { token_id: "test0".to_string() })?)?;
    match q_answer {
        QueryAnswer::TokenIdPublicInfo { token_id_info, .. } => {
            assert_eq!(token_id_info.curator, addr.b());
            assert_eq!(token_id_info.token_config.flatten().minters, vec![addr.a(), addr.c()]);
        }
        _ => panic!("query error")
    }

    // admin addr.a can also add minter addr.d
    let msg_add_minter_d = HandleMsg::AddMinters { 
        token_id: "test0".to_string(), 
        add_minters: vec![addr.d()], 
        padding: None
    };
    env.message.sender = addr.a();
    // add minter d twice -- for test later
    for _ in 0..2 {
        handle(&mut deps, env.clone(), msg_add_minter_d.clone())?;
    }
    let mut q_answer = from_binary::<QueryAnswer>(&query(&deps, QueryMsg::TokenIdPublicInfo { token_id: "test0".to_string() })?)?;
    match q_answer {
        QueryAnswer::TokenIdPublicInfo { token_id_info, .. } => {
            assert_eq!(token_id_info.token_config.flatten().minters, vec![addr.a(), addr.c(), addr.d(), addr.d()]);
        }
        _ => panic!("query error")
    }

    // admin addr.a cannot add minters for a non-existent token_id
    let msg_add_minter_nonexistent = HandleMsg::AddMinters { 
        token_id: "test-na".to_string(), 
        add_minters: vec![addr.d()], 
        padding: None
    };
    env.message.sender = addr.a();
    result = handle(&mut deps, env.clone(), msg_add_minter_nonexistent);
    assert!(extract_error_msg(&result).contains("token_id test-na does not exist"));
    
    // both minters addr.c and addr.d can mint new tokens
    env.message.sender = addr.c();
    handle(&mut deps, env.clone(), msg_mint.clone())?;
    env.message.sender = addr.d();
    handle(&mut deps, env.clone(), msg_mint.clone())?;
    assert_eq!(chk_bal(&deps.storage, "test0", &addr.a()), Some(Uint128(1200)));

    // minters cannot burn tokens
    let msg_burn = HandleMsg::BurnTokens { 
        burn_tokens: vec![TokenAmount {
            token_id: "test0".to_string(),
            balances: vec![Balance {
                address: addr.a(),
                amount: Uint128(500),
            }],
        }], 
        memo: None, padding: None 
    };
    env.message.sender = addr.c();
    result = handle(&mut deps, env.clone(), msg_burn);
    assert!(extract_error_msg(&result).contains(
        "you do not have permission to burn 500 tokens from address addr0",
    ));

    // minters can change metadata (because of config allows)
    let msg_change_metadata = HandleMsg::ChangeMetadata { 
        token_id: "test0".to_string(), 
        public_metadata: Box::new(Some(Metadata {
            token_uri: Some("new public uri".to_string()),
            extension: Some(Extension::default()),
        })),  
        private_metadata: Box::new(None), 
    };
    env.message.sender = addr.c();
    handle(&mut deps, env.clone(), msg_change_metadata)?;
    q_answer = from_binary::<QueryAnswer>(&query(&deps, QueryMsg::TokenIdPublicInfo { token_id: "test0".to_string() })?)?;
    match q_answer {
        QueryAnswer::TokenIdPublicInfo { token_id_info, .. } => {
            assert_eq!(token_id_info.public_metadata.unwrap().token_uri, Some("new public uri".to_string()))
        },
        _ => panic!("query error")
    }

    // minter cannot change metadata (when config doesn't allow) --> do as separate test?
    // ...

    // curator addr.c cannot remove minters [addr.a, addr.c] because addr.c is not the curator that created this token_id
    let msg_remove_minter_ac = HandleMsg::RemoveMinters { token_id: "test0".to_string(), remove_minters: vec![addr.c()], padding: None };
    env.message.sender = addr.c();
    result = handle(&mut deps, env.clone(), msg_remove_minter_ac.clone());
    assert!(extract_error_msg(&result).contains("You need to be either the admin or address that created token_id test0 to perform this function"));

    // token_id curator addr.b can remove minter [addr.a, addr.c]
    env.message.sender = addr.b();
    handle(&mut deps, env.clone(), msg_remove_minter_ac)?;
    q_answer = from_binary::<QueryAnswer>(&query(&deps, QueryMsg::TokenIdPublicInfo { token_id: "test0".to_string() })?)?;
    match q_answer {
        QueryAnswer::TokenIdPublicInfo { token_id_info, .. } => {
            assert_eq!(token_id_info.token_config.flatten().minters, vec![addr.a(), addr.d(), addr.d()]);
        }
        _ => panic!("query error")
    }

    // admin can remove minter too: minter addr.d; although added twice, just need one remove
    // addr.a (as admin) can also remove itself as minter
    let msg_remove_minter_d = HandleMsg::RemoveMinters { token_id: "test0".to_string(), remove_minters: vec![addr.a(), addr.d()], padding: None };
    env.message.sender = addr.a();
    handle(&mut deps, env.clone(), msg_remove_minter_d)?;
    q_answer = from_binary::<QueryAnswer>(&query(&deps, QueryMsg::TokenIdPublicInfo { token_id: "test0".to_string() })?)?;
    match q_answer {
        QueryAnswer::TokenIdPublicInfo { token_id_info, .. } => {
            assert_eq!(token_id_info.token_config.flatten().minters, vec![]);
        }
        _ => panic!("query error")
    }

    // check no one can mint tokens now
    // admin (addr.a) not a minter anymore
    // sanityaddr.b (curator of the token_id) was never a minter
    // (addr.c and addr.d) no longer minters
    for address in addr.all() {
        env.message.sender = address;
        result = handle(&mut deps, env.clone(), msg_mint.clone());
        assert!(extract_error_msg(&result).contains("Only minters are allowed to mint additional tokens for token_id test0"));
    }

    Ok(())
}