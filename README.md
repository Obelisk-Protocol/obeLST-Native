# obeSOL LST - Solana Stake Pool Program

This is a Solana program implementing a simplified native stake pool. It allows users to stake SOL, receive pool tokens (obeSOL), and earn staking rewards. The pool delegates the staked SOL to a single, pre-defined validator.

**Disclaimer:** This code is likely for educational or experimental purposes. Native staking and stake pool logic can be complex. Ensure thorough testing and auditing before deploying any funds.

## ✨ Features

*   **Initialize Pool:** Sets up the stake pool with a name, fee percentage, and the designated validator vote account. Creates necessary PDAs for pool state, token mint, stake/withdraw authorities.
*   **Stake:** Users deposit SOL and receive obeSOL pool tokens proportional to their contribution. The program creates a stake account PDA for the user and delegates the SOL to the pool's designated validator.
*   **Unstake:** Users burn their obeSOL tokens to initiate the unstaking process. The corresponding stake account is deactivated.
*   **Withdraw Stake:** After the stake account deactivation cooldown period, users can withdraw their original SOL principal.
*   **Claim Rewards:** Periodically callable (likely off-chain) to harvest staking rewards from the validator's stake account, mint new pool tokens representing the rewards, and distribute them proportionally to token holders (implicitly by updating the pool's total SOL / total shares ratio). Fees are deducted and sent to the treasury account.

## 🏗️ Program Structure

*   `src/lib.rs`: Entrypoint, routing instructions to the processor.
*   `src/processor.rs`: Contains the core logic for each instruction.
*   `src/instruction.rs`: Defines the program's instructions and their expected accounts.
*   `src/state.rs`: Defines the `StakePool` account structure used to store pool configuration and state.
*   `src/error.rs`: Defines custom program errors.
*   `src/utils.rs`: Helper functions (e.g., account creation).
*   `src/security.rs`: Potential security-related checks or utilities (contents not fully reviewed).

## 🛠️ Building

1.  Ensure you have Rust and the Solana CLI tools installed.
2.  Navigate to the `programs/obe-sol-native` directory.
3.  Run: `cargo build-sbf`

This will produce the program binary (`obe_sol.so`) in the `target/deploy/` directory.

## 🚀 Deployment

Use the Solana CLI to deploy the program:

```bash
solana program deploy target/deploy/obe_sol.so
```

Note the program ID after deployment.

## 📝 Usage (High-Level)

Interaction with the program typically involves sending transactions with specific instructions and account lists via a client application (e.g., using JavaScript/TypeScript with `@solana/web3.js`).

1.  **Initialization:** Call the `Initialize` instruction with the required accounts (authority, pool PDA, mint PDA, fee accounts, etc.) and parameters (name, fee, validator vote pubkey).
2.  **Staking:** Call the `Stake` instruction with the user's account, the stake pool account, user's token account, the derived user stake account PDA, and the amount of SOL to stake.
3.  **Unstaking:** Call the `Unstake` instruction with the user's account, stake pool, user token account, pool mint, the derived user stake account PDA, and the amount of pool tokens to unstake.
4.  **Withdrawing:** After cooldown, call `WithdrawStake` with the user account, stake pool, user stake account PDA, and withdraw authority PDA.
5.  **Claiming Rewards:** Call `ClaimRewards` (likely via a keeper bot) with necessary accounts including the validator stake account and treasury account.

*(Refer to `src/instruction.rs` for the precise account lists required for each instruction)*

## 📜 License

This program uses the MIT License (as specified in `Cargo.toml`). 