// Tests to be written here

// Loading .wasm files deps
use crate::{mock::*, CallFlags, CallStamp, Error, ExecutionProofs, ExecutionStamp};
use anyhow::{Context, Result};
use codec::Encode;
use frame_support::dispatch::DispatchError;
use frame_support::{assert_noop, assert_ok, storage::child, traits::Currency};
use sp_core::H256;
use sp_runtime::traits::Hash;
use sp_std::vec::Vec;
use std::path::Path;
use std::{fs, io::Read};
use t3rn_primitives::transfers::{BalanceOf, TransferEntry};
use versatile_wasm::{gas::Gas, runtime::get_child_storage_for_current_execution};

use sp_runtime::AccountId32;

///
/// Multistep Call - puts_code, instantiates, calls and terminates wasm contract codes on the fly.
/// Such a wasm code is called package.
/// Consists of 3 execution phases:
/// * Execute: Code results are stored on escrow account under corresponding to the call storage key.
/// * Revert:  Code results are removed out of escrow account.
/// * Commit:  Code results are moved from escrow account to target accounts.

const ZERO_ACCOUNT: AccountId32 = AccountId32::new([0u8; 32]);
const ESCROW_ACCOUNT: AccountId32 = AccountId32::new([1u8; 32]);
const TEMP_EXEC_CONTRACT: AccountId32 = AccountId32::new([2u8; 32]);
const REQUESTER: AccountId32 = AccountId32::new([3u8; 32]);
const TARGET_DEST: AccountId32 = AccountId32::new([4u8; 32]);
const OTHER_ACCOUNT: AccountId32 = AccountId32::new([5u8; 32]);

/**
 BASE GAS COSTS:
  - INSTANTIATE = 175 * 500_000
  - CALL = 135 * 500_000
  - total = 310 * 500_000 = 155_000_000
**/
fn default_multistep_call_args() -> (Option<CallFlags>, Vec<u8>, Vec<u8>, BalanceOf<Test>, Gas) {
    // let phase = 0 as u8;
    let phase = Some(CallFlags::ESCROWED_EXECUTE);
    let code: Vec<u8> = Vec::new();
    let input_data: Vec<u8> = Vec::new();
    let value = BalanceOf::<Test>::from(500_000 as u64);
    let gas_limit: Gas = 155_000_000 + 187_500_000 + 107_500_000 + 210_000; // Actual gas costs of "return_from_start_fn" instantiation cost
    return (phase, code, input_data, value, gas_limit);
}

#[test]
fn should_only_allow_to_be_called_by_escrow_account_being_sudo() {
    let (phase, code, input_data, value, gas_limit) = default_multistep_call_args();

    new_test_ext_builder(50, ESCROW_ACCOUNT).execute_with(|| {
        let _ = Balances::deposit_creating(&REQUESTER, 10_000_000_000);

        let err_rec = EscrowGateway::call(
            Origin::signed(OTHER_ACCOUNT),
            REQUESTER,
            TARGET_DEST,
            code,
            value,
            gas_limit,
            input_data,
            phase,
            None,
            None,
        );
        assert_noop!(err_rec, Error::<Test>::UnauthorizedCallAttempt);
    });
}

#[test]
fn during_execution_phase_when_given_empty_wasm_code_multistep_call_only_deferrs_transfer() {
    let (phase, _, input_data, value, gas_limit) = default_multistep_call_args();

    new_test_ext_builder(50, ESCROW_ACCOUNT).execute_with(|| {
        let _ = Balances::deposit_creating(&REQUESTER, 10_000_000_000);

        assert_ok!(EscrowGateway::call(
            Origin::signed(ESCROW_ACCOUNT),
            REQUESTER,
            TARGET_DEST,
            Vec::new(),
            value,
            gas_limit,
            input_data,
            phase,
            None,
            None,
        ));

        assert_eq!(
            EscrowGateway::deferred_transfers(&REQUESTER, &TARGET_DEST),
            [TransferEntry {
                to: H256::from_slice(&TARGET_DEST.encode()[..]),
                value: 500000,
                data: [].to_vec(),
            },]
        );
    });
}

