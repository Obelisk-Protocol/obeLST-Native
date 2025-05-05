use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    program_error::ProgramError,
    program_pack::{IsInitialized, Pack},
    pubkey::Pubkey,
    sysvar::{clock::Clock, rent::Rent, Sysvar},
    msg,
    program::{invoke, invoke_signed},
    stake::{
        instruction as stake_instruction,
        state::{Authorized, Lockup, StakeStateV2},
    },
    system_instruction,
};
use borsh::{BorshSerialize, BorshDeserialize};
use crate::{
    error::StakePoolError,
    instruction::StakePoolInstruction,
    state::{StakePool},
    utils::{assert_owned_by, create_or_allocate_account_raw},
};

pub struct Processor {}

impl Processor {
    /// Processes instructions according to the instruction data provided.
    pub fn process(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        instruction_data: &[u8],
    ) -> ProgramResult {
        // Use standard Borsh deserialization derived from the enum
        let instruction = StakePoolInstruction::try_from_slice(instruction_data)
            .map_err(|e| {
                msg!("Failed to deserialize instruction data: {}", e);
                ProgramError::InvalidInstructionData
            })?;

        // Route to the specific instruction processor based on the unpacked instruction.
        match instruction {
            StakePoolInstruction::Initialize { name, fee_percentage, helius_validator_vote } => {
                msg!("Instruction: Initialize");
                Self::process_initialize(program_id, accounts, name, fee_percentage, helius_validator_vote)
            }
            StakePoolInstruction::Stake { amount } => {
                msg!("Instruction: Stake");
                Self::process_stake(program_id, accounts, amount)
            }
            StakePoolInstruction::Unstake { amount } => {
                msg!("Instruction: Unstake");
                Self::process_unstake(program_id, accounts, amount)
            }
            StakePoolInstruction::ClaimRewards => {
                msg!("Instruction: Claim Rewards");
                Self::process_claim_rewards(program_id, accounts)
            }
            StakePoolInstruction::WithdrawStake => {
                msg!("Instruction: Withdraw Stake");
                Self::process_withdraw_stake(program_id, accounts)
            }
        }
    }

