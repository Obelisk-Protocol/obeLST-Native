use solana_program::{
    account_info::AccountInfo,
    program_error::ProgramError,
    pubkey::Pubkey,
    clock::Clock,
    sysvar::Sysvar,
};
use crate::{
    error::StakePoolError,
    state::{StakePool, ValidatorList},
};

pub struct SecurityManager;

impl SecurityManager {
    pub fn verify_admin(
        admin_info: &AccountInfo,
        stake_pool: &StakePool,
    ) -> Result<(), ProgramError> {
        if !admin_info.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }
        if stake_pool.authority != *admin_info.key {
            return Err(StakePoolError::InvalidAuthority.into());
        }
        Ok(())
    }

    pub fn verify_stake_authority(
        authority_info: &AccountInfo,
        stake_pool: &StakePool,
    ) -> Result<(), ProgramError> {
        if !authority_info.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }
        if stake_pool.stake_authority != *authority_info.key {
            return Err(StakePoolError::InvalidAuthority.into());
        }
        Ok(())
    }

    pub fn verify_withdraw_authority(
        authority_info: &AccountInfo,
        stake_pool: &StakePool,
    ) -> Result<(), ProgramError> {
        if !authority_info.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }
        if stake_pool.withdraw_authority != *authority_info.key {
            return Err(StakePoolError::InvalidAuthority.into());
        }
        Ok(())
    }

    pub fn verify_not_paused(stake_pool: &StakePool) -> Result<(), ProgramError> {
        if stake_pool.paused {
            return Err(StakePoolError::PoolPaused.into());
        }
        Ok(())
    }

    pub fn verify_stake_amount(
        amount: u64,
        stake_pool: &StakePool,
    ) -> Result<(), ProgramError> {
        if amount < stake_pool.min_stake {
            return Err(StakePoolError::StakeTooSmall.into());
        }
        if amount > stake_pool.max_stake {
            return Err(StakePoolError::StakeTooLarge.into());
        }
        Ok(())
    }

    pub fn verify_validator_stake_limit(
        validator_list: &ValidatorList,
        validator_index: usize,
        amount: u64,
    ) -> Result<(), ProgramError> {
        const MAX_VALIDATOR_STAKE_PERCENTAGE: u64 = 10; // 10% max per validator
        
        let total_stake: u64 = validator_list.validators.iter()
            .map(|v| v.active_stake_lamports)
            .sum();

        let validator = &validator_list.validators[validator_index];
        let new_validator_stake = validator.active_stake_lamports
            .checked_add(amount)
            .ok_or(StakePoolError::CalculationFailure)?;

        let max_allowed = total_stake
            .checked_mul(MAX_VALIDATOR_STAKE_PERCENTAGE)
            .ok_or(StakePoolError::CalculationFailure)?
            .checked_div(100)
            .ok_or(StakePoolError::CalculationFailure)?;

        if new_validator_stake > max_allowed {
            return Err(StakePoolError::ValidatorStakeLimitExceeded.into());
        }

        Ok(())
    }

    pub fn verify_unstake_cooldown(
        last_stake_timestamp: i64,
        current_time: i64,
    ) -> Result<(), ProgramError> {
        const MINIMUM_COOLDOWN_PERIOD: i64 = 2 * 24 * 60 * 60; // 2 days in seconds
        
        if current_time - last_stake_timestamp < MINIMUM_COOLDOWN_PERIOD {
            return Err(StakePoolError::UnstakeCooldownNotMet.into());
        }
        Ok(())
    }

    pub fn verify_account_ownership(
        account_info: &AccountInfo,
        expected_owner: &Pubkey,
    ) -> Result<(), ProgramError> {
        if account_info.owner != expected_owner {
            return Err(StakePoolError::InvalidAccountOwner.into());
        }
        Ok(())
    }

    pub fn verify_all_signers(signers: &[&AccountInfo]) -> Result<(), ProgramError> {
        for signer in signers {
            if !signer.is_signer {
                return Err(ProgramError::MissingRequiredSignature);
            }
        }
        Ok(())
    }

    pub fn derive_program_address(
        program_id: &Pubkey,
        seeds: &[&[u8]],
    ) -> Result<(Pubkey, u8), ProgramError> {
        Pubkey::find_program_address(seeds, program_id)
            .map_err(|_| StakePoolError::InvalidProgramAddress.into())
    }

    pub fn verify_program_derived_address(
        address: &Pubkey,
        program_id: &Pubkey,
        seeds: &[&[u8]],
        bump_seed: u8,
    ) -> Result<(), ProgramError> {
        let expected_address = Pubkey::create_program_address(
            &[&seeds[..], &[bump_seed]].concat(),
            program_id,
        ).map_err(|_| StakePoolError::InvalidProgramAddress)?;

        if *address != expected_address {
            return Err(StakePoolError::InvalidProgramAddress.into());
        }
        Ok(())
    }
} 