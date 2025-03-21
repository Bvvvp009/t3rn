use crate::{
    account_manager::Outcome,
    circuit::LocalStateExecutionView,
    contract_metadata::ContractType,
    contracts_registry::{AuthorInfo, RegistryContract},
    portal::{PortalExecution, PrecompileArgs as PortalPrecompileArgs},
    SpeedMode,
};
use codec::{Decode, Encode};
use frame_support::{dispatch::Weight, traits::WithdrawReasons};
use frame_system::{pallet_prelude::BlockNumberFor, Config as ConfigSystem};
use scale_info::TypeInfo;
use sp_core::{H160, H256, U256};
use sp_runtime::{
    traits::{CheckedDiv, Zero},
    DispatchError, DispatchResult, Saturating,
};
use sp_std::{fmt::Debug, result::Result, vec::Vec};
use t3rn_sdk_primitives::{
    signal::{ExecutionSignal, Signaller},
    state::SideEffects,
};

use crate::circuit::{VacuumEVM3DOrder, VacuumEVMOrder, VacuumEVMProof, VacuumEVMTeleportOrder};
use circuit_runtime_types::{EvmAddress, TokenId};

// Precompile pointers baked into the binary.
// Genesis exists only to map hashes to pointers.
pub const GET_STATE: u8 = 55;
pub const VACUUM_ORDER: u8 = 90;
pub const VACUUM_3D_ORDER: u8 = 91;
pub const VACUUM_CONFIRM: u8 = 92;
pub const VACUUM_SUBMIT_CORRECTNESS_PROOF: u8 = 93;
pub const VACUUM_SUBMIT_FAULT_PROOF: u8 = 94;
pub const VACUUM_TELEPORT_ORDER: u8 = 95;
pub const SUBMIT: u8 = 56;
pub const POST_SIGNAL: u8 = 57;
pub const PORTAL: u8 = 70;

#[derive(Encode, Decode)]
pub struct GetState<T: ConfigSystem> {
    pub xtx_id: Option<T::Hash>,
}

// FIXME: none of these work at the moment due to large updates to SFX ABI.
#[derive(Encode, Decode)]
pub enum PrecompileArgs<T, Balance>
where
    T: ConfigSystem,
    Balance: Encode + Decode,
{
    GetState(T::RuntimeOrigin, GetState<T>),
    SubmitSideEffects(
        T::RuntimeOrigin,
        SideEffects<T::AccountId, Balance, T::Hash>,
        SpeedMode,
    ),
    VacuumOrder(T::RuntimeOrigin, VacuumEVMOrder),
    Vacuum3DOrder(T::RuntimeOrigin, VacuumEVM3DOrder),
    VacuumConfirm(T::RuntimeOrigin, VacuumEVMOrder),
    VacuumSubmitCorrectnessProof(T::RuntimeOrigin, VacuumEVMProof),
    VacuumSubmitFaultProof(T::RuntimeOrigin, VacuumEVMProof),
    VacuumTeleportOrder(T::RuntimeOrigin, VacuumEVMTeleportOrder),
    Signal(T::RuntimeOrigin, ExecutionSignal<T::Hash>),
    Portal(PortalPrecompileArgs),
}

/// The happy return type of an invocation
pub enum PrecompileInvocation<T: ConfigSystem, Balance> {
    GetState(LocalStateExecutionView<T, Balance>),
    Submit(LocalStateExecutionView<T, Balance>),
    Signal,
    Portal(PortalExecution<T>),
    VacuumOrder(bool),
    VacuumConfirm(bool),
    Vacuum3DOrder(bool),
    VacuumSubmitFaultProof(bool),
    VacuumSubmitCorrectnessProof(bool),
    VacuumTeleportOrder(bool),
}

impl<T: ConfigSystem, Balance> PrecompileInvocation<T, Balance> {
    pub fn get_state(&self) -> Option<&LocalStateExecutionView<T, Balance>> {
        match self {
            PrecompileInvocation::GetState(state) => Some(state),
            _ => None,
        }
    }

    pub fn get_submit(&self) -> Option<&LocalStateExecutionView<T, Balance>> {
        match self {
            PrecompileInvocation::Submit(state) => Some(state),
            _ => None,
        }
    }
}

pub trait Contracts<AccountId, Balance, EventRecord> {
    type Outcome;
    fn call(
        origin: AccountId,
        dest: AccountId,
        value: Balance,
        gas_limit: Weight,
        storage_deposit_limit: Option<Balance>,
        data: Vec<u8>,
        debug: bool,
    ) -> Self::Outcome;
}

