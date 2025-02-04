use ethers_core::types::{
    H160,
    H256,
};
use fuel_core_interfaces::model::DaBlockHeight;
use once_cell::sync::Lazy;
use sha3::{
    Digest,
    Keccak256,
};
use std::{
    str::FromStr,
    time::Duration,
};

pub(crate) const REPORT_INIT_SYNC_PROGRESS_EVERY_N_BLOCKS: DaBlockHeight = 1000;
pub(crate) const NUMBER_OF_TRIES_FOR_INITIAL_SYNC: u64 = 10;

pub fn keccak256(data: &'static str) -> H256 {
    let out = Keccak256::digest(data.as_bytes());
    H256::from_slice(out.as_slice())
}

pub(crate) static ETH_LOG_MESSAGE: Lazy<H256> =
    Lazy::new(|| keccak256("SentMessage(bytes32,bytes32,bytes32,uint64,uint64,bytes)"));
pub(crate) static ETH_LOG_VALIDATOR_REGISTRATION: Lazy<H256> =
    Lazy::new(|| keccak256("ValidatorRegistration(bytes,bytes)"));
pub(crate) static ETH_LOG_VALIDATOR_UNREGISTRATION: Lazy<H256> =
    Lazy::new(|| keccak256("ValidatorUnregistration(bytes)"));
pub(crate) static ETH_LOG_DEPOSIT: Lazy<H256> =
    Lazy::new(|| keccak256("Deposit(address,uint256)"));
pub(crate) static ETH_LOG_WITHDRAWAL: Lazy<H256> =
    Lazy::new(|| keccak256("Withdrawal(address,uint256)"));
pub(crate) static ETH_LOG_DELEGATION: Lazy<H256> =
    Lazy::new(|| keccak256("Delegation(address,bytes[],uint256[])"));
pub(crate) static ETH_FUEL_BLOCK_COMMITTED: Lazy<H256> =
    Lazy::new(|| keccak256("BlockCommitted(bytes32,uint32)"));

#[derive(Clone, Debug)]
pub struct Config {
    /// Number of da block after which messages/stakes/validators become finalized.
    pub da_finalization: DaBlockHeight,
    /// Uri address to ethereum client.
    pub eth_client: Option<String>,
    /// Ethereum chain_id.
    pub eth_chain_id: u64,
    /// Contract to publish commit fuel block.  
    pub eth_v2_commit_contract: Option<H160>,
    /// Ethereum contract address. Create EthAddress into fuel_types.
    pub eth_v2_listening_contracts: Vec<H160>,
    /// Block number after we can start filtering events related to fuel.
    /// It does not need to be accurate and can be set in past before contracts are deployed.
    pub eth_v2_contracts_deployment: DaBlockHeight,
    /// Number of blocks that will be asked at one time from client, used for initial sync.
    pub initial_sync_step: usize,
    /// Refresh rate of waiting for eth client to finish its initial sync.
    pub initial_sync_refresh: Duration,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            da_finalization: 64,
            // Some(String::from("http://localhost:8545"))
            eth_client: None,
            eth_chain_id: 1, // ethereum mainnet
            eth_v2_commit_contract: None,
            eth_v2_listening_contracts: vec![H160::from_str(
                "0x03E4538018285e1c03CCce2F92C9538c87606911",
            )
            .unwrap()],
            eth_v2_contracts_deployment: 0,
            initial_sync_step: 1000,
            initial_sync_refresh: Duration::from_secs(5),
        }
    }
}

impl Config {
    pub fn eth_v2_contracts_deployment(&self) -> DaBlockHeight {
        self.eth_v2_contracts_deployment
    }

    pub fn eth_v2_listening_contracts(&self) -> &[H160] {
        &self.eth_v2_listening_contracts
    }

    pub fn da_finalization(&self) -> DaBlockHeight {
        self.da_finalization
    }

    pub fn eth_client(&self) -> Option<&str> {
        self.eth_client.as_deref()
    }

    pub fn initial_sync_step(&self) -> usize {
        self.initial_sync_step
    }

    pub fn initial_sync_refresh(&self) -> Duration {
        self.initial_sync_refresh
    }

    pub fn eth_chain_id(&self) -> u64 {
        self.eth_chain_id
    }

    pub fn eth_v2_commit_contract(&self) -> Option<H160> {
        self.eth_v2_commit_contract
    }
}

#[cfg(test)]
mod tests {
    use crate::config::*;

    #[test]
    pub fn test_function_signatures() {
        assert_eq!(
            *ETH_LOG_MESSAGE,
            H256::from_str(
                "0x6e777c34951035560591fac300515942821cca139ab8a514eb117129048e21b2"
            )
            .unwrap()
        );
        assert_eq!(
            *ETH_LOG_VALIDATOR_REGISTRATION,
            H256::from_str(
                "0xb880ae9a41c67ab61e670929983ea383810f2a09e384b5d1e40a6a8d123e643f"
            )
            .unwrap()
        );
        assert_eq!(
            *ETH_LOG_DEPOSIT,
            H256::from_str(
                "0xe1fffcc4923d04b559f4d29a8bfc6cda04eb5b0d3c460751c2402c5c5cc9109c"
            )
            .unwrap()
        );
        assert_eq!(
            *ETH_LOG_WITHDRAWAL,
            H256::from_str(
                "0x7fcf532c15f0a6db0bd6d0e038bea71d30d808c7d98cb3bf7268a95bf5081b65"
            )
            .unwrap()
        );
        assert_eq!(
            *ETH_LOG_DELEGATION,
            H256::from_str(
                "0xb304243c5b5465a0f6a6b44be45b6906650d542c8e1dd33b0630f72b2f454081"
            )
            .unwrap()
        );
        assert_eq!(
            *ETH_FUEL_BLOCK_COMMITTED,
            H256::from_str(
                "0xacd88c3d7181454636347207da731b757b80b2696b26d8e1b378d2ab5ed3e872"
            )
            .unwrap()
        );
    }
}
