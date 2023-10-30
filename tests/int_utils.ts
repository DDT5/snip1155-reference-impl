import { SecretNetworkClient, Wallet, Permit, ViewingKey } from "secretjs";

export type jsEnv = {
  accounts: Account[];
  contracts: ContractInfo[];
}

export type Account = {
  address: string;
  mnemonic: string;
  secretjs: SecretNetworkClient;
};

export type ContractInfo = {
  hash: string;
  address: string;
}

// export interface Auth {
//   permit?: Permit;
//   viewer?: {
//       viewing_key: ViewingKey;
//       address: string;
//   };
// }

export function getValueFromRawLog(
    rawLog: string | undefined,
    key: string,
  ): string {
    if (!rawLog) {
      return "";
    }

    for (const l of JSON.parse(rawLog)) {
      for (const e of l.events) {
        for (const a of e.attributes) {
          if (`${e.type}.${a.key}` === key) {
            return String(a.value);
          }
        }
      }
    }

    return "";
  }