pub trait Evm<Origin> {
    type Outcome;
    #[allow(clippy::too_many_arguments)] // Simply has a lot of args
    fn call(
        origin: Origin,
        target: H160,
        input: Vec<u8>,
        value: U256,
        gas_limit: u64,
        max_fee_per_gas: U256,
        max_priority_fee_per_gas: Option<U256>,
        nonce: Option<U256>,
        access_list: Vec<(H160, Vec<H256>)>,
    ) -> Self::Outcome;
}

pub trait Precompile<T, Balance>
where
    T: ConfigSystem,
    Balance: Encode + Decode,
{
    /// Looks up a precompile function pointer
    fn lookup(dest: &T::Hash) -> Option<u8>;

    /// Invoke a precompile, providing raw bytes and a pointer
    fn invoke_raw(precompile: &u8, args: &[u8], output: &mut Vec<u8>);

    /// Invoke a precompile
    fn invoke(
        args: PrecompileArgs<T, Balance>,
    ) -> Result<PrecompileInvocation<T, Balance>, DispatchError>;
}

pub trait LocalStateAccess<T, Balance>
where
    T: ConfigSystem,
{
    fn load_local_state(
        origin: &T::RuntimeOrigin,
        xtx_id: Option<&T::Hash>,
    ) -> Result<LocalStateExecutionView<T, Balance>, DispatchError>;
}

pub trait VacuumAccess<T>
where
    T: ConfigSystem,
{
    fn evm_order(
        origin: &T::RuntimeOrigin,
        vacuum_evm_order: VacuumEVMOrder,
    ) -> Result<bool, DispatchError>;

    fn evm_teleport_order(
        origin: &T::RuntimeOrigin,
        vacuum_evm_order: VacuumEVMTeleportOrder,
    ) -> Result<bool, DispatchError>;

    fn evm_confirm(
        origin: &T::RuntimeOrigin,
        vacuum_evm_order: VacuumEVMOrder,
    ) -> Result<bool, DispatchError>;

    fn evm_submit_fault_proof(
        origin: &T::RuntimeOrigin,
        vacuum_evm_proof: VacuumEVMProof,
    ) -> Result<bool, DispatchError>;

    fn evm_3d_order(
        origin: &T::RuntimeOrigin,
        vacuum_evm_order: VacuumEVM3DOrder,
    ) -> Result<bool, DispatchError>;
}

pub struct Remunerated<Hash> {
    pub remuneration_id: Option<Hash>,
}

impl<Hash> Default for Remunerated<Hash> {
    fn default() -> Self {
        Remunerated {
            remuneration_id: None,
        }
    }
}

impl<Hash> Remunerated<Hash> {
    pub fn new(id: Option<Hash>) -> Self {
        Remunerated {
            remuneration_id: id,
        }
    }
}

pub trait Remuneration<T: ConfigSystem, Balance> {
    /// Try to remunerate the fees from the given module
    fn try_remunerate<Module: ModuleOperations<T, Balance>>(
        payee: &T::AccountId,
        module: &Module,
    ) -> Result<Remunerated<T::Hash>, sp_runtime::DispatchError>;

    /// Try to remunerate the fees from the given module with a custom balance
    fn try_remunerate_exact<Module: ModuleOperations<T, Balance>>(
        payee: &T::AccountId,
        amount: Balance,
        module: &Module,
    ) -> Result<Remunerated<T::Hash>, sp_runtime::DispatchError>;

    /// Try to finalize a ledger item with an reason
    fn try_finalize(ledger_id: T::Hash, outcome: Outcome) -> DispatchResult;
}
pub enum Characteristic {
    Storage,
    Instantiate,
    Remuneration,
    Volatile,
}

/// Passthrough to validator
pub trait CharacteristicValidator {
    fn validate(characteristic: &Characteristic) -> Result<(), ()>; // TODO: handle error
}

#[derive(Encode, Decode, Debug, PartialEq, Eq)]
pub enum SignalOpcode {
    Initiated,
    Bounced,
}

pub trait ThreeVm<T, Balance>:
    Precompile<T, Balance>
    + Signaller<T::Hash, Result = Result<SignalOpcode, DispatchError>>
    + Remuneration<T, Balance>
