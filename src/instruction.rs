use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    // instruction::{AccountMeta, Instruction}, // Unused
    pubkey::Pubkey,
    // system_program, // Unused
    // sysvar, // Unused
};
// use crate::state::ValidatorStatus; // Removed as ValidatorStatus is removed

#[derive(BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq)]
pub enum StakePoolInstruction {
    /// Initialize a new stake pool
    /// 
    /// Accounts expected:
    /// 0. `[signer, writable]` Stake pool authority
    /// 1. `[writable]` Stake pool account to create
    /// 2. `[writable]` Pool token mint
    /// 3. `[writable]` Manager fee account
    /// 4. `[writable]` Treasury fee account
    /// 5. `[]` Helius validator vote account pubkey (passed in instruction data, not as account)
    /// 6. `[]` Token program id
    /// 7. `[]` System program id
    /// 8. `[]` Rent sysvar
    Initialize {
        /// Pool name
        name: String,
        /// Fee percentage (0-100)
        fee_percentage: u8,
        /// Pubkey of the single Helius validator vote account
        helius_validator_vote: Pubkey, 
    },

    /// Stake SOL in the pool
    /// 
    /// Accounts expected:
    /// 0. `[signer, writable]` User account
    /// 1. `[writable]` Stake pool
    /// 2. `[writable]` User token account
    /// 3. `[writable]` Pool token mint
    /// 4. `[writable]` Stake account (derived from user & pool)
    /// 5. `[]` Token program id
    /// 6. `[]` Stake program id
    /// 7. `[]` System program id
    /// 8. `[]` Rent sysvar
    /// 9. `[]` Clock sysvar
    /// 10. `[]` Stake history sysvar
    /// 11. `[]` Helius validator vote account (read-only)
    Stake {
        /// Amount of SOL to stake
        amount: u64,
    },

    /// Unstake SOL from the pool
    /// 
    /// Accounts expected:
    /// 0. `[signer, writable]` User account
    /// 1. `[writable]` Stake pool
    /// 2. `[writable]` User token account
    /// 3. `[writable]` Pool token mint
    /// 4. `[writable]` Stake account (derived from user & pool)
    /// 5. `[]` Token program id
    /// 6. `[]` Stake program id
    /// 7. `[]` Clock sysvar
    Unstake {
        /// Amount of pool tokens to unstake
        amount: u64,
    },

    /// Claim rewards
    /// 
    /// Accounts expected:
    /// 0. `[signer, writable]` User account (who is claiming)
    /// 1. `[writable]` Stake pool
    /// 2. `[writable]` User token account (to receive rewards)
    /// 3. `[writable]` Pool token mint
    /// 4. `[writable]` Treasury fee account (to receive fees)
    /// 5. `[writable]` Stake account (for the Helius validator - needs to be passed)
    /// 6. `[]` Token program id
    /// 7. `[]` Clock sysvar
    ClaimRewards,

    /// Withdraw SOL from a deactivated stake account
    /// Requires the stake account to be fully deactivated (cooldown passed).
    /// 
    /// Accounts expected:
    /// 0. `[signer, writable]` User account (receives SOL)
    /// 1. `[writable]` Stake pool (read-only, for withdraw authority derivation)
    /// 2. `[writable]` Stake account (PDA derived from user & pool - withdraw from)
    /// 3. `[]` Stake pool withdraw authority PDA (derived from pool)
    /// 4. `[]` Stake program id
    /// 5. `[]` Clock sysvar
    /// 6. `[]` Stake history sysvar
    WithdrawStake,

    // Removed AddValidator, RemoveValidator, UpdateValidatorStatus
}

// REMOVED ENTIRE MANUAL IMPLEMENTATION OF UNPACK
// The #[derive(BorshDeserialize)] handles this correctly.
/*
impl StakePoolInstruction {
    /// Unpacks a byte buffer into a StakePoolInstruction
    pub fn unpack(input: &[u8]) -> Result<Self, solana_program::program_error::ProgramError> {
        // ... removed all code ...
    }

    // --- Helpers for unpack (MUST remain uncommented) ---
    // ... removed all helpers ...
}
*/

// use std::mem::size_of; // Unused import 