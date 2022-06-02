use cosmwasm_std::{
    Extern, Storage, Api, Querier, BlockInfo, 
    Binary, to_binary, 
    StdResult, StdError,
    HumanAddr, Uint128,
    QueryResult, 
    // debug_print, 
};
use secret_toolkit::{
    permit::{Permit, TokenPermissions, validate, }
};

use crate::{
    msg::{
        QueryMsg, QueryWithPermit, QueryAnswer, 
    },
    state::{
        PREFIX_REVOKED_PERMITS, 
        contr_conf_r, tkn_info_r, 
        balances_r, 
        tkn_tot_supply_r,
        get_receiver_hash, read_viewing_key,         
        permissions::{PermissionKey, Permission, may_load_any_permission, list_owner_permission_keys, },
        txhistory::{get_txs,may_get_current_owner, }, blockinfo_r
    },
    vk::viewing_key::VIEWING_KEY_SIZE,
};

/////////////////////////////////////////////////////////////////////////////////
// Queries
/////////////////////////////////////////////////////////////////////////////////

pub fn query<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    msg: QueryMsg,
) -> StdResult<Binary> {
    match msg {
        QueryMsg::ContractInfo {  } => query_contract_info(deps),
        QueryMsg::TokenIdPublicInfo { token_id } => query_token_id_public_info(deps, token_id),
        QueryMsg::RegisteredCodeHash { contract } => query_registered_code_hash(deps, contract),
        QueryMsg::WithPermit { permit, query } => permit_queries(deps, permit, query),
        QueryMsg::Balance { .. } |
        QueryMsg::TransactionHistory { .. } | 
        QueryMsg::Permission { .. } |
        QueryMsg::AllPermissions { .. } |
        QueryMsg::TokenIdPrivateInfo { .. } => viewing_keys_queries(deps, msg),
    }
}

fn permit_queries<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    permit: Permit,
    query: QueryWithPermit,
) -> Result<Binary, StdError> {
    // Validate permit content
    let contract_address = contr_conf_r(&deps.storage).load()?.contract_address;

    let account = validate(deps, PREFIX_REVOKED_PERMITS, &permit, contract_address, None)?;

    if !permit.check_permission(&TokenPermissions::Owner) {
        return Err(StdError::generic_err(format!(
            "`Owner` permit required for SNIP1155 permit queries, got permissions {:?}",
            permit.params.permissions
        )));
    }

    // Permit validated! We can now execute the query.
    match query {
        QueryWithPermit::Balance { owner, token_id 
        } => query_balance(deps, &owner, &HumanAddr(account), token_id),
        QueryWithPermit::TransactionHistory { page, page_size 
        } => query_transactions(deps, &HumanAddr(account), page.unwrap_or(0), page_size),
        QueryWithPermit::Permission { owner, allowed_address, token_id } => {
            if account != owner.as_str() && account != allowed_address.as_str() {
                return Err(StdError::generic_err(format!(
                    "Cannot query permission. Requires permit for either owner {:?} or viewer||spender {:?}, got permit for {:?}",
                    owner.as_str(), allowed_address.as_str(), account.as_str()
                )));
            }

            query_permission(deps, token_id, owner, allowed_address)
        },
        QueryWithPermit::AllPermissions { page, page_size 
        } => query_all_permissions(deps, &HumanAddr(account), page.unwrap_or(0), page_size),
        QueryWithPermit::TokenIdPrivateInfo { token_id 
        } => query_token_id_private_info(deps, &HumanAddr(account), token_id)
    }
}

