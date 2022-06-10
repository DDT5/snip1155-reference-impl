import { SecretNetworkClient, toUtf8, fromUtf8, Tx, toBase64, Permit } from "secretjs";
import fs from "fs";
import assert from "assert";
import { initClient, generatePermit } from "./int_helpers";
import { Account, ContractInfo, jsEnv } from "./int_utils";
import { Metadata } from "secretjs/dist/extensions/snip721/types";


/////////////////////////////////////////////////////////////////////////////////
// Type declarations
/////////////////////////////////////////////////////////////////////////////////

type Balance = {
  address: string,
  amount: string,
};

type TknConf = TknConfFungible | TknConfNft;

interface TknConfFungible { "fungible": {
  minters: string[],
  decimals: number,
  public_total_supply: boolean,
  enable_mint: boolean, 
  enable_burn: boolean, 
  minter_may_update_metadata: boolean,
}}

interface TknConfNft { "nft": {
  public_total_supply: boolean,
  enable_burn: boolean, 
  owner_may_update_metadata: boolean,
}}

type Permission = {
  view_balance_perm: boolean,
  view_balance_exp: string,
  view_pr_metadata_perm: boolean,
  view_pr_metadata_exp: string,
  trfer_allowance_perm: string,
  trfer_allowance_exp: string
}

// type GenErr = { generic_err: { msg: string }};

/////////////////////////////////////////////////////////////////////////////////
// Upload contract and Init Message
/////////////////////////////////////////////////////////////////////////////////

// Stores and instantiaties a new contract in our network
const initializeContract = async (
  client: SecretNetworkClient,
  contractPath: string,
  initMsg: unknown, // any
) => {
  // upload contract
  const wasmCode = fs.readFileSync(contractPath);
  console.log("Uploading contract");

  const uploadReceipt = await client.tx.compute.storeCode(
    {
      wasmByteCode: wasmCode,
      sender: client.address,
      source: "",
      builder: "",
    },
    {
      broadcastCheckIntervalMs: 100,
      gasLimit: 5000000,
    }
  );

  if (uploadReceipt.code !== 0) {
    console.log(
      `Failed to get code id: ${JSON.stringify(uploadReceipt.rawLog)}`
    );
    throw new Error(`Failed to upload contract`);
  }

  const codeIdKv = uploadReceipt.jsonLog![0].events[0].attributes.find(
    (a: any) => {
      return a.key === "code_id";
    }
  );

  const codeId = Number(codeIdKv!.value);
  console.log("Contract codeId: ", codeId);

  const contractCodeHash = await client.query.compute.codeHash(codeId);
  console.log(`Contract hash: ${contractCodeHash}`);

  // instantiate contract
  const contract = await client.tx.compute.instantiateContract(
    {
      sender: client.address,
      codeId,
      initMsg: initMsg, 
      codeHash: contractCodeHash,
      // label: "My contract" + Math.ceil(Math.random() * 10000), // The label should be unique for every contract, add random string in order to maintain uniqueness
      label: "Contract " + Math.ceil(Math.random() * 10000) + client.address.slice(6),  // using random number 0..10000 is not big enough, sometimes has collision
    },
    {
      broadcastCheckIntervalMs: 100,
      gasLimit: 1000000,
    }
  );

  if (contract.code !== 0) {
    throw new Error(
      `Failed to instantiate the contract with the following error ${contract.rawLog}`
    );
  }

  const contractAddress = contract.arrayLog!.find(
    (log) => log.type === "message" && log.key === "contract_address"
  )!.value;

  console.log(`Contract address: ${contractAddress}`);

  const contractInfo: [string, string] = [contractCodeHash, contractAddress];
  return contractInfo;
};

// Initialization procedure: Initialize client, fund new accounts, and upload/instantiate contract
async function initDefault(): Promise<jsEnv> {
  const accounts = await initClient();
  const { secretjs } = accounts[0];

  const public_metadata: Metadata = {
    token_uri: "public_token_uri",
  };
  const private_metadata: Metadata = {
    token_uri: "private_token_uri",
    extension: {
      image_data: "some image data",
      protected_attributes: ["some protected attributes"]
    }
  };

  const initMsgDefault = { 
    has_admin: true,
    curators: [accounts[0].address],
    initial_tokens: [
      {
        token_info: { 
          token_id: "0", 
          name: "token0", 
          symbol: "TKN", 
          token_config: { fungible: {
              minters: [accounts[0].address],
              decimals: 6,  
              public_total_supply: true,
              enable_mint: true, 
              enable_burn: true, 
              minter_may_update_metadata: true,
          }},
          public_metadata,
          private_metadata,
        }, 
        balances: [{ 
            address: accounts[0].address, 
            amount: "1000" 
        }]
      },
      {
        token_info: { 
          token_id: "1", 
          name: "token1", 
          symbol: "TKNA", 
          token_config: { fungible: {
              minters: [accounts[0].address],
              decimals: 6,   
              public_total_supply: false,
              enable_mint: false, 
              enable_burn: false, 
              minter_may_update_metadata: true,
          }},
        }, 
        balances: [{ 
            address: accounts[0].address, 
            amount: "1000" 
        }]
      },
      {
        token_info: { 
          token_id: "2", 
          name: "nftname", 
          symbol: "NFT", 
          token_config: { nft: {
              minters: [],
              public_total_supply: true,
              owner_is_public: true,
              enable_burn: true, 
              owner_may_update_metadata: true,
              minter_may_update_metadata: true,
          }},
          public_metadata,
          private_metadata,
        }, 
        balances: [{ 
            address: accounts[0].address, 
            amount: "1" 
        }]
      },
    ],
    entropy: "entropyhere"
  };
  
  const [contractHash, contractAddress] = await initializeContract(
    secretjs,
    "contract.wasm",
    initMsgDefault,
  );

  const contract: ContractInfo = {
    hash: contractHash,
    address: contractAddress
  };

  const env: jsEnv = {
    accounts,
    contracts: [contract],
  }; 

  return env;
}