where
    T: ConfigSystem,
    Balance: Encode + Decode,
{
    fn peek_registry(
        id: &T::Hash,
    ) -> Result<RegistryContract<T::Hash, T::AccountId, Balance, BlockNumberFor<T>>, DispatchError>;

    /// Allows creating a `Module` from a binary blob from the contracts registry
    fn from_registry<Module, ModuleGen>(
        id: &T::Hash,
        module_generator: ModuleGen,
    ) -> Result<Module, DispatchError>
    where
        Module: ModuleOperations<T, Balance>,
        ModuleGen: Fn(Vec<u8>) -> Module;

    fn instantiate_check(kind: &ContractType) -> Result<(), DispatchError>;

    fn storage_check(kind: &ContractType) -> Result<(), DispatchError>;

    fn volatile_check(kind: &ContractType) -> Result<(), DispatchError>;

    fn remunerable_check(kind: &ContractType) -> Result<(), DispatchError>;

    fn try_persist_author(
        contract: &T::AccountId,
        author: Option<&AuthorInfo<T::AccountId, Balance>>,
    ) -> Result<(), DispatchError>;

    fn try_remove_author(contract: &T::AccountId) -> Result<(), DispatchError>;
}

pub struct NoopThreeVm;

impl<T, Balance> LocalStateAccess<T, Balance> for NoopThreeVm
where
    T: ConfigSystem,
{
    fn load_local_state(
        _origin: &T::RuntimeOrigin,
        _xtx_id: Option<&T::Hash>,
    ) -> Result<LocalStateExecutionView<T, Balance>, DispatchError> {
        Err("Local State Not implemented").map_err(|e| e.into())
    }
}

impl<T: ConfigSystem, Balance: Encode + Decode> Remuneration<T, Balance> for NoopThreeVm {
    fn try_remunerate<Module: ModuleOperations<T, Balance>>(
        _payee: &T::AccountId,
        _module: &Module,
    ) -> Result<Remunerated<T::Hash>, sp_runtime::DispatchError> {
        Ok(Remunerated {
            remuneration_id: None,
        })
    }

    fn try_remunerate_exact<Module: ModuleOperations<T, Balance>>(
        _payee: &T::AccountId,
        _amount: Balance,
        _module: &Module,
    ) -> Result<Remunerated<T::Hash>, sp_runtime::DispatchError> {
        Ok(Remunerated {
            remuneration_id: None,
        })
    }

    fn try_finalize(_ledger_id: T::Hash, _outcome: Outcome) -> DispatchResult {
        Ok(())
    }
}

impl<Hash: Encode + Decode + Debug + Clone> Signaller<Hash> for NoopThreeVm {
    type Result = Result<SignalOpcode, DispatchError>;

    fn signal(_signal: &ExecutionSignal<Hash>) -> Self::Result {
        Err("Signalling is not enabled".into())
    }
}

impl<T, Balance> Precompile<T, Balance> for NoopThreeVm
where
    T: ConfigSystem,
    Balance: Encode + Decode,
{
    fn lookup(_dest: &T::Hash) -> Option<u8> {
        None
    }

    fn invoke_raw(_precompile: &u8, _args: &[u8], _output: &mut Vec<u8>) {}

    fn invoke(
        _args: PrecompileArgs<T, Balance>,
    ) -> Result<PrecompileInvocation<T, Balance>, DispatchError> {
        Err("Precompile Invocation Not implemented").map_err(|e| e.into())
    }
}

// Default impl
impl<T: ConfigSystem, Balance: Encode + Decode> ThreeVm<T, Balance> for NoopThreeVm {
    fn peek_registry(
        _id: &<T as ConfigSystem>::Hash,
    ) -> Result<
        RegistryContract<
            <T as ConfigSystem>::Hash,
            <T as ConfigSystem>::AccountId,
            Balance,
            BlockNumberFor<T>,
        >,
        DispatchError,
    > {
        Err("Registry Peek Not implemented").map_err(|e| e.into())
    }

    fn from_registry<Module, ModuleGen>(
        _id: &<T as ConfigSystem>::Hash,
        _module_generator: ModuleGen,
    ) -> Result<Module, DispatchError>
    where
        Module: ModuleOperations<T, Balance>,
        ModuleGen: Fn(Vec<u8>) -> Module,
    {
        Err("From Registry Not implemented").map_err(|e| e.into())
    }

    fn instantiate_check(_kind: &ContractType) -> Result<(), DispatchError> {
        Ok(())
    }

    fn storage_check(_kind: &ContractType) -> Result<(), DispatchError> {
        Ok(())
    }

    fn volatile_check(_kind: &ContractType) -> Result<(), DispatchError> {
        Ok(())
    }

    fn remunerable_check(_kind: &ContractType) -> Result<(), DispatchError> {
        Ok(())
    }

    fn try_persist_author(
        _contract: &<T as ConfigSystem>::AccountId,
        _author: Option<&AuthorInfo<<T as ConfigSystem>::AccountId, Balance>>,
    ) -> Result<(), DispatchError> {
        Ok(())
    }

