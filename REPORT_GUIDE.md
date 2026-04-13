# Code4Arena LayerZero Audit Report Writing Guide

## 1. Understanding the Audit Landscape

### Prize Distribution
- **HM (High/Medium) Awards**: Up to $89,760 (only if valid High or Medium findings exist)
- **QA Awards**: $3,740 (for low-risk and gas optimization findings)
- **Judge Awards**: $7,000 (for judges)
- **Scout Awards**: $500 (for initial discovery)

### Critical Rules
- High or Medium findings downgraded to Low become ineligible for awards
- Low-risk findings cannot be upgraded from QA reports to Medium/High
- Choose risk levels carefully during submission
- Test files and out-of-scope files are NOT eligible for findings

---

## 2. Key Areas of Focus for Maximum Impact

### 2.1 DVN / ULN Interaction Security
**Why it matters**: This is explicitly listed as a critical concern

**What to analyze**:
- Message verification flow between DVNs and ULN302
- Signature validation and replay attack prevention
- Nonce tracking and state management
- Worker authentication patterns

**High-severity findings here** earn the most awards. Focus on:
- Authorization bypass vulnerabilities
- State inconsistency allowing double-processing
- Signature scheme weaknesses

### 2.2 Soroban Storage and TTL Handling
**Why it matters**: Unique to Stellar's Soroban platform

**What to analyze**:
- TTL (Time-To-Live) expiration edge cases
- Storage read limit enforcement (200 per transaction)
- Eager inbound nonce tracking implementation
- Pending nonce list correctness
- TTL extension strategies and gaps

**High-severity findings here**: Storage corruption, permanent loss of funds, griefing vectors

### 2.3 Censorship Resistance
**Why it matters**: Core protocol invariant

**What to analyze**:
- Can Endpoint owner censor messages? (Should be impossible)
- Can ULN owner censor through native payment path?
- Can Treasury owner block user messages?
- Permission models for configuration changes
- Delegation patterns

**Critical invariant to verify**: "Only delegate or OApp can set configs for OApp"

### 2.4 bytes32 Address Format Conversion
**Why it matters**: LayerZero uses fixed bytes32; Stellar uses variable-length addresses

**What to analyze**:
- Address encoding/decoding logic
- Collision possibilities
- Padding and truncation edge cases
- Roundtrip conversion correctness
- Cross-chain address verification

**High-severity findings**: Address collision, message routing to wrong recipient

### 2.5 Abstract Account Pattern Security
**Why it matters**: Non-standard Soroban implementation

**What to analyze**:
- `__check_auth` implementation in DVN and Executor
- Custom account interface usage
- Authorization state management
- Interaction with message library flows

**High-severity findings**: Authentication bypass, unauthorized message execution

### 2.6 Pull-Mode Message Delivery
**Why it matters**: Unique design to work around Soroban's reentrancy prohibition

**What to analyze**:
- OApp's `Endpoint.clear()` call flow
- ABA pattern correctness (executor appears twice in call stack)
- Message state transitions
- Race conditions in delivery model

**High-severity findings**: Message loss, double execution, delivery failures

---

## 3. Report Structure for Maximum Awards

### 3.1 Title (Critical for Clarity)
```
[HIGH/MEDIUM/LOW] [Component Name]: Concise Vulnerability Description
```
Examples:
- `[HIGH] ULN302 Send Path: Integer Overflow in Fee Calculation`
- `[MEDIUM] DVN Authorization: Signature Reuse Across Chains`
- `[LOW] Storage TTL: Incorrect Expiration Calculation`

### 3.2 Vulnerability Description

**Format that wins awards**:

1. **Impact Statement** (First paragraph)
   - What breaks?
   - How much value at risk?
   - User/protocol affected?

2. **Root Cause** (2-3 paragraphs)
   - Show the exact code location
   - Explain the logical flaw
   - Use code snippets from the actual files

3. **Attack Scenario** (Detailed example)
   - Step-by-step exploitation path
   - Prerequisites
   - Expected vs actual behavior

4. **Proof of Concept** (If possible)
   - Rust/Soroban code demonstrating the issue
   - Can reference test cases if applicable

### 3.3 What Makes Reports Award-Winning

✅ **Specific and Verifiable**
- Reference exact file paths and line numbers
- Show actual code snippets
- Provide reproducible test cases

✅ **Impact-Focused**
- Explain the security consequence, not just the bug
- Calculate potential losses (funds at risk, griefing cost)
- Describe affected users/transactions

