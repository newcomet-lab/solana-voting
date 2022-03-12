use crate::state::{Vote, Voter};
use arrayref::array_ref;
use byteorder::{ByteOrder, LittleEndian};
use num_derive::FromPrimitive;
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    clock::Clock,
    decode_error::DecodeError,
    entrypoint::ProgramResult,
    msg,
    program::{invoke, invoke_signed},
    program_error::ProgramError,
    program_pack::Pack,
    pubkey::Pubkey,
    sysvar::Sysvar,
};
use spl_token::state::Account as TokenAccount;
use std::{convert::TryInto, str::from_utf8};
use thiserror::Error;
pub struct Processor {}

impl Processor {
    pub fn process(program_id: &Pubkey, accounts: &[AccountInfo], input: &[u8]) -> ProgramResult {
        let _account = accounts;
        let _program_id = program_id;
        let instruction = VoteInstruction::unpack(input)?;
        match instruction {
            VoteInstruction::NewVote {
                vote_title,
                end_time,
            } => {
                msg!("Instruction NewVote");
                Self::process_newvote(program_id, accounts, &vote_title, &end_time)
            }
            VoteInstruction::Vote { is_vote_for } => {
                msg!("Instruction Vote");
                Self::process_vote(program_id, accounts, &is_vote_for)
            }
            VoteInstruction::Withdraw {} => {
                msg!("Instruction Withdraw");
                Self::process_withdraw(program_id, accounts)
            }
        }
    }

    pub fn process_newvote(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        vote_title: &[u8; 30],
        end_time: &u64,
    ) -> ProgramResult {
        msg!("process new vote+++");
        let _account = accounts;
        let _program_id = program_id;
        let accounts_iter = &mut accounts.iter();
        let vote_data_account = next_account_info(accounts_iter)?;

        if vote_data_account.owner != program_id {
            msg!("Vote data account is not owned by program");
            return Err(ProgramError::InvalidAccountData);
        }

        let mut vote_data = vote_data_account.try_borrow_mut_data()?;

        let mut vote = Vote::unpack_unchecked(&vote_data)?;

        if vote.is_initialized {
            msg!("Vote has already been initialized");
            return Err(VoteError::VoteDataAccountAlreadyInitialized.into());
        }

        vote.is_initialized = true;
        vote.title = *vote_title;
        vote.end_time = *end_time;

        msg!(
            "Creating Vote with title {:?}",
            from_utf8(&vote.title).unwrap()
        );
        msg!("New Vote: {:?}", vote);
        Vote::pack(vote, &mut vote_data).expect("Failed to write to vote data account");
        msg!("Vote created");

        Ok(())
    }

    pub fn process_vote(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        is_vote_for: &bool,
    ) -> ProgramResult {
        let accounts_iter = &mut accounts.iter();

        let temp_token_account = next_account_info(accounts_iter)?; // 1

        let vote_data_account = next_account_info(accounts_iter)?; // 2

        if vote_data_account.owner != program_id {
            msg!("Vote data account is not owned by program");
            return Err(ProgramError::InvalidAccountData);
        }
        let mut vote_data = vote_data_account.try_borrow_mut_data()?;
        let mut vote = Vote::unpack_unchecked(&vote_data)?;

        let voter_account = next_account_info(accounts_iter)?; // 3
        if !voter_account.is_signer {
            msg!("Voter account is not signer");
            return Err(ProgramError::MissingRequiredSignature);
        }

        let voter_data_account = next_account_info(accounts_iter)?; // 4

        let seed: &str = &(vote_data_account.key.to_string())[30..];

        let expected_voter_data_account =
            Pubkey::create_with_seed(voter_account.key, seed, program_id)?;
        if expected_voter_data_account != *voter_data_account.key {
            msg!("Voter data account is not valid");
            return Err(ProgramError::InvalidAccountData);
        }

        let mut voter_data_account_data = voter_data_account.try_borrow_mut_data()?;

        let mut voter = Voter::unpack_unchecked(&voter_data_account_data)?;

        if !voter.is_initialized || voter.has_voted {
            msg!(
                "Invalid voter: {} or voter has voted: {}",
                voter.is_initialized,
                voter.has_voted
            );
            return Err(ProgramError::InvalidInstructionData);
        }
        if *is_vote_for {
            vote.yes += 1;
        } else {
            vote.no += 1;
        }
        voter.has_voted = true;
        voter.temp_token_account_pubkey = *temp_token_account.key;

        Vote::pack(vote, &mut vote_data).expect("Failed to write to vote data account");
        Voter::pack(voter, &mut voter_data_account_data)
            .expect("Failed to write to voter's data account");

        let (pda, _nonce) = Pubkey::find_program_address(&[b"daoo-voting"], program_id);
        let token_program = next_account_info(accounts_iter)?; // 5

        let owner_change_ix = spl_token::instruction::set_authority(
            token_program.key,
            temp_token_account.key,
            Some(&pda),
            spl_token::instruction::AuthorityType::AccountOwner,
            voter_account.key,
            &[&voter_account.key],
        )?;

        msg!("Calling the token program to transfer token account ownership...");
        invoke(
            &owner_change_ix,
            &[
                temp_token_account.clone(),
                voter_account.clone(),
                token_program.clone(),
            ],
        )?;

        Ok(())
    }

