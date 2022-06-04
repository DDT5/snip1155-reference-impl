initSidebarItems({"enum":[["StoredTxAction","tx type and specifics for storage"],["TxAction","tx type and specifics for storage with HumanAddr"]],"fn":[["append_new_owner","stores ownership history for a given token_id. Meant to be used for nfts. In base specification, only the latest (ie: current) owner is relevant. But this design pattern is used to allow viewing a token_id’s ownership history,  which is allowed in the additional specifications"],["get_txs","Returns StdResult<(Vec, u64)> of the txs to display and the total count of txs"],["may_get_current_owner",""],["store_burn",""],["store_mint",""],["store_transfer",""]],"struct":[["StoredTx","tx in storage"],["Tx","tx in storage"]]});