fn viewing_keys_queries<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    msg: QueryMsg,
) -> QueryResult {
    let (addresses, key) = msg.get_validation_params();

    for address in addresses {
        let canonical_addr = deps.api.canonical_address(address)?;

        let expected_key = read_viewing_key(&deps.storage, &canonical_addr);

        if expected_key.is_none() {
            // Checking the key will take significant time. We don't want to exit immediately if it isn't set
            // in a way which will allow to time the command and determine if a viewing key doesn't exist
            key.check_viewing_key(&[0u8; VIEWING_KEY_SIZE]);
        } else if key.check_viewing_key(expected_key.unwrap().as_slice()) {
            return match msg {
                QueryMsg::Balance { owner, viewer, token_id, .. 
                } => query_balance(deps, &owner, &viewer, token_id),
                QueryMsg::TransactionHistory {
                    page,
                    page_size,
                    ..
                } => query_transactions(deps, address, page.unwrap_or(0), page_size),
                QueryMsg::Permission { owner, allowed_address, token_id, .. 
                } => query_permission(deps, token_id, owner, allowed_address),
                QueryMsg::AllPermissions { page, page_size, .. 
                } => query_all_permissions(deps, address, page.unwrap_or(0), page_size),
                QueryMsg::TokenIdPrivateInfo { address, token_id, .. 
                } => query_token_id_private_info(deps, &address, token_id),
                _ => panic!("This query type does not require authentication"),
            };
        }
    }

    to_binary(&QueryAnswer::ViewingKeyError {
        msg: "Wrong viewing key for this address or viewing key not set".to_string(),
    })
}

fn query_contract_info<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
) -> StdResult<Binary> {
    let contr_conf = contr_conf_r(&deps.storage).load()?;
    let response = QueryAnswer::ContractInfo { 
        admin: contr_conf.admin, 
        curators: contr_conf.curators, 
        all_token_ids: contr_conf.token_id_list, 
    };
    to_binary(&response)
}

fn query_token_id_public_info<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    token_id: String,
) -> StdResult<Binary> {
    let tkn_info_op= tkn_info_r(&deps.storage).may_load(token_id.as_bytes())?;
    match tkn_info_op {
        None => return Err(StdError::generic_err(format!(
            "token_id {} does not exist",
            token_id
        ))),
        Some(mut tkn_info) => {
            // add owner if owner_is_public == true
            let owner: Option<HumanAddr> = if tkn_info.token_config.flatten().owner_is_public {
                may_get_current_owner(&deps.storage, &token_id)?
            } else {
                None
            };

            // add public supply if public_total_supply == true 
            let total_supply: Option<Uint128> = if tkn_info.token_config.flatten().public_total_supply { 
                Some(tkn_tot_supply_r(&deps.storage).load(token_id.as_bytes())?)
            } else { None };

            // private_metadata always == None for public info query 
            tkn_info.private_metadata = None;
            let response = QueryAnswer::TokenIdPublicInfo { token_id_info: tkn_info, total_supply, owner };
            to_binary(&response) 
        },
    }
}

fn query_token_id_private_info<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    viewer: &HumanAddr,
    token_id: String
) -> StdResult<Binary> {
    let tkn_info_op= tkn_info_r(&deps.storage).may_load(token_id.as_bytes())?;
    if tkn_info_op.is_none() {
        return Err(StdError::generic_err(format!(
            "token_id {} does not exist",
            token_id
        )))
    }

    let mut tkn_info = tkn_info_op.unwrap();
    
    // add owner if owner_is_public == true
    let owner: Option<HumanAddr> =  if tkn_info.token_config.flatten().owner_is_public {
        may_get_current_owner(&deps.storage, &token_id)?
    } else {
        None
    };

    // private metadata is viewable if viewer owns at least 1 token
    let viewer_owns_some_tokens = match balances_r(&deps.storage, &token_id)
        .may_load(to_binary(&viewer)?.as_slice())? {
            None => false,
            Some(i) if i == Uint128(0) => false,
            Some(i) if i > Uint128(0) => true,
            Some(_) => unreachable!("should not reach here")
        };

    // If request owns at least 1 token, can view `private_metadata`. Otherwise check viewership permissions (permission only applicable to nfts, as
    // fungible tokens have no current `owner`). 
    if !viewer_owns_some_tokens {
        let permission_op = may_load_any_permission(
            &deps.storage, 
            // if no owner, = "" ie blank string => will not have any permission
            owner.as_ref().unwrap_or(&HumanAddr("".to_string())), 
            &token_id, 
            viewer
        )?; 
        match permission_op {
            None => return Err(StdError::generic_err("you do have have permission to view private token info")),
            Some(perm) => {
                let block: BlockInfo = blockinfo_r(&deps.storage).may_load()?.unwrap_or_else(|| BlockInfo {
                    height: 1,
                    time: 1,
                    chain_id: "not used".to_string(),
                });
                if !perm.check_view_pr_metadata_perm(&block) { tkn_info.private_metadata = None };
            },       
        }
    }

    // add public supply if public_total_supply == true
    let total_supply: Option<Uint128> = if tkn_info.token_config.flatten().public_total_supply { 
        Some(tkn_tot_supply_r(&deps.storage).load(token_id.as_bytes())?)
    } else { None };
    
    let response = QueryAnswer::TokenIdPrivateInfo { token_id_info: tkn_info, total_supply, owner };
    to_binary(&response)
}

