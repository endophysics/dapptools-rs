mod evm;
pub use evm::*;

mod forked_backend;
pub use forked_backend::ForkMemoryBackend;

pub mod cheatcodes;

use ethers::{
    providers::Middleware,
    types::{Address, H160, H256, U256},
};

use sputnik::{
    backend::MemoryVicinity,
    executor::{StackExecutor, StackState},
    Config, CreateScheme, ExitReason,
};

pub async fn vicinity<M: Middleware>(
    provider: &M,
    pin_block: Option<u64>,
) -> Result<MemoryVicinity, M::Error> {
    let block_number = if let Some(pin_block) = pin_block {
        pin_block
    } else {
        provider.get_block_number().await?.as_u64()
    };
    let (gas_price, chain_id, block) = tokio::try_join!(
        provider.get_gas_price(),
        provider.get_chainid(),
        provider.get_block(block_number)
    )?;
    let block = block.expect("block not found");

    Ok(MemoryVicinity {
        origin: Default::default(),
        chain_id,
        block_hashes: Vec::new(),
        block_number: block.number.expect("block number not found").as_u64().into(),
        block_coinbase: block.author,
        block_difficulty: block.difficulty,
        block_gas_limit: block.gas_limit,
        block_timestamp: block.timestamp,
        gas_price,
    })
}

/// Abstraction over the StackExecutor used inside of Sputnik, so that we can replace
/// it with one that implements HEVM-style cheatcodes (or other features).
pub trait SputnikExecutor<S> {
    fn config(&self) -> &Config;
    fn state(&self) -> &S;
    fn state_mut(&mut self) -> &mut S;
    fn gas_left(&self) -> U256;
    fn transact_call(
        &mut self,
        caller: H160,
        address: H160,
        value: U256,
        data: Vec<u8>,
        gas_limit: u64,
        access_list: Vec<(H160, Vec<H256>)>,
    ) -> (ExitReason, Vec<u8>);

    fn transact_create(
        &mut self,
        caller: H160,
        value: U256,
        data: Vec<u8>,
        gas_limit: u64,
        access_list: Vec<(H160, Vec<H256>)>,
    ) -> ExitReason;

    fn create_address(&self, caller: CreateScheme) -> Address;

    /// Returns a vector of string parsed logs that occurred during the previous VM
    /// execution
    fn logs(&self) -> Vec<String>;

    /// Clears all logs in the current EVM instance, so that subsequent calls to
    /// `logs` do not print duplicate logs on shared EVM instances.
    fn clear_logs(&mut self);
}

// The implementation for the base Stack Executor just forwards to the internal methods.
impl<'a, S: StackState<'a>> SputnikExecutor<S> for StackExecutor<'a, S> {
    fn config(&self) -> &Config {
        self.config()
    }

    fn state(&self) -> &S {
        self.state()
    }

    fn state_mut(&mut self) -> &mut S {
        self.state_mut()
    }

    fn gas_left(&self) -> U256 {
        // NB: We do this to avoid `function cannot return without recursing`
        U256::from(self.state().metadata().gasometer().gas())
    }

    fn transact_call(
        &mut self,
        caller: H160,
        address: H160,
        value: U256,
        data: Vec<u8>,
        gas_limit: u64,
        access_list: Vec<(H160, Vec<H256>)>,
    ) -> (ExitReason, Vec<u8>) {
        self.transact_call(caller, address, value, data, gas_limit, access_list)
    }

    fn transact_create(
        &mut self,
        caller: H160,
        value: U256,
        data: Vec<u8>,
        gas_limit: u64,
        access_list: Vec<(H160, Vec<H256>)>,
    ) -> ExitReason {
        self.transact_create(caller, value, data, gas_limit, access_list)
    }

    fn create_address(&self, scheme: CreateScheme) -> Address {
        self.create_address(scheme)
    }

    // Empty impls for non-cheatcode handlers
    fn logs(&self) -> Vec<String> {
        vec![]
    }
    fn clear_logs(&mut self) {}
}