async function initDefaultWithReceiver() {
  // upload and instantiate SNIP1155 contract
  let env: jsEnv = await initDefault();
  
  // upload and instantiate receiver
  const { secretjs } = env.accounts[0];
  const receiverInitMsg = {
    count: 10
  };
  const [contractHash, contractAddress] = await initializeContract(
    secretjs,
    "tests/example-receiver/contract.wasm",
    receiverInitMsg,
  );
  const receiverContract: ContractInfo = {
    hash: contractHash,
    address: contractAddress
  };
  env.contracts.push(receiverContract);
  
  return env;
}

/////////////////////////////////////////////////////////////////////////////////
// Handle Messages
/////////////////////////////////////////////////////////////////////////////////

async function execHandle(
  sender: Account,
  contract: ContractInfo,
  msg: object,
  handle_description?: string
) {
  const { secretjs } = sender;
  const tx = await secretjs.tx.compute.executeContract(
    {
      sender: secretjs.address,
      contractAddress: contract.address,
      codeHash: contract.hash,
      msg,
      sentFunds: [],
    },
    {
      broadcastCheckIntervalMs: 100,
      gasLimit: 200000,
    }
  );

  if (handle_description === undefined) { handle_description = "handle"}
  //const parsedTxData = JSON.parse(fromUtf8(tx.data[0])); 
  console.log(`${handle_description} used ${tx.gasUsed} gas`);
  return tx
}

/** @param token_config overrides `is_nft` */
async function curateTokenIds(
  sender: Account,
  contract: ContractInfo,
  token_id: string,
  token_name: string,
  token_symbol: string,
  is_nft: boolean,
  init_mint_to_address: string,
  init_mint_amount: string,
  minters: string[],
  token_config?: TknConf,
) {
  let tkn_conf = token_config;
  if (token_config === undefined) {
    if (is_nft == false) {
      tkn_conf = { fungible: {
        minters,
        decimals: 6,
        public_total_supply: true,
        enable_mint: true, 
        enable_burn: true, 
        minter_may_update_metadata: true
      }} as TknConfFungible;
    } else if (is_nft === true) {
      tkn_conf = { nft: {
        minters,
        public_total_supply: true,
        owner_is_public: true,
        enable_burn: true, 
        owner_may_update_metadata: true,
        minter_may_update_metadata: true
      }} as TknConfNft;
    } 
  }

  const msg = {
    curate_token_ids: { 
      initial_tokens: [{
        token_info: { 
          token_id, 
          name: token_name, 
          symbol: token_symbol, 
          token_config: tkn_conf,
        }, 
        balances: [{ 
            address: init_mint_to_address, 
            amount: init_mint_amount 
        }]
      }]
    },
  }

  const tx = await execHandle(sender, contract, msg, "curateTokenIds");
  return tx
}

async function mintTokens(
  sender: Account,
  contract: ContractInfo,
  token_id: string,
  balances: Balance[],
) {
  const msg = {
    mint_tokens: { 
      mint_tokens: [{
        token_id, 
        balances,
      }]
    },
  };

  const tx = await execHandle(sender, contract, msg, "mintTokens");
  return tx;
}

async function burnTokens(
  sender: Account,
  contract: ContractInfo,
  token_id: string,
  balances: Balance[],
) {
  const msg = {
    burn_tokens: { 
      burn_tokens: [{
        token_id, 
        balances,
      }]
    },
  };

  const tx = await execHandle(sender, contract, msg, "burnTokens");
  return tx;
}

async function setViewingKey(
  sender: Account,
  contract: ContractInfo,
  key: string,
) {
  const msg = {
        set_viewing_key: { key: key },
  };

  const tx = await execHandle(sender, contract, msg, "setViewingKey");
  return tx;
}

async function setViewingKeyAll(
  env: jsEnv,
) {
  for (const contr of env.contracts) {
    let tx: Promise<Tx>;
    for (let i=0; i<env.accounts.length; i++) {
        tx = setViewingKey(env.accounts[i], contr, "vkey"+i);
        if (i == env.accounts.length-1) { 
          await tx;
        }
    };
  };
}

async function transfer(
  sender: Account,
  contract: ContractInfo,
  token_id: string,
  from: Account,
  recipient: Account,
  amount: string,
): Promise<Tx> {

  const msg = {
    transfer: { 
      token_id,
      from: from.address,
      recipient: recipient.address,
      amount, 
    },
  };

  const tx = await execHandle(sender, contract, msg, "transfer");
  return tx;
}

/** @param msg a base64 string of a utf8 json string */
async function send(
  sender: Account,
  contract: ContractInfo,
  token_id: string,
  from: Account,
  recipient_contract: ContractInfo,
  amount: string,
  msg?: string,
): Promise<Tx> {
  const message = {
    send: { 
      token_id,
      from: from.address,
      recipient: recipient_contract.address,
      recipient_code_hash: recipient_contract.hash,
      amount, 
      msg
    },
  };

  const tx = await execHandle(sender, contract, message, "send");
  return tx;
}