✅ **Chain-Specific Knowledge**
- Demonstrate understanding of Soroban constraints
- Reference Stellar-specific behaviors
- Show knowledge of TTL mechanisms

✅ **Deep Technical Analysis**
- Go beyond "this could be an issue"
- Prove the vulnerability with evidence
- Show why existing protections fail

❌ **Avoid These**
- Vague descriptions ("potential security issue")
- Unverifiable claims without code references
- Findings in out-of-scope files
- Duplicates of known issues
- Theoretical issues without exploitability

---

## 4. Scoped Files (Where to Focus)

### Critical Components (Highest Reward Potential)
1. **ULN302 Message Library** (~700 SLoC)
   - `send_uln.rs` (264 SLoC) - Fee calculation, message queueing
   - `receive_uln.rs` (117 SLoC) - Message verification, nonce tracking
   - `types.rs` (148 SLoC) - Data structure definitions

2. **EndpointV2** (~1000 SLoC)
   - `endpoint_v2.rs` (238 SLoC) - Core message routing
   - `messaging_channel.rs` (214 SLoC) - Channel state management
   - `message_lib_manager.rs` (234 SLoC) - Library configuration

3. **DVN (Decentralized Verification Network)** (~260 SLoC)
   - `dvn.rs` (133 SLoC) - DVN core logic, multisig verification
   - `auth.rs` (96 SLoC) - Authorization and signature validation

4. **Worker Infrastructure** (~300+ SLoC)
   - `worker.rs` (210 SLoC) - Fee and execution management
   - Storage and TTL handling

5. **Common Macros & Utilities** (~800+ SLoC)
   - `storage.rs` (368 SLoC) - Storage access patterns
   - `rbac.rs` (65 SLoC + 170 SLoC) - Authorization checks
   - `buffer_reader.rs` / `buffer_writer.rs` - Serialization

### Medium Priority
- Treasury contracts (~120 SLoC) - Fee collection and owner controls
- Common utilities - Multisig validation, upgradeable patterns
- Error handling implementations

### Lower Priority (Still Valid)
- Blocked message library (~42 SLoC)
- Interface definitions
- Event logging

---

## 5. Common Vulnerability Patterns in LayerZero V2

### Access Control Violations
- Insufficient permission checks before state changes
- Missing owner/delegate validation
- Configuration changes without proper authorization

### State Management Issues
- Race conditions in multi-step message flows
- Incorrect nonce handling allowing replays
- TTL expiration not properly managed

### Encoding/Decoding Flaws
- Address format conversion bugs
- Packet serialization errors
- Buffer overflow in custom codec (packet_codec_v1.rs)

### Fee & Treasury Issues
- Incorrect fee calculations
- Missing overflow checks
- Improper fund handling

### Soroban-Specific Issues
- Storage read limit violations (exceeding 200 per transaction)
- TTL extension failures
- Incorrect state transitions under reentrancy constraints

---

## 6. Report Submission Checklist

Before submitting, verify:

- [ ] **Scope Verification**: File is in scope.txt (NOT in out_of_scope.txt)
- [ ] **Code References**: Exact file paths and line numbers provided
- [ ] **Reproducibility**: Clear steps to verify the finding
- [ ] **Risk Level**: Correctly assessed (no borderline Medium->Low downgrades)
- [ ] **Impact**: Quantified in terms of funds/functionality at risk
- [ ] **Root Cause**: Clearly explained with code snippets
- [ ] **No Duplicates**: Checked against other submissions (if public)
- [ ] **Test Evidence**: Reference to passing/failing tests if available
- [ ] **Soroban Context**: Acknowledges Stellar-specific constraints
- [ ] **Proof of Concept**: Working code or detailed exploitation steps

---

## 7. Risk Level Guidelines

### HIGH-Risk (Maximum Award Potential: ~$2,000+)
- Direct loss of funds possible
- Core protocol invariants violated
- Censorship capability exists
- Permanent state corruption
- System shutdown or freezing

**Examples**:
- Endpoint owner can censor messages
- ULN nonce tracking allows double execution
- DVN can steal fees
- TTL expiration causes message loss

### MEDIUM-Risk (Award Potential: ~$500-$2,000)
- Conditional fund loss (with attacker setup)
- Temporary service disruption
- Configuration corruption
- Griefing or DoS vectors
- Partial violation of invariants

