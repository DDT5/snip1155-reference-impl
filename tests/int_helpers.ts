import axios from "axios";
import { Wallet, SecretNetworkClient, Tx } from "secretjs";
import { Account } from "./int_utils";
import { AminoWallet } from "secretjs/dist/wallet_amino";

/** creates new accounts by funding from genesis account `a` */ 
export const initClient = async () => {
  let grpcWebUrl = "http://localhost:9091"; //endpoint
  let chainId = "secretdev-1";

  let accounts: Account[] = [];

  // Initialize genesis accounts
  const mnemonics = [
    "grant rice replace explain federal release fix clever romance raise often wild taxi quarter soccer fiber love must tape steak together observe swap guitar",
    "jelly shadow frog dirt dragon use armed praise universe win jungle close inmate rain oil canvas beauty pioneer chef soccer icon dizzy thunder meadow",
    "chair love bleak wonder skirt permit say assist aunt credit roast size obtain minute throw sand usual age smart exact enough room shadow charge",
    "word twist toast cloth movie predict advance crumble escape whale sail such angry muffin balcony keen move employ cook valve hurt glimpse breeze brick",
  ];

  for (let i = 0; i < mnemonics.length; i++) {
    const mnemonic = mnemonics[i];
    const walletAmino = new AminoWallet(mnemonic);
    accounts[i] = {
      address: walletAmino.address,
      mnemonic: mnemonic,
      walletAmino,
      walletProto: new Wallet(mnemonic),
      secretjs: await SecretNetworkClient.create({
        grpcWebUrl,
        wallet: walletAmino,
        walletAddress: walletAmino.address,
        chainId,
      }),
    };
    console.log(`Genesis wallet ${i} with address: ${walletAmino.address}`);
      // Genesis wallet a with address: secret1ap26qrlp8mcq2pg6r47w43l0y8zkqm8a450s03
      // Genesis wallet b with address: secret1fc3fzy78ttp0lwuujw7e52rhspxn8uj52zfyne
      // Genesis wallet c with address: secret1ajz54hz8azwuy34qwy9fkjnfcrvf0dzswy0lqq
      // Genesis wallet d with address: secret1ldjxljw7v4vk6zhyduywh04hpj0jdwxsmrlatf
  }

  // Generate additional accounts
  const numNewAcc = 3;
  for (let i = 4; i <= 4 - 1 + numNewAcc; i++) {
    const wallet = new AminoWallet();
    const [{ address }] = await wallet.getAccounts();
    const walletProto = new Wallet(wallet.mnemonic);

    accounts[i] = {
      address: address,
      mnemonic: wallet.mnemonic,
      walletAmino: wallet,
      walletProto: walletProto,
      secretjs: await SecretNetworkClient.create({
        grpcWebUrl,
        chainId,
        wallet: wallet,
        walletAddress: address,
      }),
    };
    console.log(`Initialized wallet ${i-4} with address: ${address}`);
  }

  // Send 100k SCRT from account 0 to each of accounts 1-`numNewAcc`

  const { secretjs } = accounts[0];

  let tx: Tx;
  try {
    tx = await secretjs.tx.bank.multiSend(
      {
        inputs: [
          {
            address: accounts[0].address,
            coins: [{ denom: "uscrt", amount: String(100_000 * 1e6 * numNewAcc) }],
          },
        ],
        outputs: accounts.slice(4).map(({ address }) => ({
          address,
          coins: [{ denom: "uscrt", amount: String(100_000 * 1e6) }],
        })),
      },
      {
        broadcastCheckIntervalMs: 100,
        gasLimit: 200_000,
      },
    );
  } catch (e) {
    throw new Error(`Failed to multisend: ${e.stack}`);
  }

  if (tx.code !== 0) {
    console.error(`failed to multisend coins`);
    throw new Error("Failed to multisend coins to initial accounts");
  }

  // returns only new accounts
  return accounts.slice(4);
};

// Below function are not used
/** The faucet drips 1_000_000_000 uscrt at a time. */
async function fillUpFromFaucet(
  client: SecretNetworkClient,
  targetBalance: Number
) {
  let balance = await getScrtBalance(client);
  while (Number(balance) < targetBalance) {
    try {
      await getFromFaucet(client.address);
    } catch (e) {
      console.error(`failed to get tokens from faucet: ${e}`);
    }
    balance = await getScrtBalance(client);
  }
  console.error(`got tokens from faucet: ${balance}`);
}

const getFromFaucet = async (address: string) => {
  await axios.get(`http://localhost:5000/faucet?address=${address}`);
};

export async function getScrtBalance(userCli: SecretNetworkClient): Promise<string> {
  let balanceResponse = await userCli.query.bank.balance({
    address: userCli.address,
    denom: "uscrt",
  });
  return balanceResponse.balance!.amount;
}