async function givePermission(
  sender: Account,
  contract: ContractInfo,
  allowed_address: Account,
  token_id: string,
  view_balance?: boolean,
  view_balance_expiry?: object,
  view_private_metadata?: boolean,
  view_private_metadata_expiry?: object,
  transfer?: string,
  transfer_expiry?: object,
): Promise<Tx> {
  const msg = {
    give_permission: { 
      allowed_address: allowed_address.address,
      token_id,
      view_balance,
      view_balance_expiry,
      view_private_metadata,
      view_private_metadata_expiry,
      transfer,
      transfer_expiry,
    },
  };
  const tx = await execHandle(sender, contract, msg, "givePermission");
  return tx;
}

/////////////////////////////////////////////////////////////////////////////////
// Query Messages
/////////////////////////////////////////////////////////////////////////////////

async function execQuery(
  sender: Account,
  contract: ContractInfo,
  msg: object,
) {
  const { secretjs } = sender;

  const response = (await secretjs.query.compute.queryContract({
    contractAddress: contract.address,
    codeHash: contract.hash,
    query: msg,
  }));

  if (JSON.stringify(response).includes('parse_err"')) {
    throw new Error(`Query parse_err: ${JSON.stringify(response)}`);
  }
  
  return response;
}

async function queryContractInfo(
  sender: Account,
  contract: ContractInfo,
) {
  const { secretjs } = sender;
  type QueryResponse = { contract_info: { 
    admin: string,
    curators: string[],
    all_token_ids: string[],
  }};

  const msg = { contract_info: {} };

  const response = await execQuery(sender, contract, msg) as QueryResponse;
  return response.contract_info;
}

async function queryBalance(
  sender: Account,
  contract: ContractInfo,
  owner: string,
  key: string,
  token_id: string,
): Promise<string> {
  type QueryResponse = { balance: { amount: string }};

  const msg = { balance: {
      owner,
      viewer: sender.address,
      key: key,
      token_id: token_id,
  } };
  const response = await execQuery(sender, contract, msg) as QueryResponse;
  return response.balance.amount;
}

async function queryBalanceQPermit(
  sender: Account,
  contract: ContractInfo,
  owner: string,
  token_id: string,
) {
  // type QueryResponse = { balance: { amount: string }};
  let permit: Permit = await generatePermit(sender, contract);

  const msg = { with_permit: { 
    permit,
    query: { balance: {
      owner,
      token_id: token_id,
    }}
  }};
  const response = await execQuery(sender, contract, msg);// as QueryResponse;
  return response;
}

async function queryTransactionHistory(
  account: Account,
  contract: ContractInfo,
  address: string,
  key: string,
  page_size: number,
  page?: number,
) {
  // type QueryResponse = { };
  const msg = { transaction_history: { 
    address,
    key,
    page,
    page_size,
   }};
  const response = await execQuery(account, contract, msg); // as QueryResponse;
  return response;
}

async function queryTransactionHistoryQPermit(
  account: Account,
  contract: ContractInfo,
  page_size: number,
  page?: number,
) {
  const permit: Permit = await generatePermit(account, contract);
  // type QueryResponse = { };
  const msg = { with_permit: {
    permit,
    query: {
      transaction_history: { 
        page,
        page_size,
      }
    }
  }}; 
    
  const response = await execQuery(account, contract, msg); // as QueryResponse;
  return response;
}

async function queryPermission(
  account: Account,
  contract: ContractInfo,
  owner: string,
  allowed_address: string,
  key: string,
  token_id: string,
) {
  type QueryResponse = { permission: Permission };
  const msg = { permission: { 
    owner,
    allowed_address,
    key,
    token_id,
   }};
  const response = await execQuery(account, contract, msg) as QueryResponse;
  return response;
}

async function queryAllPermissions(
  account: Account,
  contract: ContractInfo,
  address: string,
  key: string,
  page_size: number,
  page?: number,
) {
  // type QueryResponse = { };
  const msg = { all_permissions: { 
    address,
    key,
    page,
    page_size,
   }};
  const response = await execQuery(account, contract, msg); // as QueryResponse;
  return response;
}

async function queryAllPermissionsQPermit(
  account: Account,
  contract: ContractInfo,
  page_size: number,
  page?: number,
) {
  let permit: Permit = await generatePermit(account, contract);

  // type QueryResponse = { all_permissions: { permission_keys: object[], permissions: object[], total: number }};
  const msg = { with_permit: {
    permit,
    query: {
      all_permissions: {
        page,
        page_size,
      }
    }
  }};
  const response = await execQuery(account, contract, msg); // as QueryResponse;
  return response;
}

async function queryTokenIdPublicInfo(
  account: Account,
  contract: ContractInfo,
  token_id: string,
) {
  // type QueryResponse = { };
  const msg = { token_id_public_info: { token_id }};
  const response = await execQuery(account, contract, msg); // as QueryResponse;
  return response;
}

async function queryTokenIdPrivateInfoQPermit(
  account: Account,
  contract: ContractInfo,
  token_id: string,
) {
  const permit = await generatePermit(account, contract);
  // type QueryResponse = { };
  const msg = { with_permit: {
    permit,
    query: {
      token_id_private_info: {
        token_id,
      }
    }
  }};
  const response = await execQuery(account, contract, msg); // as QueryResponse;
  return response;
}

async function queryRegisteredCodeHash(
  account: Account,
  contract: ContractInfo,
  input_contract: string,
) {
  type QueryResponse = { registered_code_hash: { code_hash?: string } };
  const msg = { registered_code_hash: { 
    contract: input_contract,
  }};
  const response = await execQuery(account, contract, msg) as QueryResponse;
  return response;
}

