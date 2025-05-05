// #![deny(missing_docs)]

use solana_program::{
    account_info::AccountInfo,
    entrypoint,
    entrypoint::ProgramResult,
    pubkey::Pubkey,
    msg,
};

mod error;
mod instruction;
mod processor;
mod state;
mod utils;

use crate::{processor::Processor};

// Program entrypoint
#[cfg(not(feature = "no-entrypoint"))]
entrypoint!(process_instruction);

/// Program entrypoint implementation
pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    msg!("ObeSOL program entrypoint");
    if let Err(error) = Processor::process(program_id, accounts, instruction_data) {
        // catch the error so we can print it
        msg!("Error: {:?}", error);
        return Err(error);
    }
    Ok(())
}
