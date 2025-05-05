use solana_program::{
    account_info::AccountInfo,
    entrypoint::ProgramResult,
    program::{invoke, invoke_signed},
    program_error::ProgramError,
    pubkey::Pubkey,
    system_instruction,
    sysvar::{rent::Rent, Sysvar},
};

pub fn create_or_allocate_account_raw<'a>(
    program_id: &Pubkey,
    new_account_info: &AccountInfo<'a>,
    rent_sysvar_info: &AccountInfo<'a>,
    system_program_info: &AccountInfo<'a>,
    payer_info: &AccountInfo<'a>,
    size: usize,
    signer_seeds: &[&[u8]],
) -> ProgramResult {
    let rent = &Rent::from_account_info(rent_sysvar_info)?;
    let required_lamports = rent.minimum_balance(size);

    if new_account_info.lamports() < required_lamports {
        let lamports_diff = required_lamports.saturating_sub(new_account_info.lamports());
        invoke(
            &system_instruction::transfer(payer_info.key, new_account_info.key, lamports_diff),
            &[
                payer_info.clone(),
                new_account_info.clone(),
                system_program_info.clone(),
            ],
        )?;
    }

    invoke_signed(
        &system_instruction::allocate(new_account_info.key, size as u64),
        &[new_account_info.clone(), system_program_info.clone()],
        &[signer_seeds],
    )?;

    invoke_signed(
        &system_instruction::assign(new_account_info.key, program_id),
        &[new_account_info.clone(), system_program_info.clone()],
        &[signer_seeds],
    )?;

    Ok(())
}

pub fn assert_owned_by(account: &AccountInfo, owner: &Pubkey) -> ProgramResult {
    if account.owner != owner {
        Err(ProgramError::IllegalOwner)
    } else {
        Ok(())
    }
}

/* // Unused helper
pub fn assert_rent_exempt(rent: &Rent, account_info: &AccountInfo) -> ProgramResult {
    if !rent.is_exempt(account_info.lamports(), account_info.data_len()) {
        Err(ProgramError::AccountNotRentExempt)
    } else {
        Ok(())
    }
}
*/

/* // Unused trait and helpers - IsInitialized trait is defined in program_pack
pub fn assert_uninitialized<T: IsInitialized>(account: &T) -> ProgramResult {
    if account.is_initialized() {
        Err(ProgramError::AccountAlreadyInitialized)
    } else {
        Ok(())
    }
}

pub fn assert_initialized<T: IsInitialized>(account: &T) -> ProgramResult {
    if !account.is_initialized() {
        Err(ProgramError::UninitializedAccount)
    } else {
        Ok(())
    }
}

pub trait IsInitialized {
    fn is_initialized(&self) -> bool;
}
*/

/* // Unused helper
pub fn calculate_stake_rewards(
    stake_amount: u64,
    total_staked: u64,
    total_rewards: u64,
) -> Option<u64> {
    if total_staked == 0 {
        return Some(0);
    }
    stake_amount
        .checked_mul(total_rewards)?
        .checked_div(total_staked)
}
*/

/* // Unused helper - logic is now inline in processor
pub fn calculate_withdraw_amount(
    shares: u64,
    total_shares: u64,
    total_staked: u64,
) -> Option<u64> {
    if total_shares == 0 {
        return Some(0);
    }
    shares
        .checked_mul(total_staked)?
        .checked_div(total_shares)
}
*/ 