/////////////////////////////////////////////////////////////////////////////////
// Receiver messages
/////////////////////////////////////////////////////////////////////////////////

async function receiverIncrement(
  sender: Account,
  contract: ContractInfo,
): Promise<Tx> {
  const { secretjs } = sender;
  const tx = await secretjs.tx.compute.executeContract(
    {
      sender: secretjs.address,
      contractAddress: contract.address,
      codeHash: contract.hash,
      msg: {
        increment: {  },
      },
      sentFunds: [],
    },
    {
      broadcastCheckIntervalMs: 100,
      gasLimit: 200000,
    }
  );

  //const parsedTxData = JSON.parse(fromUtf8(tx.data[0])); 
  console.log(`Increment [receiver contract] used ${tx.gasUsed} gas`);
  return tx
}

async function receiverReset(
  sender: Account,
  contract: ContractInfo,
  number: number,
): Promise<Tx> {
  const { secretjs } = sender;
  const tx = await secretjs.tx.compute.executeContract(
    {
      sender: secretjs.address,
      contractAddress: contract.address,
      codeHash: contract.hash,
      msg: {
        reset: { count: number }
      },
      sentFunds: [],
    },
    {
      broadcastCheckIntervalMs: 100,
      gasLimit: 200000,
    }
  );

  //const parsedTxData = JSON.parse(fromUtf8(tx.data[0])); 
  console.log(`Reset [receiver contract] used ${tx.gasUsed} gas`);
  return tx
}

async function receiverRegister(
  sender: Account,
  contract: ContractInfo,
  reg_addr: string,
  reg_hash: string,
): Promise<Tx> {
  const { secretjs } = sender;
  const tx = await secretjs.tx.compute.executeContract(
    {
      sender: secretjs.address,
      contractAddress: contract.address,
      codeHash: contract.hash,
      msg: {
        register: {
          reg_addr,
          reg_hash,
         },
      },
      sentFunds: [],
    },
    {
      broadcastCheckIntervalMs: 100,
      gasLimit: 200000,
    }
  );

  //const parsedTxData = JSON.parse(fromUtf8(tx.data[0])); 
  console.log(`Register [receiver contract] used ${tx.gasUsed} gas`);
  return tx
}

async function receiverSnip1155Receive(
  sender: Account,
  contract: ContractInfo,
  token_id: string,
  from: string,
  amount: string,
  memo?: string,
  msg?: string, 
): Promise<Tx> {
  const { secretjs } = sender;
  const tx = await secretjs.tx.compute.executeContract(
    {
      sender: secretjs.address,
      contractAddress: contract.address,
      codeHash: contract.hash,
      msg: {
        snip1155_receive: { 
          sender: sender.address,
          token_id,
          from,
          amount,
          memo,
          msg
        }
      },
      sentFunds: [],
    },
    {
      broadcastCheckIntervalMs: 100,
      gasLimit: 200000,
    }
  );

  //const parsedTxData = JSON.parse(fromUtf8(tx.data[0])); 
  console.log(`Snip1155Receive [receiver contract] used ${tx.gasUsed} gas`);
  return tx
}

async function receiverFail(
  sender: Account,
  contract: ContractInfo,
): Promise<Tx> {
  const { secretjs } = sender;
  const tx = await secretjs.tx.compute.executeContract(
    {
      sender: secretjs.address,
      contractAddress: contract.address,
      codeHash: contract.hash,
      msg: {
        fail: {  }
      },
      sentFunds: [],
    },
    {
      broadcastCheckIntervalMs: 100,
      gasLimit: 200000,
    }
  );

  //const parsedTxData = JSON.parse(fromUtf8(tx.data[0])); 
  console.log(`Fail [receiver contract] used ${tx.gasUsed} gas`);
  return tx
}

async function queryRecieverGetCount(
  account: Account,
  contract: ContractInfo,
) {
  const { secretjs } = account;
  type QueryResponse = { count: number };

  const response = (await secretjs.query.compute.queryContract({
    contractAddress: contract.address,
    codeHash: contract.hash,
    query: { get_count: {  } },
  })) as QueryResponse;

  if ('err"' in response) {
    throw new Error(
      `Query failed with the following err: ${JSON.stringify(response)}`
    );
  }
  
  return response.count;
}

/////////////////////////////////////////////////////////////////////////////////
// Tests
/////////////////////////////////////////////////////////////////////////////////

