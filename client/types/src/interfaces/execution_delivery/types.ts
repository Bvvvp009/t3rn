// Auto-generated via `yarn polkadot-types-from-defs`, do not edit
/* eslint-disable */

import type { Bytes, Enum, Option, Struct, Type, Vec, u32, u64, u8 } from '@polkadot/types';
import type { AccountId, BalanceOf, BlockNumber, Hash } from '@polkadot/types/interfaces/runtime';
import type { ITuple } from '@polkadot/types/types';
import type { GatewayPointer } from 't3rn-circuit-typegen/interfaces/primitives';
import type { Proof } from 't3rn-circuit-typegen/interfaces/snowfork';

/** @name AllowedSideEffect */
export interface AllowedSideEffect extends Bytes {}

/** @name GatewayOutboundEvent */
export interface GatewayOutboundEvent extends Struct {
  readonly id: GatewayOutboundEventId;
  readonly signature: Option<Bytes>;
  readonly namespace: Bytes;
  readonly name: Bytes;
  readonly data: Bytes;
  readonly proof: Option<Proof>;
  readonly args_abi: Vec<Type>;
  readonly args_encoded: Vec<Bytes>;
  readonly gateway_pointer: GatewayPointer;
}

/** @name GatewayOutboundEventId */
export interface GatewayOutboundEventId extends u64 {}

/** @name GenericDFD */
export interface GenericDFD extends Bytes {}

/** @name ProofType */
export interface ProofType extends Enum {
  readonly isFullValue: boolean;
  readonly isMerklePath: boolean;
}

/** @name result_status */
export interface result_status extends Bytes {}

/** @name SideEffectsDFD */
export interface SideEffectsDFD extends Bytes {}

/** @name StepConfirmation */
export interface StepConfirmation extends Struct {
  readonly step_index: u8;
  readonly value: Bytes;
  readonly proof: Proof;
  readonly outbound_event: GatewayOutboundEvent;
}

/** @name Xtx */
export interface Xtx extends Struct {
  readonly estimated_worth: BalanceOf;
  readonly current_worth: BalanceOf;
  readonly requester: AccountId;
  readonly escrow_account: AccountId;
  readonly payload: Bytes;
  readonly current_step: u32;
  readonly steps_no: u32;
  readonly current_phase: u32;
  readonly current_round: u32;
  readonly schedule: XtxSchedule;
  readonly phases_blockstamps: ITuple<[BlockNumber, BlockNumber]>;
}

/** @name XtxId */
export interface XtxId extends Hash {}

/** @name XtxSchedule */
export interface XtxSchedule extends Struct {
  readonly result_status: Bytes;
  readonly phases_blockstamps: ITuple<[BlockNumber, BlockNumber]>;
}

export type PHANTOM_EXECUTION_DELIVERY = 'execution_delivery';
