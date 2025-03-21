// Copyright 2019-2022 PureStake Inc.
// This file is part of Moonbeam.

// Moonbeam is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Moonbeam is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Moonbeam.  If not, see <http://www.gnu.org/licenses/>.

//! A minimal runtime including the maintenance-mode pallet
use super::*;
use crate as pallet_maintenance_mode;
use frame_support::{
    construct_runtime, parameter_types,
    traits::{
        Contains, Everything, GenesisBuild, OffchainWorker, OnFinalize, OnIdle, OnInitialize,
        OnRuntimeUpgrade,
    },
    weights::Weight,
};
use frame_system::EnsureRoot;
use sp_core::H256;
use sp_runtime::{
    traits::{BlakeTwo256, IdentityLookup},
    Perbill,
};

//TODO use TestAccount once it is in a common place (currently it lives with democracy precompiles)
pub type AccountId = u64;
pub type BlockNumber = u32;

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
pub type Block = sp_runtime::generic::Block<
    sp_runtime::generic::Header<BlockNumber, sp_runtime::traits::BlakeTwo256>,
    frame_system::mocking::MockUncheckedExtrinsic<Test>,
>;
// Configure a mock runtime to test the pallet.
construct_runtime!(
    pub enum Test where
        Block = Block,
        NodeBlock = Block,
        UncheckedExtrinsic = UncheckedExtrinsic,
    {
        System: frame_system,
        MaintenanceMode: pallet_maintenance_mode,
        MockPalletMaintenanceHooks: mock_pallet_maintenance_hooks,
    }
);

parameter_types! {
    pub const BlockHashCount: u32 = 250;
    pub const MaximumBlockWeight: Weight = Weight::from_parts(1024, 1);
    pub const MaximumBlockLength: u32 = 2 * 1024;
    pub const AvailableBlockRatio: Perbill = Perbill::one();
    pub const SS58Prefix: u8 = 42;
}
impl frame_system::Config for Test {
    type AccountData = ();
    type AccountId = AccountId;
    type BaseCallFilter = MaintenanceMode;
    type Block = Block;
    type BlockHashCount = BlockHashCount;
    type BlockLength = ();
    type BlockWeights = ();
    type DbWeight = ();
    type Hash = H256;
    type Hashing = BlakeTwo256;
    type Lookup = IdentityLookup<Self::AccountId>;
    type MaxConsumers = frame_support::traits::ConstU32<16>;
    type Nonce = u64;
    type OnKilledAccount = ();
    type OnNewAccount = ();
    type OnSetCode = ();
    type PalletInfo = PalletInfo;
    type RuntimeCall = RuntimeCall;
    type RuntimeEvent = RuntimeEvent;
    type RuntimeOrigin = RuntimeOrigin;
    type SS58Prefix = SS58Prefix;
    type SystemWeightInfo = ();
    type Version = ();
}

/// During maintenance mode we will not allow any calls.
pub struct MaintenanceCallFilter;
impl Contains<RuntimeCall> for MaintenanceCallFilter {
    fn contains(_: &RuntimeCall) -> bool {
        false
    }
}

pub struct MaintenanceDmpHandler;
#[cfg(feature = "xcm-support")]
impl DmpMessageHandler for MaintenanceDmpHandler {
    // This implementation makes messages be queued
    // Since the limit is 0, messages are queued for next iteration
    fn handle_dmp_messages(
        _iter: impl Iterator<Item = (RelayBlockNumber, Vec<u8>)>,
        _limit: Weight,
    ) -> Weight {
        return Weight::from_parts(1, 0)
    }
}

pub struct NormalDmpHandler;
#[cfg(feature = "xcm-support")]
impl DmpMessageHandler for NormalDmpHandler {
    // This implementation makes messages be queued
    // Since the limit is 0, messages are queued for next iteration
    fn handle_dmp_messages(
        _iter: impl Iterator<Item = (RelayBlockNumber, Vec<u8>)>,
        _limit: Weight,
    ) -> Weight {
        Weight::zero()
    }
}

impl mock_pallet_maintenance_hooks::Config for Test {
    type RuntimeEvent = RuntimeEvent;
}

// Pallet to throw events, used to test maintenance mode hooks
#[frame_support::pallet]
pub mod mock_pallet_maintenance_hooks {
    use frame_support::pallet_prelude::*;

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type RuntimeEvent: From<Event> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
    }

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    #[pallet::call]
    impl<T: Config> Pallet<T> {}

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event {
        MaintenanceOnIdle,
        MaintenanceOnInitialize,
        MaintenanceOffchainWorker,
        MaintenanceOnFinalize,
        MaintenanceOnRuntimeUpgrade,
        NormalOnIdle,
        NormalOnInitialize,
        NormalOffchainWorker,
        NormalOnFinalize,
        NormalOnRuntimeUpgrade,
    }
}

pub struct MaintenanceHooks;

impl OnInitialize<BlockNumber> for MaintenanceHooks {
    fn on_initialize(_n: BlockNumber) -> Weight {
        MockPalletMaintenanceHooks::deposit_event(
            mock_pallet_maintenance_hooks::Event::MaintenanceOnInitialize,
        );
        Weight::from_parts(1, 0)
    }
}