async function testIntializationSanity(
  env: jsEnv
): Promise<void> {
  const contract = env.contracts[0];
  const acc0 = env.accounts[0];

  const onInitializationData = await queryContractInfo(
    env.accounts[0],
    contract,
  );
  const exp_contract_info = {
    admin: acc0.address,
    curators: [acc0.address],
    all_token_ids: ["0","1","2"],
  };
  assert(
    JSON.stringify(onInitializationData) === JSON.stringify(exp_contract_info),
    `Contract info on initialization unexpected: ${JSON.stringify(onInitializationData)}`
  );

  await setViewingKey(acc0, contract, "vkey");
  const initBalance0: string = await queryBalance(
    acc0, contract, acc0.address, "vkey", "0"
  );
  assert(
    initBalance0 === "1000",
    `Initial balance expected to be "1000" instead of ${initBalance0}`
  );
  const initBalance1: string = await queryBalance(
    acc0, contract, acc0.address, "vkey", "1"
  );
  assert(
    initBalance1 === "1000",
    `Initial balance expected to be "1000" instead of ${initBalance1}`
  );
  const initBalance2: string = await queryBalance(
    acc0, contract, acc0.address, "vkey", "2"
  );
  assert(
    initBalance2 === "1",
    `Initial balance expected to be "1" instead of ${initBalance2}`
  );

  // public token info
  const tknId0 = await queryTokenIdPublicInfo(acc0, contract, "0"); 
  const tknId0String = JSON.stringify(tknId0);
  assert(tknId0String.includes('"token_id":"0"'));
  assert(tknId0String.includes('"token_config":{"fungible"'));
  assert(tknId0String.includes('"public_total_supply":true'));
  assert(tknId0String.includes('"total_supply":"1000"'));

  const tknId1 = await queryTokenIdPublicInfo(acc0, contract, "1"); 
  const tknId1String = JSON.stringify(tknId1);
  assert(tknId1String.includes('"token_id":"1"'));
  assert(tknId1String.includes('"token_config":{"fungible"'));
  assert(tknId1String.includes('"public_total_supply":false'));
  assert(tknId1String.includes('"total_supply":null'));

  const tknId2 = await await queryTokenIdPublicInfo(acc0, contract, "2"); 
  const tknId2String = JSON.stringify(tknId2);
  assert(tknId2String.includes('"token_id":"2"'));
  assert(tknId2String.includes('"token_config":{"nft"'));
  assert(tknId2String.includes('"public_total_supply":true'));
  assert(tknId2String.includes('"total_supply":"1"'));
}

async function testCurateTokenIds(
  env: jsEnv,
) {
  const minter = env.accounts[0];
  const contract = env.contracts[0];

  let tx = await curateTokenIds(minter, contract, "test0", "tokentest0", "TKNT", false, minter.address, "1000", []);
  assert(fromUtf8(tx.data[0]).includes(`{"curate_token_ids":{"status":"success"}}`));

  tx = await setViewingKey(minter, contract, "vkey");
  assert(tx.code === 0);
  let bal: string = await queryBalance(minter, contract, minter.address, "vkey", "test0");
  assert(bal === "1000");

  // cannot mint token_id with same name
  tx = await curateTokenIds(minter, contract, "test0", "tokentest0a", "TKNTA", false, minter.address, "123", []);
  assert(tx.rawLog.includes("token_id already exists. Try a different id String"));
  bal = await queryBalance(minter, contract, minter.address, "vkey", "test0");
  assert(bal === "1000");

  // can mint NFT
  tx = await curateTokenIds(minter, contract, "test1", "a new nft", "NFT", true, minter.address, "1", []);
  assert(tx.code === 0);
  bal = await queryBalance(minter, contract, minter.address, "vkey", "test1");
  assert(bal === "1");

  // cannot mint NFT with amount != 1
  tx = await curateTokenIds(minter, contract, "test2a", "a new nft", "NFTA", true, minter.address, "0", []);
  assert(tx.rawLog.includes("token_id test2a is an NFT; there can only be one NFT. Balances.amount must == 1"));
  tx = await curateTokenIds(minter, contract, "test2b", "a new nft", "NFTA", true, minter.address, "2", []);
  assert(tx.rawLog.includes("token_id test2b is an NFT; there can only be one NFT. Balances.amount must == 1"));
  bal = await queryBalance(minter, contract, minter.address, "vkey", "test2a"); assert(bal === "0");
  bal = await queryBalance(minter, contract, minter.address, "vkey", "test2b"); assert(bal === "0");  
}

async function testMintBurnTokens(
  env: jsEnv
): Promise<void> {
  const contract = env.contracts[0];
  let Bal = {
    address: env.accounts[0].address, 
    amount: "1" 
  } as Balance;
  let Bal1 = {
    address: env.accounts[1].address, 
    amount: "100" 
  } as Balance;

  // mint ------------------
  // cannot mint if not a minter
  let tx = await mintTokens(env.accounts[1], contract, "0", [Bal]);
  assert(tx.rawLog.includes('Only minters are allowed to mint'));
  
  // cannot mint non-existent token_ids
  tx = await mintTokens(env.accounts[0], contract, "na", [Bal]);
  assert(tx.rawLog.includes("token_id does not exist. Cannot mint non-existent `token_ids`. Use `curate_token_ids` to create tokens on new `token_ids`"));

  // cannot mint NFT
  tx = await mintTokens(env.accounts[0], contract, "2", [Bal]);
  assert(tx.rawLog.includes('minting is not enabled for this token_id'));

  // cannot mint if enable_mint == false
  tx = await mintTokens(env.accounts[0], contract, "1", [Bal]);
  assert(tx.rawLog.includes('minting is not enabled for this token_id'));
  
  // success: can mint
  tx = await mintTokens(env.accounts[0], contract, "0", [Bal]);
  assert(tx.code === 0);

  // success: can mint multiple
  tx = await mintTokens(env.accounts[0], contract, "0", [Bal, Bal1]);
  assert(tx.code === 0);

  await setViewingKeyAll(env);
  let balance0: string = await queryBalance(env.accounts[0], contract, env.accounts[0].address, "vkey0", "0");
  let balance1: string = await queryBalance(env.accounts[1], contract, env.accounts[1].address, "vkey1", "0");
  assert(balance0 === "1002");
  assert(balance1 === "100");

  // burn --------------------
  // cannot burn if not owner
  tx = await burnTokens(env.accounts[1], contract, "0", [Bal]);
  assert(tx.rawLog.includes(`you do not have permission to burn 1 tokens from address ${env.accounts[0].address}`));

  // cannot burn non-existent token_ids
  tx = await burnTokens(env.accounts[0], contract, "na", [Bal]);
  assert(tx.rawLog.includes("token_id does not exist. Cannot burn non-existent `token_ids`. Use `curate_token_ids` to create tokens on new `token_ids`"));

  // cannot burn if enable_burn == false
  tx = await burnTokens(env.accounts[0], contract, "1", [Bal]);
  assert(tx.rawLog.includes('burning is not enabled for this token_id'));

  // success: can burn
  tx = await burnTokens(env.accounts[0], contract, "0", [Bal]);
  assert(tx.code === 0);

  // success: can burn multiple
  tx = await burnTokens(env.accounts[0], contract, "0", [Bal, Bal]);
  assert(tx.code === 0);

  // fails if one of the attempted burn actions is invalid
  tx = await burnTokens(env.accounts[0], contract, "0", [Bal, Bal, Bal1]);
  assert(tx.rawLog.includes(`you do not have permission to burn 100 tokens from address ${env.accounts[1].address}`));
  balance0 = await queryBalance(env.accounts[0], contract, env.accounts[0].address, "vkey0", "0");
  balance1 = await queryBalance(env.accounts[1], contract, env.accounts[1].address, "vkey1", "0");
  assert(balance0 === "999");
  assert(balance1 === "100");
}

