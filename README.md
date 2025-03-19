# **ZeroXBridge Sequencer**  

🚀 **ZeroXBridge Sequencer** is a **Rust-based system** that manages **cross-chain deposits and withdrawals** between **Ethereum (L1)** and **Starknet (L2)**. It ensures secure, **trustless synchronization** of user balances using **Merkle Trees** and **Zero-Knowledge Proofs (ZKPs)**.

---

## **🔍 Overview**  

The sequencer is responsible for:  

- **Processing deposit requests on L1** and enabling users to claim pegged tokens on L2.  
- **Processing withdrawal requests on L2** and enabling users to redeem assets on L1.  
- **Maintaining a Merkle Tree** that tracks deposits and withdrawals.  
- **Generating STARK proofs** for verification.  
- **Relaying transactions** to Ethereum and Starknet.  

---

## **⚙️ System Architecture**  

The sequencer is divided into four core services:

### **1️⃣ API Service**  

- Exposes endpoints to **accept deposit and withdrawal requests**.  
- Stores requests in a **PostgreSQL database**.  
- Returns a **commitment hash** for tracking.  

### **2️⃣ Queue Service**  

- Fetches **pending transactions** from the database.  
- Pushes transactions into the **L1 or L2 processing queue**:  
  - **L1 Queue → Primarily deposit requests**.  
  - **L2 Queue → Primarily withdrawal requests**.  
- **Delays & Retries** prevent excessive blockchain queries.  
- **Performs validation checks before sending to the Proof Generation Service**.  

### **3️⃣ Proof Generation Service**  

- Uses `stwo` or `stone` prover to generate **STARK proofs**.  
- Works with **Merkle Roots from L1 and L2**.  
- **Only processes proofs** (no blockchain validation).  

### **4️⃣ Relayer Service**  

- Fetches **"ready for relay" transactions**.  
- Sends transactions to **Ethereum (L1)** or **Starknet (L2)**.  
- Waits for **on-chain confirmation** before marking requests as `complete`.  

---

## **🔄 How It Works**  

### **📥 Deposits (L1 → L2)**  

1. **User deposits funds on L1**.  
2. **API Service** stores the deposit request (`pending` status).  
3. **Queue Service** picks up the request, waits, and validates:  
   - **Commitment hash exists on L1**.  
   - **Merkle Root has been updated**.  
4. **If valid**, request is sent to **Proof Generation Service**.  
5. **Proof Generation Service**:  
   - Generates a **STARK proof** for deposit inclusion.  
6. **Relayer Service**:  
   - Bundles transaction data.  
   - Sends the proof to **L2**.  
7. **L2 verification succeeds**, and the user can **claim their pegged tokens**.  

---

### **📤 Withdrawals (L2 → L1)**  

1. **User requests withdrawal on L2**.  
2. **API Service** stores the withdrawal request (`pending` status).  
3. **Queue Service** picks up the request, waits, and validates:  
   - **Commitment hash exists on L2**.  
   - **Merkle Root has been updated**.  
4. **If valid**, request is sent to **Proof Generation Service**.  
5. **Proof Generation Service**:  
   - Generates a **STARK proof** for withdrawal inclusion.  
6. **Relayer Service**:  
   - Bundles transaction data.  
   - Sends the proof to **L1**.  
7. **L1 verification succeeds**, and the user can **claim their tokens**.  

---

## **🗂 Project Structure**  

```
zeroXBridge-sequencer/
│── src/
│   ├── api/                 # API Service (Handles deposit/withdrawal requests)
│   │   ├── routes.rs        # API endpoints
│   │   ├── handlers.rs      # Request processing logic
│   │   ├── database.rs      # DB connections
│   ├── queue/               # Queue Service (Processes pending commitments)
│   │   ├── l1_queue.rs      # Handles L1 deposit requests
│   │   ├── l2_queue.rs      # Handles L2 withdrawal requests
│   ├── proof_generator/     # Proof Generation Service (ZKP computation)
│   │   ├── stark_prover.rs  # Generates proofs using `stwo` or `stone`
│   ├── relayer/             # Relayer Service (Sends proofs to L1/L2)
│   │   ├── ethereum_relayer.rs  # Sends proofs to Ethereum
│   │   ├── starknet_relayer.rs  # Sends proofs to Starknet
│   ├── merkle_tree.rs       # Merkle tree implementation
│   ├── main.rs              # Main entry point
│── tests/                   # Unit & integration tests
│── config.toml                 # Config files (DB, RPC endpoints, etc.)
│── scripts/                 # Deployment & management scripts
│── README.md                # Documentation
│── Cargo.toml               # Rust dependencies
```

---

## **⚡ Setup & Run**  

### **1️⃣ Install Dependencies**  

```bash
cargo build
```

### **2️⃣ Start the Sequencer**  

```bash
cargo run
```

---

## **🤝 How to Contribute**  

### **1️⃣ Pick an Issue**  

- Browse open issues.  
- Ask to be **assigned** to an issue.  

### **2️⃣ Implement & Test**  

- Follow the **project structure**.  
- Write **unit tests** for your feature.  
- Run `cargo test` to verify your changes.  

### **3️⃣ Submit a PR**  

- Link your PR to the issue.  
- Provide **clear documentation**.  
- Include **logs/test results** in your PR description.  

### **4️⃣ Get Reviewed & Merged**  

- Maintain **clean, readable code**.  
- Respond to feedback and **iterate as needed**.  

---

## **🎯 Final Notes**  

- High-quality contributions are **eligible for OnlyDust Retroactive Grant Funding**.  
- Minor fixes (typos, small tweaks) **will not be rewarded**.  
- Code quality **matters more than speed**—ensure proper tests and documentation.  

---

🚀 **Yo, Let’s Go!**  
🔥 **ZeroXBridge Sequencer is ready for contributors!**  

Drop into the issues section and let’s build! 🎯🔥