#[test]
fn commit_phase_cannot_be_triggered_without_preceeding_execution() {
    let (_phase, _, input_data, value, _gas_limit) = default_multistep_call_args();
    let correct_wasm_path = Path::new(
        "../contracts-gateway/fixtures/32b-account-and-u64-balance/transfer_return_code.wasm",
    );
    let correct_wasm_code = load_contract_code(&correct_wasm_path).unwrap();
    // Set fees
    let sufficient_gas_limit = (170_000_000 + 17_500_000) as u64; // base (exact init costs) + exec_cost = 187_500_000
    let endowment = 100_000_000;
    let subsistence_threshold = 66;
    let inner_contract_transfer_value = 100;

    new_test_ext_builder(50, ESCROW_ACCOUNT).execute_with(|| {
        let _ = Balances::deposit_creating(
            &REQUESTER,
            sufficient_gas_limit
                + endowment
                + subsistence_threshold
                + (value)
                + inner_contract_transfer_value,
        );

        assert_noop!(
            EscrowGateway::call(
                Origin::signed(ESCROW_ACCOUNT),
                REQUESTER,
                TARGET_DEST,
                correct_wasm_code.clone(),
                value,
                sufficient_gas_limit,
                input_data.clone(),
                Some(CallFlags::ESCROWED_COMMIT),
                None,
                None,
            ),
            Error::<Test>::CommitOnlyPossibleAfterSuccessfulExecutionPhase
        );
    });
}

#[test]
fn should_succeed_for_return_from_fn() {
    let (phase, _, input_data, value, gas_limit) = default_multistep_call_args();
    let correct_wasm_path = Path::new(
        "../contracts-gateway/fixtures/32b-account-and-u64-balance/return_from_start_fn.wasm",
    );
    let correct_wasm_code = load_contract_code(&correct_wasm_path).unwrap();

    new_test_ext_builder(50, ESCROW_ACCOUNT).execute_with(|| {
        let _ = Balances::deposit_creating(&REQUESTER, 10_000_000_000);
        assert_ok!(EscrowGateway::call(
            Origin::signed(ESCROW_ACCOUNT),
            REQUESTER,
            TARGET_DEST,
            correct_wasm_code,
            value,
            gas_limit,
            input_data,
            phase,
            None,
            None,
        ));
    });
}

// Balance Specific

#[test]
fn fails_for_insufficient_gas_limit() {
    let (phase, _, input_data, value, _) = default_multistep_call_args();
    let correct_wasm_path = Path::new(
        "../contracts-gateway/fixtures/32b-account-and-u64-balance/transfer_return_code.wasm",
    );
    let correct_wasm_code = load_contract_code(&correct_wasm_path).unwrap();
    // Make the gas limit too little
    let gas_limit = 1000;

    new_test_ext_builder(50, ESCROW_ACCOUNT).execute_with(|| {
        let _ = Balances::deposit_creating(&REQUESTER, 10_000_000_000);
        let err = EscrowGateway::call(
            Origin::signed(ESCROW_ACCOUNT),
            REQUESTER,
            TARGET_DEST,
            correct_wasm_code,
            value,
            gas_limit,
            input_data,
            phase,
            None,
            None,
        );
        assert_noop!(err, DispatchError::Other("Out of gas"));
    });
}

#[test]
fn successful_execution_phase_when_given_correct_wasm_code_stores_correct_result() {
    let (phase, _, input_data, value, gas_limit) = default_multistep_call_args();
    let correct_wasm_path = Path::new(
        "../contracts-gateway/fixtures/32b-account-and-u64-balance/transfer_return_code.wasm",
    );
    let correct_wasm_code = load_contract_code(&correct_wasm_path).unwrap();

    new_test_ext_builder(50, ESCROW_ACCOUNT).execute_with(|| {
        let _ = Balances::deposit_creating(&REQUESTER, 10_000_000_000);

        assert_ok!(EscrowGateway::call(
            Origin::signed(ESCROW_ACCOUNT),
            REQUESTER,
            TARGET_DEST,
            correct_wasm_code.clone(),
            value,
            gas_limit,
            input_data,
            phase,
            None,
            None,
        ));

        // Expect return success execution code - 0.
        assert_eq!(
            EscrowGateway::deferred_results(
                &REQUESTER,
                &<Test as frame_system::Config>::Hashing::hash(&correct_wasm_code.clone())
            ),
            vec![0, 0, 0, 0],
        );
    });
}