    /// Initializes a new ObeSOL stake pool.
    fn process_initialize(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        name: String,
        fee_percentage: u8,
        helius_validator_vote: Pubkey,
    ) -> ProgramResult {
        msg!("Processing Initialize: Creating new stake pool");
        let account_info_iter = &mut accounts.iter();
        
        // --- Account Parsing --- 
        // Load accounts according to the order specified in instruction.rs
        let authority_info = next_account_info(account_info_iter)?; // Pays for setup, becomes initial authority
        let stake_pool_info = next_account_info(account_info_iter)?; // Account to store pool state
        let pool_mint_info = next_account_info(account_info_iter)?; // Mint for the obeSOL tokens
        let manager_fee_info = next_account_info(account_info_iter)?; // Currently unused fee recipient
        let treasury_fee_info = next_account_info(account_info_iter)?; // Receives fees
        let token_program_info = next_account_info(account_info_iter)?; // SPL Token program ID
        let system_program_info = next_account_info(account_info_iter)?; // Needed for account creation
        let rent_info = next_account_info(account_info_iter)?; // Rent sysvar
        let stake_authority_info = next_account_info(account_info_iter)?; // <-- ADDED Account #13

        // --- Validation --- 
        // Ensure the provided authority signed the transaction.
        if !authority_info.is_signer {
            msg!("Authority signature missing");
            return Err(ProgramError::MissingRequiredSignature);
        }
        // Validate business logic constraints.
        if fee_percentage > 100 {
            msg!("Fee percentage must be 0-100");
            return Err(StakePoolError::InvalidFeePercentage.into());
        }
        if name.len() < 3 || name.len() > 32 {
            msg!("Pool name length invalid");
            return Err(StakePoolError::InvalidPoolName.into());
        }

        // --- Stake Pool PDA Derivation & Validation ---
        const poolSeedString: &str = "obelisk_pool_04"; // Use NEW seed for clean initialization
        let (expected_stake_pool_pda, bump_seed) = Pubkey::find_program_address(
            &[authority_info.key.as_ref(), poolSeedString.as_bytes()],
            program_id
        );
        if expected_stake_pool_pda != *stake_pool_info.key {
            msg!("Provided stake pool account {} does not match derived PDA {}", *stake_pool_info.key, expected_stake_pool_pda);
            return Err(ProgramError::InvalidSeeds);
        }
        let stake_pool_signer_seeds = &[
            authority_info.key.as_ref(),
            poolSeedString.as_bytes(),
            &[bump_seed]
        ];

        // --- Pre-calculate Authorities and Create Initial State Object ---
        // Derive authorities FIRST, as they are needed in the StakePool struct
        let (stake_authority, stake_authority_bump) = Pubkey::find_program_address(
            &[b"stake_authority", expected_stake_pool_pda.as_ref()], // Use expected_pda key
            program_id,
        );
        let (withdraw_authority, withdraw_authority_bump) = Pubkey::find_program_address(
            &[b"withdraw_authority", expected_stake_pool_pda.as_ref()], // Use expected_pda key
            program_id,
        );

        let initial_stake_pool = StakePool {
            version: 1,
            authority: *authority_info.key,
            stake_authority: stake_authority,
            withdraw_authority: withdraw_authority,
            name: name.clone(), // Use the provided name
            fee_percentage: fee_percentage,
            total_staked: 0,
            total_shares: 0,
            mint: Pubkey::default(), // Placeholder, set after mint is created
            reserve: Pubkey::default(),
            helius_validator_vote: helius_validator_vote,
            manager_fee_account: *manager_fee_info.key,
            treasury_fee_account: *treasury_fee_info.key,
            paused: false,
            last_update_epoch: Clock::get()?.epoch,
            min_stake: 1_000_000_000,
            max_stake: 1_000_000 * 1_000_000_000,
            stake_authority_bump_seed: stake_authority_bump,
            withdraw_authority_bump_seed: withdraw_authority_bump,
            reserved: [0u8; 62],
        };

        // --- Serialize the state to get the exact required size --- 
        let serialized_data = initial_stake_pool.try_to_vec()?;
        let required_size = serialized_data.len();
        msg!("Serialized initial StakePool data size: {}", required_size);

        // --- Create Stake Pool Account using CORRECT size --- 
        // let required_size_mem = std::mem::size_of::<StakePool>(); // Calculate size
        // msg!("Calculated StakePool size via std::mem::size_of: {}", required_size_mem); // OLD WRONG METHOD
        msg!("Creating or allocating stake pool account PDA with size: {}", required_size);
        create_or_allocate_account_raw(
            program_id,             // Owner of the new account
            stake_pool_info,        // Account to create/allocate
            rent_info,              // Rent sysvar
            system_program_info,    // System program
            authority_info,         // Payer for rent/creation
            required_size,          // CORRECT Size needed
            stake_pool_signer_seeds, // Seeds for invoke_signed
        )?;

        // --- Write the pre-serialized data to the account --- 
        // Use explicit scope for the first borrow
        {
            let mut account_data = stake_pool_info.data.borrow_mut(); // BORROW 1
            if account_data.len() != required_size {
                msg!("Account data length mismatch after creation! Expected {}, got {}", required_size, account_data.len());
                return Err(ProgramError::AccountDataTooSmall);
            }
            account_data.copy_from_slice(&serialized_data);
            // BORROW 1 ends when account_data goes out of scope here
        }
        msg!("Initialized StakePool data written to account.");

        // --- Mint PDA Derivation & Validation (Depends on stake_pool_info.key) ---
        let (expected_mint_pda, mint_bump_seed) = Pubkey::find_program_address(
            &[stake_pool_info.key.as_ref(), b"mint"], // Use the NOW CREATED stake pool's key
            program_id
        );
        if expected_mint_pda != *pool_mint_info.key {
            msg!("Provided pool mint account {} does not match derived PDA {}", *pool_mint_info.key, expected_mint_pda);
            return Err(ProgramError::InvalidSeeds);
        }
        let mint_signer_seeds = &[
            stake_pool_info.key.as_ref(),
            b"mint",
            &[mint_bump_seed]
        ];

        // --- Create Mint Account --- 
        msg!("Creating or allocating pool mint account PDA");
        create_or_allocate_account_raw(
            &spl_token::id(),
            pool_mint_info,
            rent_info,
            system_program_info,
            authority_info,
            spl_token::state::Mint::LEN,
            mint_signer_seeds,
        )?;

        // --- Update StakePool Data In-Place with Mint Address --- 
        // Calculate the offset of the 'mint' field
        // version(1) + authority(32) + stake_authority(32) + withdraw_authority(32) +
        // name_len(4) + name_bytes(name.len()) + fee(1) + total_staked(8) + total_shares(8) = offset
        let name_len = name.len(); // Use the actual name length used in initial serialization
        let mint_offset = 1 + (3 * 32) + 4 + name_len + 1 + (2 * 8);
        msg!("Calculated mint offset: {}", mint_offset);

        // Borrow mutably AGAIN (should now be safe after previous scope ended)
        // Revert back to direct borrow_mut() as it panics, not returns Result
        let mut account_data_for_mint = stake_pool_info.data.borrow_mut(); // BORROW 2 
        
        // Ensure slice is long enough (should be)
        if account_data_for_mint.len() >= mint_offset + 32 {
             account_data_for_mint[mint_offset..mint_offset + 32].copy_from_slice(&pool_mint_info.key.to_bytes());
             msg!("Updated mint address directly in account data.");
        } else {
            msg!("Error: Account data too small to write mint address at offset {}", mint_offset);
            return Err(ProgramError::AccountDataTooSmall);
        }
        // Drop the mutable borrow explicitly (or let it go out of scope)
        drop(account_data_for_mint);
        // -------------------------------------------------------
        
        // --- Initialize Mint --- 
        msg!("Initializing pool token mint");
        invoke(
            &spl_token::instruction::initialize_mint(
                &spl_token::id(),
                pool_mint_info.key,
                &stake_authority, // Use the derived stake_authority PDA
                None, // No freeze authority
                0,    // Decimals
            )?,
            &[
                token_program_info.clone(),
                pool_mint_info.clone(),
                rent_info.clone(),
            ],
        )?;

        // --- Remove Old Size/Serialization Logs --- 
        // match initial_stake_pool.try_to_vec() { // This was based on the state BEFORE mint was added
        //     Ok(data) => msg!("Calculated serialized StakePool size: {}", data.len()),
        //     Err(e) => msg!("Failed to calculate serialized size: {}", e),
        // };
        // stake_pool.serialize(&mut *stake_pool_info.data.borrow_mut())?; // Done above

        Ok(())
    }

