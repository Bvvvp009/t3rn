// Auto-generated via `yarn polkadot-types-from-defs`, do not edit
/* eslint-disable */

import type { Bytes, Option, Struct, Vec, u64 } from '@polkadot/types';
import type { ChainId } from '@polkadot/types/interfaces/bridges';
import type { AccountId, Hash } from '@polkadot/types/interfaces/runtime';
import type { GatewayABIConfig, GatewayGenesisConfig, GatewaySysProps, GatewayType, GatewayVendor } from 't3rn-circuit-typegen/interfaces/primitives';

/** @name FetchXdnsRecordsResponse */
export interface FetchXdnsRecordsResponse extends Struct {
  readonly xdns_records: Vec<XdnsRecord>;
}

/** @name XdnsRecord */
export interface XdnsRecord extends Struct {
  readonly url: Bytes;
  readonly gateway_abi: GatewayABIConfig;
  readonly gateway_genesis: GatewayGenesisConfig;
  readonly gateway_vendor: GatewayVendor;
  readonly gateway_type: GatewayType;
  readonly gateway_id: ChainId;
  readonly gateway_sys_props: GatewaySysProps;
  readonly registrant: Option<AccountId>;
  readonly last_finalized: Option<u64>;
}

/** @name XdnsRecordId */
export interface XdnsRecordId extends Hash {}

export type PHANTOM_XDNS = 'xdns';