#[test]
fn successful_execution_phase_generates_call_stamps_and_proofs() {
    let (phase, _, input_data, value, gas_limit) = default_multistep_call_args();
    let correct_wasm_path = Path::new(
        "../contracts-gateway/fixtures/32b-account-and-u64-balance/transfer_return_code.wasm",
    );
    let correct_wasm_code = load_contract_code(&correct_wasm_path).unwrap();

    new_test_ext_builder(50, ESCROW_ACCOUNT).execute_with(|| {
        let _ = Balances::deposit_creating(&REQUESTER, 10_000_000_000);

        assert_ok!(EscrowGateway::call(
            Origin::signed(ESCROW_ACCOUNT),
            REQUESTER,
            TARGET_DEST,
            correct_wasm_code.clone(),
            value,
            gas_limit,
            input_data,
            phase,
            None,
            None,
        ));

        assert_eq!(
            EscrowGateway::execution_stamps(
                &REQUESTER,
                &<Test as frame_system::Config>::Hashing::hash(&correct_wasm_code.clone())
            ),
            ExecutionStamp {
                timestamp: 0,
                phase: 0,
                proofs: Some(ExecutionProofs {
                    result: Some(vec![
                        17, 218, 109, 31, 118, 29, 223, 155, 219, 76, 157, 110, 83, 3, 235, 212,
                        31, 97, 133, 141, 10, 86, 71, 161, 167, 191, 224, 137, 191, 146, 27, 233
                    ]),
                    storage: Some(vec![
                        251, 157, 122, 148, 72, 142, 85, 179, 78, 9, 191, 10, 233, 122, 212, 27,
                        172, 57, 71, 192, 40, 9, 217, 136, 38, 77, 99, 3, 206, 138, 53, 31
                    ]),
                    deferred_transfers: vec![
                        TransferEntry {
                            to: H256::from_slice(&TARGET_DEST.encode()[..]),
                            value: 500000,
                            data: vec![]
                        },
                        TransferEntry {
                            to: H256::from_slice(&ZERO_ACCOUNT.encode()[..]),
                            value: 100,
                            data: vec![]
                        }
                    ]
                }),
                call_stamps: vec![CallStamp {
                    // Storage isn't changing.
                    pre_storage: vec![
                        3, 23, 10, 46, 117, 151, 183, 183, 227, 216, 76, 5, 57, 29, 19, 154, 98,
                        177, 87, 231, 135, 134, 216, 192, 130, 242, 157, 207, 76, 17, 19, 20
                    ],
                    post_storage: vec![
                        3, 23, 10, 46, 117, 151, 183, 183, 227, 216, 76, 5, 57, 29, 19, 154, 98,
                        177, 87, 231, 135, 134, 216, 192, 130, 242, 157, 207, 76, 17, 19, 20
                    ],
                    // Dest is set for escrow account.
                    dest: vec![
                        1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
                        1, 1, 1, 1, 1, 1, 1
                    ]
                }],
                failure: None
            }
        );

        assert_eq!(
            EscrowGateway::deferred_transfers(&REQUESTER, &TARGET_DEST),
            [
                TransferEntry {
                    to: H256::from_slice(&TARGET_DEST.encode()[..]),
                    value: 500000,
                    data: [].to_vec(),
                },
                TransferEntry {
                    to: H256::from_slice(&ZERO_ACCOUNT.encode()[..]),
                    value: 100,
                    data: [].to_vec(),
                }
            ]
        );
    });
}