    /// Processes a user's request to stake SOL in the pool.
    fn process_stake(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        amount: u64,
    ) -> ProgramResult {
        msg!("Processing Stake: Amount {}", amount);
        let account_info_iter = &mut accounts.iter();
        
        // --- Account Parsing --- 
        // Order must exactly match the SDK's `keys` array in createStakeInstruction
        // 0. `[signer, writable]` User account (payer for tx, owner of token account)
        let user_info = next_account_info(account_info_iter)?;
        // 1. `[writable]` Stake pool state account
        let stake_pool_info = next_account_info(account_info_iter)?;
        // 2. `[writable]` User's obeSOL token account (ATA)
        let user_token_account_info = next_account_info(account_info_iter)?;
        // 3. `[writable]` Pool token mint (obeSOL mint)
        let pool_mint_info = next_account_info(account_info_iter)?;
        // 4. `[writable]` User's stake account PDA (derived from user, pool, authority)
        let stake_account_info = next_account_info(account_info_iter)?;
        // 5. `[]` Token program id
        let token_program_info = next_account_info(account_info_iter)?;
        // 6. `[]` Stake program id
        let stake_program_info = next_account_info(account_info_iter)?;
        // 7. `[]` System program id
        let system_program_info = next_account_info(account_info_iter)?;
        // 8. `[]` Rent sysvar
        let rent_info = next_account_info(account_info_iter)?;
        // 9. `[]` Clock sysvar
        let clock_info = next_account_info(account_info_iter)?;
        // 10. `[]` Stake History sysvar
        let stake_history_info = next_account_info(account_info_iter)?;
        // 11. `[]` Stake Config ID (Constant address, needed for delegate_stake CPI)
        let stake_config_info = next_account_info(account_info_iter)?;
        // 12. `[]` Helius validator vote account (read-only)
        let helius_validator_vote_info = next_account_info(account_info_iter)?;
        // 13. `[]` Stake Authority account (read-only)
        let stake_authority_info = next_account_info(account_info_iter)?; // <-- ADDED Account #13
        
        // --- Validation --- 
        // Verify signer
        if !user_info.is_signer {
            msg!("User signature missing");
            return Err(ProgramError::MissingRequiredSignature);
        }
        // Verify account ownerships
        msg!("Checking ownership of stake_pool_info ({}) -> Expected Owner: {}", stake_pool_info.key, program_id);
        msg!(" -> Actual Owner: {}", stake_pool_info.owner);
        assert_owned_by(stake_pool_info, program_id)?;

        msg!("Checking ownership of pool_mint_info ({}) -> Expected Owner: {}", pool_mint_info.key, spl_token::id());
        msg!(" -> Actual Owner: {}", pool_mint_info.owner);
        assert_owned_by(pool_mint_info, &spl_token::id())?;

        msg!("Checking ownership of user_token_account_info ({}) -> Expected Owner: {}", user_token_account_info.key, spl_token::id());
        msg!(" -> Actual Owner: {}", user_token_account_info.owner);
        assert_owned_by(user_token_account_info, &spl_token::id())?;

        // Stake account PDA will be checked/created below

        // Load stake pool state
        msg!("Attempting to deserialize StakePool state from account: {}", stake_pool_info.key);
        msg!(" -> Account data length: {}", stake_pool_info.data.borrow().len());
        let mut stake_pool = StakePool::try_from_slice(&stake_pool_info.data.borrow())?;
        if !stake_pool.is_initialized() {
            msg!("Stake pool not initialized");
            return Err(ProgramError::UninitializedAccount);
        }
        if stake_pool.paused {
            msg!("Stake pool is paused");
            return Err(StakePoolError::PoolPaused.into());
        }
        // Check stake amount against limits
        if amount < stake_pool.min_stake {
            msg!("Stake amount below minimum");
            return Err(StakePoolError::StakeTooSmall.into());
        }
        if amount > stake_pool.max_stake {
            msg!("Stake amount above maximum");
            return Err(StakePoolError::StakeTooLarge.into());
        }
        // Verify the passed validator vote account matches the one in the pool state
        if *helius_validator_vote_info.key != stake_pool.helius_validator_vote {
            msg!("Incorrect Helius validator vote account passed");
            return Err(StakePoolError::InvalidStakeAccountDelegation.into());
        }

        // --- Calculate Pool Token Amount --- 
        // Based on current pool ratio (total_staked / total_shares)
        // Using u128 for intermediate calculations to prevent overflow.
        let pool_tokens_to_mint = if stake_pool.total_shares == 0 || stake_pool.total_staked == 0 {
            amount // If pool is empty, 1 SOL = 1 obeSOL (lamport basis)
        } else {
            (amount as u128)
                .checked_mul(stake_pool.total_shares as u128)
                .ok_or(StakePoolError::MathOverflow)?
                .checked_div(stake_pool.total_staked as u128)
                .ok_or(StakePoolError::MathOverflow)?
                .try_into()
                .map_err(|_| StakePoolError::MathOverflow)?
        };

        if pool_tokens_to_mint == 0 {
            msg!("Calculated pool tokens to mint is zero");
            return Err(StakePoolError::CalculationFailure.into());
        }

        // --- Derive Stake Authority PDA --- 
        // This PDA signs for minting tokens and delegating stake.
        let stake_authority_seeds = &[b"stake_authority", stake_pool_info.key.as_ref(), &[stake_pool.stake_authority_bump_seed]];
        // Verify derived stake authority PDA matches the one stored in the pool state
        let (expected_stake_authority_pda, _stake_auth_bump) = Pubkey::find_program_address(
            &[b"stake_authority", stake_pool_info.key.as_ref()],
            program_id,
        );
        if expected_stake_authority_pda != stake_pool.stake_authority || expected_stake_authority_pda != *stake_authority_info.key {
            msg!("Stake Authority PDA mismatch. Expected {}, Pool {}, Passed {}", 
                 expected_stake_authority_pda, stake_pool.stake_authority, *stake_authority_info.key);
            return Err(StakePoolError::InvalidStakeAuthority.into());
        }
        // Need the AccountInfo for the PDA to pass to CPIs if it's used as a signer *and* account
        // However, for invoke_signed, only the seeds are needed if it's just a signer.
        // We'll derive it again below for the stake account seeds if needed.

        // --- Derive User's Stake Account PDA --- 
        // Seeds: "stake_account", pool_pubkey, user_pubkey, stake_authority_pubkey
        let (stake_account_pda, stake_account_bump) = Pubkey::find_program_address(
            &[
                b"stake_account",
                stake_pool_info.key.as_ref(),
                user_info.key.as_ref(),
                &stake_pool.stake_authority.to_bytes(), // Ensure authority bytes are part of seed
            ],
            program_id
        );
        msg!("Derived user stake account PDA: {}", stake_account_pda);

        // Verify the derived PDA matches the passed account info
        if stake_account_pda != *stake_account_info.key {
            msg!("Provided stake account {} does not match derived PDA {}", stake_account_info.key, stake_account_pda);
            return Err(ProgramError::InvalidSeeds);
        }
        let stake_account_pda_seeds = &[
            b"stake_account",
            stake_pool_info.key.as_ref(),
            user_info.key.as_ref(),
            &stake_pool.stake_authority.to_bytes(),
            &[stake_account_bump]
        ];

        // --- Create or Load Stake Account PDA --- 
        let rent = Rent::get()?;
        let stake_account_size = std::mem::size_of::<StakeStateV2>();
        let required_lamports = rent.minimum_balance(stake_account_size);
        
        msg!("Checking if stake account PDA needs creation (lamports == 0)... Stake Account Lamports: {}", stake_account_info.lamports());
        let stake_account_state = if stake_account_info.lamports() == 0 {
            msg!("-> Entering block to CREATE and INITIALIZE stake account PDA.");
            // PDA doesn't exist, create it using system_instruction::create_account directly.
            msg!("   Derived Stake Account PDA: {}", stake_account_pda);
            msg!("   Stake Program ID: {}", stake_program_info.key);
            msg!("Attempting to create Stake Account PDA via CPI...");
            invoke_signed(
                &system_instruction::create_account(
                    user_info.key,             // Payer
                    stake_account_info.key,    // Account to create
                    required_lamports,         // Lamports
                    stake_account_size as u64, // Space
                    stake_program_info.key,    // Owner MUST be Stake Program
                ),
                &[
                    user_info.clone(),
                    stake_account_info.clone(),
                    system_program_info.clone(),
                ],
                &[stake_account_pda_seeds], // Seeds for the PDA account being created
            )?;
            msg!("Stake Account PDA created via CPI successfully.");

            // Initialize the stake account using Stake Program CPI (this should now work).
            msg!("Attempting to initialize Stake Account PDA via CPI...");
            msg!(" -> Rent Sysvar: {}", rent_info.key);
            msg!(" -> Stake Config Authority: {}", stake_config_info.key);
            invoke_signed(
                &stake_instruction::initialize(
                    stake_account_info.key, // The PDA we just created
                    &Authorized {              // Use Authorized struct
                        staker: stake_pool.stake_authority, // <-- Set Staker to Pool's Authority PDA
                        withdrawer: *user_info.key,        // <-- Set Withdrawer to User
                    },
                    &Lockup::default(),    // No lockup
                ),
                &[
                    stake_account_info.clone(), // The account to initialize
                    rent_info.clone(),          // Rent sysvar
                ],
                &[stake_account_pda_seeds], // Seeds for the PDA account being initialized
            )?;
            // Return Default state as it's newly initialized but not delegated
            StakeStateV2::default() 
        } else {
            msg!("-> Entering block to LOAD existing stake account PDA.");
            // PDA exists, load its state.
            msg!("   Stake account PDA {} already exists, loading state.", stake_account_pda);
            msg!("   Current stake account owner: {}", stake_account_info.owner);
            msg!("   Expected stake account owner: {}", stake_program_info.key);
            // Check ownership
            assert_owned_by(stake_account_info, stake_program_info.key)?;
            msg!("   Stake account ownership check passed.");
            // Deserialize state - CORRECTED: Use try_borrow_mut_data and pass &mut slice
            let mut account_data = stake_account_info.try_borrow_mut_data()?;
            StakeStateV2::deserialize(&mut &account_data[..])? // Create slice and pass mut ref
        };

        // --- CPI: Transfer SOL --- 
        // Transfer user's SOL to the derived stake account PDA.
        msg!("Transferring {} lamports from user to stake account PDA", amount);
        invoke(
            &system_instruction::transfer(
                user_info.key, 
                stake_account_info.key, 
                amount
            ),
            &[
                user_info.clone(),
                stake_account_info.clone(),
                system_program_info.clone(),
            ]
        )?;

        // --- CPI: Delegate Stake --- 
        // Delegate the stake account to the Helius validator.
        // Requires the stake_authority PDA to sign.
        msg!("Delegating stake account PDA to validator {}", stake_pool.helius_validator_vote);
        invoke_signed(
            &stake_instruction::delegate_stake(
                stake_account_info.key, 
                &stake_pool.stake_authority, // Authority PDA pubkey for instruction data
                &stake_pool.helius_validator_vote, 
            ),
            &[
                stake_program_info.clone(),         // Stake Program
                stake_account_info.clone(),         // Stake Account to delegate
                helius_validator_vote_info.clone(), // Validator Vote Acc
                clock_info.clone(),                 // Clock Sysvar
                stake_history_info.clone(),         // Stake History Sysvar
                stake_config_info.clone(),          // Stake Config Acc
                stake_authority_info.clone(),       // Stake Authority Acc <-- ADDED
            ],
            &[stake_authority_seeds] // Sign with stake_authority PDA seeds
        )?;

        // --- CPI: Mint Pool Tokens --- 
        msg!("Minting {} obeSOL tokens to user {}", pool_tokens_to_mint, user_token_account_info.key);
        invoke_signed(
            &spl_token::instruction::mint_to(
                token_program_info.key,
                pool_mint_info.key,
                user_token_account_info.key,
                &stake_pool.stake_authority, // Mint authority is the stake_authority PDA
                &[], // No multisig
                pool_tokens_to_mint,
            )?,
            &[
                token_program_info.clone(),     // Token Program
                pool_mint_info.clone(),         // Mint to mint from
                user_token_account_info.clone(),// Account to mint to
                stake_authority_info.clone(),   // Mint Authority Account <-- ADDED
            ],
            &[stake_authority_seeds] // Sign with stake_authority PDA seeds
        )?;

        // --- Update Stake Pool State --- 
        stake_pool.total_staked = stake_pool.total_staked
            .checked_add(amount)
            .ok_or(StakePoolError::MathOverflow)?;
        stake_pool.total_shares = stake_pool.total_shares
            .checked_add(pool_tokens_to_mint)
            .ok_or(StakePoolError::MathOverflow)?;

        msg!("Updating stake pool state: total_staked={}, total_shares={}", 
            stake_pool.total_staked, stake_pool.total_shares);
        stake_pool.serialize(&mut *stake_pool_info.data.borrow_mut())?;

        msg!("Stake processing complete.");
        Ok(())
    }

