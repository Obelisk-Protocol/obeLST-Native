use num_derive::FromPrimitive;
use solana_program::{
    decode_error::DecodeError,
    program_error::ProgramError,
    // msg, // Unused
};
use thiserror::Error;

#[derive(Error, Debug, Copy, Clone, FromPrimitive)]
pub enum StakePoolError {
    #[error("Invalid instruction")]
    InvalidInstruction,
    
    #[error("Invalid fee percentage")]
    InvalidFeePercentage,
    
    #[error("Pool name must be between 3 and 32 characters")]
    InvalidPoolName,
    
    #[error("Invalid mint authority")]
    InvalidMintAuthority,
    
    #[error("Invalid fee account")]
    InvalidFeeAccount,
    
    #[error("Stake amount must be greater than minimum stake")]
    StakeTooSmall,
    
    #[error("Stake amount must be less than maximum stake")]
    StakeTooLarge,
    
    #[error("Pool is paused")]
    PoolPaused,
    
    #[error("Math operation overflow")]
    MathOverflow,
    
    #[error("Insufficient balance")]
    InsufficientBalance,
    
    #[error("Cooldown period not elapsed")]
    CooldownNotElapsed,
    
    #[error("Invalid owner")]
    InvalidOwner,
    
    #[error("Account not initialized")]
    UninitializedAccount,

    #[error("Invalid program address")]
    InvalidProgramAddress,

    #[error("Invalid authority")]
    InvalidAuthority,

    #[error("Invalid account owner")]
    InvalidAccountOwner,

    #[error("Unstake cooldown period not met")]
    UnstakeCooldownNotMet,

    #[error("Calculation failed")]
    CalculationFailure,

    #[error("Already claimed rewards this epoch")]
    AlreadyClaimedThisEpoch,

    #[error("No rewards to collect")]
    NoRewardsToCollect,

    #[error("Wrong stake state")]
    WrongStakeState,

    #[error("Stake account not delegated to the pool validator")]
    InvalidStakeAccountDelegation,

    #[error("Incorrect withdraw authority provided")]
    InvalidWithdrawAuthority,

    #[error("Stake account specified withdrawer does not match pool authority")]
    InvalidStakeAccountAuthority,

    #[error("Stake account is not deactivated")]
    StakeNotDeactivated,

    #[error("Stake account cooldown period has not passed")]
    CooldownNotPassed,

    #[error("Invalid stake authority")]
    InvalidStakeAuthority,
}

impl From<StakePoolError> for ProgramError {
    fn from(e: StakePoolError) -> Self {
        ProgramError::Custom(e as u32)
    }
}

impl<T> DecodeError<T> for StakePoolError {
    fn type_of() -> &'static str {
        "StakePoolError"
    }
} 