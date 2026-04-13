
# Audit Post-Mortem: LayerZero Stellar (Soroban)
**Severity Breakdown:** 1 High, 1 Medium  
**Focus Area:** Rust Smart Contracts, Soroban State Expiration, Trust Assumptions

During a recent review of the LayerZero Stellar implementation, two critical vulnerabilities were identified. These findings highlight the importance of challenging test-environment assumptions and bounding administrative privileges. 

Below is a technical breakdown of the vulnerabilities, their root causes, and how they were mitigated.

---

## [M-01] Soft Censorship via Unbounded Fee Extraction

**Severity:** Medium  
**Category:** Economic / Centralization Risk  
**Target:** `treasury.rs`

### The Threat Model
In cross-chain bridging protocols, the Treasury usually takes a small cut of the messaging fee on top of what is required to pay the decentralized verifier networks (DVNs) and Executors. However, if the protocol allows an admin to arbitrarily hike this fee without a hardcoded ceiling, it creates a centralization risk known as **Soft Censorship**. 

### The Root Cause
In `treasury.rs`, the admin configures the protocol's fee using the following function:

```rust
    #[only_auth]
    pub fn set_native_fee_bp(env: &Env, native_fee_bp: u32) {
        assert_with_error!(env, native_fee_bp <= BPS_DENOMINATOR, TreasuryError::InvalidNativeFeeBp);
        TreasuryStorage::set_native_fee_bp(env, &native_fee_bp);
        NativeFeeBpSet { native_fee_bp }.publish(env);
    }
```

The only invariant check here is `native_fee_bp <= BPS_DENOMINATOR`. Because `BPS_DENOMINATOR` is `10000` (representing basis points), the contract allows the admin to set the protocol tax to **10,000 bps (100%)**.

### The Impact
If a compromised key, malicious admin, or fat-finger error sets the fee to 100%, the Treasury will extract a tax exactly equal to the underlying gas/worker costs. This immediately doubles the cost of bridging for all end-users. In Web3 security, pricing users out of a protocol via unbounded parameter manipulation is categorized as a denial-of-service/soft censorship vector.

### The Fix
Administrative parameters must always have hardcoded sanity bounds. We mitigated this by introducing a maximum fee ceiling (e.g., 20%).

```diff
+   const MAX_NATIVE_FEE_BP: u32 = 2000; // 20% limit

    #[only_auth]
    pub fn set_native_fee_bp(env: &Env, native_fee_bp: u32) {
-       assert_with_error!(env, native_fee_bp <= BPS_DENOMINATOR, TreasuryError::InvalidNativeFeeBp);
+       assert_with_error!(env, native_fee_bp <= MAX_NATIVE_FEE_BP, TreasuryError::InvalidNativeFeeBp);
        TreasuryStorage::set_native_fee_bp(env, &native_fee_bp);
        // ...
    }
```

---

## [H-01] The TTL Timebomb: Permanent Lock of Bridged Funds

**Severity:** High  
**Category:** Architecture / State Management  
**Target:** `contract_ttl.rs` (Procedural Macro)

### The Threat Model
Unlike Ethereum, which uses gas to keep data alive forever, Stellar's Soroban VM uses a strict Time-To-Live (TTL) eviction model. Data that is not routinely "bumped" (extended) will expire and be permanently deleted by the network's garbage collector. LayerZero's core invariant is: *"Critical state must never be lost."* ### The Root Cause
To automate TTL management, the development team wrote a procedural macro (`contractimpl_with_ttl`) to inject a TTL-extension statement into every single contract interaction.

However, Soroban has two distinct storage types for long-term data:
1. **Instance Storage:** For global configurations.
2. **Persistent Storage:** For critical user data.

Looking at the macro's Abstract Syntax Tree (AST) injection logic:

```rust
        if method.sig.ident == "__constructor" {
            method.block.stmts.insert(0, init_default_ttl_configs_stmt(&env_param));
        } else {
            // Inject TTL extension at the start of other methods
            method.block.stmts.insert(0, extend_instance_ttl_stmt(&env_param));
        }
```

The macro explicitly tells the compiler to extend `Instance` storage, but completely **neglects `Persistent` storage**. 

Meanwhile, in `storage.rs`, the most critical piece of user data—the cross-chain payload hash—is defined as persistent:

```rust
    #[persistent(BytesN<32>)]
    InboundPayloadHash { receiver: Address, src_eid: u32, sender: BytesN<32>, nonce: u64 },
```

### The Impact
Because the macro excludes Persistent storage, the user's `InboundPayloadHash` will silently tick toward expiration. If an attacker delays execution, or if network congestion prevents the destination application from clearing the payload within the ~30-day window, the Stellar network will permanently delete the payload hash. The user's bridged funds will be permanently locked on the origin chain, while the contract itself remains blissfully unaware, kept alive by the Instance TTL bumps.

*Note on Testing:* This bug evaded local CI pipelines because local test environments (`Env::default()`) default `max_ttl` to `u32::MAX` and do not run a background garbage collector. It requires static analysis of the macro to catch.

### The Fix
The procedural macro was patched to ensure both state trees are extended simultaneously on every interaction.

```diff
        } else {
            // Inject TTL extension at the start of other methods
            method.block.stmts.insert(0, extend_instance_ttl_stmt(&env_param));
+           method.block.stmts.insert(1, extend_persistent_ttl_stmt(&env_param));
        }
```

---

### Key Takeaways for Developers
1. **The Test-Net Illusion:** Never assume that passing local tests means your state management is secure. Local VMs often lack the physical constraints of Mainnet (like garbage collection and rigid TTL limits). 

2. **Audit your Macros:** Procedural macros abstract logic away from the naked eye. A single missed line in a macro injects a vulnerability into every contract in the workspace.