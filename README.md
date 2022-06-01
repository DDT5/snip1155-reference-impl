SNIP1155 Reference Implementation: Private Multitokens  <!-- omit in toc --> 
==============

This repository contains the [SNIP1155 Standard Specifications](#base-specifications) and the standard reference implementation.

Note that schemas presented here are simplified for readability. The canonical schemas that developers should rely on can be found [here](https://github.com/DDT5/snip1155-reference-impl/tree/master/schema). If there are any discrepancies, the canonical schemas should be used.


## Table of contents <!-- omit in toc --> 
- [Abstract](#abstract)
- [Terms](#terms)
- [Base specifications](#base-specifications)
  - [Token hierarchy](#token-hierarchy)
  - [Instantiation](#instantiation)
  - [The admin](#the-admin)
  - [The curator(s) and minter(s)](#the-curators-and-minters)
  - [NFT vs fungible tokens](#nft-vs-fungible-tokens)
  - [Handle messages](#handle-messages)
    - [Transfer](#transfer)
    - [Send](#send)
    - [BatchTransfer and BatchSend](#batchtransfer-and-batchsend)
    - [CreateViewingKey and SetViewingKey](#createviewingkey-and-setviewingkey)
    - [RevokePermit](#revokepermit)
    - [Allowances and private metadata viewership](#allowances-and-private-metadata-viewership)
    - [RevokePermission](#revokepermission)
    - [CurateTokenIds](#curatetokenids)
    - [MintTokens](#minttokens)
    - [BurnTokens](#burntokens)
    - [AddCurators and RemoveCurators](#addcurators-and-removecurators)
    - [AddMinters and RemoveMinters](#addminters-and-removeminters)
    - [ChangeAdmin](#changeadmin)
    - [BreakAdminKey](#breakadminkey)
  - [Queries](#queries)
    - [ContractInfo](#contractinfo)
    - [Minters](#minters)
    - [NumTokens](#numtokens)
  - [Authenticated Queries](#authenticated-queries)
    - [Balance](#balance)
    - [BatchBalance](#batchbalance)
    - [Approved](#approved)
    - [IsApproved](#isapproved)
    - [OwnerOf](#ownerof)
    - [TokenInfo](#tokeninfo)
    - [PrivateMetadata](#privatemetadata)
    - [TransactionHistory](#transactionhistory)
  - [Receiver functions](#receiver-functions)
    - [RegisterReceive](#registerreceive)
    - [Snip1155Receive](#snip1155receive)
  - [Miscellaneous](#miscellaneous)
  - [Schema](#schema)
    - [Instantiation message](#instantiation-message)
    - [Handle messages](#handle-messages-1)
    - [Handle responses](#handle-responses)
    - [Query messages](#query-messages)
    - [Query responses](#query-responses)
- [Additional specifications](#additional-specifications)
- [Design decisions](#design-decisions)
  - [Reference implementation goals](#reference-implementation-goals)
  - [Starting from a blank slate](#starting-from-a-blank-slate)
  - [Permissionless design](#permissionless-design)
  - [Familiar interface](#familiar-interface)
  - [Keeping base and additional features separate](#keeping-base-and-additional-features-separate)



# Abstract

SNIP1155 is a [Secret Network](https://github.com/scrtlabs/SecretNetwork) contract that can create and manage multiple tokens from a single contract instance. Tokens can be a combination of fungible tokens and non-fungible tokens, each with separate configurations, attributes and metadata. 

This specification writeup ("spec" or "specs") outlines the functionality and interface. The design is loosely based on [CW1155](https://lib.rs/crates/cw1155) which is in turn based on Ethereum's [ERC1155](https://eips.ethereum.org/EIPS/eip-1155), with an additional privacy layer made possible as a Secret Network contract.
Fungible and non-fungible tokens are mostly treated equally, but each has a different set of available token configurations which define their possible behaviors. For example, NFTs can only be minted once. Unlike CW1155 where approvals must cover the entire set of tokens, SNIP1155 contracts allow users to control which tokens fall in scope for a given approval (a feature from [ERC1761](https://eips.ethereum.org/EIPS/eip-1761)). In addition, SNIP1155 users can control the type of approval it grants other addresses: token transfer allowances, balance viewership, or private metadata viewership. 

The ability to hold multiple token types can provide new functionality, improve developer and user experience, and reduce gas fees. For example, users can batch transfer multiple token types, developers could eliminate inter-contract messages and factory contracts, and users may need to approve only once to cover all tokens for an application.

See [design decisions](#design-decisions) for more details.

# Terms
*The key words "MUST", "MUST NOT", "REQUIRED", "SHALL", "SHALL NOT", "SHOULD", "SHOULD NOT", "RECOMMENDED", "MAY", and "OPTIONAL" in this document are to be interpreted as described in [RFC 2119](https://datatracker.ietf.org/doc/html/rfc2119).*

This memo uses the terms defined below:
* Message - an on-chain interface. It is triggered by sending a transaction, and receiving an on-chain response which is read by the client. Messages are authenticated both by the blockchain and by the Secret enclave.
* Query - an off-chain interface. Queries are done by returning data that a node has locally, and are not public. Query responses are returned immediately, and do not have to wait for blocks. In addition, queries cannot be authenticated using the standard interfaces. Any contract that wishes to strongly enforce query permissions must implement it themselves.
* [Cosmos Message] Sender or User - the account found under the sender field in a standard Cosmos SDK message. This is also the signer of the message.
* Native Asset - a coin which is defined and managed by the blockchain infrastructure, not a Secret contract.


# Base specifications

## Token hierarchy
SNIP1155 has a token hierarchy with three layers: (A) SNIP1155 contract > (B) token_id > (C) token(s). 

(A) At the highest level, all tokens of a given SNIP1155 contract MUST share:
* admin (if enabled)
* curator(s)

(B) A SNIP1155 contract MUST have the ability to hold multiple `tokens_id`s. Each `token_id` MUST have their own:
* token_id (unique identifier within a contract)
* name
* symbol
* token_config (which may optionally include a list of minters)
* public metadata
* private metadata

(C) Each `token_id` can have 1 or more supply of `tokens`, which are indistinguishable from one another (hence fungible). Non-fungible `token_id`s MUST have a total supply of 1, and MUST only be minted once. 

For example, a given SNIP1155 contract may have the following token hierarchy:
```
SNIP1155 contract
├── token_id 0 (total supply = 2)
│   ├── fungible token
│   └── fungible token
├── token_id 1 (total supply = 3)
│   ├── fungible token
│   ├── fungible token
│   └── fungible token
└── token_id 2 (total supply = 1)
    └── non-fungible token
```


The table below describes each variable in more detail: 

| Variable         | Type             | Description                                                | Optional |
| ---------------- | ---------------- | ---------------------------------------------------------- | -------- |
| admin            | HumanAddr        | Can to add/remove minters and break admin key              | Yes      |
| minter           | `Vec<HumanAddr>` | Can mint tokens and change metadata if config allows       | Yes      |
| token_id         | String           | `token_id` unique identifier                               | No       |
| name             | String           | Name of fungible tokens or NFT. Does not need to be unqiue | No       |
| symbol           | String           | Symbol of fungible tokens or NFT                           | No       |
| token_config     | TokenConfig      | Includes enable burn, owner is public, .. (see below)      | No       |
| public metadata  | Metadata         | Publicly viewable `uri` and `extension`                    | Yes      |
| private metadata | Metadata         | Non-publicly viewable `uri` and `extension`                | Yes      |

`Token_config` is MUST be an enum with at least these two variants:

```rust
{
  fungible: {
    minters: Vec<HumanAddr>,
    decimals: u8,
    public_total_supply: bool,
    enable_mint: bool,
    enable_burn: bool,
    minter_may_update_metadata: bool,
  }
}
{
  nft: {
    public_total_supply: bool,
    owner_is_public: bool,
    enable_burn: bool,
    owner_may_update_metadata: bool,
  }
}
```

Metadata:
```rust
{
  token_uri: Option<String>,
  extension: Option<Extension>,
}
```

Application developers MAY change `extension` to any `struct` that fits their use case.


## Instantiation
The instantiator MUST have the option to specify the admin, minter(s) and initial balances. See [Instantiation message](#instantiation-message).

If no admin is specified, the instantiator MAY be used as the default admin; in this setup, there MUST be an input field `has_admin: bool` which allows the instantiator to instantiate a no-admin contract. If `has_admin == false`, any admin input MUST be ignored by the contract.  

The `initial_balances` input SHOULD allow an arbitrary number of token_ids and tokens to be created at instantiation. This design makes it convenient to instantiate permissionless contracts with no admin and minters, as all the required tokens can be minted upon instantiation.

## The admin
The role of the admin (if exists) is to add and remove minters. 

The admin MUST be able to perform admin-only transactions. Admin-only transactions MUST NOT be callable by non-admins. Admin-only function MUST include `add_minters`, `remove_minters` and `change_admin` and `break_admin_key`.

The existence of the `break_admin_key` function is enforced in SNIP1155 standards in order to encourage permissionless designs. Users MUST be able to query `contract_info` to get proof on whether an admin role exists.

## The curator(s) and minter(s)
There are two types of roles that can create tokens: 
* `curators` curates new token_ids and mints initial balances. They cannot mint additional tokens of existing token_ids, unless they are also minters. 
* `minters` are specific to each token_id. These addresses can mint incremental fungible tokens of existing token_ids if the token_id configuration allows this. These addresses cannot mint the initial token balances. Minters of a given token_id may also change the public and private metadata if the token_config of the token_id allows this. They are not relevant to NFTs in the base specification. 

## NFT vs fungible tokens
Minters MUST have the option to set public metadata and and private metadata for a token_id. An NFT owner MUST be able to change the metadata if the token_id configuration allows it to.

Fungible token metadata cannot be changed in the base specification[^1] but is OPTIONAL in the [additional specifications](#additional-specifications). The rules that govern how fungible metadata is changed is left to application developers to decide.

Private metadata of an NFT MUST NOT be viewable by any address, other than the owner's address or addresses that have been given permission to[^2]. The standard implementation allows any owner of a fungible token to view private metadata of the fungible `token_id`, but different rules MAY be implemented by application developers.  

[^1]: See the [fractionalized NFT reference implementation](https://github.com/DDT5/frac-snft-ref-impl) if the use case requires multiple addresses owning an NFT.
[^2]: The base specifications do not have sealed metadata functionality, which is included in the additional specifications.


## Handle messages

See schemas [here](#handle-messages-1)

### Transfer
Transfers a specified amount of tokens of a single `token_id` from one address to another. If the transaction caller is not the current owner of the token, a successful transaction MUST require that the caller has the required transfer allowances.  

The SNIP1155 `Transfer` interface more closely reflects SNIP721 than SNIP20. SNIP20's `Transfer` and `TransferFrom` functions can both be performed using SNIP1155's `Transfer` function. 

### Send
Similar to `Transfer`. If the recipient has registered itself with a `RegisterReceive` message, this function MUST also send an inter-contract message ("callback") to the recipient. It is also RECOMMENDED that `Send` includes an optional code hash input, so the recipient contract does not need to have to first `RegisterReceive`.

The SNIP1155 `Send` interface more closely reflects SNIP721 than SNIP20. SNIP20's `Send` and `SendFrom` functions can both be performed using SNIP1155's `Send` function. 

### BatchTransfer and BatchSend
These functions perform multiple `Transfer`, or `Send` actions in a single transaction. Multiple `token_id`s and recipients MUST be allowed in a batch, including a mix of NFTs and fungible tokens of the same SNIP1155 contract.

`BatchSend` MUST allow different callback messages to be sent for each `Send` action.

### CreateViewingKey and SetViewingKey 
These perform the same functions as specified in the SNIP20 standards.

### RevokePermit
Similar to SNIP20 and SNIP721; allows an address revoke a query permit that it may have previously created and shared.

### Allowances and private metadata viewership
`GivePermission` is used by an address to grant other addresses permissions to transfer or view private information of its tokens. This function MUST allow the transaction caller to set the `token_id`s that fall in scope of a given approval (unlike in CW1155, where approvals are global. Permissions MUST include the ability for a token owner to allow another address to:
* view token owner's balance
* view private metadata 
* transfer tokens up to a specified allowance

It is OPTIONAL to additionally include `IncreaseAllowance` and `DecreaseAllowance` messages, as these are familiar SNIP20 interfaces.

### RevokePermission
An operator with existing permissions (not to be confused with Query Permits) can use this to revoke (or more accurately, renounce) the permissions it has received. A token owner can also call this function to revoke permissions, although `GivePermission` can also be used for this purpose.  

### CurateTokenIds
Curators MUST be able to access this function. Other addresses MUST NOT be able to call this function. (Note that admins cannot mint unless it is also a curator). 

A curator MUST be able to create new `token_id`s. The curator MUST be able to configure the token_id and set initial balances. A curator MUST NOT be able to mint a token_id with a `token_id` unique identifier that already exists.

`CurateTokenIds` MUST be able to create multiple `token_id`s and set multiple initial balances in a single transaction. Therefore, `BatchCurateTokenIds` is not necesary. 

### MintTokens
Minters of a given token_id MUST be able to access this function. Other addresses MUST NOT be able to call this function. (Note that admins cannot mint unless it is also a minter). 

A minter MUST be able to mint tokens on existing `token_id`s if the configuration allows it to. If a token_id is an NFT, minters MUST NOT be able to mint additional tokens; NFTs SHALL only be minted at most once. The token configuration SHOULD specify whether minters are allowed to mint additional tokens (for fungible tokens).

`MintTokens` MUST be able to mint multiple tokens across multiple `token_id`s in a single transaction. Therefore, `BatchMintTokens` is not necessary.

### BurnTokens
Owners of tokens MUST be allowed to burn their tokens only if the `token_id` configuration allows it to. The base specification does not allow any address to burn tokens they do not own, but this feature is OPTIONAL.

`BurnTokens` MUST be able to burn multiple tokens across multiple `token_id`s in a single transaction. Therefore, `BatchBurnTokens` is not necessary.

### AddCurators and RemoveCurators
The admin MUST be able to access this function. Other addresses MUST NOT be able to call this function. AddCurators add one or more curators to the list of curators, while RemoveCurators remove one or more curators from the list of curators. Note that a given SNIP1155 contract instance share a consistent list of curators.

### AddMinters and RemoveMinters
The admin and token_id curator MUST be able to access this function. Other addresses MUST NOT be able to call this function. AddMinters add one or more minters to the list of minters for a given token_id, while RemoveMinters remove one or more minters from the list of minters for a given token_id.

### ChangeAdmin
The admin MUST be able to access this function. Other addresses MUST NOT be able to call this function. This function allows an admin to change the admin address. When this happens, the message caller SHOULD lose its own admin rights. The base specifications allow only one admin at a time. If multiple admins are implemented, a public query MUST be available for anyone to view all the admin addresses. 

### BreakAdminKey
The admin MUST be able to access this function. Other addresses MUST NOT be able to call this function. This function allows an admin to revoke its admin rights, without assigning a new admin. Doing this results in a contract with no admin.


## Queries

### ContractInfo



### Minters

### NumTokens


## Authenticated Queries
Authenticated queries can be made using viewing keys or query permits.

### Balance

### BatchBalance 

### Approved

<!-- ### ApprovedForAll -->

### IsApproved

<!-- ### IsApprovedForAll -->

### OwnerOf
### TokenInfo
<!-- ### AllTokenInfo -->
### PrivateMetadata
<!-- ### TokenDossier -->

<!-- ### InventoryApprovals -->
<!-- ### Tokens -->
### TransactionHistory


## Receiver functions

### RegisterReceive
This message is used to pair a code hash with a contract address. The SNIP1155 contract MUST store the `code_hash` sent in this message, and use it when calling the `Snip1155Receive` function.

### Snip1155Receive
When a the `Send` or `BatchSend` function is called, the SNIP1155 sends a callback to a registered address. When doing so, the SNIP1155 contract MUST call the `Snip1155Receive` handle function of the recipient contract. The callback message is in the following format:

```json
{
  "snip_1155_receive": {
    "sender": "<HumanAddr that called the transaction>",
    "token_id": "<String representing unique token_id being sent>",
    "from": "<HumanAddr of the current owner of the tokens>",
    "amount": "<Amount of tokens sent in Uint128>",
    "memo": "<optional String>",
    "msg": "<optional message in Binary>"
  }
}
```

## Miscellaneous

### Padding <!-- omit in toc --> 
Users may want to enforce constant length messages to avoid leaking data. To support this functionality, SNIP1155 tokens MUST support the option to include a padding field in every message. This optional padding field may be sent with ANY of the messages in this spec. Contracts and Clients MUST ignore this field if sent, either in the request or response fields.

### Requests <!-- omit in toc --> 
Requests SHOULD be sent as base64 encoded JSON. Future versions of Secret Network may add support for other formats as well, but at this time we recommend usage of JSON only. For this reason the parameter descriptions specify the JSON type which must be used. In addition, request parameters will include in parentheses a CosmWasm (or other) underlying type that this value must conform to. E.g. a recipient address is sent as a string, but must also be parsed to a bech32 address.

### Queries <!-- omit in toc --> 
Queries are off-chain requests, that are not cryptographically validated. This means that contracts that wish to validate the caller of a query MUST implement some sort of authentication. SNIP1155 uses an "API key" scheme, which validates a (viewing key, account) pair, as well as query permits (which utilizes digital signatures).

Authentication MUST happen on each query that reveals private account-specific information. Authentication MUST be a resource intensive operation, that takes a significant amount of time to compute. This is because such queries are open to offline brute-force attacks, which can be parallelized to scale linearly with the resources of a motivated attacker. If viewing keys are used, the authentication MUST perform the same computation even if the user does not have a viewing key set; and the authentication response MUST be indistinguishable for both the case of a wrong viewing key and the case of a non-existent viewing key.

### Responses <!-- omit in toc --> 
Unless specified otherwise, all message & query responses will be JSON encoded in the data field of the Cosmos response, rather than in the logs. This is meant to reduce the potential for data-leakage through side-channel attacks. In addition, since all keys will be encrypted, it is not possible to use the log events for event triggering.

### Success status <!-- omit in toc --> 
Some of the messages detailed in this document contain a "status" field. This field MUST hold one of two values: "success" or "failure".

While errors during execution of contract functions should usually result in a proper and detailed error response, the "failure" status is reserved for cases where a contract might choose to obfuscate the exact cause of failure, or otherwise indicate that while nothing failed to happen, the operation itself could not be completed for some valid reason.

### Balances and amounts <!-- omit in toc --> 
Note that all amounts are represented as numerical strings (the Uint128 type). Handling decimals is left to the UI.


## Schema



### Instantiation message
```js
{
  has_admin: boolean,
  admin?: string,
  minters: string[],
  initial_tokens: [{
    token_info: [{
      token_id: string, 
      name: string, 
      symbol: string, 
      token_config: "<token_config>",
      public_metadata: "<metadata>",
      private_metadata: "<metadata>",
    }],
    balances: [{
      address: string,
      amount: string,
    }]
  }],
  entropy: string,
} 
```

### Handle messages
```js
{
  mint_token_ids: {
    initial_tokens: [{
      token_info: [{
        token_id: string, 
        name: string, 
        symbol: string, 
        token_config: "<token_config>",
        public_metadata: "<metadata>",
        private_metadata: "<metadata>",
      }],
      balances: [{
        address: string,
        amount: string,
      }]
    }],
    memo?: string,
    padding?: string,
  }
}
{
  mint_tokens: {
    mint_tokens: [{
      token_id: string,
      balances: [{
        address: string,
        amount: string,
      }]
    }],
    memo?: string,
    padding?: string,
  }
}
{   
  burn_tokens {
    burn_tokens: [{
      token_id: string,
      balances: [{
        address: string,
        amount: string,
      }]
    }],
    memo?: string,
    padding?: string,
  }
}
{
  change_metadata {
      token_id: string,
      public_metadata?: "<metadata>",
      private_metadata?: "<metadata>",
  }
}
{
  transfer {
    token_id: string,
    from: string,
    recipient: string,
    amount: string,
    memo?: string,
    padding?: string,
  },
}
{
  batch_transfer {
    actions: [{
      token_id: string,
      from: string,
      recipient: string,
      amount: string,
      memo?: string,
    }],
    padding?: string,
  },
}
{
  send {
    token_id: string,
    from: string,
    recipient: string,
    recipient_code_hash?: string,
    amount: string,
    msg?: "<binary>", // the binary of object x is: toBase64(toUtf8(JSON.stringify(x)))
    memo?: string,
    padding?: string,
  },
}
{
  batch_send {
    actions: [{
      token_id: string,
      from: string,
      recipient: string,
      recipient_code_hash?: string,
      amount: string,
      msg?: "<binary>", // the binary of object x is: toBase64(toUtf8(JSON.stringify(x))),
      memo?: string,
    }],
    padding?: string,
  },
}
{
  give_permission {
    allowed_address: string,
    token_id: String,
    view_balance?: boolean,
    view_balance_expiry?: "<expiration>",
    view_private_metadata?: boolean,
    view_private_metadata_expiry?: "<expiration>",
    transfer?: string,
    transfer_expiry?: "<expiration>",
    padding?: string,
  },
}
{
  revoke_permission {
    token_id: string,
    owner: string,
    allowed_address: string,
    padding?: string,
  },
}
{
  register_receive {
    code_hash: string,
    padding?: string,
  },
}
{
  create_viewing_key {
    entropy: string,
    padding?: string,
  },
}
{
  set_viewing_key {
    key: string,
    padding?: string,
  },
}
{
  add_minters {
    add_minters: string[],
    padding?: string,
  },
}
{
  remove_minters {
    remove_minters: string[],
    padding?: string,
  },
}
{
  change_admin {
    new_admin: string,
    padding?: string,
  },
}
{
  break_admin_key {
    current_admin: string,
    contract_address: string,
    padding?: string,
  },
}
{
  revoke_permit {
    permit_name: string,
    padding?: string,
  },
}
```

Object schemas

`token_config` can be one of the two variants below (see [token hierarchy](#token-hierarchy)):
```js
{
  fungible: {
    decimals: number,
    public_total_supply: boolean,
    enable_mint: boolean,
    enable_burn: boolean,
    minter_may_update_metadata: boolean,
  }
}
{
  nft: {
    public_total_supply: boolean,
    owner_is_public: boolean,
    enable_burn: boolean,
    minter_may_update_metadata: boolean,
    owner_may_update_metadata: boolean,
  }
}
```

`metadata` (see [token hierarchy](#token-hierarchy)):
```js
{
  token_uri?: string,
  extension?: "<any object>",
}
```

`expiration` can be one of the three variants below
```js
{
  at_height: number,
}
{
  at_time: number,
}
{
  never,
}
```

### Handle responses
```js
{ mint_token_ids { status: "success" || "failure" }},
{ mint_tokens { status: "success" || "failure" }},
{ burn_tokens { status: "success" || "failure" }},
{ change_metadata { status: "success" || "failure" }},
{ transfer { status: "success" || "failure" }},
{ batch_transfer { status: "success" || "failure" }},
{ send { status: "success" || "failure" }},
{ batch_send { status: "success" || "failure" }},
{ give_permission { status: "success" || "failure" }},
{ revoke_permission { status: "success" || "failure" }},
{ register_receive { status: "success" || "failure" }},
{ create_viewing_key { key: "<viewing key string>" }},
{ set_viewing_key{ status: "success" || "failure" }},
{ add_minters { status: "success" || "failure" }},
{ remove_minters { status: "success" || "failure" }},
{ change_admin { status: "success" || "failure" }},
{ break_admin_key { status: "success" || "failure" }},
{ revoke_permit { status: "success" || "failure" }},
```

### Query messages

```js
{
  contract_info {  }
}
{
  balance {
    owner: string,
    viewer: string,
    key: string,
    token_id: string,
  }
}
{
  transaction_history {
    address: string,
    key: string,
    page?: number,
    page_size: number,
  }
}
{
  permission {
    owner: string,
    allowed_address: string,
    key: string,
    token_id: string,
  }
}
{
  all_permissions {
    address: string,
    key: string,
    page?: number,
    page_size: number,
  }
}
{
  token_id_public_info { 
    token_id: string 
    }
}
{
  token_id_private_info { 
    address: string,
    key: string,
    token_id: string,
  }
}
{
  registered_code_hash {
    contract: string
  }
}
{
  with_permit {
    permit: "<permit>",
    query: "<query_with_permit>",
  }
}
```

`query_with_permit` schema
```js
{
  balance { 
    owner: string, 
    token_id: string 
  }
}
{
  transaction_history {
    page?: number,
    page_size: number,
  }
}
{
  permission {
    owner: string,
    allowed_address: string,
    token_id: string,
  }
}
{
  all_permissions {
    page?: number,
    page_size: number,
  }
}
{
  token_id_private_info { 
    token_id: string,
  }
}
```

### Query responses

```js
{
  contract_info {
    admin: string,
    minters: string[],
    all_token_ids: string[],
  }
}
{
  balance {
    amount: string,
  }
}
{
  transaction_history {
    txs: [{
      tx_id: number,
      block_height: number,
      block_time: number,
      token_id: string,
      action: "<tx_action>",
      memo?: string,
    }],
    total?: number,
  }
}
// returns null if no permission 
{
  permission?: {
    view_balance_perm: boolean,
    view_balance_exp: "<expiration>",
    view_pr_metadata_perm: boolean,
    view_pr_metadata_exp: "<expiration>",
    trfer_allowance_perm: string, 
    trfer_allowance_exp: "<expiration>",
  }
}
{
  all_permissions{
    permission_keys: [{
      token_id: string,
      allowed_addr: string,
    }],
    permissions: [{
      view_balance_perm: boolean,
      view_balance_exp: "<expiration>",
      view_pr_metadata_perm: boolean,
      view_pr_metadata_exp: "<expiration>",
      trfer_allowance_perm: string, 
      trfer_allowance_exp: "<expiration>",
    }],
    total: number,
  }
}
{
  token_id_public_info {
    token_id_info: {
      token_id: string, 
      name: string, 
      symbol: string, 
      token_config: "<token_config>",
      public_metadata: "<metadata>",
      private_metadata: null,
    },
    total_supply?: string,
    owner?: string
  }
}
{
  token_id_private_info {
    token_id_info: {
      token_id: string, 
      name: string, 
      symbol: string, 
      token_config: "<token_config>",
      public_metadata: "<metadata>",
      private_metadata: "<metadata>",
    },
    total_supply?: string,
    owner?: string
  }
}
/// returns None if contract has not registered with SNIP1155 contract
{
  registered_code_hash {
    code_hash?: string,
  }
}
{
  viewing_key_error {
    msg: string,
  }
}
```

`tx_action` can be one of the following variants
```js
  mint {
    minter: string,
    recipient: string,
    amount: string,
  },
  burn {
    burner?: string,
    owner: string,
    amount: string,
  },
  transfer {
    from: string,
    sender?: string,
    recipient: string,
    amount: string,
  },
```

# Additional specifications

Additional specifications include:
* Royalty for NFTs
* Private metadata for fungible tokens
* Ability for owners to give other addresses permission to burn their tokens 
* Ability to view nft ownership history, including configuration on whether this should be public. In the base standard reference implementation, this information is already being saved, but not accessible through queries.
* Ability for an address to view list of all permissions that it has been granted by others (currently only granter can view comprehensive list of its permissions)
* Sealed metadata and Reveal functionality that mirrors SNIP721
* Ability for admin to restrict certain types of transactions (as seen in SNIP20 and SNIP721). A design decision was made on SNIP1155 NOT to include this functionality in the base specifications, in order to encourage more permissionless contract designs.
* Ability for an owner to give another address batch permission that covers all its token_ids. 
* Ability for query permits to selectively allow access to other query functions (in the base specifications, selective viewership permissions cover balances and private metadata only)


# Design decisions

## Reference implementation goals
The SNIP1155 standards and reference implementation aims to provide developers with an additional option to use as a base contract for creating tokens. The aim is to complement the existing SNIP20 and SNIP721 standards by offering a lean contract whose core architecture focuses on the ability to create and manage multiple token types.

Non-goals include:
* providing a feature-rich base contract that emcompasses both SNIP20 and SNIP721 functionality
* superseding previous token standards
* backward compatability with previous token standards, although [familiar](#familiar-interface) interfaces are used where possible)

## Starting from a blank slate
The current Secret Network token standard reference implementations at the time of writing (particularly SNIP721) is feature-rich and implements many useful features beyond the base standards. This makes it possible to use one of these contracts as a starting point and add functionality in order to create the SNIP1155 reference implementation.

However, the decision was to build the SNIP1155 reference implementation mostly from a blank slate. This creates a leaner contract with several advantages from a systems design point of view:
* Mitigates code bloat. The SNIP1155 standard implementation generally aims to avoid implementing non-core features, and starting from a blank slate avoids adopting features from another standard that are not critical for SNIP1155.
* Reduces surface area of attack and potential for bugs.
* Offers developers an additional option to use as a base contract that is meaningfully different to existing templates, so developers can choose the reference architecture that most closely fits their requirements.  
* Allows SNIP1155 to follow an independent update cycle and its own development path, making it more responsive to developments in feature requirements or changes to the network infrastructure that is impactful to SNIP1155 use cases.

The disadvantages of this design decision are:
* No (full) backward compatability with SNIP20 or SNIP721
* Requires more developer hours to create

## Permissionless design
The standard implementations of both SNIP20 and SNIP721 (at the time of writing) are designed to have an admin with special rights. This can be a critical feature depending on the required use case, but has the downside of not being truly permissionless.    

With this in mind, the SNIP1155 base standard implementation natively provides the ability to break admin keys at any point, as well as for contract instantiators to create a no-admin contract instances. 

## Familiar interface
The interface reflects some of the message schemas in [SNIP20](https://github.com/scrtlabs/snip20-reference-impl) and [SNIP721](https://github.com/baedrik/snip721-reference-impl), in order to have a familiar interface across Secret Network. However, SNIP1155 does not aim to have backward compatability with SNIP20 and SNIP721.

## Keeping base and additional features separate
The reference implementation aims to maintain a base implementation that is lean (with no additional features), while eventually also offering template code for some additional feature(s) as separate packages within this repository. Keeping the base implementation and additional features in separate packages avoids the situation where developers are forced to adopt all additional features even if their use case does not require them.