    /// Processes a user's request to unstake (burn obeSOL tokens).
    /// This is the first step of a two-step process due to stake deactivation cooldown.
    fn process_unstake(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        pool_token_amount: u64,
    ) -> ProgramResult {
        msg!("Processing Unstake: Burning {} pool tokens", pool_token_amount);
        let account_info_iter = &mut accounts.iter();

        // 0. `[signer]` User account (signer, authority for token burn)
        let user_info = next_account_info(account_info_iter)?;
        // 1. `[writable]` Stake pool
        let stake_pool_info = next_account_info(account_info_iter)?;
        // 2. `[writable]` User token account (burning from here)
        let user_token_account_info = next_account_info(account_info_iter)?;
        // 3. `[writable]` Pool token mint 
        let pool_mint_info = next_account_info(account_info_iter)?;
        // 4. `[writable]` Stake account (derived from user & pool - to be deactivated)
        let stake_account_info = next_account_info(account_info_iter)?;
        // 5. `[]` Token program id
        let token_program_info = next_account_info(account_info_iter)?;
        // 6. `[]` Stake program id
        let stake_program_info = next_account_info(account_info_iter)?;
        // 7. `[]` Clock sysvar
        let clock_info = next_account_info(account_info_iter)?;
        // (Implicit) Stake pool withdraw authority PDA (used for signing burn/deactivate)

        // Basic checks
        if !user_info.is_signer {
            msg!("User signature missing");
            return Err(ProgramError::MissingRequiredSignature);
        }
        assert_owned_by(stake_pool_info, program_id)?;
        assert_owned_by(pool_mint_info, &spl_token::id())?;
        assert_owned_by(user_token_account_info, &spl_token::id())?;
        // Stake account ownership is checked by stake program CPI

        // Load stake pool state
        let mut stake_pool = StakePool::try_from_slice(&stake_pool_info.data.borrow())?;
        if !stake_pool.is_initialized() {
            msg!("Stake pool not initialized");
            return Err(ProgramError::UninitializedAccount);
        }
        if stake_pool.paused {
            msg!("Stake pool is paused");
            return Err(StakePoolError::PoolPaused.into());
        }

        // Check pool token amount
        if pool_token_amount == 0 {
            return Err(StakePoolError::StakeTooSmall.into());
        }

        // --- Share to SOL Calculation --- 
        // Calculate the proportional amount of SOL the user *should* receive back
        // based on the current pool ratio. This SOL is not transferred yet.
        let sol_to_withdraw = if stake_pool.total_shares > 0 && stake_pool.total_staked > 0 {
            (pool_token_amount as u128)
                .checked_mul(stake_pool.total_staked as u128)
                .ok_or(StakePoolError::MathOverflow)?
                .checked_div(stake_pool.total_shares as u128)
                .ok_or(StakePoolError::MathOverflow)?
                .try_into()
                .map_err(|_| StakePoolError::MathOverflow)?
        } else {
            // Should not happen if pool_token_amount > 0 and tokens exist, but handle defensively
            0 
        };
        msg!("Calculated SOL to withdraw (deferred): {}", sol_to_withdraw);

        // --- CPI: Burn Pool Tokens --- 
        // Burns the specified amount of obeSOL tokens from the user's token account.
        // The user signs as the authority to burn their own tokens.
        msg!("Burning pool tokens");
        invoke(
            &spl_token::instruction::burn(
                token_program_info.key, 
                user_token_account_info.key, 
                pool_mint_info.key, 
                user_info.key, // User authorizes burning their own tokens 
                &[], 
                pool_token_amount
            )?,
            &[
                token_program_info.clone(),
                user_token_account_info.clone(),
                pool_mint_info.clone(),
                user_info.clone(),
            ]
        )?;

        // --- CPI: Deactivate Stake Account --- 
        // Initiates the deactivation of the user's stake account PDA via the Stake program.
        // The stake account must be fully deactivated before SOL can be withdrawn.
        // Requires the stake_authority PDA (derived from pool) to sign.
        // First, derive the stake_authority PDA and its seeds using the stored bump.
        let stake_authority_seeds = &[b"stake_authority", stake_pool_info.key.as_ref(), &[stake_pool.stake_authority_bump_seed]]; // Use stored bump
        // Verify derived stake authority matches the one stored in the pool state.
        let (stake_authority_pda, _stake_auth_bump) = Pubkey::find_program_address(
            &[b"stake_authority", stake_pool_info.key.as_ref()],
            program_id,
        );
        if stake_authority_pda != stake_pool.stake_authority {
             return Err(StakePoolError::InvalidStakeAuthority.into());
        }
        
        // Derive the expected user stake account PDA to confirm the correct account was passed.
        let (expected_stake_pda, _stake_pda_bump) = Pubkey::find_program_address(
            &[
                b"stake_account",
                stake_pool_info.key.as_ref(),
                user_info.key.as_ref(),
                &stake_pool.stake_authority.to_bytes(), // CORRECTED: Include authority in seeds
            ],
            program_id
        );
        if expected_stake_pda != *stake_account_info.key {
            msg!("Provided stake account {} does not match derived PDA {}", *stake_account_info.key, expected_stake_pda);
            return Err(ProgramError::InvalidSeeds);
        }

        // Authority for deactivation is the stake_pool.stake_authority PDA
        msg!("Deactivating stake account");
        invoke_signed(
            &stake_instruction::deactivate_stake(
                stake_account_info.key,
                &stake_pool.stake_authority, // The PDA is the authority
            ),
            &[
                stake_program_info.clone(),
                stake_account_info.clone(),
                clock_info.clone(),
                // The authority account_info needs to be passed if it exists separate from pool
                // If it's just a PDA derived from pool, it doesn't need separate AccountInfo
            ],
            &[stake_authority_seeds], // Sign with the PDA authority seeds
        )?;

        // --- Update Stake Pool State --- 
        stake_pool.total_staked = stake_pool.total_staked
            .checked_sub(sol_to_withdraw)
            .ok_or(StakePoolError::MathOverflow)?;
        stake_pool.total_shares = stake_pool.total_shares
            .checked_sub(pool_token_amount)
            .ok_or(StakePoolError::MathOverflow)?;

        // TODO: Potentially record unstake request info (e.g., epoch, amount)
        // in a separate account or modify stake_pool state if needed for later withdrawal claim.

        msg!("Updating stake pool state");
        stake_pool.serialize(&mut *stake_pool_info.data.borrow_mut())?;

        msg!("Unstake processing complete. User must wait for cooldown and call withdraw instruction.");
        Ok(())
    }