/**
    TRANSFERS
**/
#[test]
fn transfer_during_execution_phase_succeeds_and_consumes_costs_correctly_and_deferrs_transfers() {
    let (phase, _, input_data, value, _gas_limit) = default_multistep_call_args();
    let correct_wasm_path = Path::new(
        "../contracts-gateway/fixtures/32b-account-and-u64-balance/transfer_return_code.wasm",
    );
    let correct_wasm_code = load_contract_code(&correct_wasm_path).unwrap();
    // Set fees
    let sufficient_gas_limit = 22_000_000 as u64; // exact gas costs
    let _endowment = 100_000_000;
    let subsistence_threshold = 66;
    let inner_contract_transfer_value = 100;

    new_test_ext_builder(50, ESCROW_ACCOUNT).execute_with(|| {
        let _ = Balances::deposit_creating(
            &REQUESTER,
            // Note, no endowment needed.
            sufficient_gas_limit + subsistence_threshold + (value) + inner_contract_transfer_value,
        );
        assert_ok!(EscrowGateway::call(
            Origin::signed(ESCROW_ACCOUNT),
            REQUESTER,
            TARGET_DEST,
            correct_wasm_code,
            value,
            sufficient_gas_limit,
            input_data,
            phase,
            None,
            None,
        ));

        // Escrow Account is now pre-charged by requester to cover:
        // 10_000_000 gas_fees + 500_000 requested balance transfer to &target_dest + 100 requested by contract value transfer to &0
        assert_eq!(Balances::total_balance(&ESCROW_ACCOUNT), 22_500_100);

        // Requester is only left with subsistence threshold
        assert_eq!(Balances::total_balance(&REQUESTER), subsistence_threshold);

        // Account of temporary execution contract is now charged with endowment
        assert_eq!(Balances::total_balance(&TEMP_EXEC_CONTRACT), 0);

        // Still nothing on target_dest account as it is only the execution phase.
        assert_eq!(Balances::total_balance(&TARGET_DEST), 0);

        // There should be an entry with deferred transfer to the target dest though as well as the requested by contract value transfer of 100 to &0
        assert_eq!(
            EscrowGateway::deferred_transfers(&REQUESTER, &TARGET_DEST),
            [
                TransferEntry {
                    to: H256::from_slice(&TARGET_DEST.encode()[..]),
                    value: 500000,
                    data: [].to_vec(),
                },
                TransferEntry {
                    to: H256::from_slice(&ZERO_ACCOUNT.encode()[..]),
                    value: 100,
                    data: [].to_vec(),
                }
            ]
        );
    });
}

#[test]
fn successful_commit_phase_transfers_move_from_deferred_to_target_destinations() {
    let (_phase, _, input_data, value, _gas_limit) = default_multistep_call_args();
    let correct_wasm_path = Path::new(
        "../contracts-gateway/fixtures/32b-account-and-u64-balance/transfer_return_code.wasm",
    );
    let correct_wasm_code = load_contract_code(&correct_wasm_path).unwrap();
    // Set fees
    let sufficient_gas_limit = 22_000_000 as u64; // exact gas limit
    let _endowment = 100_000_000;
    let subsistence_threshold = 66;
    let inner_contract_transfer_value = 100;

    new_test_ext_builder(subsistence_threshold, ESCROW_ACCOUNT).execute_with(|| {
        let _ = Balances::deposit_creating(
            &REQUESTER,
            sufficient_gas_limit + subsistence_threshold + (value) + inner_contract_transfer_value,
        );
        assert_ok!(EscrowGateway::call(
            Origin::signed(ESCROW_ACCOUNT),
            REQUESTER,
            TARGET_DEST,
            correct_wasm_code.clone(),
            value,
            sufficient_gas_limit,
            input_data.clone(),
            Some(CallFlags::ESCROWED_EXECUTE),
            None,
            None,
        ));

        // There should be an entry with deferred transfer to the target dest though as well as the requested by contract value transfer of 100 to &0
        assert_eq!(Balances::total_balance(&TARGET_DEST), 0);
        assert_eq!(Balances::total_balance(&ZERO_ACCOUNT), 0);
        assert_eq!(Balances::total_balance(&REQUESTER), subsistence_threshold); // 66
        assert_eq!(
            Balances::total_balance(&ESCROW_ACCOUNT),
            sufficient_gas_limit + inner_contract_transfer_value + value
        ); // 10000100

        assert_ok!(EscrowGateway::call(
            Origin::signed(ESCROW_ACCOUNT),
            REQUESTER,
            TARGET_DEST,
            correct_wasm_code.clone(),
            value,
            sufficient_gas_limit,
            input_data.clone(),
            Some(CallFlags::ESCROWED_COMMIT),
            None,
            None,
        ));

        assert_eq!(Balances::total_balance(&TARGET_DEST), 500_000);
        assert_eq!(Balances::total_balance(&ZERO_ACCOUNT), 100);
        assert_eq!(Balances::total_balance(&REQUESTER), subsistence_threshold); // 500166
        assert_eq!(
            Balances::total_balance(&ESCROW_ACCOUNT),
            sufficient_gas_limit
        ); // 10000000
    });
}

