use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    program_pack::{IsInitialized, Sealed},
    pubkey::Pubkey,
};

#[derive(BorshSerialize, BorshDeserialize, Debug, PartialEq)]
pub struct StakePool {
    /// Pool version for upgrade compatibility
    pub version: u8,
    
    /// The pool's authority account
    pub authority: Pubkey,
    
    /// Authority that can stake
    pub stake_authority: Pubkey,
    
    /// Authority that can withdraw
    pub withdraw_authority: Pubkey,
    
    /// Pool name
    pub name: String,
    
    /// Fee percentage (0-100)
    pub fee_percentage: u8,
    
    /// Total SOL staked
    pub total_staked: u64,
    
    /// Total shares issued
    pub total_shares: u64,
    
    /// Pool token mint
    pub mint: Pubkey,
    
    /// Reserve account
    pub reserve: Pubkey,
    
    /// Pubkey of the single Helius validator vote account
    pub helius_validator_vote: Pubkey,
    
    /// Manager fee account (Kept for potential future use, but fees go to treasury currently)
    pub manager_fee_account: Pubkey,
    
    /// Treasury fee account
    pub treasury_fee_account: Pubkey,
    
    /// Is the pool paused?
    pub paused: bool,
    
    /// Last epoch when rewards were collected
    pub last_update_epoch: u64,
    
    /// Minimum stake amount
    pub min_stake: u64,
    
    /// Maximum stake amount
    pub max_stake: u64,

    /// Bump seed for the stake authority PDA
    pub stake_authority_bump_seed: u8,

    /// Bump seed for the withdraw authority PDA
    pub withdraw_authority_bump_seed: u8,

    /// Reserved space for future features (NGO donations, service payments)
    pub reserved: [u8; 62], // Reduced size to accommodate bumps
}

impl Default for StakePool {
    fn default() -> Self {
        StakePool {
            version: 0, // Usually 0 indicates uninitialized, but using for default
            authority: Pubkey::default(),
            stake_authority: Pubkey::default(),
            withdraw_authority: Pubkey::default(),
            name: String::new(), // Default empty string
            fee_percentage: 0,
            total_staked: 0,
            total_shares: 0,
            mint: Pubkey::default(),
            reserve: Pubkey::default(),
            helius_validator_vote: Pubkey::default(),
            manager_fee_account: Pubkey::default(),
            treasury_fee_account: Pubkey::default(),
            paused: false,
            last_update_epoch: 0,
            min_stake: 0,
            max_stake: 0,
            stake_authority_bump_seed: 0,
            withdraw_authority_bump_seed: 0,
            reserved: [0u8; 62], // Default zeroed array
        }
    }
}

impl Sealed for StakePool {}

impl IsInitialized for StakePool {
    fn is_initialized(&self) -> bool {
        self.version > 0
    }
}

/* // Unused struct
#[derive(BorshSerialize, BorshDeserialize, Debug, Default, PartialEq)]
pub struct UnstakeInfo {
    /// Owner of unstake request
    pub owner: Pubkey,
    
    /// Amount of SOL to unstake
    pub amount: u64,
    
    /// Amount of pool tokens to burn
    pub pool_tokens: u64,
    
    /// Epoch when unstake was requested
    pub epoch_requested: u64,
    
    /// Validator to unstake from (Will always be Helius validator)
    pub validator: Pubkey,

    /// Reserved space for future features (service agreements, NGO allocations)
    pub reserved: [u8; 64],
}

impl Sealed for UnstakeInfo {}

impl IsInitialized for UnstakeInfo {
    fn is_initialized(&self) -> bool {
        self.amount > 0
    }
} 
*/ 