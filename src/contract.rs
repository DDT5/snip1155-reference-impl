use cosmwasm_std::{
    Env, Extern, Storage, Api, Querier, 
    InitResponse, HandleResponse, Binary, to_binary,
    StdResult, StdError,
    HumanAddr, Uint128,
    // debug_print, 
};
use secret_toolkit::utils::space_pad;

use crate::{
    msg::{
        InitMsg, HandleMsg, HandleAnswer, QueryMsg,
        ResponseStatus::{Success}, //Failure 
    },
    state::{
        RESPONSE_BLOCK_SIZE, ContrConf,
        contr_conf_w, tkn_info_r, TknInfo,
        MintTokenId, MintToken, tkn_info_w, balances_w, balances_r, contr_conf_r,
    }
};

/////////////////////////////////////////////////////////////////////////////////
// Init
/////////////////////////////////////////////////////////////////////////////////


pub fn init<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    msg: InitMsg,
) -> StdResult<InitResponse> {
    // set admin. If `has_admin` == None => no admin. 
    // If `has_admin` == true && msg.admin == None => admin is the instantiator
    let admin = match msg.has_admin {
        false => None,
        true => match msg.admin {
            Some(i) => Some(i),
            None => Some(env.message.sender),
        },
    };
    
    // save contract config
    let config = ContrConf { 
        admin, 
        minters: msg.minters
    };
    contr_conf_w(&mut deps.storage).save(&config)?;

    // set initial balances
    for initial_token in msg.initial_tokens {
        exec_mint_token_id(
            &mut deps.storage, 
            initial_token
        )?;
    }
    
    Ok(InitResponse::default())
}

/////////////////////////////////////////////////////////////////////////////////
// Handles
/////////////////////////////////////////////////////////////////////////////////

pub fn handle<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    msg: HandleMsg,
) -> StdResult<HandleResponse> {
    let response = match msg {
        HandleMsg::MintTokenIds(
            initial_tokens
        ) => try_mint_token_ids(
            deps,
            env,
            initial_tokens,
        ),
        HandleMsg::MintTokens(
            mint_tokens
        ) => try_mint(
            deps, 
            env,
            mint_tokens,
        ),
        HandleMsg::Transfer { 
            token_id,
            sender,
            recipient, 
            amount 
        } => try_transfer(
            deps,
            env,
            token_id,
            sender,
            recipient,
            amount
        ),
    };
    pad_response(response)
}

pub fn try_mint_token_ids<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    initial_tokens: Vec<MintTokenId>
) -> StdResult<HandleResponse> {
    // check if sender is a minter
    verify_minter(&deps.storage, &env)?;

    // mint new token_ids
    for initial_token in initial_tokens {
        exec_mint_token_id(
            &mut deps.storage, 
            initial_token
        )?;
    } 

    Ok(HandleResponse {
        messages: vec![],
        log: vec![],
        data: Some(to_binary(&HandleAnswer::NewTokenIds { status: Success })?)
    })
}


pub fn try_mint<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    mint_tokens: Vec<MintToken>,
) -> StdResult<HandleResponse> {
    // check if sender is a minter
    verify_minter(&deps.storage, &env)?;

    // mint tokens
    for mint_token in mint_tokens {
        let token_info_op = tkn_info_r(&deps.storage).may_load(mint_token.token_id.as_bytes())?;
    
        if token_info_op.is_none() {
            return Err(StdError::generic_err(
                "token_id does not exist. Cannot mint or transfer non-existent `token_ids`.
                Use `mint_token_ids` to create tokens on new `token_ids`"
            ))
        }

        // add balances
        for add_balance in mint_token.add_balances {
            exec_change_balance(
                &mut deps.storage, 
                mint_token.token_id.clone(), 
                None, 
                add_balance.address, 
                add_balance.amount, 
                token_info_op.clone().unwrap()
            )?;
        }
    }

    Ok(HandleResponse {
        messages: vec![],
        log: vec![],
        data: Some(to_binary(&HandleAnswer::Mint { status: Success })?)
    })
}

pub fn try_transfer<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    token_id: String,
    sender: HumanAddr,
    recipient: HumanAddr,
    amount: Uint128,
) -> StdResult<HandleResponse> {
    // check that token_id exists
    let token_info_op = tkn_info_r(&deps.storage).may_load(token_id.as_bytes())?;

    if token_info_op.is_none() {
        return Err(StdError::generic_err(
            "token_id does not exist. Cannot mint or transfer non-existent `token_ids`.
            Use `mint_token_ids` to create tokens on new `token_ids`"
        ))
    }

    // check if `sender` == message sender || has permission to send tokens
    // permission logic todo!()
    let permission = false;

    // perform check
    if sender != env.message.sender && !permission {
        return Err(StdError::generic_err("you need to either be the owner of or have permission to transfer the tokens"))
    }

    // transfer tokens
    exec_change_balance(
        &mut deps.storage, 
        token_id, 
        Some(sender), 
        recipient, 
        amount, 
        token_info_op.unwrap()
    )?;

    Ok(HandleResponse {
        messages: vec![],
        log: vec![],
        data: Some(to_binary(&HandleAnswer::Transfer { status: Success })?)
    })
}