#[test]
fn successful_revert_phase_removes_deferred_transfers_and_refunds_from_escrow_to_requester() {
    let (phase, _, input_data, value, _gas_limit) = default_multistep_call_args();
    let correct_wasm_path = Path::new(
        "../contracts-gateway/fixtures/32b-account-and-u64-balance/transfer_return_code.wasm",
    );
    let correct_wasm_code = load_contract_code(&correct_wasm_path).unwrap();
    // Set fees
    let sufficient_gas_limit = 22_000_000 as u64; // exact gas costs
    let _endowment = 100_000_000;
    let subsistence_threshold = 66;
    let inner_contract_transfer_value = 100;

    new_test_ext_builder(subsistence_threshold, ESCROW_ACCOUNT).execute_with(|| {
        let _ = Balances::deposit_creating(
            &REQUESTER,
            sufficient_gas_limit + subsistence_threshold + (value) + inner_contract_transfer_value,
        );
        assert_ok!(EscrowGateway::call(
            Origin::signed(ESCROW_ACCOUNT),
            REQUESTER,
            TARGET_DEST,
            correct_wasm_code.clone(),
            value,
            sufficient_gas_limit,
            input_data.clone(),
            phase,
            None,
            None,
        ));
        // Expect return success execution code - 0.
        assert_eq!(
            EscrowGateway::deferred_results(
                &REQUESTER,
                &<Test as frame_system::Config>::Hashing::hash(&correct_wasm_code.clone())
            ),
            vec![0, 0, 0, 0],
        );

        assert_eq!(
            EscrowGateway::deferred_transfers(&REQUESTER, &TARGET_DEST),
            [
                TransferEntry {
                    to: H256::from_slice(&TARGET_DEST.encode()[..]),
                    value: 500000,
                    data: [].to_vec(),
                },
                TransferEntry {
                    to: H256::from_slice(&ZERO_ACCOUNT.encode()[..]),
                    value: 100,
                    data: [].to_vec(),
                }
            ]
        );

        // There should be an entry with deferred transfer to the target dest though as well as the requested by contract value transfer of 100 to &0
        assert_eq!(Balances::total_balance(&TARGET_DEST), 0);
        assert_eq!(Balances::total_balance(&ZERO_ACCOUNT), 0);
        assert_eq!(Balances::total_balance(&REQUESTER), subsistence_threshold); // 66
        assert_eq!(
            Balances::total_balance(&ESCROW_ACCOUNT),
            sufficient_gas_limit + inner_contract_transfer_value + value
        ); // 188000100

        assert_ok!(EscrowGateway::call(
            Origin::signed(ESCROW_ACCOUNT),
            REQUESTER,
            TARGET_DEST,
            correct_wasm_code.clone(),
            value,
            sufficient_gas_limit,
            input_data.clone(),
            Some(CallFlags::ESCROWED_REVERT),
            None,
            None,
        ));

        assert_eq!(
            EscrowGateway::deferred_results(
                &REQUESTER,
                &<Test as frame_system::Config>::Hashing::hash(&correct_wasm_code.clone())
            ),
            Vec::<u8>::new(),
        );

        assert_eq!(Balances::total_balance(&TARGET_DEST), 0);
        assert_eq!(Balances::total_balance(&ZERO_ACCOUNT), 0);
        assert_eq!(
            Balances::total_balance(&REQUESTER),
            subsistence_threshold + value + inner_contract_transfer_value
        ); // 500166
        assert_eq!(
            Balances::total_balance(&ESCROW_ACCOUNT),
            sufficient_gas_limit
        ); // 186999900
    });
}

