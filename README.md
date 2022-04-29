SNIP1155 Reference Implementation: Private Multitokens  <!-- omit in toc --> 
==============

This is the standard reference implementation of the [SNIP1155 Standard Specifications](#base-specifications).


## Table of contents <!-- omit in toc --> 
- [Base specifications](#base-specifications)
  - [Abstract](#abstract)
  - [Terms](#terms)
  - [Token hierarchy](#token-hierarchy)
  - [Instantiation](#instantiation)
  - [The admin role](#the-admin-role)
  - [The minter(s) role](#the-minters-role)
  - [NFT vs fungible tokens](#nft-vs-fungible-tokens)
  - [Handle messages](#handle-messages)
    - [Transfer](#transfer)
    - [Send](#send)
    - [BatchTransfer and BatchSend](#batchtransfer-and-batchsend)
    - [CreateViewingKey and SetViewingKey](#createviewingkey-and-setviewingkey)
    - [Allowances and private metadata viewership](#allowances-and-private-metadata-viewership)
    - [Revoke](#revoke)
    - [Mint, BatchMint, Burn and BatchBurn](#mint-batchmint-burn-and-batchburn)
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
- [Additional specifications](#additional-specifications)
  - [Royalty](#royalty)
  - [Sealed metadata and Reveal](#sealed-metadata-and-reveal)
- [Design decisions](#design-decisions)
  - [Reference implementation goals](#reference-implementation-goals)
  - [Starting from a blank slate](#starting-from-a-blank-slate)
  - [Permissionless design](#permissionless-design)
  - [Familiar interface](#familiar-interface)
  - [Keeping base and additional features separate](#keeping-base-and-additional-features-separate)



# Base specifications

## Abstract

This SNIP1155 specification ("spec" or "specs") outlines the functionality and interface of a [Secret Network](https://github.com/scrtlabs/SecretNetwork) contract that can manage multiple token types. The specifications are loosely based on [CW1155](https://lib.rs/crates/cw1155) which is in turn based on Ethereum's [ERC1155](https://eips.ethereum.org/EIPS/eip-1155#non-fungible-tokens), with an additional privacy layer made possible as a Secret Network contract.

SNIP1155 allows a single contract to manage multiple tokens. This can be a combination of fungible tokens and non-fungible tokens, each with separate configurations, attributes and metadata. Fungible and non-fungible tokens are mostly treated equally; the key difference is that NFTs have total_supply of 1 and can only be minted once. Unlike CW1155 where approvals must cover the entire set of tokens, users interacting with SNIP1155 contracts are able to control which tokens fall in scope for a given approval. This is a feature from [ERC1761](https://eips.ethereum.org/EIPS/eip-1761). 

The ability to hold multiple token types provides new functionality (such as batch transferring multiple token types), as well as improved developer and user experience. For example, using a SNIP1155 contract instead of multiple SNIP20 and SNIP721 contracts can reduce the required number of approval transactions, inter-contract messages, and dedicated factory contracts. These translate to better user experiences, simpler application development, and lower gas fees.

See [design decisions](#design-decisions) for more details.

## Terms
*The key words "MUST", "MUST NOT", "REQUIRED", "SHALL", "SHALL NOT", "SHOULD", "SHOULD NOT", "RECOMMENDED", "MAY", and "OPTIONAL" in this document are to be interpreted as described in [RFC 2119](https://datatracker.ietf.org/doc/html/rfc2119).*

This memo uses the terms defined below:
* Message - an on-chain interface. It is triggered by sending a transaction, and receiving an on-chain response which is read by the client. Messages are authenticated both by the blockchain and by the Secret enclave.
* Query - an off-chain interface. Queries are done by returning data that a node has locally, and are not public. Query responses are returned immediately, and do not have to wait for blocks. In addition, queries cannot be authenticated using the standard interfaces. Any contract that wishes to strongly enforce query permissions must implement it themselves.
* Cosmos Message Sender - the account found under the sender field in a standard Cosmos SDK message. This is also the signer of the message.
* Native Asset - a coin which is defined and managed by the blockchain infrastructure, not a Secret contract.


## Token hierarchy

SNIP1155 contract > token_id > token(s)

At the highest level, all tokens of a given SNIP1155 contract MUST share:
* admin (if enabled)
* minter(s)

A SNIP1155 contract SHOULD have the ability to hold multiple `tokens_id`s, each of which MUST have their own:
* token_id
* name
* symbol
* total supply
* is_nft
* token_config
* public metadata
* private metadata
* extension

Each `token_id` can have 1 or more `tokens`, which are indistinguishable from one another (hence fungible).

| Variable         | Type           | Description                                               | Optional |
| ---------------- | -------------- | --------------------------------------------------------- | -------- |
| admin            | HumanAddr      | Can to add/remove minters and break admin key             | Yes      |
| minter           | Vec<HumanAddr> | Can mint/burn tokens and change metadata if config allows | Yes      |
| token_id         | String         | Unique identifier                                         | No       |
| name             | String         | Name of fungible tokens or NFT                            | No       |
| symbol           | String         | Symbol of fungible tokens or NFT                          | No       |
| total_supply     | Uint128        | Total tokens of a given token_id. MUST be == 1 for NFTs   | No       |
| is_nft           | bool           | Determines if token_id is an NFT                          | No       |
| token_config     | TokenConfig    | Includes enable burn, enable additional minting. etc      | No       |
| public metadata  | Metadata       | Publicly viewable `uri` and `extension`                   | No       |
| private metadata | Metadata       | Non-publicly viewable `uri` and `extension`                   | No       |

Application developers MAY change `extension` to any `struct` that fits their use case.


## Instantiation
The instantiator MUST have the option to specify the admin, minter(s) and initial balances. 

If no admin is specified, the instantiator MAY be used as the default admin. Additionally, there SHOULD be an input field `has_admin: bool` which allows the instantiator to instantiate a no-admin contract. If `has_admin == false`, any admin input MUST be ignored by the contract.  

The `initial_balances` input SHOULD allow an arbitrary number of token_ids and tokens to be created at instantiation. This design makes it convenient to instantiate permissionless contracts with no admin and minters, as all the required tokens can be minted upon instantiation.

## The admin role
The role of the admin (if exists) is to add and remove minters. 

The admin MUST be able to perform admin-only transactions. Admin-only transactions MUST NOT be callable by non-admins. Admin-only function MUST include `add_minters`, `remove_minters` and `change_admin` and `break_admin_key`.

The existence of the `break_admin_key` function is enforced in SNIP1155 standards in order to encourage permissionless designs. 

## The minter(s) role
A minter mints and burns tokens. Minters can also change the public and private metadata if the configuration of the token_id allows this. 

Details of functions accessible to minters are described [here](#mint-batchmint-burn-and-batchburn).

## NFT vs fungible tokens
Public metadata and and private metadata (together "metadata") can be optionally set for both NFTs and fungible tokens. An NFT owner SHOULD be able to change the metadata if the configuration allows it to; fungible token metadata cannot be changed in the standard implementation[^1] but is OPTIONAL within this standards. The rules that govern how fungible metadata is changed is left to application developers to decide.

Private metadata of an NFT MUST NOT be viewable by any address, other than the owner address or addresses that have been given permission to[^2]. The standard implementation allows any owner of a fungible token to view private metadata of the fungible `token_id`, but different rules MAY be implemented by application developers.  

[^1]: See the [fractionalized NFT reference implementation](https://github.com/DDT5/frac-snft-ref-impl) if the use case requires multiple addresses to owning an NFT.
[^2]: The base specifications do not have sealed metadata functionality, which is included in the additional specifications.


## Handle messages

### Transfer
Transfers a specified amount of tokens of a single `token_id` from one address to another. If the transaction caller is not the current owner of the token, a successful transaction MUST require that the caller has the required allowances to transfer the tokens.  

The SNIP1155 `Transfer` interface more closely reflects SNIP721 than SNIP20. SNIP20's `Transfer` and `TransferFrom` functions can both be performed using SNIP1155's `Transfer` function. 

### Send
Similar to `Transfer`. If the recipient has registered itself with a `RegisterReceive` message, this function MUST also send an inter-contract message ("callback") to the recipient. 

The SNIP1155 `Send` interface more closely reflects SNIP721 than SNIP20. SNIP20's `Send` and `SendFrom` functions can both be performed using SNIP1155's `Send` function. 

### BatchTransfer and BatchSend
These functions perform multiple `Transfer`, or `Send` actions in a single transaction. Multiple `token_ids` MUST be allowed to be transfered or sent in a batch, including a mix of NFTs and fungible tokens of the same SNIP1155 contract.

`BatchSend` MUST allow different callback messages to be sent for each `Send` action.

### CreateViewingKey and SetViewingKey 
These perform the same functions as specified in the SNIP20 standards.

### Allowances and private metadata viewership
`SetWhitelistApproval` reflects the functionality in SNIP721. For SNIP1155, this is used by an address to grant other addresses permissions to view or transfer its tokens. This function MUST allow the transaction caller to set the `token_id`s that fall in scope of a given approval (unlike in CW1155, where approvals are global).  

It is OPTIONAL to include `IncreaseAllowance` and `DecreaseAllowance` messages, as these are familiar SNIP20 interfaces. These are optional because their functionalities can be performed with `SetWhitelistApproval`. 

### Revoke
An operator with existing permissions can use this to revoke (or more accurately, renounce) the permissions it has received. A token owner can also call this function to revoke permissions, although `SetWhitelistApproval` can also be used for this purpose.  

### Mint, BatchMint, Burn and BatchBurn
These functions MUST NOT be accessible to any address other than minters'. (Note that admins cannot mint unless it is also a minter). 

A minter MUST be able to create new token_ids. A minter MUST be able to mint and burn tokens on existing token_ids if the configuration allows it to. If a token_id is an NFT, minters MUST NOT be able to mint additional tokens; NFTs SHALL only be minted at most once. The token configuration SHOULD specify whether minters are allowed to mint additional tokens (for fungible tokens) or burn tokens (both fungible and non-fungible tokens).

### AddMinters and RemoveMinters
These functions MUST NOT be accessible to any address other than the admin's. AddMinters add one or more minters to the list of minters, while RemoveMinters remove one or more minters from the list of minters.

### ChangeAdmin
This function MUST NOT be accessible to any address other than the admin's. This function allows an admin to change the admin address. When this happens, the message caller SHOULD lose its own admin rights. The base specifications allow only one admin at a time. If multiple admins are implemented, a public query SHOULD be available for anyone to view all the admin addresses. 

### BreakAdminKey
This function MUST NOT be accessible to any address other than the admin's. This function allows an admin to revoke its admin rights, without assigning a new admin. Doing this results in a contract with no admin.


<!-- 

### Approve

### ApproveAll

### RevokeAll

-->

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
    "from": "<HumanAddr of the current owner of the tokens>",
    "token_id": "<String representing unique token_id being sent>",
    "amount": "<Amount of tokens sent in Uint128>",
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


# Additional specifications

## Royalty


## Sealed metadata and Reveal



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
The interface reflects some of the message schemas in [SNIP20](https://github.com/scrtlabs/snip20-reference-impl) and [SNIP721](https://github.com/baedrik/snip721-reference-impl), in order to have a familiar interface across Secret Network. However, SNIP1155 does not aim to have full backward compatability with SNIP20 and SNIP721.

## Keeping base and additional features separate
The reference implementation aims to maintain a base implementation that is lean (with no additional features), while eventually also offering template code for some additional feature(s) as separate packages within this repository. Keeping the base implementation and additional features in separate packages avoids the situation where developers are forced to adopt all additional features even if their use case does not require them.

