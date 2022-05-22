use cosmwasm_std::{
    from_binary, to_binary, Api, Binary, Context, CosmosMsg, Env, Extern,
    HandleResponse, HumanAddr, InitResponse, Querier, StdError, StdResult, Storage, Uint128,
    WasmMsg,
};

use crate::msg::{CountResponse, HandleMsg, InitMsg, QueryMsg, Snip1155Msg};
use crate::state::{config, config_read, State};

pub fn init<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    msg: InitMsg,
) -> StdResult<InitResponse> {
    let state = State {
        count: msg.count,
        owner: deps.api.canonical_address(&env.message.sender)?,
        known_snip_1155: vec![],
    };

    config(&mut deps.storage).save(&state)?;

    Ok(InitResponse::default())
}

pub fn handle<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    msg: HandleMsg,
) -> StdResult<HandleResponse> {
    match msg {
        HandleMsg::Increment {} => try_increment(deps, env),
        HandleMsg::Reset { count } => try_reset(deps, env, count),
        HandleMsg::Register { reg_addr, reg_hash } => try_register(deps, env, reg_addr, reg_hash),
        HandleMsg::Snip1155Receive {
            sender,
            token_id,
            from,
            amount,
            msg,
            memo: _,
        } => try_receive(deps, env, sender, token_id, from, amount, msg),
        HandleMsg::Fail {} => try_fail(),
    }
}

pub fn try_increment<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    _env: Env,
) -> StdResult<HandleResponse> {
    let mut count = 0;
    config(&mut deps.storage).update(|mut state| {
        state.count += 1;
        count = state.count;
        Ok(state)
    })?;

    let mut context = Context::new();
    context.add_log("count", count.to_string());

    Ok(context.into())
}

pub fn try_reset<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    count: i32,
) -> StdResult<HandleResponse> {
    let sender_address_raw = deps.api.canonical_address(&env.message.sender)?;
    config(&mut deps.storage).update(|mut state| {
        if sender_address_raw != state.owner {
            return Err(StdError::Unauthorized { backtrace: None });
        }
        state.count = count;
        Ok(state)
    })?;
    Ok(HandleResponse::default())
}

pub fn try_register<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    reg_addr: HumanAddr,
    reg_hash: String,
) -> StdResult<HandleResponse> {
    let mut conf = config(&mut deps.storage);
    let mut state = conf.load()?;
    if !state.known_snip_1155.contains(&reg_addr) {
        state.known_snip_1155.push(reg_addr.clone());
    }
    conf.save(&state)?;

    let msg = to_binary(&Snip1155Msg::register_receive(env.contract_code_hash))?;
    let message = CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: reg_addr,
        callback_code_hash: reg_hash,
        msg,
        send: vec![],
    });

    Ok(HandleResponse {
        messages: vec![message],
        log: vec![],
        data: None,
    })
}

pub fn try_receive<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    _sender: HumanAddr,
    _token_id: String,
    _from: HumanAddr,
    _amount: Uint128,
    msg: Binary,
) -> StdResult<HandleResponse> {
    let msg: HandleMsg = from_binary(&msg)?;

    if matches!(msg, HandleMsg::Snip1155Receive { .. }) {
        return Err(StdError::generic_err(
            "Recursive call to Snip1155Receive() is not allowed",
        ));
    }

    let state = config_read(&deps.storage).load()?;
    if !state.known_snip_1155.contains(&env.message.sender) && state.owner != deps.api.canonical_address(&env.message.sender)? {
        return Err(StdError::generic_err(format!(
            "{} is not receiver creator, or a known SNIP-1155 coin that this contract registered to",
            env.message.sender
        )));
    }

    /* use sender & amount */
    handle(deps, env, msg)
}

fn try_fail() -> StdResult<HandleResponse> {
    Err(StdError::generic_err("intentional failure"))
}

pub fn query<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    msg: QueryMsg,
) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetCount {} => to_binary(&query_count(deps)?),
    }
}

fn query_count<S: Storage, A: Api, Q: Querier>(deps: &Extern<S, A, Q>) -> StdResult<CountResponse> {
    let state = config_read(&deps.storage).load()?;
    Ok(CountResponse { count: state.count })
}