impl OnIdle<BlockNumber> for MaintenanceHooks {
    fn on_idle(_n: BlockNumber, _max_weight: Weight) -> Weight {
        MockPalletMaintenanceHooks::deposit_event(
            mock_pallet_maintenance_hooks::Event::MaintenanceOnIdle,
        );
        Weight::from_parts(1, 0)
    }
}

impl OnRuntimeUpgrade for MaintenanceHooks {
    fn on_runtime_upgrade() -> Weight {
        MockPalletMaintenanceHooks::deposit_event(
            mock_pallet_maintenance_hooks::Event::MaintenanceOnRuntimeUpgrade,
        );
        Weight::from_parts(1, 0)
    }
}

impl OnFinalize<BlockNumber> for MaintenanceHooks {
    fn on_finalize(_n: BlockNumber) {
        MockPalletMaintenanceHooks::deposit_event(
            mock_pallet_maintenance_hooks::Event::MaintenanceOnFinalize,
        );
    }
}

impl OffchainWorker<BlockNumber> for MaintenanceHooks {
    fn offchain_worker(_n: BlockNumber) {
        MockPalletMaintenanceHooks::deposit_event(
            mock_pallet_maintenance_hooks::Event::MaintenanceOffchainWorker,
        );
    }
}

pub struct NormalHooks;

impl OnInitialize<BlockNumber> for NormalHooks {
    fn on_initialize(_n: BlockNumber) -> Weight {
        MockPalletMaintenanceHooks::deposit_event(
            mock_pallet_maintenance_hooks::Event::NormalOnInitialize,
        );
        Weight::zero()
    }
}

impl OnIdle<BlockNumber> for NormalHooks {
    fn on_idle(_n: BlockNumber, _max_weight: Weight) -> Weight {
        MockPalletMaintenanceHooks::deposit_event(
            mock_pallet_maintenance_hooks::Event::NormalOnIdle,
        );
        Weight::zero()
    }
}

impl OnRuntimeUpgrade for NormalHooks {
    fn on_runtime_upgrade() -> Weight {
        MockPalletMaintenanceHooks::deposit_event(
            mock_pallet_maintenance_hooks::Event::NormalOnRuntimeUpgrade,
        );
        Weight::zero()
    }
}

impl OnFinalize<BlockNumber> for NormalHooks {
    fn on_finalize(_n: BlockNumber) {
        MockPalletMaintenanceHooks::deposit_event(
            mock_pallet_maintenance_hooks::Event::NormalOnFinalize,
        );
    }
}

impl OffchainWorker<BlockNumber> for NormalHooks {
    fn offchain_worker(_n: BlockNumber) {
        MockPalletMaintenanceHooks::deposit_event(
            mock_pallet_maintenance_hooks::Event::NormalOffchainWorker,
        );
    }
}

impl Config for Test {
    type MaintenanceCallFilter = MaintenanceCallFilter;
    #[cfg(feature = "xcm-support")]
    type MaintenanceDmpHandler = MaintenanceDmpHandler;
    type MaintenanceExecutiveHooks = MaintenanceHooks;
    type MaintenanceOrigin = EnsureRoot<AccountId>;
    type NormalCallFilter = Everything;
    #[cfg(feature = "xcm-support")]
    type NormalDmpHandler = NormalDmpHandler;
    type NormalExecutiveHooks = NormalHooks;
    type RuntimeEvent = RuntimeEvent;
    #[cfg(feature = "xcm-support")]
    type XcmExecutionManager = ();
}

/// Externality builder for pallet maintenance mode's mock runtime
#[derive(Default)]
pub(crate) struct ExtBuilder {
    maintenance_mode: bool,
}

use sp_runtime::BuildStorage;
impl ExtBuilder {
    pub(crate) fn with_maintenance_mode(mut self, m: bool) -> Self {
        self.maintenance_mode = m;
        self
    }

    pub(crate) fn build(self) -> sp_io::TestExternalities {
        let mut t = frame_system::GenesisConfig::<Test>::default()
            .build_storage()
            .expect("Frame system builds valid default genesis config");

        pallet_maintenance_mode::GenesisConfig::<Test> {
            start_in_maintenance_mode: self.maintenance_mode,
            _config: Default::default(),
        }
        .assimilate_storage(&mut t)
        .expect("Pallet maintenance can be assimilated");

        let mut ext = sp_io::TestExternalities::new(t);
        ext.execute_with(|| System::set_block_number(1));
        ext
    }
}

pub(crate) fn events() -> Vec<pallet_maintenance_mode::Event> {
    System::events()
        .into_iter()
        .map(|r| r.event)
        .filter_map(|e| {
            if let RuntimeEvent::MaintenanceMode(inner) = e {
                Some(inner)
            } else {
                None
            }
        })
        .collect::<Vec<_>>()
}

pub(crate) fn mock_events() -> Vec<mock_pallet_maintenance_hooks::Event> {
    System::events()
        .into_iter()
        .map(|r| r.event)
        .filter_map(|e| {
            if let RuntimeEvent::MockPalletMaintenanceHooks(inner) = e {
                Some(inner)
            } else {
                None
            }
        })
        .collect::<Vec<_>>()
}