**Examples**:
- Specific user can be censored under conditions
- Fee calculation incorrect in edge cases
- Storage read limit exceeded in certain flows
- TTL extension fails in specific scenarios

### LOW-Risk / QA (Award Potential: ~$100-$500)
- Non-critical issues
- Gas optimization opportunities
- Code quality improvements
- Informational findings
- Spec deviation without impact

**Examples**:
- Unused variables or imports
- Non-optimal gas usage
- Redundant checks
- Missing error messages

---

## 8. Winning Report Examples

### Example 1: HIGH Finding
**Title**: `[HIGH] EndpointV2: Endpoint Owner Can Censor Messages via message_lib_manager Restriction`

**Structure**:
1. Impact: Breaks core invariant "Endpoint owner should not be able to censor messages"
2. Root Cause: message_lib_manager.rs allows owner to disable message libraries without checks
3. Scenario: Owner disables all DVNs for specific OApp, messages fail permanently
4. PoC: Code showing owner call sequence
5. Fix Recommendation: Whitelist mechanism or time-lock

### Example 2: MEDIUM Finding
**Title**: `[MEDIUM] ULN302 Receive Path: Nonce Tracking Allows Replay Attack in Low-Cost Verification`

**Structure**:
1. Impact: Message can be verified multiple times, leading to fund loss
2. Root Cause: Nonce validation in receive_uln.rs doesn't atomic-check-and-set
3. Scenario: Two DVNs submit verification for same nonce, both accepted
4. PoC: Test case with two verification submissions
5. Fix: Atomic nonce increment before external calls

### Example 3: QA Finding
**Title**: `[LOW] ULN302: Missing gas optimization in worker_options validation loop`

**Structure**:
1. Issue: Redundant bounds checking in loop
2. Location: worker_options.rs, lines 45-67
3. Gas Save: ~200 gas per call
4. Recommendation: Move bounds check outside loop

---

## 9. Strategic Tips for Maximum Awards

1. **Focus on the listed "Areas of Concern"**
   - These are explicitly flagged as high-risk
   - Judges expect findings here
   - Higher award multipliers likely

2. **Read the Main Invariants Section**
   - "Endpoint is immutable"
   - "Endpoint owner should not be able to censor messages"
   - "ULN is immutable"
   - "DVN cannot suffer a replay attack"
   - Build reports around proving/disproving these

3. **Understand Soroban Constraints**
   - 200 storage reads per transaction limit
   - No reentrancy - requires pull-model verification
   - TTL expiration for all storage
   - These create unique vulnerability vectors

4. **Look for Edge Cases**
   - Empty inputs, zero values
   - Message boundaries in buffer reading
   - TTL near expiration
   - Storage limits near threshold
   - Address format edge cases

5. **Cross-Chain Thinking**
   - How does Stellar differ from EVM/other chains?
   - What LayerZero invariants might break?
   - How do constraints interact?

6. **Timestamp Reports Early**
   - Judging phase starts after April 14
   - Earlier discoveries may have advantage
   - But accuracy matters more than speed

---

## 10. Resources and Context

### Key Documentation
- **Stellar Docs**: Understanding Soroban platform constraints
- **LayerZero Docs**: V2 protocol specifications
- **Audit README**: Lists trusted roles, main invariants, areas of concern

### Files to Study First
1. `contracts/protocol/stellar/docs/` - Protocol documentation
2. `endpoint-v2/src/endpoint_v2.rs` - Core routing logic
3. `message-libs/uln-302/src/` - Message library implementation
4. `workers/dvn/src/dvn.rs` - Verification logic

### Testing Environment
```bash
cd contracts/protocol/stellar
cargo test                    # Run all tests
cargo test --package uln-302 # Test specific package
```

---

## Summary: The Path to Maximum Awards

1. **Pick High-Impact Area**: DVN/ULN interaction, Soroban constraints, censorship resistance
2. **Deep Code Review**: Multiple passes with security mindset
3. **Prove Exploitability**: Show concrete attack with code
4. **Quantify Impact**: How much value at risk? How severe?
5. **Clear Communication**: Specific files, line numbers, reproduction steps
6. **Risk Assessment**: Honest evaluation of severity level
7. **Submit Early**: Don't wait until last minute

**Target**: Even one solid HIGH finding can earn $1,500-$2,500+. Two MEDIUMs can earn $1,000-$3,000. Build quality over quantity.

Good luck! 🚀