#[test]
fn successful_revert_phase_removes_associated_storage_for_that_call() {
    let (phase, _, _input_data, value, _gas_limit) = default_multistep_call_args();
    let correct_wasm_path =
        Path::new("../contracts-gateway/fixtures/32b-account-and-u64-balance/storage_size.wasm");
    let correct_wasm_code = load_contract_code(&correct_wasm_path).unwrap();
    // Set fees
    let sufficient_gas_limit = (170_000_000 + 17_500_000) as u64; // base (exact init costs) + exec_cost = 187_500_000
    let _endowment = 100_000_000;
    let subsistence_threshold = 66;
    let inner_contract_transfer_value = 100;

    let empty_storage_at_dest_root: Vec<u8> = vec![
        3, 23, 10, 46, 117, 151, 183, 183, 227, 216, 76, 5, 57, 29, 19, 154, 98, 177, 87, 231, 135,
        134, 216, 192, 130, 242, 157, 207, 76, 17, 19, 20,
    ];

    new_test_ext_builder(50, ESCROW_ACCOUNT).execute_with(|| {
        let _ = Balances::deposit_creating(
            &REQUESTER,
            sufficient_gas_limit + subsistence_threshold + (value) + inner_contract_transfer_value,
        );
        assert_ok!(EscrowGateway::call(
            Origin::signed(ESCROW_ACCOUNT),
            REQUESTER,
            TARGET_DEST,
            correct_wasm_code.clone(),
            value,
            sufficient_gas_limit,
            Encode::encode(&17),
            phase,
            None,
            None,
        ));

        // After the execution phase changes should already be there for this particular entry for that code hash in the storage root.
        assert_ne!(
            child::root(&get_child_storage_for_current_execution::<Test>(
                &ESCROW_ACCOUNT,
                <Test as frame_system::Config>::Hashing::hash(&correct_wasm_code.clone())
            )),
            empty_storage_at_dest_root,
        );

        assert_ok!(EscrowGateway::call(
            Origin::signed(ESCROW_ACCOUNT),
            REQUESTER,
            TARGET_DEST,
            correct_wasm_code.clone(),
            value,
            sufficient_gas_limit,
            Encode::encode(&17),
            Some(CallFlags::ESCROWED_REVERT),
            None,
            None,
        ));

        assert_eq!(
            child::root(&get_child_storage_for_current_execution::<Test>(
                &ESCROW_ACCOUNT,
                <Test as frame_system::Config>::Hashing::hash(&correct_wasm_code.clone())
            )),
            empty_storage_at_dest_root,
        );

        assert_eq!(
            EscrowGateway::deferred_results(
                &REQUESTER,
                &<Test as frame_system::Config>::Hashing::hash(&correct_wasm_code.clone())
            ),
            Vec::<u8>::new(),
        );
    });
}

