# LayerZero audit details
- Total Prize Pool: $101,000 in USDC
    - HM awards: up to $89,760 in USDC
        - If no valid Highs or Mediums are found, the HM pool is $0
    - QA awards: $3,740 in USDC
    - Judge awards: $7,000 in USDC
    - Scout awards: $500 USDC
- [Read our guidelines for more details](https://docs.code4rena.com/competitions)
- Starts April 1, 2026 20:00 UTC
- Ends April 14, 2026 20:00 UTC

### ❗ Important notes for wardens
1. Judging phase risk adjustments (upgrades/downgrades):
    - High- or Medium-risk submissions downgraded by the judge to Low-risk (QA) will be ineligible for awards.
    - Upgrading a Low-risk finding from a QA report to a Medium- or High-risk finding is not supported.
    - As such, wardens are encouraged to select the appropriate risk level carefully during the submission phase.

## Publicly known issues

_Anything included in this section is considered a publicly known issue and is therefore ineligible for awards._

- [Pending list of informational issues]

# Overview

The LayerZero V2 on Stellar implementation enables cross-chain messaging between Stellar and other blockchains. Built on Soroban (Stellar's smart contract platform) using Rust, it follows a modular, plugin-based architecture that maintains compatibility with LayerZero's V2 protocol design while adapting to Stellar's unique characteristics.

The protocol preserves LayerZero's core values of permissionlessness, immutability, and censorship-resistance.

The Stellar LayerZero implementation maintains the same four-step messaging flow as other LayerZero V2 implementations:

- **Send**: OApp calls EndpointV2 with message parameters
- **Verify**: DVNs verify messages and submit their validation to the message lib
- **Commit**: Via a permissionless call, the message library asserts commitment requirements and commits payload hash to the endpoint
- **Execute**: Via a pull model, the OApp calls endpoint.clear to receive the verified message


Stellar-Specific Design Considerations
The implementation addresses four major Stellar/Soroban constraints:

- **bytes32 Address Format**: Stellar uses variable-length addresses while LayerZero V2 uses fixed bytes32. LayerZero treats all OApp addresses as contract addresses.
- **TTL-Based Storage**: Soroban storage entries have TTL (Time-To-Live). The protocol uses a hybrid extension strategy to ensure critical state is never lost.
- **Storage Read Limits**: Soroban limits reads to 200 per transaction. The protocol uses eager inbound nonce tracking with pending nonce lists to stay within limits.
- **No Reentrancy**: Soroban prohibits reentrancy. The implementation uses Abstract Account patterns for DVN and Executor, and adopts pull-mode message delivery to support the ABA pattern (where the executor appears twice in the call stack).

## Links

- **Previous audits:** Reports are still in the draft phase and cannot be shared at the moment.
- **Documentation:** [`contracts/protocol/stellar/docs`](https://github.com/code-423n4/2026-04-layerzero/tree/main/contracts/protocol/stellar/docs)
- **Website:** https://layerzero.network/
- **X/Twitter:** https://x.com/LayerZero_Core

---

# Scope

### Files in scope

| Contract | SLoC |
| --- | --- |
| [contracts/protocol/stellar/contracts/common-macros/src/storage.rs](https://github.com/code-423n4/2026-04-layerzero/blob/main/contracts/protocol/stellar/contracts/common-macros/src/storage.rs) | 368 |
| [contracts/protocol/stellar/contracts/common-macros/src/upgradeable.rs](https://github.com/code-423n4/2026-04-layerzero/blob/main/contracts/protocol/stellar/contracts/common-macros/src/upgradeable.rs) | 82 |
| [contracts/protocol/stellar/contracts/common-macros/src/lz_contract.rs](https://github.com/code-423n4/2026-04-layerzero/blob/main/contracts/protocol/stellar/contracts/common-macros/src/lz_contract.rs) | 70 |
| [contracts/protocol/stellar/contracts/common-macros/src/lib.rs](https://github.com/code-423n4/2026-04-layerzero/blob/main/contracts/protocol/stellar/contracts/common-macros/src/lib.rs) | 65 |
| [contracts/protocol/stellar/contracts/common-macros/src/rbac.rs](https://github.com/code-423n4/2026-04-layerzero/blob/main/contracts/protocol/stellar/contracts/common-macros/src/rbac.rs) | 65 |
| [contracts/protocol/stellar/contracts/common-macros/src/contract_ttl.rs](https://github.com/code-423n4/2026-04-layerzero/blob/main/contracts/protocol/stellar/contracts/common-macros/src/contract_ttl.rs) | 60 |
| [contracts/protocol/stellar/contracts/common-macros/src/auth.rs](https://github.com/code-423n4/2026-04-layerzero/blob/main/contracts/protocol/stellar/contracts/common-macros/src/auth.rs) | 44 |
| [contracts/protocol/stellar/contracts/common-macros/src/utils.rs](https://github.com/code-423n4/2026-04-layerzero/blob/main/contracts/protocol/stellar/contracts/common-macros/src/utils.rs) | 40 |
| [contracts/protocol/stellar/contracts/common-macros/src/error.rs](https://github.com/code-423n4/2026-04-layerzero/blob/main/contracts/protocol/stellar/contracts/common-macros/src/error.rs) | 31 |
| [contracts/protocol/stellar/contracts/common-macros/src/ttl_configurable.rs](https://github.com/code-423n4/2026-04-layerzero/blob/main/contracts/protocol/stellar/contracts/common-macros/src/ttl_configurable.rs) | 13 |
| [contracts/protocol/stellar/contracts/common-macros/src/ttl_extendable.rs](https://github.com/code-423n4/2026-04-layerzero/blob/main/contracts/protocol/stellar/contracts/common-macros/src/ttl_extendable.rs) | 13 |
| [contracts/protocol/stellar/contracts/endpoint-v2/src/endpoint_v2.rs](https://github.com/code-423n4/2026-04-layerzero/blob/main/contracts/protocol/stellar/contracts/endpoint-v2/src/endpoint_v2.rs) | 238 |
| [contracts/protocol/stellar/contracts/endpoint-v2/src/message_lib_manager.rs](https://github.com/code-423n4/2026-04-layerzero/blob/main/contracts/protocol/stellar/contracts/endpoint-v2/src/message_lib_manager.rs) | 234 |
| [contracts/protocol/stellar/contracts/endpoint-v2/src/messaging_channel.rs](https://github.com/code-423n4/2026-04-layerzero/blob/main/contracts/protocol/stellar/contracts/endpoint-v2/src/messaging_channel.rs) | 214 |
| [contracts/protocol/stellar/contracts/endpoint-v2/src/events.rs](https://github.com/code-423n4/2026-04-layerzero/blob/main/contracts/protocol/stellar/contracts/endpoint-v2/src/events.rs) | 190 |
| [contracts/protocol/stellar/contracts/endpoint-v2/src/messaging_composer.rs](https://github.com/code-423n4/2026-04-layerzero/blob/main/contracts/protocol/stellar/contracts/endpoint-v2/src/messaging_composer.rs) | 72 |
| [contracts/protocol/stellar/contracts/endpoint-v2/src/interfaces/message_lib_manager.rs](https://github.com/code-423n4/2026-04-layerzero/blob/main/contracts/protocol/stellar/contracts/endpoint-v2/src/interfaces/message_lib_manager.rs) | 65 |
| [contracts/protocol/stellar/contracts/endpoint-v2/src/interfaces/layerzero_endpoint_v2.rs](https://github.com/code-423n4/2026-04-layerzero/blob/main/contracts/protocol/stellar/contracts/endpoint-v2/src/interfaces/layerzero_endpoint_v2.rs) | 58 |
| [contracts/protocol/stellar/contracts/endpoint-v2/src/storage.rs](https://github.com/code-423n4/2026-04-layerzero/blob/main/contracts/protocol/stellar/contracts/endpoint-v2/src/storage.rs) | 46 |
| [contracts/protocol/stellar/contracts/endpoint-v2/src/interfaces/messaging_channel.rs](https://github.com/code-423n4/2026-04-layerzero/blob/main/contracts/protocol/stellar/contracts/endpoint-v2/src/interfaces/messaging_channel.rs) | 34 |
| [contracts/protocol/stellar/contracts/endpoint-v2/src/interfaces/send_lib.rs](https://github.com/code-423n4/2026-04-layerzero/blob/main/contracts/protocol/stellar/contracts/endpoint-v2/src/interfaces/send_lib.rs) | 31 |
| [contracts/protocol/stellar/contracts/endpoint-v2/src/errors.rs](https://github.com/code-423n4/2026-04-layerzero/blob/main/contracts/protocol/stellar/contracts/endpoint-v2/src/errors.rs) | 29 |
| [contracts/protocol/stellar/contracts/endpoint-v2/src/util.rs](https://github.com/code-423n4/2026-04-layerzero/blob/main/contracts/protocol/stellar/contracts/endpoint-v2/src/util.rs) | 28 |
| [contracts/protocol/stellar/contracts/endpoint-v2/src/interfaces/message_lib.rs](https://github.com/code-423n4/2026-04-layerzero/blob/main/contracts/protocol/stellar/contracts/endpoint-v2/src/interfaces/message_lib.rs) | 25 |
| [contracts/protocol/stellar/contracts/endpoint-v2/src/interfaces/messaging_composer.rs](https://github.com/code-423n4/2026-04-layerzero/blob/main/contracts/protocol/stellar/contracts/endpoint-v2/src/interfaces/messaging_composer.rs) | 20 |
| [contracts/protocol/stellar/contracts/endpoint-v2/src/interfaces/layerzero_receiver.rs](https://github.com/code-423n4/2026-04-layerzero/blob/main/contracts/protocol/stellar/contracts/endpoint-v2/src/interfaces/layerzero_receiver.rs) | 16 |
| [contracts/protocol/stellar/contracts/endpoint-v2/src/interfaces/mod.rs](https://github.com/code-423n4/2026-04-layerzero/blob/main/contracts/protocol/stellar/contracts/endpoint-v2/src/interfaces/mod.rs) | 16 |
| [contracts/protocol/stellar/contracts/endpoint-v2/src/lib.rs](https://github.com/code-423n4/2026-04-layerzero/blob/main/contracts/protocol/stellar/contracts/endpoint-v2/src/lib.rs) | 16 |
| [contracts/protocol/stellar/contracts/endpoint-v2/src/interfaces/layerzero_composer.rs](https://github.com/code-423n4/2026-04-layerzero/blob/main/contracts/protocol/stellar/contracts/endpoint-v2/src/interfaces/layerzero_composer.rs) | 14 |
| [contracts/protocol/stellar/contracts/message-libs/blocked-message-lib/src/lib.rs](https://github.com/code-423n4/2026-04-layerzero/blob/main/contracts/protocol/stellar/contracts/message-libs/blocked-message-lib/src/lib.rs) | 42 |
| [contracts/protocol/stellar/contracts/message-libs/message-lib-common/src/worker_options.rs](https://github.com/code-423n4/2026-04-layerzero/blob/main/contracts/protocol/stellar/contracts/message-libs/message-lib-common/src/worker_options.rs) | 115 |
| [contracts/protocol/stellar/contracts/message-libs/message-lib-common/src/packet_codec_v1.rs](https://github.com/code-423n4/2026-04-layerzero/blob/main/contracts/protocol/stellar/contracts/message-libs/message-lib-common/src/packet_codec_v1.rs) | 48 |
| [contracts/protocol/stellar/contracts/message-libs/message-lib-common/src/interfaces/dvn.rs](https://github.com/code-423n4/2026-04-layerzero/blob/main/contracts/protocol/stellar/contracts/message-libs/message-lib-common/src/interfaces/dvn.rs) | 25 |
| [contracts/protocol/stellar/contracts/message-libs/message-lib-common/src/interfaces/executor.rs](https://github.com/code-423n4/2026-04-layerzero/blob/main/contracts/protocol/stellar/contracts/message-libs/message-lib-common/src/interfaces/executor.rs) | 21 |
| [contracts/protocol/stellar/contracts/message-libs/message-lib-common/src/errors.rs](https://github.com/code-423n4/2026-04-layerzero/blob/main/contracts/protocol/stellar/contracts/message-libs/message-lib-common/src/errors.rs) | 18 |
| [contracts/protocol/stellar/contracts/message-libs/message-lib-common/src/lib.rs](https://github.com/code-423n4/2026-04-layerzero/blob/main/contracts/protocol/stellar/contracts/message-libs/message-lib-common/src/lib.rs) | 9 |
| [contracts/protocol/stellar/contracts/message-libs/message-lib-common/src/interfaces/mod.rs](https://github.com/code-423n4/2026-04-layerzero/blob/main/contracts/protocol/stellar/contracts/message-libs/message-lib-common/src/interfaces/mod.rs) | 6 |
| [contracts/protocol/stellar/contracts/message-libs/message-lib-common/src/interfaces/treasury.rs](https://github.com/code-423n4/2026-04-layerzero/blob/main/contracts/protocol/stellar/contracts/message-libs/message-lib-common/src/interfaces/treasury.rs) | 5 |
| [contracts/protocol/stellar/contracts/message-libs/treasury/src/treasury.rs](https://github.com/code-423n4/2026-04-layerzero/blob/main/contracts/protocol/stellar/contracts/message-libs/treasury/src/treasury.rs) | 73 |
| [contracts/protocol/stellar/contracts/message-libs/treasury/src/events.rs](https://github.com/code-423n4/2026-04-layerzero/blob/main/contracts/protocol/stellar/contracts/message-libs/treasury/src/events.rs) | 23 |
| [contracts/protocol/stellar/contracts/message-libs/treasury/src/lib.rs](https://github.com/code-423n4/2026-04-layerzero/blob/main/contracts/protocol/stellar/contracts/message-libs/treasury/src/lib.rs) | 15 |
| [contracts/protocol/stellar/contracts/message-libs/treasury/src/storage.rs](https://github.com/code-423n4/2026-04-layerzero/blob/main/contracts/protocol/stellar/contracts/message-libs/treasury/src/storage.rs) | 13 |
| [contracts/protocol/stellar/contracts/message-libs/treasury/src/errors.rs](https://github.com/code-423n4/2026-04-layerzero/blob/main/contracts/protocol/stellar/contracts/message-libs/treasury/src/errors.rs) | 8 |
| [contracts/protocol/stellar/contracts/message-libs/treasury/src/interfaces/zro_fee_lib.rs](https://github.com/code-423n4/2026-04-layerzero/blob/main/contracts/protocol/stellar/contracts/message-libs/treasury/src/interfaces/zro_fee_lib.rs) | 5 |
| [contracts/protocol/stellar/contracts/message-libs/treasury/src/interfaces/mod.rs](https://github.com/code-423n4/2026-04-layerzero/blob/main/contracts/protocol/stellar/contracts/message-libs/treasury/src/interfaces/mod.rs) | 2 |
| [contracts/protocol/stellar/contracts/message-libs/uln-302/src/send_uln.rs](https://github.com/code-423n4/2026-04-layerzero/blob/main/contracts/protocol/stellar/contracts/message-libs/uln-302/src/send_uln.rs) | 264 |
| [contracts/protocol/stellar/contracts/message-libs/uln-302/src/types.rs](https://github.com/code-423n4/2026-04-layerzero/blob/main/contracts/protocol/stellar/contracts/message-libs/uln-302/src/types.rs) | 148 |
| [contracts/protocol/stellar/contracts/message-libs/uln-302/src/receive_uln.rs](https://github.com/code-423n4/2026-04-layerzero/blob/main/contracts/protocol/stellar/contracts/message-libs/uln-302/src/receive_uln.rs) | 117 |
| [contracts/protocol/stellar/contracts/message-libs/uln-302/src/uln302.rs](https://github.com/code-423n4/2026-04-layerzero/blob/main/contracts/protocol/stellar/contracts/message-libs/uln-302/src/uln302.rs) | 76 |
| [contracts/protocol/stellar/contracts/message-libs/uln-302/src/events.rs](https://github.com/code-423n4/2026-04-layerzero/blob/main/contracts/protocol/stellar/contracts/message-libs/uln-302/src/events.rs) | 74 |
| [contracts/protocol/stellar/contracts/message-libs/uln-302/src/storage.rs](https://github.com/code-423n4/2026-04-layerzero/blob/main/contracts/protocol/stellar/contracts/message-libs/uln-302/src/storage.rs) | 27 |
| [contracts/protocol/stellar/contracts/message-libs/uln-302/src/errors.rs](https://github.com/code-423n4/2026-04-layerzero/blob/main/contracts/protocol/stellar/contracts/message-libs/uln-302/src/errors.rs) | 25 |
| [contracts/protocol/stellar/contracts/message-libs/uln-302/src/interfaces/send_uln.rs](https://github.com/code-423n4/2026-04-layerzero/blob/main/contracts/protocol/stellar/contracts/message-libs/uln-302/src/interfaces/send_uln.rs) | 18 |
| [contracts/protocol/stellar/contracts/message-libs/uln-302/src/lib.rs](https://github.com/code-423n4/2026-04-layerzero/blob/main/contracts/protocol/stellar/contracts/message-libs/uln-302/src/lib.rs) | 17 |
| [contracts/protocol/stellar/contracts/message-libs/uln-302/src/interfaces/receive_uln.rs](https://github.com/code-423n4/2026-04-layerzero/blob/main/contracts/protocol/stellar/contracts/message-libs/uln-302/src/interfaces/receive_uln.rs) | 13 |
| [contracts/protocol/stellar/contracts/message-libs/uln-302/src/interfaces/mod.rs](https://github.com/code-423n4/2026-04-layerzero/blob/main/contracts/protocol/stellar/contracts/message-libs/uln-302/src/interfaces/mod.rs) | 4 |
| [contracts/protocol/stellar/contracts/utils/src/rbac.rs](https://github.com/code-423n4/2026-04-layerzero/blob/main/contracts/protocol/stellar/contracts/utils/src/rbac.rs) | 170 |
| [contracts/protocol/stellar/contracts/utils/src/multisig.rs](https://github.com/code-423n4/2026-04-layerzero/blob/main/contracts/protocol/stellar/contracts/utils/src/multisig.rs) | 136 |
| [contracts/protocol/stellar/contracts/utils/src/buffer_reader.rs](https://github.com/code-423n4/2026-04-layerzero/blob/main/contracts/protocol/stellar/contracts/utils/src/buffer_reader.rs) | 121 |
| [contracts/protocol/stellar/contracts/utils/src/ownable.rs](https://github.com/code-423n4/2026-04-layerzero/blob/main/contracts/protocol/stellar/contracts/utils/src/ownable.rs) | 92 |
| [contracts/protocol/stellar/contracts/utils/src/ttl_configurable.rs](https://github.com/code-423n4/2026-04-layerzero/blob/main/contracts/protocol/stellar/contracts/utils/src/ttl_configurable.rs) | 84 |
| [contracts/protocol/stellar/contracts/utils/src/buffer_writer.rs](https://github.com/code-423n4/2026-04-layerzero/blob/main/contracts/protocol/stellar/contracts/utils/src/buffer_writer.rs) | 76 |
| [contracts/protocol/stellar/contracts/utils/src/errors.rs](https://github.com/code-423n4/2026-04-layerzero/blob/main/contracts/protocol/stellar/contracts/utils/src/errors.rs) | 61 |
| [contracts/protocol/stellar/contracts/utils/src/upgradeable.rs](https://github.com/code-423n4/2026-04-layerzero/blob/main/contracts/protocol/stellar/contracts/utils/src/upgradeable.rs) | 48 |
| [contracts/protocol/stellar/contracts/utils/src/lib.rs](https://github.com/code-423n4/2026-04-layerzero/blob/main/contracts/protocol/stellar/contracts/utils/src/lib.rs) | 17 |
| [contracts/protocol/stellar/contracts/utils/src/option_ext.rs](https://github.com/code-423n4/2026-04-layerzero/blob/main/contracts/protocol/stellar/contracts/utils/src/option_ext.rs) | 17 |
| [contracts/protocol/stellar/contracts/utils/src/auth.rs](https://github.com/code-423n4/2026-04-layerzero/blob/main/contracts/protocol/stellar/contracts/utils/src/auth.rs) | 14 |
| [contracts/protocol/stellar/contracts/utils/src/bytes_ext.rs](https://github.com/code-423n4/2026-04-layerzero/blob/main/contracts/protocol/stellar/contracts/utils/src/bytes_ext.rs) | 13 |
| [contracts/protocol/stellar/contracts/utils/src/ttl_extendable.rs](https://github.com/code-423n4/2026-04-layerzero/blob/main/contracts/protocol/stellar/contracts/utils/src/ttl_extendable.rs) | 6 |
| [contracts/protocol/stellar/contracts/workers/dvn/src/dvn.rs](https://github.com/code-423n4/2026-04-layerzero/blob/main/contracts/protocol/stellar/contracts/workers/dvn/src/dvn.rs) | 133 |
| [contracts/protocol/stellar/contracts/workers/dvn/src/auth.rs](https://github.com/code-423n4/2026-04-layerzero/blob/main/contracts/protocol/stellar/contracts/workers/dvn/src/auth.rs) | 96 |
| [contracts/protocol/stellar/contracts/workers/dvn/src/interfaces/dvn.rs](https://github.com/code-423n4/2026-04-layerzero/blob/main/contracts/protocol/stellar/contracts/workers/dvn/src/interfaces/dvn.rs) | 48 |
| [contracts/protocol/stellar/contracts/workers/dvn/src/lib.rs](https://github.com/code-423n4/2026-04-layerzero/blob/main/contracts/protocol/stellar/contracts/workers/dvn/src/lib.rs) | 17 |
| [contracts/protocol/stellar/contracts/workers/dvn/src/storage.rs](https://github.com/code-423n4/2026-04-layerzero/blob/main/contracts/protocol/stellar/contracts/workers/dvn/src/storage.rs) | 15 |
| [contracts/protocol/stellar/contracts/workers/dvn/src/errors.rs](https://github.com/code-423n4/2026-04-layerzero/blob/main/contracts/protocol/stellar/contracts/workers/dvn/src/errors.rs) | 13 |
| [contracts/protocol/stellar/contracts/workers/dvn/src/events.rs](https://github.com/code-423n4/2026-04-layerzero/blob/main/contracts/protocol/stellar/contracts/workers/dvn/src/events.rs) | 12 |
| [contracts/protocol/stellar/contracts/workers/dvn/src/interfaces/mod.rs](https://github.com/code-423n4/2026-04-layerzero/blob/main/contracts/protocol/stellar/contracts/workers/dvn/src/interfaces/mod.rs) | 2 |
| [contracts/protocol/stellar/contracts/workers/fee-lib-interfaces/src/price_feed.rs](https://github.com/code-423n4/2026-04-layerzero/blob/main/contracts/protocol/stellar/contracts/workers/fee-lib-interfaces/src/price_feed.rs) | 23 |
| [contracts/protocol/stellar/contracts/workers/fee-lib-interfaces/src/executor_fee_lib.rs](https://github.com/code-423n4/2026-04-layerzero/blob/main/contracts/protocol/stellar/contracts/workers/fee-lib-interfaces/src/executor_fee_lib.rs) | 21 |
| [contracts/protocol/stellar/contracts/workers/fee-lib-interfaces/src/dvn_fee_lib.rs](https://github.com/code-423n4/2026-04-layerzero/blob/main/contracts/protocol/stellar/contracts/workers/fee-lib-interfaces/src/dvn_fee_lib.rs) | 20 |
| [contracts/protocol/stellar/contracts/workers/fee-lib-interfaces/src/lib.rs](https://github.com/code-423n4/2026-04-layerzero/blob/main/contracts/protocol/stellar/contracts/workers/fee-lib-interfaces/src/lib.rs) | 7 |
| [contracts/protocol/stellar/contracts/workers/worker/src/worker.rs](https://github.com/code-423n4/2026-04-layerzero/blob/main/contracts/protocol/stellar/contracts/workers/worker/src/worker.rs) | 210 |
| [contracts/protocol/stellar/contracts/workers/worker/src/events.rs](https://github.com/code-423n4/2026-04-layerzero/blob/main/contracts/protocol/stellar/contracts/workers/worker/src/events.rs) | 61 |
| [contracts/protocol/stellar/contracts/workers/worker/src/storage.rs](https://github.com/code-423n4/2026-04-layerzero/blob/main/contracts/protocol/stellar/contracts/workers/worker/src/storage.rs) | 32 |
| [contracts/protocol/stellar/contracts/workers/worker/src/errors.rs](https://github.com/code-423n4/2026-04-layerzero/blob/main/contracts/protocol/stellar/contracts/workers/worker/src/errors.rs) | 20 |
| [contracts/protocol/stellar/contracts/workers/worker/src/lib.rs](https://github.com/code-423n4/2026-04-layerzero/blob/main/contracts/protocol/stellar/contracts/workers/worker/src/lib.rs) | 9 |
| **Total** | **5,002** |

*For a machine-readable version, see [scope.txt](https://github.com/code-423n4/2026-04-layerzero/blob/main/scope.txt)*

### Files out of scope

| File/Directory | File Count |
| --- | --- |
| All test files in in-scope directories (`**/tests/*`, `**/integration_tests/*`, `**/integration-tests/*`) | 170 |
| [contracts/protocol/stellar/contracts/oapps/*](https://github.com/code-423n4/2026-04-layerzero/tree/main/contracts/protocol/stellar/contracts/oapps) | 134 |
| [contracts/protocol/stellar/contracts/workers/executor/*](https://github.com/code-423n4/2026-04-layerzero/tree/main/contracts/protocol/stellar/contracts/workers/executor) | 12 |
| [contracts/protocol/stellar/contracts/workers/executor-fee-lib/*](https://github.com/code-423n4/2026-04-layerzero/tree/main/contracts/protocol/stellar/contracts/workers/executor-fee-lib) | 8 |
| [contracts/protocol/stellar/contracts/workers/executor-helper/*](https://github.com/code-423n4/2026-04-layerzero/tree/main/contracts/protocol/stellar/contracts/workers/executor-helper) | 5 |
| [contracts/protocol/stellar/contracts/workers/dvn-fee-lib/*](https://github.com/code-423n4/2026-04-layerzero/tree/main/contracts/protocol/stellar/contracts/workers/dvn-fee-lib) | 5 |
| [contracts/protocol/stellar/contracts/workers/price-feed/*](https://github.com/code-423n4/2026-04-layerzero/tree/main/contracts/protocol/stellar/contracts/workers/price-feed) | 9 |
| [contracts/protocol/stellar/contracts/message-libs/simple-message-lib/*](https://github.com/code-423n4/2026-04-layerzero/tree/main/contracts/protocol/stellar/contracts/message-libs/simple-message-lib) | 7 |
| [contracts/protocol/stellar/contracts/layerzero-views/*](https://github.com/code-423n4/2026-04-layerzero/tree/main/contracts/protocol/stellar/contracts/layerzero-views) | 9 |
| [contracts/protocol/stellar/contracts/upgrader/*](https://github.com/code-423n4/2026-04-layerzero/tree/main/contracts/protocol/stellar/contracts/upgrader) | 3 |
| [contracts/protocol/stellar/contracts/macro-integration-tests/*](https://github.com/code-423n4/2026-04-layerzero/tree/main/contracts/protocol/stellar/contracts/macro-integration-tests) | 129 |
| [packages/*](https://github.com/code-423n4/2026-04-layerzero/tree/main/packages) | 58 |
| [tools/*](https://github.com/code-423n4/2026-04-layerzero/tree/main/tools) | 29 |
| [configs/*](https://github.com/code-423n4/2026-04-layerzero/tree/main/configs) | 1 |
| [tooling-configs/*](https://github.com/code-423n4/2026-04-layerzero/tree/main/tooling-configs) | 16 |
| **Total** | **625** |

*For a machine-readable version, see [out_of_scope.txt](https://github.com/code-423n4/2026-04-layerzero/blob/main/out_of_scope.txt)*

# Additional context

## Areas of concern (where to focus for bugs)

- **DVN / ULN interaction security**: Verify correctness of the verification flow between DVNs and the ULN302 message library
- **Soroban storage and TTL handling**: Is there a way to grief the Endpoint or ULN (which will be immutable) through TTL expiration or storage manipulation?
- **Censorship resistance**: Is there a way on the native payment path that LayerZero can use its permissions to censor user messages? This should not be possible under proper configurations
- **Soroban-specific behaviour**: No reentrancy, storage read limits (200 per transaction), TTL expiration edge cases
- **bytes32 address format conversion**: Correctness of conversion between Stellar variable-length addresses and LayerZero's fixed-length bytes32 format
- **Abstract Account pattern security**: DVN and Executor use Soroban's custom account interface (`__check_auth`) instead of self-calls. Has this pattern been implemented securely?
- **Pull-mode message delivery**: Correctness of the pull-mode delivery model (OApp calls `Endpoint.clear()`) under Soroban's reentrancy prohibition

## Main invariants

- Endpoint owner should not be able to censor messages
- Endpoint is immutable
- Only delegate or OApp can set configs for OApp
- ULN is immutable
- ULN owner should not be able to censor messages through native path
- DVN is secured through its multisig
- DVN cannot suffer a replay attack
- Only Admin can execute arbitrary signed payloads through DVN
- DVN can set Admin through signed payload without Admin's permission
- Owner should not be able to block user messages
- Treasury is immutable
- Treasury owner config should not be able to block user messages when using native token fee path

## All trusted roles in the protocol

N/A

## Running tests

```bash
git clone https://github.com/code-423n4/2026-04-layerzero
cd 2026-04-layerzero/contracts/protocol/stellar
```

Prerequisites:
- Rust 1.90.0 with `wasm32v1-none` target (automatically installed via `rust-toolchain.toml` when using rustup)
- [Stellar CLI](https://developers.stellar.org/docs/tools/developer-tools/cli/install-cli) for building WASM contracts

Build all contracts to WASM:

```bash
stellar contract build
```

Run all tests:

```bash
cargo test
```

## Miscellaneous

Employees of LayerZero and employees' family members are ineligible to participate in this audit.

Code4rena's rules cannot be overridden by the contents of this README. In case of doubt, please check with C4 staff.
