import { SecretNetworkClient, toUtf8, fromUtf8, Tx, toBase64, fromBase64} from "secretjs";
import fs from "fs";
import assert from "assert";
import { initClient } from "./int_helpers";
import { Account, ContractInfo, jsEnv } from "./int_utils";
import exp from "constants";


/////////////////////////////////////////////////////////////////////////////////
// Type declarations
/////////////////////////////////////////////////////////////////////////////////

type Balance = {
  address: string,
  amount: string,
};

// type TknInfo = {
//   token_id: string,
//   name: string,
//   symbol: string,
//   decimals: number,
//   is_nft: boolean, 
//   token_config: TknConf,
//   public_metadata?: unknown,
//   private_metadata?: unknown,
// }

type TknConf = {
    public_total_supply: boolean,
    enable_mint: boolean, 
    enable_burn: boolean, 
};

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

  var contractInfo: [string, string] = [contractCodeHash, contractAddress];
  return contractInfo;
};

// Initialization procedure: Initialize client, fund new accounts, and upload/instantiate contract
async function initDefault(): Promise<jsEnv> {
  const accounts = await initClient();
  const { secretjs } = accounts[0];

  const defaultInitMsg = { 
    has_admin: true,
    minters: [accounts[0].address],
    initial_tokens: [
      {
        token_info: { 
          token_id: "0", 
          name: "token0", 
          symbol: "TKN", 
          decimals: 6,
          is_nft: false, 
          token_config: {
              public_total_supply: true,
              enable_mint: true, 
              enable_burn: true, 
          },
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
          decimals: 6,
          is_nft: false, 
          token_config: {
              public_total_supply: false,
              enable_mint: false, 
              enable_burn: false, 
          },
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
          decimals: 6,
          is_nft: true, 
          token_config: {
              public_total_supply: true,
              enable_mint: true, 
              enable_burn: true, 
          },
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
    defaultInitMsg,
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

async function mintTokenIds(
  account: Account,
  contract: ContractInfo,
  token_id: string,
  token_name: string,
  token_symbol: string,
  is_nft: boolean,
  mint_to_address: string,
  mint_amount: string,
  token_config?: TknConf,
) {
  let tkn_conf: TknConf;
  if (token_config === undefined) {
    tkn_conf = {
      public_total_supply: true,
      enable_mint: true, 
      enable_burn: true, 
    } as TknConf;
  } else {
    tkn_conf = token_config
  }

  const { secretjs } = account;
  const tx = await secretjs.tx.compute.executeContract(
    {
      sender: secretjs.address,
      contractAddress: contract.address,
      codeHash: contract.hash,
      msg: {
        mint_token_ids: { 
          initial_tokens: [{
            token_info: { 
              token_id, 
              name: token_name, 
              symbol: token_symbol, 
              decimals: 0,
              is_nft, 
              token_config: tkn_conf,
            }, 
            balances: [{ 
                address: mint_to_address, 
                amount: mint_amount 
            }]
          }]
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
  console.log(`mintTokenIds used ${tx.gasUsed} gas`);
  return tx
}

async function mintTokens(
  account: Account,
  contract: ContractInfo,
  token_id: string,
  balances: Balance[],
) {
  const { secretjs } = account;
  const tx = await secretjs.tx.compute.executeContract(
    {
      sender: secretjs.address,
      contractAddress: contract.address,
      codeHash: contract.hash,
      msg: {
        mint_tokens: { 
          mint_tokens: [{
            token_id, 
            balances,
          }]
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
  console.log(`mintTokens used ${tx.gasUsed} gas`);
  return tx
}

async function burnTokens(
  account: Account,
  contract: ContractInfo,
  token_id: string,
  balances: Balance[],
) {
  const { secretjs } = account;
  const tx = await secretjs.tx.compute.executeContract(
    {
      sender: secretjs.address,
      contractAddress: contract.address,
      codeHash: contract.hash,
      msg: {
        burn_tokens: { 
          burn_tokens: [{
            token_id, 
            balances,
          }]
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
  console.log(`burnTokens used ${tx.gasUsed} gas`);
  return tx
}

async function setViewingKey(
  account: Account,
  contract: ContractInfo,
  key: string,
) {
  const { secretjs } = account;
  const tx = await secretjs.tx.compute.executeContract(
    {
      sender: secretjs.address,
      contractAddress: contract.address,
      codeHash: contract.hash,
      msg: {
        set_viewing_key: { key: key },
      },
      sentFunds: [],
    },
    {
      broadcastCheckIntervalMs: 100,
      gasLimit: 200000,
    }
  );

  //const parsedTxData = JSON.parse(fromUtf8(tx.data[0])); 
  console.log(`setViewingKey used ${tx.gasUsed} gas`);
  return tx
}

async function setViewingKeyAll(
  env: jsEnv,
) {
  for (const contr of env.contracts) {
    let tx: Promise<Tx>;
    for (let i=0; i<env.accounts.length; i++) {
        tx = setViewingKey(env.accounts[i], contr, "vkey");
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
  const { secretjs } = sender;
  const tx = await secretjs.tx.compute.executeContract(
    {
      sender: secretjs.address,
      contractAddress: contract.address,
      codeHash: contract.hash,
      msg: {
        transfer: { 
          token_id,
          from: from.address,
          recipient: recipient.address,
          amount, 
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
  console.log(`Transfer used ${tx.gasUsed} gas`);
  return tx
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
  const { secretjs } = sender;
  const tx = await secretjs.tx.compute.executeContract(
    {
      sender: secretjs.address,
      contractAddress: contract.address,
      codeHash: contract.hash,
      msg: {
        send: { 
          token_id,
          from: from.address,
          recipient: recipient_contract.address,
          recipient_code_hash: recipient_contract.hash,
          amount, 
          msg
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
  console.log(`Send used ${tx.gasUsed} gas`);
  return tx
}

async function givePermission(
  sender: Account,
  contract: ContractInfo,
  allowed_address: Account,
  token_id: string,
  view_owner?: boolean,
  view_private_metadata?: boolean,
  transfer?: string,
): Promise<Tx> {
  const { secretjs } = sender;
  const tx = await secretjs.tx.compute.executeContract(
    {
      sender: secretjs.address,
      contractAddress: contract.address,
      codeHash: contract.hash,
      msg: {
        give_permission: { 
          allowed_address: allowed_address.address,
          token_id,
          view_owner,
          view_private_metadata,
          transfer,
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
  console.log(`GivePermission used ${tx.gasUsed} gas`);
  return tx
}

/////////////////////////////////////////////////////////////////////////////////
// Query Messages
/////////////////////////////////////////////////////////////////////////////////

async function queryContractInfo(
  env: jsEnv,
  contract: ContractInfo,
) {
  const { secretjs } = env.accounts[0];
  type queryResponse = { contract_info: { 
    admin: string,
    minters: string[],
    all_token_ids: string[],
  }};

  const response = (await secretjs.query.compute.queryContract({
    contractAddress: contract.address,
    codeHash: contract.hash,
    query: { contract_info: {} },
  })) as queryResponse;

  if ('err"' in response) {
    throw new Error(
      `Query failed with the following err: ${JSON.stringify(response)}`
    );
  }

  return response.contract_info;
}

async function queryBalance(
  account: Account,
  contract: ContractInfo,
  key: string,
  token_id: string,
): Promise<string> {
  const { secretjs } = account;
  type queryResponse = { balance: { amount: string }};

  const response = (await secretjs.query.compute.queryContract({
    contractAddress: contract.address,
    codeHash: contract.hash,
    query: { balance: {
      address: account.address,
      key: key,
      token_id: token_id,
    } },
  })) as queryResponse;

  if ('err"' in response) {
    throw new Error(
      `Query failed with the following err: ${JSON.stringify(response)}`
    );
  }
  
  return response.balance.amount;
}

async function queryTokenIdPublicInfo(
  account: Account,
  contract: ContractInfo,
  token_id: string,
) {
  const { secretjs } = account;
  // type queryResponse = { token_id_public_info: { 
  //   token_id_info: TknInfo,
  //   total_supply?: string, 
  // }};
  type queryResponse = unknown;

  const response = (await secretjs.query.compute.queryContract({
    contractAddress: contract.address,
    codeHash: contract.hash,
    query: { token_id_public_info: {
      token_id,
    } },
  })) as queryResponse;

  // if ('err"' in response) {
  //   throw new Error(
  //     `Query failed with the following err: ${JSON.stringify(response)}`
  //   );
  // }
  
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
  console.log(`Fail [receiver contract] used ${tx.gasUsed} gas`);
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
  type queryResponse = { count: number };

  const response = (await secretjs.query.compute.queryContract({
    contractAddress: contract.address,
    codeHash: contract.hash,
    query: { get_count: {  } },
  })) as queryResponse;

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
  const addr0 = env.accounts[0];

  const onInitializationData = await queryContractInfo(
    env,
    contract,
  );
  let exp_contract_info = {
    admin: addr0.address,
    minters: [addr0.address],
    all_token_ids: ["0","1","2"],
  };
  assert(
    JSON.stringify(onInitializationData) === JSON.stringify(exp_contract_info),
    `Contract info on initialization unexpected: ${JSON.stringify(onInitializationData)}`
  );

  await setViewingKey(addr0, contract, "vkey");
  const initBalance0: string = await queryBalance(
    addr0, contract, "vkey", "0"
  );
  assert(
    initBalance0 === "1000",
    `Initial balance expected to be "1000" instead of ${initBalance0}`
  );
  const initBalance1: string = await queryBalance(
    addr0, contract, "vkey", "1"
  );
  assert(
    initBalance1 === "1000",
    `Initial balance expected to be "1000" instead of ${initBalance1}`
  );
  const initBalance2: string = await queryBalance(
    addr0, contract, "vkey", "2"
  );
  assert(
    initBalance2 === "1",
    `Initial balance expected to be "1" instead of ${initBalance2}`
  );

  // public token info
  const tknId0 = await queryTokenIdPublicInfo(addr0, contract, "0"); 
  const tknId0String = JSON.stringify(tknId0);
  assert(tknId0String.includes('"token_id":"0"'));
  assert(tknId0String.includes('"is_nft":false'));
  assert(tknId0String.includes('"public_total_supply":true'));
  assert(tknId0String.includes('"total_supply":"1000"'));

  const tknId1 = await queryTokenIdPublicInfo(addr0, contract, "1"); 
  const tknId1String = JSON.stringify(tknId1);
  assert(tknId1String.includes('"token_id":"1"'));
  assert(tknId1String.includes('"is_nft":false'));
  assert(tknId1String.includes('"public_total_supply":false'));
  assert(tknId1String.includes('"total_supply":null'));

  const tknId2 = await queryTokenIdPublicInfo(addr0, contract, "2"); 
  const tknId2String = JSON.stringify(tknId2);
  assert(tknId2String.includes('"token_id":"2"'));
  assert(tknId2String.includes('"is_nft":true'));
  assert(tknId2String.includes('"public_total_supply":true'));
  assert(tknId2String.includes('"total_supply":"1"'));
}

async function testMintTokenIds(
  env: jsEnv,
) {
  const minter = env.accounts[0];
  const contract = env.contracts[0];

  let tx = await mintTokenIds(minter, contract, "test0", "tokentest0", "TKNT", false, minter.address, "1000");
  assert(fromUtf8(tx.data[0]).includes(`{"mint_token_ids":{"status":"success"}}`));

  tx = await setViewingKey(minter, contract, "vkey");
  assert(tx.code === 0);
  let bal: string = await queryBalance(minter, contract, "vkey", "test0");
  assert(bal === "1000");

  // cannot mint token_id with same name
  tx = await mintTokenIds(minter, contract, "test0", "tokentest0a", "TKNTA", false, minter.address, "123");
  assert(tx.rawLog.includes("token_id already exists. Try a different id String"));
  bal = await queryBalance(minter, contract, "vkey", "test0");
  assert(bal === "1000");

  // can mint NFT
  tx = await mintTokenIds(minter, contract, "test1", "a new nft", "NFT", true, minter.address, "1");
  assert(tx.code === 0);
  bal = await queryBalance(minter, contract, "vkey", "test1");
  assert(bal === "1");

  // cannot mint NFT with amount != 1
  tx = await mintTokenIds(minter, contract, "test2a", "a new nft", "NFTA", true, minter.address, "0");
  assert(tx.rawLog.includes("token_id test2a is an NFT; there can only be one NFT. Balances.amount must == 1"));
  tx = await mintTokenIds(minter, contract, "test2b", "a new nft", "NFTA", true, minter.address, "2");
  assert(tx.rawLog.includes("token_id test2b is an NFT; there can only be one NFT. Balances.amount must == 1"));
  bal = await queryBalance(minter, contract, "vkey", "test2a"); assert(bal === "0");
  bal = await queryBalance(minter, contract, "vkey", "test2b"); assert(bal === "0");  
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
  assert(tx.rawLog.includes("token_id does not exist. Cannot mint non-existent `token_ids`. Use `mint_token_ids` to create tokens on new `token_ids`"));

  // cannot mint NFT
  tx = await mintTokens(env.accounts[0], contract, "2", [Bal]);
  assert(tx.rawLog.includes('{"generic_err":{"msg":"NFTs can only be minted once using `mint_token_ids`"}}'));

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
  let balance0: string = await queryBalance(env.accounts[0], contract, "vkey", "0");
  let balance1: string = await queryBalance(env.accounts[1], contract, "vkey", "0");
  assert(balance0 === "1002");
  assert(balance1 === "100");

  // burn --------------------
  // cannot burn if not owner
  tx = await burnTokens(env.accounts[1], contract, "0", [Bal]);
  assert(tx.rawLog.includes(`you do not have permission to burn 1 tokens from address ${env.accounts[0].address}`));

  // cannot burn non-existent token_ids
  tx = await burnTokens(env.accounts[0], contract, "na", [Bal]);
  assert(tx.rawLog.includes("token_id does not exist. Cannot burn non-existent `token_ids`. Use `mint_token_ids` to create tokens on new `token_ids`"));

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
  balance0 = await queryBalance(env.accounts[0], contract, "vkey", "0");
  balance1 = await queryBalance(env.accounts[1], contract, "vkey", "0");
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

  await setViewingKey(from, contract, "vkey");
  await setViewingKey(to, contract, "vkey");
  let balance0: string = await queryBalance(from, contract, "vkey", "0");
  let balance1: string = await queryBalance(to, contract, "vkey", "0");
  assert(balance0 === "1000");
  assert(balance1 === "0");

  // can transfer own tokens
  let tx = await transfer(from, contract, "0", from, to, "200");
  assert(tx.code === 0);
  balance0 = await queryBalance(from, contract, "vkey", "0");
  balance1 = await queryBalance(to, contract, "vkey", "0");
  assert(balance0 === "800");
  assert(balance1 === "200");

  // cannot transfer if no permission
  tx = await transfer(addr2, contract, "0", from, to, "200");
  assert(tx.rawLog.includes("These tokens do not exist or you have no permission to transfer"));

  // cannot transfer if not enough transfer allowance
  await givePermission(from, contract, addr2, "0", undefined, undefined, "100");
  tx = await transfer(addr2, contract, "0", from, to, "200");
  assert(tx.rawLog.includes("Insufficient transfer allowance: "));

  // transfer successful with enough transfer allowance
  await givePermission(from, contract, addr2, "0", undefined, undefined, "300");
  tx = await transfer(addr2, contract, "0", from, to, "200");
  assert(fromUtf8(tx.data[0]).includes(`{"transfer":{"status":"success"}}`));

  // allowances get consumed. Cannot make second transfer such that combined  transfers exceed allowance
  tx = await transfer(addr2, contract, "0", from, to, "101");
  assert(tx.rawLog.includes("Insufficient transfer allowance: 100"));

  // can use remaining allowance
  tx = await transfer(addr2, contract, "0", from, to, "100");
  assert(fromUtf8(tx.data[0]).includes(`{"transfer":{"status":"success"}}`));
  balance0 = await queryBalance(from, contract, "vkey", "0");
  balance1 = await queryBalance(to, contract, "vkey", "0");
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
  await setViewingKeyAll(env);
  let bal = await queryBalance(account, snip1155, "vkey", "0"); assert(bal === "990");
  let count = await queryRecieverGetCount(account, receiver); assert(count === 11); 
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
  await runTest(testMintTokenIds, env);

  env = await initDefault();
  await runTest(testMintBurnTokens, env);

  env = await initDefault();
  await runTest(testTransferTokens, env);

  env = await initDefaultWithReceiver();
  await runTest(testRegreceiveSend, env);

  console.log("All tests COMPLETED SUCCESSFULLY")
  
})();