    fn try_remove_author(_conztract: &<T as ConfigSystem>::AccountId) -> Result<(), DispatchError> {
        Ok(())
    }
}

pub trait ModuleOperations<T: ConfigSystem, Balance> {
    fn get_bytecode(&self) -> &Vec<u8>;
    fn get_author(&self) -> Option<&AuthorInfo<T::AccountId, Balance>>;
    fn set_author(&mut self, author: AuthorInfo<T::AccountId, Balance>);
    fn get_type(&self) -> &ContractType;
    fn set_type(&mut self, kind: ContractType);
}

#[derive(Clone, Encode, Decode, TypeInfo, Default)]
#[scale_info(skip_type_params(T))]
pub struct ThreeVmInfo<T: ConfigSystem, Balance> {
    author: AuthorInfo<T::AccountId, Balance>,
    kind: ContractType,
    bytecode: Vec<u8>,
}

impl<T: ConfigSystem, Balance> ModuleOperations<T, Balance> for ThreeVmInfo<T, Balance> {
    fn get_bytecode(&self) -> &Vec<u8> {
        &self.bytecode
    }

    fn get_author(&self) -> Option<&AuthorInfo<T::AccountId, Balance>> {
        Some(&self.author)
    }

    fn set_author(&mut self, author: AuthorInfo<T::AccountId, Balance>) {
        self.author = author;
    }

    fn get_type(&self) -> &ContractType {
        &self.kind
    }

    fn set_type(&mut self, kind: ContractType) {
        self.kind = kind;
    }
}

/// The index from which the endoded AssetId bytes will be encoded into an EVM address
pub const H160_POSITION_ASSET_ID_TYPE: usize = 16;

pub trait Erc20Mapping {
    /// Encode the AssetId to EvmAddress.
    fn encode_evm_address(v: TokenId) -> Option<EvmAddress>;
    /// Decode the AssetId from EvmAddress.
    fn decode_evm_address(v: EvmAddress) -> Option<TokenId>;
}

/// A mapping between `AccountId` and `EvmAddress`.
pub trait AddressMapping<AccountId> {
    /// Returns the AccountId used go generate the given EvmAddress.
    fn into_account_id(evm: &EvmAddress) -> AccountId;
    /// Returns the EvmAddress associated with a given AccountId or the
    /// underlying EvmAddress of the AccountId.
    /// Returns None if there is no EvmAddress associated with the AccountId
    /// and there is no underlying EvmAddress in the AccountId.
    fn get_evm_address(account_id: &AccountId) -> Option<EvmAddress>;
    /// Returns the EVM address associated with an account ID and generates an
    /// account mapping if no association exists.
    fn get_or_create_evm_address(account_id: &AccountId) -> EvmAddress;
    /// Returns the default EVM address associated with an account ID.
    fn get_default_evm_address(account_id: &AccountId) -> EvmAddress;
    /// Returns true if a given AccountId is associated with a given EvmAddress
    /// and false if is not.
    fn is_linked(account_id: &AccountId, evm: &EvmAddress) -> bool;
}

/// Convert decimal between TRN(12) and EVM(18) and therefore the 1_000_000 conversion.
pub const DECIMALS_VALUE: u32 = 1_000_000u32;

/// Convert decimal from native(TRN 12) to EVM(18).
pub fn convert_decimals_to_evm<B: Zero + Saturating + From<u32>>(b: B) -> B {
    if b.is_zero() {
        return b
    }
    b.saturating_mul(DECIMALS_VALUE.into())
}

/// Convert decimal from native EVM(18) to TRN(12).
pub fn convert_decimals_from_evm<
    B: Zero + Saturating + CheckedDiv + PartialEq + Copy + From<u32>,
>(
    b: B,
) -> Option<B> {
    if b.is_zero() {
        return Some(b)
    }
    let res = b
        .checked_div(&Into::<B>::into(DECIMALS_VALUE))
        .expect("divisor is non-zero; qed");

    if res.saturating_mul(DECIMALS_VALUE.into()) == b {
        Some(res)
    } else {
        None
    }
}

pub fn get_tokens_precompile_address(index: u32) -> H160 {
    let mut address = [9u8; 20];
    let index_bytes = index.to_be_bytes().to_vec();
    for byte_index in H160_POSITION_ASSET_ID_TYPE..20 {
        address[byte_index] = index_bytes[byte_index - H160_POSITION_ASSET_ID_TYPE];
    }
    H160(address)
}