async function testTransferTokens(
  env: jsEnv,
): Promise<void> {
  const contract = env.contracts[0]
  const from = env.accounts[0];
  const to = env.accounts[1];
  const addr2 = env.accounts[2];

  await setViewingKeyAll(env);
  let balance0: string = await queryBalance(from, contract, from.address, "vkey0", "0");
  let balance1: string = await queryBalance(to, contract, to.address, "vkey1", "0");
  assert(balance0 === "1000");
  assert(balance1 === "0");

  // can transfer own tokens
  let tx = await transfer(from, contract, "0", from, to, "200");
  assert(tx.code === 0);
  balance0 = await queryBalance(from, contract, from.address, "vkey0", "0");
  balance1 = await queryBalance(to, contract, to.address, "vkey1", "0");
  assert(balance0 === "800");
  assert(balance1 === "200");

  // cannot transfer if no permission
  tx = await transfer(addr2, contract, "0", from, to, "200");
  assert(tx.rawLog.includes("These tokens do not exist or you have no permission to transfer"));

  // cannot transfer if not enough transfer allowance
  await givePermission(from, contract, addr2, "0", undefined, undefined, undefined, undefined, "100", undefined);
  tx = await transfer(addr2, contract, "0", from, to, "200");
  assert(tx.rawLog.includes("Insufficient transfer allowance: "));

  // transfer successful with enough transfer allowance
  await givePermission(from, contract, addr2, "0", undefined, undefined, undefined, undefined, "300", undefined);
  tx = await transfer(addr2, contract, "0", from, to, "200");
  assert(fromUtf8(tx.data[0]).includes(`{"transfer":{"status":"success"}}`));

  // allowances get consumed. Cannot make second transfer such that combined  transfers exceed allowance
  tx = await transfer(addr2, contract, "0", from, to, "101");
  assert(tx.rawLog.includes("Insufficient transfer allowance: 100"));

  // can use remaining allowance
  tx = await transfer(addr2, contract, "0", from, to, "100");
  assert(fromUtf8(tx.data[0]).includes(`{"transfer":{"status":"success"}}`));
  balance0 = await queryBalance(from, contract, from.address, "vkey0", "0");
  balance1 = await queryBalance(to, contract, to.address, "vkey1", "0");
  assert(balance0 === "500");
  assert(balance1 === "500");
}

async function testReceiverSanity(
  env: jsEnv,
): Promise<void> {
  const account = env.accounts[0];
  const receiver = env.contracts[1];
  
  let count = await queryRecieverGetCount(account, receiver);
  assert(count === 10);
  let tx = await receiverIncrement(account, receiver);
  assert(tx.code === 0);
  count = await queryRecieverGetCount(account, receiver);
  assert(count === 11);
  tx = await receiverReset(account, receiver, 20);
  assert(tx.code === 0);
  count = await queryRecieverGetCount(account, receiver);
  assert(count === 20);
  tx = await receiverFail(account, receiver);
  assert(tx.rawLog.includes("intentional failure"));

  const msg = { reset: { count: 30 }};
  const msg_bin = toBase64(toUtf8(JSON.stringify(msg)));
  
  // cannot receive if not registered
  tx = await receiverSnip1155Receive(env.accounts[1], receiver, "0", "0", "0", undefined, msg_bin);
  assert(tx.rawLog.includes("is not receiver creator, or a known SNIP-1155 coin that this contract registered to"));  

  // can call SNIP1155Receive (owner is authorized)
  tx = await receiverSnip1155Receive(account, receiver, "0", "0", "0", undefined, msg_bin);
  assert(tx.code === 0);
  count = await queryRecieverGetCount(account, receiver); assert(count === 30); 
}


async function testRegreceiveSend(
  env: jsEnv,
): Promise<void> {
  const account = env.accounts[0];
  const snip1155 = env.contracts[0];
  const receiver = env.contracts[1];
  
  const msg = { increment: {  }};
  const msg_bin = toBase64(toUtf8(JSON.stringify(msg)));

  // can register with contract
  let tx = await receiverRegister(account, receiver, env.contracts[0].address, env.contracts[0].hash);
  assert(tx.code === 0);

  // can receive now, and count has incremented to [ 11 ] per the message sent
  tx = await send(account, snip1155, "0", account, receiver, "10", msg_bin)
  assert(tx.code === 0);
  await setViewingKey(account, snip1155, "vkey");
  let bal = await queryBalance(account, snip1155, account.address, "vkey", "0"); assert(bal === "990");
  let count = await queryRecieverGetCount(account, receiver); assert(count === 11); 
}

