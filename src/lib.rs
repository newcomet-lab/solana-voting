pub mod processor;
pub mod state;

use solana_program::{
    account_info::AccountInfo, entrypoint, entrypoint::ProgramResult, msg, pubkey::Pubkey,
};

use crate::processor::Processor;

entrypoint!(process_instruction);

fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    if let Err(err) = Processor::process(program_id, accounts, instruction_data) {
        msg!("Error occured: {:?}", err);
        // err.print::<VoteError>();
        return Err(err);
    }
    Ok(())
}