#[test]
fn successful_commit_phase_applies_storage_writes_on_the_dedicated_for_that_code_storage_tree() {
    let (phase, _, _input_data, value, _gas_limit) = default_multistep_call_args();
    let correct_wasm_path =
        Path::new("../contracts-gateway/fixtures/32b-account-and-u64-balance/storage_size.wasm");
    let correct_wasm_code = load_contract_code(&correct_wasm_path).unwrap();
    // Set fees
    let sufficient_gas_limit = (170_000_000 + 17_500_000) as u64; // base (exact init costs) + exec_cost = 187_500_000
    let endowment = 100_000_000;
    let subsistence_threshold = 66;
    let inner_contract_transfer_value = 100;
    let _empty_storage_at_dest_root: Vec<u8> = vec![
        3, 23, 10, 46, 117, 151, 183, 183, 227, 216, 76, 5, 57, 29, 19, 154, 98, 177, 87, 231, 135,
        134, 216, 192, 130, 242, 157, 207, 76, 17, 19, 20,
    ];

    new_test_ext_builder(50, ESCROW_ACCOUNT).execute_with(|| {
        let _ = Balances::deposit_creating(
            &REQUESTER,
            sufficient_gas_limit
                + endowment
                + subsistence_threshold
                + (value)
                + inner_contract_transfer_value,
        );
        assert_ok!(EscrowGateway::call(
            Origin::signed(ESCROW_ACCOUNT),
            REQUESTER,
            TARGET_DEST,
            correct_wasm_code.clone(),
            value,
            sufficient_gas_limit,
            Encode::encode(&17),
            phase,
            None,
            None,
        ));

        assert_eq!(
            EscrowGateway::execution_stamps(
                &REQUESTER,
                &<Test as frame_system::Config>::Hashing::hash(&correct_wasm_code.clone())
            ),
            ExecutionStamp {
                timestamp: 0,
                phase: 0,
                proofs: Some(ExecutionProofs {
                    result: Some(vec![
                        14, 87, 81, 192, 38, 229, 67, 178, 232, 171, 46, 176, 96, 153, 218, 161,
                        209, 229, 223, 71, 119, 143, 119, 135, 250, 171, 69, 205, 241, 47, 227,
                        168
                    ]),
                    storage: Some(vec![
                        178, 206, 244, 111, 248, 96, 88, 251, 76, 234, 134, 126, 69, 13, 41, 152,
                        172, 155, 45, 135, 194, 90, 7, 160, 253, 207, 71, 120, 217, 217, 169, 27
                    ]),
                    deferred_transfers: vec![TransferEntry {
                        to: H256::from_slice(&TARGET_DEST.encode()[..]),
                        value: 500000,
                        data: vec![]
                    }]
                }),
                call_stamps: vec![CallStamp {
                    pre_storage: vec![
                        3, 23, 10, 46, 117, 151, 183, 183, 227, 216, 76, 5, 57, 29, 19, 154, 98,
                        177, 87, 231, 135, 134, 216, 192, 130, 242, 157, 207, 76, 17, 19, 20
                    ],
                    // Post storage changes!
                    post_storage: vec![
                        239, 9, 11, 245, 184, 188, 16, 206, 229, 101, 254, 122, 124, 19, 195, 45,
                        136, 217, 68, 247, 139, 114, 81, 232, 168, 149, 76, 71, 229, 104, 207, 92
                    ],
                    dest: vec![
                        1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
                        1, 1, 1, 1, 1, 1, 1
                    ]
                }],
                failure: None
            }
        );

        // After the execution phase changes should already be there for this particular entry for that code hash in the storage root.
        assert_eq!(
            child::root(&get_child_storage_for_current_execution::<Test>(
                &ESCROW_ACCOUNT,
                <Test as frame_system::Config>::Hashing::hash(&correct_wasm_code.clone())
            )),
            EscrowGateway::execution_stamps(
                &REQUESTER,
                &<Test as frame_system::Config>::Hashing::hash(&correct_wasm_code.clone())
            )
            .call_stamps[0]
                .post_storage
        );
    });
}

//// Check calling custom host functions out of Flipper module.
#[test]
fn successfully_executes_flip_fn_from_host_runtime_module() {
    let (phase, _, input_data, value, _gas_limit) = default_multistep_call_args();
    let correct_wasm_path = Path::new(
        "../contracts-gateway/fixtures/32b-account-and-u64-balance/call_flipper_runtime.wasm",
    );
    let _correct_wasm_code = load_contract_code(&correct_wasm_path).unwrap();
    let correct_wasm_code = load_contract_code(&correct_wasm_path).unwrap();
    // Set fees
    let sufficient_gas_limit = (170_000_000 + 17_500_000) as u64; // base (exact init costs) + exec_cost = 187_500_000
    let _endowment = 100_000_000;
    let subsistence_threshold = 66;
    let inner_contract_transfer_value = 100;

    new_test_ext_builder(50, ESCROW_ACCOUNT).execute_with(|| {
        let _ = Balances::deposit_creating(
            &REQUESTER,
            sufficient_gas_limit + subsistence_threshold + (value) + inner_contract_transfer_value,
        );
        assert_ok!(EscrowGateway::call(
            Origin::signed(ESCROW_ACCOUNT),
            REQUESTER,
            TARGET_DEST,
            correct_wasm_code.clone(),
            value,
            sufficient_gas_limit,
            input_data,
            phase,
            None,
            None,
        ));
    });
}

