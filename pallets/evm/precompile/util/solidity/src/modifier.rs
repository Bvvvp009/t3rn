// Copyright 2019-2022 PureStake Inc.
// This file is part Utils package, originally developed by PureStake

// Utils is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Utils is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Utils.  If not, see <http://www.gnu.org/licenses/>.

//! Provide checks related to function modifiers (view/payable).
use crate::EvmResult;
use pallet_3vm_evm_primitives::{
    Context, ExitError, ExitRevert, PrecompileFailure, PrecompileResult,
};
use sp_core::U256;
use sp_std::prelude::ToOwned;

/// Represents modifiers a Solidity function can be annotated with.
#[derive(Copy, Clone, PartialEq, Eq)]
pub enum FunctionModifier {
    /// Function that doesn't modify the state.
    View,
    /// Function that modifies the state but refuse receiving funds.
    /// Correspond to a Solidity function with no modifiers.
    NonPayable,
    /// Function that modifies the state and accept funds.
    Payable,
}

#[must_use]
/// Check that a function call is compatible with the context it is
/// called into.
pub fn check_function_modifier(
    context: &Context,
    is_static: bool,
    modifier: FunctionModifier,
) -> EvmResult {
    if is_static && modifier != FunctionModifier::View {
        return Err(PrecompileFailure::Revert {
            exit_status: ExitRevert::Reverted,
            output: "Can't call non-static function in static context"
                .as_bytes()
                .to_owned(),
        })
    }

    if modifier != FunctionModifier::Payable && context.apparent_value > U256::zero() {
        return Err(PrecompileFailure::Revert {
            exit_status: ExitRevert::Reverted,
            output: "Function is not payable".as_bytes().to_owned(),
        })
    }

    Ok(())
}