    /// Processes reward epoch updates. (Simplified)
    /// NOTE: In this simplified model, rewards are not actively calculated or distributed here.
    /// Rewards accrue implicitly in the underlying stake accounts, increasing the value 
    /// of each obeSOL pool token over time. This instruction only marks the epoch as processed.
    fn process_claim_rewards(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        
        // Accounts needed: Signer, Stake Pool, Clock
        let user_info = next_account_info(account_info_iter)?; // Signer who triggers the epoch update
        let stake_pool_info = next_account_info(account_info_iter)?;
        // let user_token_account_info = next_account_info(account_info_iter)?;
        // let pool_mint_info = next_account_info(account_info_iter)?;
        // let treasury_fee_account = next_account_info(account_info_iter)?;
        // let stake_account_info = next_account_info(account_info_iter)?;
        // let token_program_info = next_account_info(account_info_iter)?;
        let clock_info = next_account_info(account_info_iter)?;

        // Verify signer
        if !user_info.is_signer {
            // Allow anyone to trigger epoch update? Or restrict to pool authority?
            // Keeping user signer requirement for now.
            return Err(ProgramError::MissingRequiredSignature);
        }
        assert_owned_by(stake_pool_info, program_id)?;

        // Load stake pool and validate
        let mut stake_pool = StakePool::try_from_slice(&stake_pool_info.data.borrow())?;
        if !stake_pool.is_initialized() {
            msg!("Stake pool not initialized");
            return Err(ProgramError::UninitializedAccount);
        }
        if stake_pool.paused {
            // Can we update epoch even if paused? Probably yes.
            // return Err(StakePoolError::PoolPaused.into());
        }

        // Get current epoch
        let clock = Clock::from_account_info(clock_info)?;
        let current_epoch = clock.epoch;

        // Ensure we haven't already claimed rewards this epoch
        if stake_pool.last_update_epoch >= current_epoch {
            msg!("Pool epoch {} already processed.", current_epoch);
            return Ok(()); // Not an error, just nothing to do
        }

        // --- Reward Calculation Removed --- 
        // Rewards are implicit in the value accrual of the underlying stake accounts.
        // This instruction now only serves to mark the epoch as processed.
        msg!("Updating pool last processed epoch.");
        
        // Update only the epoch marker
        stake_pool.last_update_epoch = current_epoch;

        // Save state
        stake_pool.serialize(&mut *stake_pool_info.data.borrow_mut())?;

        msg!("Pool epoch updated to {}", current_epoch);
        Ok(())
    }

