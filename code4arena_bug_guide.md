# 🏆 Code4rena LayerZero Stellar Audit - Winning Strategy Guide

<div align="center">

**Prize Pool: $101,000 USDC**  
**Contest Duration: April 1-14, 2026**  
**Your Mission: Maximize valid High/Medium findings**

</div>

---

## 📋 Table of Contents

1. [Understanding Code4rena Submission Process](#understanding-code4rena-submission-process)
2. [Severity Classification - The Make-or-Break Decision](#severity-classification---the-make-or-break-decision)
3. [The Downgrade Problem & How to Avoid It](#the-downgrade-problem--how-to-avoid-it)
4. [Finding Patterns for LayerZero Stellar](#finding-patterns-for-layerzero-stellar)
5. [Report Writing Framework](#report-writing-framework)
6. [Pre-Submission Validation](#pre-submission-validation)
7. [Time Management Strategy](#time-management-strategy)
8. [Common Mistakes That Cost Wardens Money](#common-mistakes-that-cost-wardens-money)

---

## Understanding Code4rena Submission Process

### How Submissions Work

```
Code4rena Platform Flow:
┌─────────────────────────────────────┐
│ 1. Navigate to Contest Page         │
│    https://code4rena.com/contests   │
└─────────────────────────────────────┘
           ↓
┌─────────────────────────────────────┐
│ 2. Click "Submit Finding"           │
│    (Available during contest only)  │
└─────────────────────────────────────┘
           ↓
┌─────────────────────────────────────┐
│ 3. Fill Dropdown Fields:            │
│    • Severity: High/Medium/QA       │
│    • Title: Brief description       │
│    • Impact: What breaks?           │
│    • Proof of Concept: Code/steps   │
│    • Mitigation: How to fix         │
└─────────────────────────────────────┘
           ↓
┌─────────────────────────────────────┐
│ 4. Submit (creates GitHub issue)    │
│    • Each finding = separate issue  │
│    • QA Report = single issue       │
└─────────────────────────────────────┘
```

### What You DON'T Need

❌ **NO PDF documentation required**  
❌ **NO formal report formatting**  
❌ **NO executive summary**  
❌ **NO complex diagrams** (helpful but optional)

### What You DO Need

✅ **Direct issue submission via form**  
✅ **Clear title** (shows up in GitHub issues)  
✅ **Severity selection** (High/Medium/QA dropdown)  
✅ **Impact description** (what breaks/loses funds)  
✅ **Proof of Concept** (code or detailed steps)  
✅ **Recommended fix** (shows you understand the code)

---

## Severity Classification - The Make-or-Break Decision

### 💰 Why Severity Matters

```
Prize Pool Distribution:
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
High/Medium Pool:    $89,760 (89%)
QA Pool:              $3,740 (3.7%)
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

Points System:
High:    10 points each
Medium:   3 points each
QA:      Split equally (typically 0.5-2 points)

Example Impact:
┌──────────────────────────────────────┐
│ Submit as HIGH (accepted):           │
│   → 10 points → ~$4,500 share       │
├──────────────────────────────────────┤
│ Submit as MEDIUM (accepted):         │
│   → 3 points → ~$1,350 share        │
├──────────────────────────────────────┤
│ Submit as HIGH (downgraded to QA):  │
│   → INVALID → $0 (ineligible)       │
└──────────────────────────────────────┘
```

### 🚨 Critical Rule from Contest Page

> **"High- or Medium-risk submissions downgraded by the judge to Low-risk (QA) will be ineligible for awards."**

**Translation:** If you mark a bug as High/Medium and the judge disagrees, you get **$0** instead of QA pool share.

**Consequence:** Conservative severity selection often better than aggressive inflation.

---

## The Downgrade Problem & How to Avoid It

### Why Judges Downgrade Findings

#### ❌ Reason 1: No Direct Fund Loss

```markdown
❌ WARDEN SUBMISSION (marked as HIGH):

## [H-01] Missing Event Emission in setDelegate

### Impact
Events are not emitted when delegate changes, making off-chain 
monitoring difficult.

### Severity: HIGH
```

**Judge Response:**  
> "No fund loss, no functionality break. Downgraded to Low (QA). Ineligible for HM awards."

**Warden Gets:** $0

---

```markdown
✅ CORRECT SUBMISSION (marked as QA):

## [L-01] Missing Event Emission in setDelegate

### Impact  
Off-chain monitoring systems cannot track delegate changes, 
degrading user experience.

### Severity: LOW (QA)
```

**Judge Response:**  
> "Accepted as Low. Valid QA finding."

**Warden Gets:** Share of $3,740 QA pool (~$100-300)

---

#### ❌ Reason 2: Requires Unrealistic Conditions

```markdown
❌ WARDEN SUBMISSION (marked as HIGH):

## [H-01] Owner Can Censor Messages

### Impact
Treasury owner can set fees to MAX_UINT, preventing all message sends.

### Severity: HIGH
```

**Judge Response:**  
> "README states: 'Treasury owner config should not be able to block messages when using native token fee path.' This is a known design consideration, and proper configuration prevents this. Additionally, owner is trusted role. Invalid."

**Warden Gets:** $0

---

```markdown
✅ CORRECT SUBMISSION (marked as MEDIUM):

## [M-01] Treasury Owner Can DoS Messages If Misconfigured

### Impact
If treasury owner is malicious AND native fee path is not properly 
configured, they can set excessive fees to grief users.

### Severity: MEDIUM
Reason: Requires misconfiguration + malicious owner (trusted role)
```

**Judge Response:**  
> "Valid Medium. Shows understanding of trust assumptions."

**Warden Gets:** 3 points → ~$1,350

---

#### ❌ Reason 3: Temporary/Recoverable Issues

```markdown
❌ WARDEN SUBMISSION (marked as HIGH):

## [H-01] Storage Read Limit Prevents Batch Processing

### Impact
Functions fail if they exceed 200 storage reads, causing DoS.

### Severity: HIGH
```

**Judge Response:**  
> "This is a DoS but transactions can be retried with smaller batches. No permanent fund loss. Downgraded to Medium."

**Warden Gets:** $0 (was marked High, now Medium = ineligible)

---

```markdown
✅ CORRECT SUBMISSION (marked as MEDIUM):

## [M-01] Storage Read Limit Enables Griefing Attack

### Impact
Attacker can force victims to waste gas fees by causing transactions 
to revert at 200 read limit. Victim must retry multiple times, 
paying gas each time.

Cost: ~$50 per failed attempt × 10 attempts = $500 loss

### Severity: MEDIUM
Reason: Causes financial loss via griefing, but not permanent
```

**Judge Response:**  
> "Valid Medium. Clear financial impact quantified."

**Warden Gets:** 3 points → ~$1,350

---

### The Severity Decision Tree

```
START: I found a bug
    ↓
┌───────────────────────────────────────┐
│ Can attacker STEAL/LOCK funds?       │
│ (Direct, permanent fund loss)         │
└───────────────────────────────────────┘
    │
    ├─ YES → Is it GUARANTEED to work?
    │         │
    │         ├─ YES → Submit as HIGH ✅
    │         │
    │         └─ NO (needs conditions) → Is it REALISTIC?
    │                   │
    │                   ├─ YES → Submit as HIGH ✅
    │                   │         (add clear conditions)
    │                   │
    │                   └─ NO → Submit as MEDIUM
    │
    └─ NO → Does it BREAK core functionality?
              │
              ├─ YES → Is it PERMANENT?
              │         │
              │         ├─ YES → Submit as HIGH ✅
              │         │
              │         └─ NO → Submit as MEDIUM
              │
              └─ NO → Does it cause FINANCIAL loss?
                        │
                        ├─ YES → Can you QUANTIFY it?
                        │         │
                        │         ├─ YES (>$100) → MEDIUM
                        │         │
                        │         └─ NO or minimal → QA
                        │
                        └─ NO → QA (Low/Info)
```

---

## Finding Patterns for LayerZero Stellar

### 🔴 HIGH Severity Patterns

#### Pattern H-1: TTL Expiration Causes Permanent Lock

```rust
// VULNERABLE CODE PATTERN:
pub fn commit_verification(env: Env, nonce: u64) {
    // ❌ No extend_ttl before read
    let verification = env.storage().persistent().get(&nonce).unwrap();
    //                                                         ^^^^^^^^
    // PANICS if TTL expired → Message stuck FOREVER
}
```

**Why HIGH:**
- ✅ Permanent fund loss (messages with funds can't execute)
- ✅ No recovery mechanism
- ✅ Attacker can force this state

**Search Command:**
```bash
rg "\.get\(&.*\)\.unwrap\(\)" --type rust | rg -v "extend_ttl"
```

**Expected Finding:**
```markdown
## [H-01] TTL Expiration Causes Permanent Message Lock Without Fund Recovery

### Impact
Messages can become permanently stuck if storage TTL expires before 
commit_verification is called. Since LayerZero endpoint and ULN are 
immutable, locked messages cannot be recovered, resulting in permanent 
loss of user funds sent with those messages.

Attack Cost: Minimal (attacker just waits for natural TTL expiration)
User Loss: Total message value (could be millions)

### Proof of Concept
[Full PoC code showing attack]

### Severity: HIGH
Reason: Permanent, irrecoverable fund loss
```

---

#### Pattern H-2: Signature Replay Across Messages

```rust
// VULNERABLE CODE PATTERN:
pub fn verify(env: Env, packet_header: Vec<u8>, signatures: Vec<Signature>) {
    let packet_id = derive_packet_id(&packet_header);
    
    for sig in signatures {
        // ❌ Only checks signature is valid for packet_id
        // Doesn't bind to specific payload_hash
        verify_ecdsa(&packet_id, &sig);
    }
}
```

**Why HIGH:**
- ✅ Attacker can execute unauthorized messages
- ✅ Direct fund theft possible
- ✅ Bypasses core security (DVN verification)

**Search Command:**
```bash
rg "verify.*signature" --type rust -A 10 | rg -v "payload.*hash"
```

**Expected Finding:**
```markdown
## [H-02] DVN Signature Replay Allows Unauthorized Message Execution

### Impact
Attacker can replay valid DVN signatures from one message to verify 
a completely different malicious message. This bypasses the entire 
DVN security model, allowing arbitrary cross-chain message execution 
without proper verification.

Example: Signatures for "transfer 100 USDC" replayed to verify 
"transfer 1M USDC to attacker"

### Severity: HIGH
Reason: Complete security bypass, direct fund theft
```

---

#### Pattern H-3: Access Control Bypass

```rust
// VULNERABLE CODE PATTERN:
pub fn set_config(env: Env, oapp: Address, config: UlnConfig) {
    // ❌ No authorization check!
    // Anyone can set anyone's config
    env.storage().persistent().set(&oapp, &config);
}
```

**Why HIGH:**
- ✅ Attacker controls security parameters
- ✅ Can set own malicious DVNs
- ✅ Leads to fund theft

**Search Command:**
```bash
rg "pub fn set" --type rust -A 5 | rg -v "require_auth"
```

---

### 🟡 MEDIUM Severity Patterns

#### Pattern M-1: Storage Read Limit Griefing

```rust
// VULNERABLE CODE PATTERN:
pub fn batch_commit(env: Env, packets: Vec<Packet>) {
    // ❌ No limit on packets.len()
    for packet in packets {
        let data = env.storage().persistent().get(&packet.id);
        // Each read counts toward 200 limit
    }
}
```

**Why MEDIUM (not HIGH):**
- ✅ Causes financial loss (wasted gas)
- ✅ Temporary DoS (can retry with smaller batch)
- ❌ No permanent fund loss
- ❌ No direct theft

**Expected Finding:**
```markdown
## [M-01] Storage Read Limit Allows Griefing Via Forced Transaction Failures

### Impact
Attacker can craft transactions that force victims to hit Soroban's 
200 storage read limit, causing transaction failures. Victim wastes 
gas fees on failed attempts.

Cost Analysis:
- Gas per failed attempt: ~$2
- Victim must retry 5-10 times
- Total loss: $10-20 per attack
- Attack cost: $0 (just crafting parameters)

### Severity: MEDIUM
Reason: Financial loss via griefing, not permanent lock
```

---

#### Pattern M-2: Nonce Manipulation Allows Message Reordering

```rust
// VULNERABLE CODE PATTERN:
pub fn receive(env: Env, nonce: u64, message: Vec<u8>) {
    // ❌ No check if nonce is next expected
    // ❌ No gap protection
    process_message(&env, &message);
    set_last_nonce(&env, nonce);
}
```

**Why MEDIUM (not HIGH):**
- ✅ Breaks message ordering (intended behavior)
- ✅ Can cause business logic issues
- ❌ No direct fund loss (depends on OApp implementation)

**Expected Finding:**
```markdown
## [M-02] Missing Nonce Sequence Validation Allows Message Reordering

### Impact
Messages can be executed out of order, violating LayerZero's ordered 
delivery guarantee. OApps relying on message sequence (e.g., updating 
oracle prices sequentially) will malfunction.

Example Attack:
1. Message 1: Set price = $100
2. Message 2: Set price = $200
3. Attacker executes Message 2 first → wrong price accepted

### Severity: MEDIUM
Reason: Breaks core functionality, potential financial impact 
depending on OApp, but not guaranteed fund loss
```

---

### 🟢 QA (LOW/INFO) Patterns

#### Pattern L-1: Missing Input Validation

```rust
pub fn set_treasury(env: Env, treasury: Address) {
    // Missing: treasury != zero_address check
    env.storage().instance().set(&TREASURY_KEY, &treasury);
}
```

**Why LOW:**
- No direct fund loss
- Admin can fix by calling again
- Best practice violation

---

#### Pattern L-2: Event Not Indexed

```rust
#[event(name = "PacketSent")]
pub struct PacketSent {
    pub nonce: u64,        // Should be indexed
    pub sender: Address,   // Should be indexed
}
```

**Why LOW:**
- No fund loss
- No functionality break
- UX degradation only

---

## Report Writing Framework

### Form Fields Breakdown

When you click "Submit Finding" on Code4rena, you'll see:

```
┌────────────────────────────────────────────────┐
│ SEVERITY (Dropdown)                            │
│ ○ High                                         │
│ ○ Medium                                       │
│ ○ QA (Low/Info)                               │
├────────────────────────────────────────────────┤
│ TITLE (Text Field)                             │
│ Brief description of issue                     │
├────────────────────────────────────────────────┤
│ VULNERABILITY DETAILS (Markdown Editor)        │
│ Full description of the bug                    │
├────────────────────────────────────────────────┤
│ IMPACT (Markdown Editor)                       │
│ What breaks? Who loses money?                  │
├────────────────────────────────────────────────┤
│ PROOF OF CONCEPT (Markdown Editor)             │
│ Code or step-by-step reproduction              │
├────────────────────────────────────────────────┤
│ TOOLS USED (Optional)                          │
│ Manual review, Slither, etc.                   │
├────────────────────────────────────────────────┤
│ RECOMMENDED MITIGATION (Markdown Editor)       │
│ How to fix the bug                             │
└────────────────────────────────────────────────┘
```

---

### Template for HIGH Severity

#### TITLE Field:
```
DVN Signature Replay Allows Unauthorized Cross-Chain Message Execution
```

Keep it:
- Under 80 characters
- Descriptive of root cause AND impact
- Specific (not "Security Issue in ULN")

---

#### VULNERABILITY DETAILS Field:

```markdown
## Summary

The `verify()` function in ULN302 does not bind DVN signatures to 
specific payload hashes, allowing attackers to replay valid signatures 
from one message to authorize a completely different malicious message.

## Vulnerability Details

**Location:** `contracts/message-libs/uln-302/src/receive_uln.rs:156-178`

The verification logic only checks that signatures are valid for the 
packet_id (derived from header), but does NOT verify signatures include 
the payload_hash:

```rust
pub fn verify(
    env: Env,
    packet_header: Vec<u8>,
    payload_hash: BytesN<32>,
    signatures: Vec<BytesN<64>>
) {
    let packet_id = derive_packet_id(&packet_header, &payload_hash);
    
    for (i, sig) in signatures.iter().enumerate() {
        let dvn = get_dvn(&env, i);
        
        // ❌ VULNERABILITY: Only verifies signature matches packet_id
        // Does NOT verify signature includes payload_hash
        verify_signature(&dvn, &packet_id, sig);
        
        mark_verified(&env, &dvn, &packet_id);
    }
}
```

**Root Cause:**  
`packet_id` is derived from header only, not including payload_hash 
uniquely. Same header can be used with different payloads.

**Attack Vector:**  
1. Legitimate message A sent with payload "transfer 100 USDC"
2. DVNs sign packet_id for message A
3. Attacker intercepts signatures
4. Attacker creates message B with payload "transfer 1M USDC to attacker"
5. Attacker replays signatures from message A with message B's payload
6. Verification passes (same packet_id)
7. Malicious message executes
```

---

#### IMPACT Field:

```markdown
## Impact Assessment

**Severity: HIGH**

**Who is affected:**  
- All users sending cross-chain messages via LayerZero
- All OApps using ULN302 for verification

**What is lost:**  
- User funds sent in messages (could be millions per message)
- Protocol security model completely bypassed

**Attack feasibility:**  
- Cost: $0 (just signature replay)
- Skill: Medium (requires understanding of LayerZero flow)
- Detection: Difficult (appears as legitimate verification)

**Permanent impact:**  
- Yes, funds stolen cannot be recovered
- No admin function can reverse executed messages

**Real-world scenario:**
1. Alice sends 100,000 USDC from Ethereum to Stellar
2. DVNs verify legitimately, signatures recorded on-chain
3. Bob (attacker) replays signatures with modified payload
4. Bob's message: "Send 100,000 USDC to Bob's address"
5. Message executes, Bob steals Alice's funds
```

---

#### PROOF OF CONCEPT Field:

```markdown
## Proof of Concept

### Test Setup

Add to `contracts/message-libs/uln-302/src/tests/replay_attack.rs`:

```rust
#[test]
fn test_signature_replay_attack() {
    let env = Env::default();
    let uln = setup_uln302(&env);
    
    // === STEP 1: Legitimate message ===
    let legit_header = create_packet_header(
        src_eid: 30101,  // Ethereum
        sender: bytes32(alice_eth_address),
        dst_eid: 40254,  // Stellar
        receiver: bytes32(alice_stellar_address),
        nonce: 1
    );
    
    let legit_payload = encode_message("transfer", 100_USDC, alice_stellar_address);
    let legit_payload_hash = keccak256(&legit_payload);
    
    // DVNs sign legitimate message
    let dvn1_sig = dvn1.sign(&legit_header, &legit_payload_hash);
    let dvn2_sig = dvn2.sign(&legit_header, &legit_payload_hash);
    
    // Verify legitimate message
    uln.verify(
        &env,
        legit_header.clone(),
        legit_payload_hash,
        vec![dvn1_sig, dvn2_sig]
    );
    // ✅ Passes correctly
    
    // === STEP 2: Attacker's malicious message ===
    let malicious_payload = encode_message("transfer", 100_USDC, attacker_address);
    let malicious_payload_hash = keccak256(&malicious_payload);
    
    // === STEP 3: Replay attack ===
    // Attacker uses SAME header, SAME signatures, but DIFFERENT payload
    uln.verify(
        &env,
        legit_header,  // ← Same header
        malicious_payload_hash,  // ← Attacker's payload!
        vec![dvn1_sig, dvn2_sig]  // ← Replayed signatures
    );
    // ❌ Should FAIL but PASSES!
    
    // === STEP 4: Execute malicious message ===
    uln.commit_verification(&env, legit_header, malicious_payload_hash);
    let result = endpoint.clear(&env, legit_header, malicious_payload);
    
    // === VERIFY ATTACK SUCCESS ===
    assert_eq!(
        token_balance(attacker_address),
        100_USDC  // Attacker stole funds!
    );
    
    assert_eq!(
        token_balance(alice_stellar_address),
        0  // Alice got nothing
    );
}
```

### Running the PoC

```bash
cd contracts/protocol/stellar/contracts/message-libs/uln-302
cargo test test_signature_replay_attack
```

**Expected Output:**
```
test replay_attack::test_signature_replay_attack ... ok
```

This confirms the vulnerability is exploitable.
```

---

#### RECOMMENDED MITIGATION Field:

```markdown
## Recommended Mitigation

### Short-term Fix (Immediate)

Modify `verify()` to bind signatures to payload_hash:

```rust
pub fn verify(
    env: Env,
    packet_header: Vec<u8>,
    payload_hash: BytesN<32>,
    signatures: Vec<BytesN<64>>
) {
    for (i, sig) in signatures.iter().enumerate() {
        let dvn = get_dvn(&env, i);
        
        // ✅ FIX: Include payload_hash in signed message
        let signing_message = create_signing_message(
            &packet_header,
            &payload_hash  // ← Bind to specific payload
        );
        
        verify_signature(&dvn, &signing_message, sig);
        mark_verified(&env, &dvn, &packet_header, &payload_hash);
    }
}

fn create_signing_message(
    header: &[u8],
    payload_hash: &BytesN<32>
) -> BytesN<32> {
    // Hash BOTH header AND payload together
    env.crypto().keccak256(&[
        header,
        payload_hash.as_slice()
    ].concat())
}
```

### Long-term Improvements

1. **Add nonce to signed message** (prevent cross-nonce replay)
2. **Add destination chain ID** (prevent cross-chain replay)
3. **Add expiration timestamp** (prevent delayed replay)

### Updated DVN Signature Structure

```rust
struct SignedMessage {
    packet_header: Vec<u8>,
    payload_hash: BytesN<32>,
    nonce: u64,
    destination_chain_id: u32,
    expiration: u64
}

let message_hash = keccak256(&encode(SignedMessage { ... }));
let signature = dvn.sign(message_hash);
```

This ensures signatures are:
- ✅ Unique per message
- ✅ Non-replayable across chains
- ✅ Time-limited
```

---

### Template for MEDIUM Severity

#### TITLE Field:
```
Storage Read Limit Enables Griefing Attack on Batch Operations
```

#### VULNERABILITY DETAILS Field:

```markdown
## Summary

Batch processing functions do not limit the number of items processed, 
allowing attackers to force transactions to exceed Soroban's 200 storage 
read limit and fail, wasting victim's gas fees.

## Vulnerability Details

**Location:** `contracts/message-libs/uln-302/src/receive_uln.rs:203-225`

```rust
pub fn batch_verify(
    env: Env,
    packets: Vec<Packet>,
    signatures: Vec<Vec<BytesN<64>>>
) {
    // ❌ No limit on packets.len()
    for (i, packet) in packets.iter().enumerate() {
        for sig in &signatures[i] {
            let dvn = env.storage().persistent().get(&sig.dvn_id);
            // Each get() = 1 storage read
        }
    }
}
```

**Attack Scenario:**
- Attacker calls with 201 packets
- Each packet requires 1 read → 201 total reads
- Soroban limit: 200 reads
- Transaction reverts, victim wastes gas
```

#### IMPACT Field:

```markdown
## Impact Assessment

**Severity: MEDIUM**

**Financial Impact:**
- Gas cost per failed attempt: ~$2
- Victim must retry 5-10 times with smaller batches
- Total loss per attack: $10-20
- Attack cost: $0 (just parameter crafting)

**Why NOT High:**
- ✅ Causes financial loss (griefing)
- ❌ NOT permanent (victim can retry)
- ❌ NOT direct theft (attacker doesn't profit)
- ✅ Temporary DoS (workaround exists)

**Affected Users:**
- Anyone calling batch operations
- Particularly harmful for high-frequency message users
```

---

### Template for QA Report

QA Report is ONE submission with MULTIPLE findings:

#### TITLE Field:
```
QA Report - Multiple Low Severity Issues
```

#### VULNERABILITY DETAILS Field:

```markdown
# QA Report - LayerZero Stellar

## Summary

| ID | Title | Instances |
|----|-------|-----------|
| [L-01] | Missing zero address validation | 8 |
| [L-02] | Events not indexed for filtering | 15 |
| [L-03] | Unsafe unwrap() calls without error handling | 12 |
| [NC-01] | Inconsistent error naming conventions | 20 |
| [NC-02] | Missing NatSpec documentation | 45 |

---

## [L-01] Missing zero address validation in critical setters

### Impact
Setting addresses to zero can brick contract functionality.

### Instances

**[L-01-1]** `endpoint-v2/src/message_lib_manager.rs:89`
```rust
pub fn set_send_library(env: Env, sender: Address, lib: Address) {
    // ❌ Missing: require!(!lib.is_zero())
    env.storage().persistent().set(&sender, &lib);
}
```

**[L-01-2]** `message-libs/treasury/src/treasury.rs:56`
```rust
pub fn set_treasury_fee_handler(env: Env, handler: Address) {
    // ❌ Missing validation
    env.storage().instance().set(&HANDLER_KEY, &handler);
}
```

[... List remaining 6 instances with file:line ...]

### Recommendation
```rust
pub fn set_send_library(env: Env, sender: Address, lib: Address) {
    require!(!lib.is_zero(), Error::ZeroAddress);
    env.storage().persistent().set(&sender, &lib);
}
```

---

## [L-02] Events not indexed for efficient filtering

### Impact
Off-chain systems cannot efficiently filter events, degrading UX.

### Instances

**[L-02-1]** `endpoint-v2/src/events.rs:45`
```rust
#[event(name = "DelegateSet")]
pub struct DelegateSet {
    pub oapp: Address,      // ❌ Should be indexed
    pub delegate: Address,  // ❌ Should be indexed
}
```

[... Continue for all Low/NC findings ...]
```

---

## Pre-Submission Validation

### The Self-Audit Checklist

Before submitting EACH finding, ask yourself:

```markdown
## HIGH SEVERITY CHECKLIST

□ Is there DIRECT fund loss?
  - NOT just "could lead to" or "might cause"
  - ACTUAL theft/lock of funds

□ Is it PERMANENT?
  - NOT recoverable via retry/workaround
  - NOT fixable by admin action

□ Is attack FEASIBLE?
  - NOT requires 51% attack
  - NOT requires breaking cryptography
  - Cost < Profit for attacker

□ Is it GUARANTEED to work?
  - NOT "if owner is malicious" (trusted role)
  - NOT "if misconfigured" (user error)
  - Works in DEFAULT/EXPECTED configuration

□ Does PoC actually compile and run?
  - Tested locally
  - No syntax errors
  - Demonstrates actual exploit

If ALL ✅ → Submit as HIGH
If ANY ❌ → Re-evaluate severity
```

```markdown
## MEDIUM SEVERITY CHECKLIST

□ Does it cause FINANCIAL loss?
  - Griefing with quantifiable cost
  - Forced gas waste > $10

□ Does it BREAK core functionality?
  - NOT minor features
  - NOT edge cases
  - Core messaging flow affected

□ Is there a WORKAROUND?
  - If yes, but costly → still Medium
  - If yes, and easy → might be Low

□ Are conditions REALISTIC?
  - NOT requires multiple unlikely events
  - NOT requires trusted roles to be malicious

If ALL ✅ → Submit as MEDIUM
If ANY ❌ → Likely QA (Low)
```

```markdown
## QA (LOW) CHECKLIST

□ Is it best practice violation?
□ Is it informational improvement?
□ Is it gas optimization?
□ Is it UX degradation (not break)?
□ Does it NOT cause fund loss?
□ Does it NOT break functionality?

If ALL ✅ → Submit to QA Report
```

---

### The Downgrade Prevention Test

**Run this for EVERY High/Medium submission:**

```markdown
## Imagine You're The Judge

"I'm about to mark this as [HIGH/MEDIUM]. The judge will ask:"

1. "Can you show me exactly where the money gets stolen?"
   - If NO → Might be Medium/Low

2. "What if admin just calls [some function] to fix it?"
   - If FIXABLE → Might be Medium/Low

3. "Isn't [role] trusted according to README?"
   - If YES and issue requires malicious trusted role → Likely Low/Invalid

4. "Hasn't this been discussed in the docs/previous audits?"
   - If YES → Check contest README "Publicly Known Issues" section

5. "Does this actually work on Soroban or are you thinking of EVM?"
   - Reentrancy: IMPOSSIBLE on Soroban
   - Integer overflow: Rust prevents this
   - etc.
```

**If you can't confidently answer these, downgrade severity.**

---

## Time Management Strategy

### 2-Week Contest Timeline

```
┌─────────────────────────────────────────────────────┐
│ WEEK 1: Deep Research & Pattern Recognition         │
├─────────────────────────────────────────────────────┤
│ Day 1-2: Environment Setup & Orientation           │
│   □ Clone repo, build contracts                     │
│   □ Read README, scope.txt thoroughly              │
│   □ Understand main invariants (12 listed)          │
│   □ Review Stellar/Soroban docs                     │
│                                                      │
│ Day 3-5: Manual Code Review (Focus Mode)            │
│   □ Map critical paths (send/verify/commit/execute) │
│   □ Track ALL storage.get() calls → TTL issues     │
│   □ Track ALL signature verifications → replays    │
│   □ Track ALL access control → bypasses            │
│   □ Document 10-15 suspicious patterns              │
│                                                      │
│ Day 6-7: PoC Development Sprint                     │
│   □ Pick top 5 most promising findings              │
│   □ Write coded PoCs for each                       │
│   □ 2-3 should be High, 2-3 Medium                  │
│   □ Discard findings without working PoCs           │
└─────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────┐
│ WEEK 2: Refinement & Submission                     │
├─────────────────────────────────────────────────────┤
│ Day 8-9: Severity Validation                        │
│   □ Run each finding through downgrade test         │
│   □ Demote inflated severities                      │
│   □ Verify against main invariants                  │
│   □ Check for duplicates (search GitHub issues)     │
│                                                      │
│ Day 10-11: Report Writing                           │
│   □ Write full reports for top 3-5 findings         │
│   □ Clear impact statements                         │
│   □ Polished PoCs                                   │
│   □ Concrete mitigations                            │
│                                                      │
│ Day 12-13: QA Report Compilation                    │
│   □ Gather all Low/Info findings                    │
│   □ Group similar issues                            │
│   □ Write comprehensive QA report                   │
│   □ Final review of all submissions                 │
│                                                      │
│ Day 14: Submit 6+ Hours Before Deadline             │
│   □ Submit High findings first (priority)           │
│   □ Submit Medium findings                          │
│   □ Submit QA report last                           │
│   □ Save local copies of everything                 │
└─────────────────────────────────────────────────────┘
```

---

### Daily Time Allocation

```
Recommended: 6-8 hours/day (realistic for part-time)

Hour 1-2:   Manual code review (new files)
Hour 3-4:   PoC development/testing
Hour 5-6:   Report writing/refinement
Hour 7-8:   Research/tool exploration

Total: 84-112 hours over 2 weeks
```

---

## Common Mistakes That Cost Wardens Money

### ❌ Mistake 1: Severity Inflation

**Problem:**
```markdown
Warden marks EVERYTHING as High:
- "Missing event" → HIGH ❌
- "Gas optimization" → HIGH ❌
- "Typo in comment" → HIGH ❌

Judge downgrades all to QA → Warden gets $0
```

**Solution:**
```markdown
Be HONEST with severity:
- Clear fund loss → HIGH ✅
- Functional break → MEDIUM ✅
- Everything else → QA ✅

Result: Valid findings, actual payout
```

---

### ❌ Mistake 2: No Proof of Concept

**Problem:**
```markdown
## [H-01] Reentrancy in withdraw()

The function doesn't check for reentrancy.

Judge: "Where's the PoC? Also, Soroban prevents reentrancy."
Result: Invalid → $0
```

**Solution:**
```markdown
ALWAYS include:
1. Coded PoC (Rust test)
2. Step-by-step attack scenario
3. Evidence it works (test output)

If you can't write a PoC, it's probably not a real bug.
```

---

### ❌ Mistake 3: Ignoring Trust Assumptions

**Problem:**
```markdown
## [H-01] Owner Can Pause Contract

The owner can call pause() and stop all operations!

Judge: "Owner is trusted role per README. Invalid."
Result: $0
```

**Solution:**
```markdown
READ THE README "Main Invariants" section:
- Identify trusted roles
- Only report violations of stated invariants
- Don't report "trusted admin can be malicious"
```

---

### ❌ Mistake 4: Duplicate Submissions

**Problem:**
```markdown
Warden A finds signature replay → Submits as H-01
Warden B finds signature replay → Submits as H-02
Warden C finds signature replay → Submits as H-03

Judge deduplicates → Only best report counts
Result: A gets 10 points, B gets 0, C gets 0
```

**Solution:**
```markdown
1. Search existing GitHub issues during contest
2. If duplicate, make yours BETTER (clearer PoC)
3. OR find different bugs
4. Quality > Quantity
```

---

### ❌ Mistake 5: Platform-Specific Bugs

**Problem:**
```markdown
## [H-01] Integer Overflow in add()

```rust
pub fn add(a: u64, b: u64) -> u64 {
    a + b  // Overflow possible!
}
```

Judge: "Rust panics on overflow in debug mode and wraps in release 
with explicit wrapping types. This isn't a bug. Invalid."
Result: $0
```

**Solution:**
```markdown
Understand the platform:
- Rust: No silent overflow
- Soroban: No reentrancy
- Stellar: TTL expiration IS an issue
- etc.

Don't copy-paste EVM bugs into Rust audits.
```

---

## Final Pre-Submission Checklist

```markdown
□ Each HIGH finding:
  □ Has DIRECT fund loss or permanent DoS
  □ Has working coded PoC (tested locally)
  □ Has clear attack scenario (numbered steps)
  □ References exact code location (file:line)
  □ Includes concrete mitigation
  □ Severity justified in Impact section
  □ NOT a duplicate (searched GitHub issues)
  □ NOT a known issue (checked README)

□ Each MEDIUM finding:
  □ Has financial loss OR functional break
  □ Conditions are realistic
  □ NOT recoverable easily
  □ Attack scenario documented
  □ Severity justified

□ QA Report:
  □ All Low/Info findings grouped
  □ Summary table at top
  □ Each instance has file:line
  □ Recommendations provided

□ General:
  □ Submitted 6+ hours before deadline
  □ All markdown renders correctly
  □ Grammar/spelling checked
  □ Local copies saved
```

---

## Expected Outcomes

### Conservative Estimate
```
2 High findings (accepted)    = 20 points
2 Medium findings (accepted)  = 6 points
1 QA Report                   = ~1 point
─────────────────────────────────────────
TOTAL:                         27 points

If 150 total valid points:
Your share: (27/150) × $89,760 = $16,157

ROI: $16,157 / 100 hours = $161/hour
```

### Realistic Estimate
```
1 High (accepted)             = 10 points
3 Medium (accepted)           = 9 points
1 QA Report                   = ~1 point
─────────────────────────────────────────
TOTAL:                         20 points

If 200 total valid points:
Your share: (20/200) × $89,760 = $8,976

ROI: $8,976 / 100 hours = $89/hour
```

### Worst Case (Learning Experience)
```
0 Highs (all downgraded)      = 0 points
1 Medium (accepted)           = 3 points
1 QA Report                   = ~1 point
─────────────────────────────────────────
TOTAL:                         4 points

If 250 total valid points:
Your share: (4/250) × $89,760 = $1,436

ROI: $1,436 / 100 hours = $14/hour
BUT: Valuable learning for next contest
```

---

## 🎯 Your Action Plan

### Starting NOW:

1. **Clone and build the repo**
   ```bash
   git clone https://github.com/code-423n4/2026-04-layerzero
   cd 2026-04-layerzero/contracts/protocol/stellar
   stellar contract build
   cargo test
   ```

2. **Create your research notes file**
   ```bash
   touch FINDINGS.md
   ```

3. **Start with high-value targets**
   ```
   Priority files (order matters):
   1. receive_uln.rs (verification logic)
   2. dvn.rs (signature handling)
   3. messaging_channel.rs (nonce tracking)
   4. endpoint_v2.rs (core routing)
   5. send_uln.rs (fee calculation)
   ```

4. **Search for vulnerability patterns**
   ```bash
   # TTL issues
   rg "\.get\(&.*\)\.unwrap\(\)" --type rust
   
   # Missing access control
   rg "pub fn set" --type rust -A 5 | rg -v "require_auth"
   
   # Signature verification
   rg "verify.*sig" --type rust -i
   ```

5. **Write PoCs immediately**
   Don't just note "suspicious" - prove it works

6. **Self-audit before submitting**
   Use the severity checklists above

---

## Go Hunt!

Remember:
- **Quality > Quantity**
- **Correct Severity > Impressive Findings**
- **Working PoC > Theoretical Attack**
- **Clear Report > Verbose Report**

You've got this! Now go find those bugs! 💰

---

<div align="center">

**Good luck, warden!**

*Last updated: April 7, 2026*

</div>