#[test]
fn successfully_interacts_with_storage_runtime_module_and_is_billed_correctly() {
    let (phase, _, _input_data, value, _gas_limit) = default_multistep_call_args();
    let correct_wasm_path = Path::new(
        "../contracts-gateway/fixtures/32b-account-and-u64-balance/storage_runtime_calls.wasm",
    );
    let correct_wasm_code = load_contract_code(&correct_wasm_path).unwrap();
    // Set fees
    let sufficient_gas_limit = (170_000_000 + 17_500_000) as u64; // base (exact init costs) + exec_cost = 187_500_000
    let _endowment = 100_000_000;
    let subsistence_threshold = 66;
    let inner_contract_transfer_value = 100;

    new_test_ext_builder(50, ESCROW_ACCOUNT).execute_with(|| {
        let _ = Balances::deposit_creating(
            &REQUESTER,
            sufficient_gas_limit + subsistence_threshold + (value) + inner_contract_transfer_value,
        );
        assert_ok!(EscrowGateway::call(
            Origin::signed(ESCROW_ACCOUNT),
            REQUESTER,
            TARGET_DEST,
            correct_wasm_code.clone(),
            value,
            sufficient_gas_limit,
            Encode::encode(&17),
            phase,
            None,
            None,
        ));
        // Contract stores input value (17)
        assert_eq!(Weights::stored_value(), 17);
    });
}

#[test]
fn successfully_executes_runtime_storage_demo_is_billed_correctly() {
    let (phase, _, _input_data, _value, _gas_limit) = default_multistep_call_args();
    let correct_wasm_path = Path::new(
        "../contracts-gateway/fixtures/32b-account-and-u64-balance/storage_runtime_demo.wasm",
    );
    let correct_wasm_code = load_contract_code(&correct_wasm_path).unwrap();
    let exact_gas_cost = 377_030_230 as u64; // 377_030_230
    let _endowment = 100_000_000;
    let subsistence_threshold = 1; // 355_259_637_71

    new_test_ext_builder(subsistence_threshold, ESCROW_ACCOUNT).execute_with(|| {
        let _ = Balances::deposit_creating(&REQUESTER, (exact_gas_cost) + subsistence_threshold);
        assert_ok!(EscrowGateway::call(
            Origin::signed(ESCROW_ACCOUNT),
            REQUESTER,
            TARGET_DEST,
            correct_wasm_code.clone(),
            0 as u64,
            exact_gas_cost,
            Encode::encode(&17),
            phase,
            None,
            None,
        ));
        // Demo contract stores input value (17), then calls double (34)
        // then complex_calculations with y = 8 and x = 9 (X : (8 * 2) + Y : (9 ^ 2 + 34) = 18 + 115 = 131
        assert_eq!(Weights::stored_value(), 131);

        assert_eq!(Balances::total_balance(&REQUESTER), subsistence_threshold);
    });
}

#[test]
fn successfully_executes_runtime_storage_demo_and_refunds_gas_excess() {
    let (_phase, _, _input_data, _value, _gas_limit) = default_multistep_call_args();
    let correct_wasm_path = Path::new(
        "../contracts-gateway/fixtures/32b-account-and-u64-balance/storage_runtime_demo.wasm",
    );
    let correct_wasm_code = load_contract_code(&correct_wasm_path).unwrap();
    let exact_gas_cost = 377_030_230 as u64;
    let gas_excess = 100 as u64;
    let _endowment = 100_000_000;
    let subsistence_threshold = 1;

    new_test_ext_builder(subsistence_threshold, ESCROW_ACCOUNT).execute_with(|| {
        let _ = Balances::deposit_creating(
            &REQUESTER,
            exact_gas_cost + gas_excess + subsistence_threshold,
        );
        assert_ok!(EscrowGateway::call(
            Origin::signed(ESCROW_ACCOUNT),
            REQUESTER,
            TARGET_DEST,
            correct_wasm_code.clone(),
            0 as u64,
            exact_gas_cost + gas_excess,
            Encode::encode(&17),
            Some(CallFlags::ESCROWED_EXECUTE),
            None,
            None,
        ));

        assert_eq!(
            Balances::total_balance(&REQUESTER),
            gas_excess + subsistence_threshold
        );
    });
}

/// Load the wasm blob from the specified path.
///
/// Defaults to the target contract wasm in the current project, inferred via the crate metadata.
fn load_contract_code(path: &Path) -> Result<Vec<u8>> {
    let mut data = Vec::new();
    let mut file = fs::File::open(path).context(format!("Failed to open {}", path.display()))?;
    file.read_to_end(&mut data)?;

    Ok(data)
}
