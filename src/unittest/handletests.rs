use super::{
    testhelpers::*
};

use super::super::{
    contract::*,
    msg::*,
    state::*,
    receiver::{Snip1155ReceiveMsg, ReceiverHandleMsg},
};

use cosmwasm_std::{
    testing::*, 
    StdResult, 
    InitResponse, 
    HumanAddr, Uint128, 
    to_binary, 
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
    assert_eq!(chk_bal(&deps.storage, "3", &addr2).unwrap(), Uint128(1));
    // 1 initial balance, 3 mint_token_id 
    assert_eq!(contr_conf_r(&deps.storage).load()?.tx_cnt, 4u64);

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
    assert_eq!(chk_bal(&deps.storage, "3", &addr1), None);
    assert_eq!(chk_bal(&deps.storage, "3", &addr1), None);
    assert_eq!(chk_bal(&deps.storage, "3", &addr2).unwrap(), Uint128(1));

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
    mint.token_info.token_id = "4".to_string();
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
    mint.token_info.token_id = "5".to_string();
    mint.token_info.is_nft = true;
    mint.balances[0].amount = Uint128(2);
    msg = HandleMsg::MintTokenIds{initial_tokens: vec![mint], memo: None, padding: None };
    result = handle(&mut deps, env.clone(), msg);
    assert!(extract_error_msg(&result).contains("is an NFT; there can only be one NFT. Balances.amount must == 1"));

    // non-minter cannot mint
    env.message.sender = addr1.clone();
    let mut mint = MintTokenId::default();
    mint.token_info.token_id = "6".to_string();
    msg = HandleMsg::MintTokenIds{initial_tokens: vec![mint], memo: None, padding: None };
    result = handle(&mut deps, env, msg);
    assert!(extract_error_msg(&result).contains("Only minters are allowed to mint"));

    // check balances
    assert_eq!(chk_bal(&deps.storage, "0", &addr0).unwrap(), Uint128(1000));
    assert_eq!(chk_bal(&deps.storage, "1", &addr1).unwrap(), Uint128(500));
    assert_eq!(chk_bal(&deps.storage, "2", &addr2).unwrap(), Uint128(1));
    assert_eq!(chk_bal(&deps.storage, "3", &addr2).unwrap(), Uint128(1));
    assert_eq!(chk_bal(&deps.storage, "4", &addr0), None); assert_eq!(chk_bal(&deps.storage, "4", &addr1), None);
    assert_eq!(chk_bal(&deps.storage, "5", &addr0), None);
    assert_eq!(chk_bal(&deps.storage, "6", &addr0), None);
    // 1 initial balance, 3 mint_token_id, 0 additional
    assert_eq!(contr_conf_r(&deps.storage).load()?.tx_cnt, 4u64);

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
    // 1 initial balance, 3 mint_token_id, 2 mint_token 
    assert_eq!(contr_conf_r(&deps.storage).load()?.tx_cnt, 6u64);

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
    // 1 initial balance, 3 mint_token_id, 2 mint_token 
    assert_eq!(contr_conf_r(&deps.storage).load()?.tx_cnt, 6u64);
    
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

    // 1 initial balance, 3 mint_token_id, 2 burns 
    assert_eq!(contr_conf_r(&deps.storage).load()?.tx_cnt, 6u64);

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
    // 1 initial balance, 3 mint_token_id, 2 transfers 
    assert_eq!(contr_conf_r(&deps.storage).load()?.tx_cnt, 6u64);

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
        address: addr1.clone(), 
        token_id: "0".to_string(), 
        view_owner: None, view_private_metadata: None, 
        transfer: Some(Uint128(9)), 
        padding: None, 
    };  
    handle(&mut deps, env.clone(), msg0_perm_1)?;

    env.message.sender = addr1.clone();
    result = handle(&mut deps, env.clone(), msg_trnsf_0.clone());
    assert!(extract_error_msg(&result).contains("Insufficient transfer allowance: "));

    // cannot transfer with wrong allowances: wrong spender address: addr2 has the transfer permission
    env.message.sender = addr0.clone();
    let msg0_perm_2 = HandleMsg::GivePermission { 
        address: addr2.clone(), 
        token_id: "0".to_string(), 
        view_owner: None, view_private_metadata: None, 
        transfer: Some(Uint128(15)), 
        padding: None, 
    };  
    handle(&mut deps, env.clone(), msg0_perm_2)?;

    env.message.sender = addr1.clone();
    result = handle(&mut deps, env.clone(), msg_trnsf_0.clone());
    assert!(extract_error_msg(&result).contains("Insufficient transfer allowance: "));

    // cannot transfer with wrong allowances: wrong owner address: addr1 giving permission
    env.message.sender = addr1.clone();
    let msg1_perm_1 = HandleMsg::GivePermission { 
        address: addr1.clone(), 
        token_id: "0".to_string(), 
        view_owner: None, view_private_metadata: None, 
        transfer: Some(Uint128(10)), 
        padding: None, 
    };  
    handle(&mut deps, env.clone(), msg1_perm_1)?;
    result = handle(&mut deps, env.clone(), msg_trnsf_0.clone());
    assert!(extract_error_msg(&result).contains("Insufficient transfer allowance: "));

    // can transfer with enough allowance: addr2 has enough allowance
    env.message.sender = addr2.clone();
    handle(&mut deps, env.clone(), msg_trnsf_0.clone())?;
    assert_eq!(chk_bal(&deps.storage, "0", &addr0).unwrap(), Uint128(990));
    assert_eq!(chk_bal(&deps.storage, "0", &addr1).unwrap(), Uint128(10));

    // allowance gets consumed: cannot exceed allowance with a second tx 
    result = handle(&mut deps, env.clone(), msg_trnsf_0.clone());
    assert!(extract_error_msg(&result).contains("Insufficient transfer allowance: 5"));

    // allowance for different address does not get consumed
    assert_eq!(permission_r(&deps.storage, &addr0, "0").load(addr1_u8)?.trfer_allowance_perm, Uint128(9));
    assert_eq!(permission_r(&deps.storage, &addr0, "0").load(addr2_u8)?.trfer_allowance_perm, Uint128(5));
    assert_eq!(permission_r(&deps.storage, &addr1, "0").load(addr1_u8)?.trfer_allowance_perm, Uint128(10));

    // owner can transfer regardless of allowance
    env.message.sender = addr0.clone();
    handle(&mut deps, env.clone(), msg_trnsf_0.clone())?; handle(&mut deps, env.clone(), msg_trnsf_0.clone())?;
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

    // mint additional token_ids
    let mut env = mock_env("addr0", &[]);
    mint_addtl_default(&mut deps, &env)?;

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
        address: addr1.clone(), 
        token_id: "2".to_string(), 
        view_owner: None, view_private_metadata: None, 
        transfer: Some(Uint128(1)), 
        padding: None, 
    };  
    handle(&mut deps, env.clone(), msg2_perm_1)?;

    // addr1 can now transfer addr2's nft to addr0
    env.message.sender = addr1.clone();
    handle(&mut deps, env.clone(), msg1_trnsf_0.clone())?;
    assert_eq!(chk_bal(&deps.storage, "2", &addr2).unwrap(), Uint128(0));
    assert_eq!(chk_bal(&deps.storage, "2", &addr0).unwrap(), Uint128(1));

    // cannot transfer again: insufficient balance
    result = handle(&mut deps, env.clone(), msg1_trnsf_0);
    assert!(extract_error_msg(&result).contains("Insufficient transfer allowance: 0"));
    // balance is unchanged
    assert_eq!(chk_bal(&deps.storage, "2", &addr2).unwrap(), Uint128(0));
    assert_eq!(chk_bal(&deps.storage, "2", &addr0).unwrap(), Uint128(1));

    // give permission to transfern token 3
    env.message.sender = addr2.clone();
    let msg2_perm_1 = HandleMsg::GivePermission { 
        address: addr1.clone(), 
        token_id: "3".to_string(), 
        view_owner: None, view_private_metadata: None, 
        transfer: Some(Uint128(1)), 
        padding: None, 
    };  
    handle(&mut deps, env.clone(), msg2_perm_1)?;
    // double check that addr1 has permission to transfer token 3
    assert_eq!(
        permission_r(&deps.storage, &addr2, "3").load(addr1_u8)?, 
        Permission { view_owner_perm: false, view_pr_metadata_perm: false, trfer_allowance_perm: Uint128(1) }
    );
    
    // addr2 transfers away token 3
    env.message.sender = addr2.clone();
    let msg = HandleMsg::Transfer {
        token_id: "3".to_string(),
        from: addr2.clone(),
        recipient: addr0.clone(),
        amount: Uint128(1),
        memo: None, padding: None,
    };  
    handle(&mut deps, env.clone(), msg)?;
    assert_eq!(chk_bal(&deps.storage, "3", &addr2).unwrap(), Uint128(0));
    assert_eq!(chk_bal(&deps.storage, "3", &addr0).unwrap(), Uint128(1));

    // user1 cannot transfer nft (now owned by user0), even though it previously had allowance 
    // when it was owned by user2
    env.message.sender = addr1.clone();
    let msg = HandleMsg::Transfer {
        token_id: "3".to_string(),
        from: addr0.clone(),
        recipient: addr1.clone(),
        amount: Uint128(1),
        memo: None, padding: None,
    };  
    result = handle(&mut deps, env, msg);
    assert!(extract_error_msg(&result).contains("These tokens do not exist or you have no permission to transfer"));
    assert_eq!(chk_bal(&deps.storage, "3", &addr1), None);
    assert_eq!(chk_bal(&deps.storage, "3", &addr0).unwrap(), Uint128(1));

    Ok(())
}