    /// Processes withdrawal of SOL from a user's deactivated stake account PDA.
    /// This is the second step after `Unstake` and requires the cooldown period to have passed.
    fn process_withdraw_stake(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
    ) -> ProgramResult {
        msg!("Processing Withdraw Stake");
        let account_info_iter = &mut accounts.iter();

        // 0. `[signer, writable]` User account (receives SOL)
        let user_info = next_account_info(account_info_iter)?;
        // 1. `[]` Stake pool (read-only, for withdraw authority derivation)
        let stake_pool_info = next_account_info(account_info_iter)?;
        // 2. `[writable]` Stake account (PDA derived from user & pool - withdraw from)
        let stake_account_info = next_account_info(account_info_iter)?;
        // 3. `[]` Stake pool withdraw authority PDA (derived from pool, passed for verification)
        let withdraw_authority_info = next_account_info(account_info_iter)?;
        // 4. `[]` Stake program id
        let stake_program_info = next_account_info(account_info_iter)?;
        // 5. `[]` Clock sysvar
        let clock_info = next_account_info(account_info_iter)?;
        // 6. `[]` Stake history sysvar
        let stake_history_info = next_account_info(account_info_iter)?;

        // Basic Checks
        if !user_info.is_signer {
            msg!("User signature missing");
            return Err(ProgramError::MissingRequiredSignature);
        }
        assert_owned_by(stake_pool_info, program_id)?;
        assert_owned_by(stake_account_info, &solana_program::stake::program::id())?;

        // Load stake pool state (needed for withdraw authority)
        let stake_pool = StakePool::try_from_slice(&stake_pool_info.data.borrow())?;
        if !stake_pool.is_initialized() {
            msg!("Stake pool not initialized");
            return Err(ProgramError::UninitializedAccount);
        }
        // It's okay if pool is paused for withdrawals

        // Verify passed withdraw authority PDA matches the one in the pool state
        if *withdraw_authority_info.key != stake_pool.withdraw_authority {
            msg!("Incorrect withdraw authority provided");
            return Err(StakePoolError::InvalidWithdrawAuthority.into()); // Need new error
        }

        // Load stake account state
        let stake_state = StakeStateV2::try_from_slice(&stake_account_info.data.borrow())?;
        let (deactivation_epoch, stake_lamports) = match stake_state {
            StakeStateV2::Stake(meta, stake, _stake_flags) => {
                 // Verify the designated withdrawer matches the pool's withdraw authority PDA.
                 if meta.authorized.withdrawer != stake_pool.withdraw_authority {
                    msg!("Stake account withdraw authority mismatch");
                    return Err(StakePoolError::InvalidStakeAccountAuthority.into());
                 }
                 // Check if the stake account has actually been deactivated.
                 if stake.delegation.deactivation_epoch == std::u64::MAX {
                    msg!("Stake account is not deactivated");
                    return Err(StakePoolError::StakeNotDeactivated.into());
                 }
                 (stake.delegation.deactivation_epoch, stake_account_info.lamports())
            },
            _ => {
                msg!("Stake account not in correct Stake state for withdrawal");
                return Err(StakePoolError::WrongStakeState.into());
            }
        };

        // Check cooldown period
        let clock = Clock::from_account_info(clock_info)?;
        if clock.epoch <= deactivation_epoch { // Should be strictly greater? Check stake program logic. Using <= for safety.
            msg!("Stake account cooldown period not yet passed (current: {}, deactivation: {})", clock.epoch, deactivation_epoch);
            return Err(StakePoolError::CooldownNotPassed.into());
        }

        // Derive withdraw authority PDA seeds for signing
        let withdraw_authority_seeds = &[b"withdraw_authority", stake_pool_info.key.as_ref(), &[stake_pool.withdraw_authority_bump_seed]];
        // Double check derivation matches stored authority
        let (_withdraw_pda, _withdraw_bump) = Pubkey::find_program_address(
            &[b"withdraw_authority", stake_pool_info.key.as_ref()],
            program_id,
        );
        if _withdraw_pda != stake_pool.withdraw_authority {
             msg!("Derived withdraw authority PDA mismatch");
            return Err(StakePoolError::InvalidWithdrawAuthority.into());
        }

        // --- CPI: Withdraw SOL from Stake Account --- 
        // Withdraws the full SOL balance from a fully deactivated stake account PDA
        // to the user's main account. Requires cooldown period to have passed.
        // Requires the withdraw_authority PDA to sign.
        msg!("Withdrawing {} lamports from stake account {} to user {}", 
             stake_lamports, stake_account_info.key, user_info.key);
        invoke_signed(
            &stake_instruction::withdraw(
                stake_account_info.key,
                &stake_pool.withdraw_authority, // The PDA is the authority
                user_info.key, // Recipient of SOL
                stake_lamports, // Withdraw the full balance
                None, // No custodian needed
            ),
            &[
                stake_program_info.clone(),
                stake_account_info.clone(), // Source
                user_info.clone(),          // Destination
                clock_info.clone(),
                stake_history_info.clone(),
                withdraw_authority_info.clone(), // Authority account
            ],
            &[withdraw_authority_seeds], // Sign with the PDA withdraw authority seeds
        )?;

        // Optional: Close the stake account PDA and return rent to user?
        // This would require making user_info writable and passing system_program.
        // For simplicity, leaving the account open for now.

        msg!("Withdrawal successful.");
        Ok(())
    }
} // <-- ADDED Closing brace for impl Processor