    pub fn process_withdraw(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
        let accounts_iter = &mut accounts.iter();

        let vote_data_account = next_account_info(accounts_iter)?; // 1

        if vote_data_account.owner != program_id {
            msg!("Vote data account is not owned by program");
            return Err(ProgramError::InvalidAccountData);
        }
        let vote_data = vote_data_account.try_borrow_mut_data()?;
        let vote = Vote::unpack_unchecked(&vote_data)?;

        let voter_account = next_account_info(accounts_iter)?; // 2
        if !voter_account.is_signer {
            msg!("Voter account is not signer");
            return Err(ProgramError::MissingRequiredSignature);
        }

        let clock = Clock::get()?;
        if vote.end_time > clock.unix_timestamp.try_into().unwrap() {
            return Err(VoteError::VoteInProgress.into());
        }

        let pdas_temp_token_account = next_account_info(accounts_iter)?; // 3
        let pdas_temp_token_account_info =
            TokenAccount::unpack(&pdas_temp_token_account.try_borrow_data()?)?;
        let (pda, nonce) = Pubkey::find_program_address(&[b"daoo-voting"], program_id);

        let main_account = next_account_info(accounts_iter)?; // 4
        let receive_account = next_account_info(accounts_iter)?; // 5
        let token_program = next_account_info(accounts_iter)?; // 6
        let pda_account = next_account_info(accounts_iter)?; // 7

        let transfer_to_member_ix = spl_token::instruction::transfer(
            token_program.key,
            pdas_temp_token_account.key,
            receive_account.key,
            &pda,
            &[&pda],
            pdas_temp_token_account_info.amount,
        )?;
        msg!("Calling the token program to transfer staked ape to the elected member...");
        invoke_signed(
            &transfer_to_member_ix,
            &[
                pdas_temp_token_account.clone(),
                receive_account.clone(),
                pda_account.clone(),
                token_program.clone(),
            ],
            &[&[&b"daoo-voting"[..], &[nonce]]],
        )?;

        let close_pdas_temp_acc_ix = spl_token::instruction::close_account(
            token_program.key,
            pdas_temp_token_account.key,
            main_account.key,
            &pda,
            &[&pda],
        )?;
        msg!("Calling the token program to close pda's temp account...");
        invoke_signed(
            &close_pdas_temp_acc_ix,
            &[
                pdas_temp_token_account.clone(),
                main_account.clone(),
                pda_account.clone(),
                token_program.clone(),
            ],
            &[&[&b"daoo-voting"[..], &[nonce]]],
        )?;

        Ok(())
    }
}

#[repr(C)]
#[derive(Clone, Debug, PartialEq)]
pub enum VoteInstruction {
    NewVote { vote_title: [u8; 30], end_time: u64 },
    Vote { is_vote_for: bool },
    Withdraw {},
}

impl VoteInstruction {
    pub fn unpack(input: &[u8]) -> Result<Self, ProgramError> {
        let (&tag, rest) = input.split_first().ok_or(VoteError::InvalidInstruction)?;
        Ok(match tag {
            0 => {
                let vote_title = *array_ref![rest, 0, 30];
                let end_time = LittleEndian::read_u64(&rest[31..38]);
                Self::NewVote {
                    vote_title,
                    end_time,
                }
            }
            1 => {
                let (&is_vote_for, _rest) =
                    rest.split_first().ok_or(VoteError::InvalidInstruction)?;
                let is_vote_for = match is_vote_for {
                    0 => false,
                    1 => true,
                    _ => false,
                };
                Self::Vote { is_vote_for }
            }
            2 => Self::Withdraw {},
            _ => return Err(VoteError::InvalidInstruction.into()),
        })
    }
}

#[derive(Clone, Debug, Eq, Error, FromPrimitive, PartialEq)]
pub enum VoteError {
    /// Invalid instruction
    #[error("Invalid instruction")]
    InvalidInstruction,

    /// VoteDataAccountAlreadyInitialized
    #[error("Vote data account has already been initialized")]
    VoteDataAccountAlreadyInitialized,

    /// VoteInProgress
    #[error("Vote is still in progress")]
    VoteInProgress,
}

impl From<VoteError> for ProgramError {
    fn from(e: VoteError) -> Self {
        ProgramError::Custom(e as u32)
    }
}
impl<T> DecodeError<T> for VoteError {
    fn type_of() -> &'static str {
        "VoteError"
    }
}
