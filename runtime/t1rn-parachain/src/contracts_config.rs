use crate::{
    accounts_config::EscrowAccount, AccountId, AccountManager, Aura, Balance, Balances, Circuit,
    ContractsRegistry, Portal, RandomnessCollectiveFlip, RuntimeCall, RuntimeEvent, ThreeVm,
    Timestamp, Weight, AVERAGE_ON_INITIALIZE_RATIO, *,
};
use frame_support::{
    pallet_prelude::ConstU32,
    parameter_types,
    traits::{ConstBool, FindAuthor},
};

use circuit_runtime_types::{
    BLOCK_GAS_LIMIT, GAS_LIMIT_POV_SIZE_RATIO, GAS_PRICE, GAS_WEIGHT, MILLIUNIT, UNIT,
    WEIGHT_PER_GAS,
};

// use evm_precompile_util::KnownPrecompile;
use frame_support::PalletId;
pub use pallet_3vm_account_mapping::EvmAddressMapping;
use pallet_3vm_contracts::NoopMigration;
use pallet_3vm_evm::{EnsureAddressTruncated, HashedAddressMapping, SubstrateBlockHashMapping};
use pallet_3vm_evm_primitives::FeeCalculator;
#[cfg(feature = "std")]
pub use pallet_3vm_evm_primitives::GenesisAccount as EvmGenesisAccount;
use sp_core::{H160, U256};
use sp_runtime::{
    traits::{AccountIdConversion, Keccak256},
    ConsensusEngineId, RuntimeAppPublic,
};

const _EXISTENTIAL_DEPOSIT: Balance = MILLIUNIT;

const fn deposit(items: u32, bytes: u32) -> Balance {
    (items as Balance * UNIT + (bytes as Balance) * (5 * MILLIUNIT / 100)) / 10
}

parameter_types! {
    pub const CreateSideEffectsPrecompileDest: AccountId = AccountId::new([51u8; 32]); // 0x333...3
    pub const CircuitTargetId: t3rn_primitives::ChainId = [3, 3, 3, 3];

    pub const MaxValueSize: u32 = 16_384;
    // The lazy deletion runs inside on_initialize.
    pub DeletionWeightLimit: Weight = AVERAGE_ON_INITIALIZE_RATIO *
        RuntimeBlockWeights::get().max_block;
    pub Schedule: pallet_3vm_contracts::Schedule<Runtime> = Default::default();
    pub const MaxCodeSize: u32 = 2 * 1024;
    pub const DepositPerItem: Balance = deposit(1, 0);
    pub const DepositPerByte: Balance = deposit(0, 1);
    pub const DefaultDepositLimit: Balance = 10_000_000;
}

impl pallet_3vm::Config for Runtime {
    type AccountManager = AccountManager;
    type AddressMapping = EvmAddressMapping<Runtime>;
    type AssetId = AssetId;
    type CircuitTargetId = CircuitTargetId;
    type ContractsRegistry = ContractsRegistry;
    type Currency = Balances;
    type EscrowAccount = EscrowAccount;
    type OnLocalTrigger = Circuit;
    type Portal = Portal;
    type RuntimeEvent = RuntimeEvent;
    type SignalBounceThreshold = ConstU32<2>;
    type VacuumEVMApi = Vacuum;
}

parameter_types! {
    pub const T3rnPalletId: PalletId = PalletId(*b"trn/trsy");
    pub TreasuryModuleAccount: AccountId = T3rnPalletId::get().into_account_truncating();
    pub const StorageDepositFee: Balance = MILLIUNIT / 100;
}

impl pallet_3vm_account_mapping::Config for Runtime {
    type AddressMapping = EvmAddressMapping<Runtime>;
    type ChainId = ChainId;
    type Currency = Balances;
    type NetworkTreasuryAccount = TreasuryModuleAccount;
    type RuntimeEvent = RuntimeEvent;
    type StorageDepositFee = StorageDepositFee;
}

