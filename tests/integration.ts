import { SecretNetworkClient, fromUtf8, Tx } from "secretjs";
import fs from "fs";
import assert from "assert";
import { initClient } from "./int_helpers";
import { Account, ContractInfo, jsEnv } from "./int_utils";


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
      label: "Contract " + client.address.slice(6),
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
async function init_default(): Promise<jsEnv> {
  const accounts = await initClient();
  const { secretjs } = accounts[0] 

  const defaultInitMsg = { 
    has_admin: true,
    minters: [accounts[0].address],
    initial_tokens: [
      {
        token_info: { 
          token_id: "0", 
          name: "token0", 
          symbol: "TKN0", 
          is_nft: false, 
          token_config: {
              public_total_supply: false,
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
          symbol: "TKN1", 
          is_nft: true, 
          token_config: {
              public_total_supply: false,
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
) {
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
              is_nft, 
              token_config: {
                  public_total_supply: true,
                  enable_burn: true, 
              },
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
  console.log(`MintTokenIds used ${tx.gasUsed} gas`);
  return tx
}

async function setViewingKey(
  account: Account,
  contract: ContractInfo,
) {
  const { secretjs } = account;
  const tx = await secretjs.tx.compute.executeContract(
    {
      sender: secretjs.address,
      contractAddress: contract.address,
      codeHash: contract.hash,
      msg: {
        set_viewing_key: { key: "vkey" },
      },
      sentFunds: [],
    },
    {
      broadcastCheckIntervalMs: 100,
      gasLimit: 200000,
    }
  );

  //const parsedTxData = JSON.parse(fromUtf8(tx.data[0])); 
  console.log(`SetViewingKey used ${tx.gasUsed} gas`);
  return tx
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

async function givePermission(
  sender: Account,
  contract: ContractInfo,
  address: Account,
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
          address: address.address,
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
): Promise<string> {
  const { secretjs } = env.accounts[0];
  type ContractInfo = { contract_info: { info: string }};

  const contractInfoResponse = (await secretjs.query.compute.queryContract({
    contractAddress: contract.address,
    codeHash: contract.hash,
    query: { contract_info: {} },
  })) as ContractInfo;

  if ('err"' in contractInfoResponse) {
    throw new Error(
      `Query failed with the following err: ${JSON.stringify(contractInfoResponse)}`
    );
  }

  return contractInfoResponse.contract_info.info;
}

async function queryBalance(
  account: Account,
  contract: ContractInfo,
  key: string,
  token_id: string,
): Promise<string> {
  const { secretjs } = account;
  type Balance = { balance: { amount: string }};

  const BalanceResponse = (await secretjs.query.compute.queryContract({
    contractAddress: contract.address,
    codeHash: contract.hash,
    query: { balance: {
      address: account.address,
      key: key,
      token_id: token_id,
    } },
  })) as Balance;

  if ('err"' in BalanceResponse) {
    throw new Error(
      `Query failed with the following err: ${JSON.stringify(BalanceResponse)}`
    );
  }
  
  return BalanceResponse.balance.amount;
}

/////////////////////////////////////////////////////////////////////////////////
// Tests
/////////////////////////////////////////////////////////////////////////////////

async function test_intialization_sanity(
  env: jsEnv
): Promise<void> {
  const contract = env.contracts[0];
  const account = env.accounts[0];
  const onInitializationData: string = await queryContractInfo(
    env,
    contract,
  );
  assert(
    onInitializationData === "data",
    `The contract info on initialization expected to be "data" instead of ${onInitializationData}`
  );

  await setViewingKey(account, contract);
  const initBalance0: string = await queryBalance(
    account, contract, "vkey", "0"
  );
  assert(
    initBalance0 === "1000",
    `Initial balance expected to be "1000" instead of ${initBalance0}`
  );
  const initBalance1: string = await queryBalance(
    account, contract, "vkey", "1"
  );
  assert(
    initBalance1 === "1",
    `Initial balance expected to be "1000" instead of ${initBalance1}`
  );
}

async function test_mint_token_ids(
  env: jsEnv,
) {
  const minter = env.accounts[0];
  const contract = env.contracts[0];

  let tx = await mintTokenIds(minter, contract, "2", "token1", "TKN1", false, minter.address, "1000");
  assert(fromUtf8(tx.data[0]).includes(`{"mint_token_ids":{"status":"success"}}`));

  await setViewingKey(minter, contract);
  let bal: string = await queryBalance(minter, contract, "vkey", "2");
  assert(bal === "1000");

  // cannot mint token_id with same name
  tx = await mintTokenIds(minter, contract, "2", "token1a", "TKN1a", false, minter.address, "123");
  assert(tx.rawLog.includes("token_id already exists. Try a different id String"));
  bal = await queryBalance(minter, contract, "vkey", "2");
  assert(bal === "1000");

  // can mint NFT
  await mintTokenIds(minter, contract, "3", "a new nft", "NFT3", true, minter.address, "1");
  bal = await queryBalance(minter, contract, "vkey", "3");
  assert(bal === "1");

  // cannot mint NFT with amount != 1
  tx = await mintTokenIds(minter, contract, "4a", "a new nft", "NFT2", true, minter.address, "0");
  assert(tx.rawLog.includes("token_id 4a is an NFT; there can only be one NFT. Balances.amount must == 1"));
  tx = await mintTokenIds(minter, contract, "4b", "a new nft", "NFT2", true, minter.address, "2");
  assert(tx.rawLog.includes("token_id 4b is an NFT; there can only be one NFT. Balances.amount must == 1"));
  bal = await queryBalance(minter, contract, "vkey", "4a"); assert(bal === "0");
  bal = await queryBalance(minter, contract, "vkey", "4b"); assert(bal === "0");
  
}

async function test_transfer_tokens(
  env: jsEnv,
): Promise<void> {
  const contract = env.contracts[0]
  const from = env.accounts[0];
  const to = env.accounts[1];
  const addr2 = env.accounts[2];

  await setViewingKey(from, contract);
  await setViewingKey(to, contract);
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
  const env0 = await init_default();
  await runTest(test_intialization_sanity, env0);
  await runTest(test_mint_token_ids, env0);

  const env1 = await init_default();
  await runTest(test_transfer_tokens, env1);

  console.log("All tests COMPLETED SUCCESSFULLY")

})();