async function testQueries(
  env: jsEnv
) {
  const acc0 = env.accounts[0];
  const acc1 = env.accounts[1];
  const acc2 = env.accounts[2];
  const snip1155 = env.contracts[0];
  const receiver = env.contracts[1];

  let tx = await givePermission(acc0, snip1155, acc1, "0", true, undefined, true, undefined, "100", undefined);
  assert(tx.code === 0);
  tx = await givePermission(acc0, snip1155, acc1, "1", true, undefined, true, undefined, "10", undefined);
  assert(tx.code === 0);
  tx = await givePermission(acc0, snip1155, acc2, "0", true, undefined, true, undefined, "200", undefined);
  assert(tx.code === 0);

  await setViewingKeyAll(env);

  // query permission --------------------------------
  let q_perm = await queryPermission(acc0, snip1155, acc0.address, acc1.address, "vkey0", "0")
  const expPerm = {
    permission: {
      view_balance_perm: true,
      view_balance_exp: 'never',
      view_pr_metadata_perm: true,
      view_pr_metadata_exp: 'never',
      trfer_allowance_perm: '100',
      trfer_allowance_exp: 'never'
    }
  };
  assert(JSON.stringify(q_perm).includes(JSON.stringify(expPerm)));
  
  // acc1 (grantee) can also view permission: test using Q permits 
  let permit: Permit = await generatePermit(acc1, snip1155);
  let msg = { with_permit: {
    permit,
    query: {
      permission: {
        owner: acc0.address,
        allowed_address: acc1.address,
        token_id: "0"
      }
    }
  }};
  type QueryResponse = { permission: Permission };
  q_perm = await execQuery(acc0, snip1155, msg) as QueryResponse;
  assert(JSON.stringify(q_perm).includes(JSON.stringify(expPerm)));
  
  // cannot query using permit not from owner or allowed_address
  permit = await generatePermit(acc2, snip1155);
  msg.with_permit.permit = permit;
  q_perm = await execQuery(acc0, snip1155, msg) as QueryResponse; 
  const err_msg = `Cannot query permission. Requires permit for either owner \\"${acc0.address}\\" or viewer||spender \\"${acc1.address}\\", got permit for \\"${acc2.address}\\"`;
  assert(JSON.stringify(q_perm).includes(err_msg));  

  // transactionHistory --------------------------------
  tx = await transfer(acc0, snip1155, "0", acc0, acc1, "50");
  assert(tx.code === 0);

  // can view own tx history + check that permit query results == viewing key query results
  let q = await queryTransactionHistory(acc0, snip1155, acc0.address, "vkey0", 10);
  let qp = await queryTransactionHistoryQPermit(acc0, snip1155, 10);
  assert(JSON.stringify(q).includes('"total":4'));
  assert(JSON.stringify(q) === JSON.stringify(qp));
  q = await queryTransactionHistory(acc1, snip1155, acc1.address, "vkey1", 10);
  qp = await queryTransactionHistoryQPermit(acc1, snip1155, 10);
  assert(JSON.stringify(q).includes('"total":1'));
  assert(JSON.stringify(q) === JSON.stringify(qp));
  
  // cannot view another account's history
  q = await queryTransactionHistory(acc1, snip1155, acc0.address, "vkey1", 10);
  assert(JSON.stringify(q).includes("Wrong viewing key for this address or viewing key not set"))

  // query allPermissions --------------------------------
  q = await queryAllPermissionsQPermit(acc0, snip1155, 5);
  const expPerms = [
    {
      view_balance_perm: true,
      view_balance_exp: "never",
      view_pr_metadata_perm: true,
      view_pr_metadata_exp: "never",
      trfer_allowance_perm: "200",
      trfer_allowance_exp: "never"
    },
    {
      view_balance_perm: true,
      view_balance_exp: "never",
      view_pr_metadata_perm: true,
      view_pr_metadata_exp: "never",
      trfer_allowance_perm: "10",
      trfer_allowance_exp: "never"
    },
    {
      view_balance_perm: true,
      view_balance_exp: "never",
      view_pr_metadata_perm: true,
      view_pr_metadata_exp: "never",
      trfer_allowance_perm: "100",
      trfer_allowance_exp: "never"
    }
  ];
  assert(JSON.stringify(q).includes(JSON.stringify(expPerms)));
  
  // grantee cannot see list of permissions that the account has been granted 
  // not in base specification
  q = await queryAllPermissionsQPermit(acc1, snip1155, 5);
  assert(JSON.stringify(q).includes('{"all_permissions":{"permission_keys":[],"permissions":[],"total":0}}'));

  // query token_id private info --------------------------------
  const expPrivInfo = {
    token_id_private_info: {
      token_id_info: {
        token_id: "2",
        name: "nftname",
        symbol: "NFT",
        token_config: {
          nft: {
            minters: [],
            public_total_supply: true,
            owner_is_public: true,
            enable_burn: true,
            owner_may_update_metadata: true,
            minter_may_update_metadata: true,
          }
        },
        public_metadata: {
          token_uri: "public_token_uri",
          extension: null
        },
        private_metadata: {
          token_uri: "private_token_uri",
          extension: {
            image: null,
            image_data: "some image data",
            external_url: null,
            description: null,
            name: null,
            attributes: null,
            background_color: null,
            animation_url: null,
            youtube_url: null,
            media: null,
            protected_attributes: [
              "some protected attributes"
            ],
            token_subtype: null
          }
        },
        curator: acc0.address,
      },
      total_supply: "1",
      owner: acc0.address,
    }
  }
  q = await queryTokenIdPrivateInfoQPermit(acc0, snip1155, "2");
  assert(JSON.stringify(q).includes(JSON.stringify(expPrivInfo)));

  // cannot view private info using permit from another address
  q = await queryTokenIdPrivateInfoQPermit(acc1, snip1155, "2",);
  assert(JSON.stringify(q).includes("you do have have permission to view private token info"));

  // if granted WRONG permission, 
  // i) can see public token info, but not private info (private metadata). owner viewable because it is configured as public info
  // ii) cannot see balance
  tx = await givePermission(acc0, snip1155, acc1, "2", undefined, undefined, undefined, undefined, "100", undefined);
  assert(tx.code === 0); 
  q = await queryTokenIdPrivateInfoQPermit(acc1, snip1155, "2",);
  assert(JSON.stringify(q).includes('"private_metadata":null'));
  assert(JSON.stringify(q).includes(`"owner":"${acc0.address}"`));  
  q = await queryBalanceQPermit(acc1, snip1155, acc0.address, "2",);
  assert(JSON.stringify(q).includes("you do have have permission to view balance"));

  // can view balance ONLY, has no effect on the TokenIdPrivateInfo query
  tx = await givePermission(acc0, snip1155, acc1, "2", true, undefined, undefined, undefined, "0", undefined,);
  assert(tx.code === 0); 
  q = await queryTokenIdPrivateInfoQPermit(acc1, snip1155, "2",);
  assert(JSON.stringify(q).includes('"private_metadata":null'));
  assert(JSON.stringify(q).includes(`"owner":"${acc0.address}"`));  
  q = await queryBalanceQPermit(acc1, snip1155, acc0.address, "2",);
  assert(JSON.stringify(q).includes('"amount":"1"'));

  // can view only private metadata if granted only that permission
  tx = await givePermission(acc0, snip1155, acc1, "2", false, undefined, true, undefined, "0", undefined);
  assert(tx.code === 0); 
  q = await queryTokenIdPrivateInfoQPermit(acc1, snip1155, "2",);
  assert(JSON.stringify(q).includes('{"token_uri":"private_token_uri","extension":{"image":null,"image_data":"some image data"'));
  assert(JSON.stringify(q).includes(`"owner":"${acc0.address}"`));  
  q = await queryBalanceQPermit(acc1, snip1155, acc0.address, "2",);
  assert(JSON.stringify(q).includes("you do have have permission to view balance"));

  // can view private info if granted all permissions -- checks that leaving a field blank
  // (ie: priv_metadata viewership is undefined, or `None` in Rust), leaves setting unchanged
  tx = await givePermission(acc0, snip1155, acc1, "2", true, undefined, undefined, undefined, "0", undefined);
  assert(tx.code === 0); 
  q = await queryTokenIdPrivateInfoQPermit(acc1, snip1155, "2",);
  assert(JSON.stringify(q).includes('{"token_uri":"private_token_uri","extension":{"image":null,"image_data":"some image data"'));
  assert(JSON.stringify(q).includes(`"owner":"${acc0.address}"`));  
  q = await queryBalanceQPermit(acc1, snip1155, acc0.address, "2",);
  assert(JSON.stringify(q).includes('"amount":"1"'));

  // can query token_id private info on fungible tokens, if owner
  q = await queryTokenIdPrivateInfoQPermit(acc0, snip1155, "0");
  assert(JSON.stringify(q).includes('{"token_uri":"private_token_uri","extension":{"image":null,"image_data":"some image data"'));

  // cannot query token_id private info on fungible tokens, even if given permission by owner, because it has no `owner`
  tx = await givePermission(acc0, snip1155, acc1, "2", true, undefined, true, undefined, "10", undefined);
  assert(tx.code === 0); 
  q = await queryTokenIdPrivateInfoQPermit(acc2, snip1155, "0");
  assert(JSON.stringify(q).includes('{"generic_err":{"msg":"you do have have permission to view private token info"}}'));

  // registeredcodehash query --------------------------------
  let q_hash = await queryRegisteredCodeHash(acc0, snip1155, receiver.address);
  assert(q_hash.registered_code_hash.code_hash === null);

  tx = await receiverRegister(acc0, receiver, snip1155.address, snip1155.hash);
  q_hash = await queryRegisteredCodeHash(acc0, snip1155, receiver.address);
  assert(q_hash.registered_code_hash.code_hash === receiver.hash);
}



/////////////////////////////////////////////////////////////////////////////////
// Main
/////////////////////////////////////////////////////////////////////////////////

async function runTest(
  tester: (
    env: jsEnv,
  ) => void,
  env: jsEnv
) {
  console.log(`[TESTING...]: ${tester.name}`);
  await tester(env);
  console.log(`[SUCCESS] ${tester.name}`);
}

(async () => {
  let env: jsEnv;

  env = await initDefaultWithReceiver();
  await runTest(testIntializationSanity, env);
  await runTest(testReceiverSanity, env);

  env = await initDefault();
  await runTest(testCurateTokenIds, env);

  env = await initDefault();
  await runTest(testMintBurnTokens, env);

  env = await initDefault();
  await runTest(testTransferTokens, env);

  env = await initDefaultWithReceiver();
  await runTest(testRegreceiveSend, env);

  env = await initDefaultWithReceiver();
  await runTest(testQueries, env);

  console.log("All tests COMPLETED SUCCESSFULLY")
  
})();