fn query_registered_code_hash<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    contract: HumanAddr,
) -> StdResult<Binary> {
    let may_hash_res = get_receiver_hash(&deps.storage, &contract);
    let response: QueryAnswer = match may_hash_res {
        Some(hash_res) => {
            QueryAnswer::RegisteredCodeHash { code_hash: Some(hash_res?) }
        }
        None => { QueryAnswer::RegisteredCodeHash { code_hash: None }},
    };

    to_binary(&response)
}

fn query_balance<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    owner: &HumanAddr,
    viewer: &HumanAddr,
    token_id: String,
) -> StdResult<Binary> {
    if owner != viewer {
        let permission_op = may_load_any_permission(
            &deps.storage, 
            owner, 
            &token_id, 
            viewer,
        )?;
        match permission_op {
            None => return Err(StdError::generic_err("you do have have permission to view balance")),
            Some(perm) => {
                let block: BlockInfo = blockinfo_r(&deps.storage).may_load()?.unwrap_or_else(|| BlockInfo {
                    height: 1,
                    time: 1,
                    chain_id: "not used".to_string(),
                });
                if !perm.check_view_balance_perm(&block) {
                    return Err(StdError::generic_err("you do have have permission to view balance"))
                } else {  }
            },
        }
    }

    let owner_canon = deps.api.canonical_address(owner)?;
    let amount_op = balances_r(&deps.storage, &token_id)
        .may_load(to_binary(&deps.api.human_address(&owner_canon)?)?.as_slice())?;
    let amount = match amount_op {
        Some(i) => i,
        None => Uint128(0),
    };
    let response = QueryAnswer::Balance { amount };
    to_binary(&response)
}

fn query_transactions<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    account: &HumanAddr,
    page: u32,
    page_size: u32,
) -> StdResult<Binary> {
    let address = deps.api.canonical_address(account)?;
    let (txs, total) = get_txs(&deps.api, &deps.storage, &address, page, page_size)?;

    let response = QueryAnswer::TransactionHistory {
        txs,
        total: Some(total),
    };
    to_binary(&response)
}

fn query_permission<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    token_id: String,
    owner: HumanAddr,
    allowed_addr: HumanAddr,
) -> StdResult<Binary> {
    let permission = may_load_any_permission(&deps.storage, &owner, &token_id, &allowed_addr)?;

    let response = QueryAnswer::Permission(permission);
    to_binary(&response)
}

fn query_all_permissions<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    account: &HumanAddr,
    page: u32,
    page_size: u32,
) -> StdResult<Binary> {
    let (permission_keys, total) = list_owner_permission_keys(&deps.storage, account, page, page_size)?;
    let mut permissions: Vec<Permission> = vec![];
    let mut valid_pkeys: Vec<PermissionKey> = vec![]; 
    for pkey in permission_keys {
        let permission = may_load_any_permission(
            &deps.storage, 
            account,
            &pkey.token_id,
            &pkey.allowed_addr,
        )?;
        if let Some(i) = permission { 
            permissions.push(i);
            valid_pkeys.push(pkey);
        };
    }
    
    let response = QueryAnswer::AllPermissions { permission_keys: valid_pkeys, permissions, total };
    to_binary(&response)
}