/////////////////////////////////////////////////////////////////////////////////
// Private functions
/////////////////////////////////////////////////////////////////////////////////

fn pad_response(
    response: StdResult<HandleResponse>
) -> StdResult<HandleResponse> {
    response.map(|mut response| {
        response.data = response.data.map(|mut data| {
            space_pad(&mut data.0, RESPONSE_BLOCK_SIZE);
            data
        });
        response
    })
}

/// verifies if sender is a minter
fn verify_minter<S: Storage>(
    storage: &S,
    env: &Env
) -> StdResult<()> {
    // check if sender is a minter
    let minters = contr_conf_r(storage).load()?.minters;
    if !minters.contains(&env.message.sender) {
        return Err(StdError::generic_err(
            "Only minters are allowed to mint",
        ));
    }
    Ok(())
}

/// checks if `token_id` is available (ie: not yet created), then creates new `token_id` and initial balances
fn exec_mint_token_id<S: Storage>(
    storage: &mut S,
    initial_token: MintTokenId,
) -> StdResult<()> {
    // check: token_id has not been created yet
    if tkn_info_r(storage).may_load(initial_token.token_info.token_id.as_bytes())?.is_some() {
        return Err(StdError::generic_err("token_id already exists. Try a different id String"))
    }

    // check: token_id is an NFT => cannot create more than one
    if initial_token.token_info.is_nft {
        if initial_token.balances.len() > 1 {
            return Err(StdError::generic_err(format!(
                "token_id {} is an NFT; there can only be one NFT. Balances should only have one address",
                initial_token.token_info.token_id
            )))
        } else if initial_token.balances[0].amount != Uint128(1) {
            return Err(StdError::generic_err(format!(
                "token_id {} is an NFT; there can only be one NFT. Balances.amount must == 1",
                initial_token.token_info.token_id
            )))
        }
    }

    // crate and save new token info
    tkn_info_w(storage).save(initial_token.token_info.token_id.as_bytes(), &initial_token.token_info)?;

    // set initial balances
    for balance in initial_token.balances {
        balances_w(storage, &initial_token.token_info.token_id)
        .save(to_binary(&balance.address)?.as_slice(), &balance.amount)?;
    }

    Ok(())
}

/// change token balance of an existing `token_id`. If `remove_from`==None, new tokens will be minted.
/// Check that `token_id` already exists before calling this function.
/// If is_nft == true, then `remove_from` MUST be Some(_).
fn exec_change_balance<S: Storage>(
    storage: &mut S,
    token_id: String,
    remove_from: Option<HumanAddr>,
    add_to: HumanAddr,
    amount: Uint128,
    token_info: TknInfo,
) -> StdResult<()> {
    // check whether token_id is an NFT => cannot mint
    if token_info.is_nft && remove_from == None {
        return Err(StdError::generic_err("NFTs can only be minted once using `mint_token_ids`"))
    }

    // check whether token_id is an NFT => assert!(amount == 1). 
    if token_info.is_nft && amount != Uint128(1) {
        return Err(StdError::generic_err("NFT amount must == 1"))
    }

    // remove balance
    if let Some(ref from) = remove_from {
        let from_existing_bal = balances_r(storage, &token_id).load(to_binary(&from)?.as_slice())?;
        let from_new_amount_op = from_existing_bal.u128().checked_sub(amount.u128());
        if from_new_amount_op.is_none() {
            return Err(StdError::generic_err("sender has insufficient funds"))
        }    
        balances_w(storage, &token_id)
        .save(to_binary(&from)?.as_slice(), &Uint128(from_new_amount_op.unwrap()))?;
    }

    // add balance
    let to_existing_bal_op = balances_r(storage, &token_id).may_load(to_binary(&add_to)?.as_slice())?; 
    let to_existing_bal = match to_existing_bal_op {
        Some(i) => i,
        None => Uint128(0),
    };
    let to_new_amount_op = to_existing_bal.u128().checked_add(amount.u128());
    if to_new_amount_op.is_none() {
        return Err(StdError::generic_err("recipient will become too rich. Total tokens exceeds 2^128"))
    }

    // save new balances
    balances_w(storage, &token_id)
    .save(to_binary(&add_to)?.as_slice(), &Uint128(to_new_amount_op.unwrap()))?;

    Ok(())
}

/////////////////////////////////////////////////////////////////////////////////
// Queries
/////////////////////////////////////////////////////////////////////////////////


pub fn query<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    msg: QueryMsg,
) -> StdResult<Binary> {
    match msg {
        QueryMsg::ContractInfo {  } => query_contract_info(deps),
    }
}

pub fn query_contract_info<S: Storage, A: Api, Q: Querier>(
    _deps: &Extern<S, A, Q>,
) -> StdResult<Binary> {
    to_binary(&"data".to_string())
}

