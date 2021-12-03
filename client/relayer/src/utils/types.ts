import { ApiPromise } from '@polkadot/api';
import { SideEffect } from './../../../types/src/interfaces/primitives/types';
import { XtxId } from './../../../types/src/interfaces/execution_delivery/types';
import type { Hash, AccountId } from '@polkadot/types/interfaces/runtime';
import type { Vec, Compact, u128 } from '@polkadot/types';
import type { AnyNumber } from '@polkadot/types/types';
import events from 'events';

export interface TransactionResult {
  blockHash: Hash;
  status: boolean;
}

export interface NewSideEffectsAvailableEvent {
  requester: AccountId,
  xtx_id: XtxId,
  sideEffects: Vec<SideEffect>
}

export declare interface Emitter {
  on(event: 'NewSideEffect', listener: (payload: NewSideEffectsAvailableEvent, api: ApiPromise) => void): this;
}

export class Emitter extends events.EventEmitter {
  emitSideEffect(payload: NewSideEffectsAvailableEvent, api: ApiPromise): void {
    this.emit('NewSideEffect', payload, api);
  }
}

// there is no transfer exposed which also takes from parameter. Hmm interesting.
// then how will it work if the relayer signs the tx.
// I dont know.
export interface TransferArguments {
  to: AccountId,
  from: AccountId,
  amount: Compact<u128> | AnyNumber | Uint8Array
}