impl pallet_3vm_contracts::Config for Runtime {
    type AddressGenerator = pallet_3vm_contracts::DefaultAddressGenerator;
    /// The safest default is to allow no calls at all.
    ///
    /// Runtimes should whitelist dispatchables that are allowed to be called from contracts
    /// and make sure they are stable. Dispatchables exposed to contracts are not allowed to
    /// change because that would break already deployed contracts. The `Call` structure itself
    /// is not allowed to change the indices of existing pallets, too.
    type CallFilter = frame_support::traits::Nothing;
    type CallStack = [pallet_3vm_contracts::Frame<Self>; 5];
    type ChainExtension = ();
    type Currency = Balances;
    type DefaultDepositLimit = DefaultDepositLimit;
    type DepositPerByte = DepositPerByte;
    type DepositPerItem = DepositPerItem;
    type MaxCodeLen = ConstU32<{ 123 * 1024 }>;
    type MaxDebugBufferLen = ConstU32<{ 2 * 1024 * 1024 }>;
    type MaxStorageKeyLen = ConstU32<128>;
    type Migrations = (NoopMigration<1>, NoopMigration<2>);
    type Randomness = RandomnessCollectiveFlip;
    type RuntimeCall = RuntimeCall;
    type RuntimeEvent = RuntimeEvent;
    type Schedule = Schedule;
    type ThreeVm = ThreeVm;
    type Time = Timestamp;
    type UnsafeUnstableInterface = ConstBool<true>;
    type WeightInfo = pallet_3vm_contracts::weights::SubstrateWeight<Self>;
    type WeightPrice = pallet_transaction_payment::Pallet<Self>;
}

pub struct FindAuthorTruncated<F>(sp_std::marker::PhantomData<F>);
impl<F: FindAuthor<u32>> FindAuthor<H160> for FindAuthorTruncated<F> {
    fn find_author<'a, I>(digests: I) -> Option<H160>
    where
        I: 'a + IntoIterator<Item = (ConsensusEngineId, &'a [u8])>,
    {
        if let Some(author_index) = F::find_author(digests) {
            let authority_id = Aura::authorities()[author_index as usize].clone();
            return Some(H160::from_slice(&authority_id.to_raw_vec()[4..24]))
        }
        None
    }
}

pub struct FixedGasPrice;
impl FeeCalculator for FixedGasPrice {
    fn min_gas_price() -> (U256, Weight) {
        // Return some meaningful gas price and weight
        (GAS_PRICE.into(), GAS_WEIGHT)
    }
}

parameter_types! {
    pub BlockGasLimit: U256 = U256::from(BLOCK_GAS_LIMIT);
    pub const GasLimitPovSizeRatio: u64 = GAS_LIMIT_POV_SIZE_RATIO;
    pub const ChainId: u64 = 3331;
    pub WeightPerGas: Weight = WEIGHT_PER_GAS;
    // pub PrecompilesValue: evm_precompile_util::T3rnPrecompiles<Runtime> = evm_precompile_util::T3rnPrecompiles::<_>::new();;
}

// pub struct Precompiles<Runtime> {
//     pub inner: BTreeMap<H160, KnownPrecompile<Runtime>>,
//     phantom: PhantomData<Runtime>,
// }
// TODO[https://github.com/t3rn/3vm/issues/102]: configure this appropriately
impl pallet_3vm_evm::Config for Runtime {
    type AddressMapping = HashedAddressMapping<Keccak256>;
    type BlockGasLimit = BlockGasLimit;
    type BlockHashMapping = SubstrateBlockHashMapping<Self>;
    type CallOrigin = EnsureAddressTruncated;
    type ChainId = ChainId;
    type Currency = Balances;
    // BaseFee pallet may be better from frontier TODO
    type FeeCalculator = FixedGasPrice;
    type FindAuthor = FindAuthorTruncated<Aura>;
    type GasLimitPovSizeRatio = GasLimitPovSizeRatio;
    type GasWeightMapping = pallet_3vm_evm::FixedGasWeightMapping<Runtime>;
    type OnChargeTransaction = ();
    type OnCreate = ();
    type PrecompilesType = ();
    // type PrecompilesValue = PrecompilesValue;
    // fixme: add and compile pre-compiles compile_error!("the wasm*-unknown-unknown targets are not supported by \
    type PrecompilesValue = ();
    type Runner = pallet_3vm_evm::runner::stack::Runner<Self>;
    type RuntimeEvent = RuntimeEvent;
    type ThreeVm = ThreeVm;
    type Timestamp = Timestamp;
    type WeightInfo = ();
    type WeightPerGas = WeightPerGas;
    type WithdrawOrigin = EnsureAddressTruncated;
}
