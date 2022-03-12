use arrayref::{array_mut_ref, array_ref, array_refs, mut_array_refs};
use byteorder::{ByteOrder, LittleEndian};
use solana_program::{
    program_error::ProgramError,
    program_pack::{IsInitialized, Pack, Sealed},
    pubkey::Pubkey,
};

pub const MAX_LEN: usize = 30;

pub struct VoteManager {}
impl Sealed for VoteManager {}

impl Pack for VoteManager {
    const LEN: usize = 0;

    fn unpack_from_slice(src: &[u8]) -> Result<Self, ProgramError> {
        let _src = src;
        Ok(VoteManager {})
    }
    fn pack_into_slice(&self, dst: &mut [u8]) {
        let _dst = dst;
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Vote {
    pub yes: u32, // 4
    pub no: u32,  // 4
    pub is_initialized: bool,
    pub title: [u8; MAX_LEN], // 1*30
    // pub owner: Pubkey, // 32
    pub end_time: u64, // 8
}

impl Sealed for Vote {}

impl IsInitialized for Vote {
    fn is_initialized(&self) -> bool {
        self.is_initialized
    }
}

impl Pack for Vote {
    const LEN: usize = 47; // 47+32

    fn unpack_from_slice(src: &[u8]) -> Result<Self, ProgramError> {
        let yes = LittleEndian::read_u32(&src[0..4]);
        let no = LittleEndian::read_u32(&src[4..8]);
        let is_initialized = *array_ref![src, 8, 1];
        let is_initialized = match is_initialized {
            [0] => false,
            [1] => true,
            _ => return Err(ProgramError::InvalidAccountData),
        };
        let title = *array_ref![src, 9, 30];
        let end_time = LittleEndian::read_u64(&src[0..8]);
        Ok(Vote {
            yes,
            no,
            is_initialized,
            title,
            end_time,
        })
    }
    fn pack_into_slice(&self, dst: &mut [u8]) {
        let dst = array_mut_ref![dst, 0, 47];
        let (yes_dst, no_dst, is_initialized_dst, title_dst, end_time_dst) =
            mut_array_refs![dst, 4, 4, 1, 30, 8];
        let &Vote {
            yes,
            no,
            is_initialized,
            title,
            end_time,
        } = self;
        *yes_dst = yes.to_le_bytes();
        *no_dst = no.to_le_bytes();
        is_initialized_dst[0] = is_initialized as u8;
        *title_dst = title;
        *end_time_dst = end_time.to_le_bytes();
    }
}

#[repr(C)]
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Voter {
    pub is_initialized: bool,              // 1
    pub has_voted: bool,                   // 1
    pub temp_token_account_pubkey: Pubkey, // 32
}

impl Sealed for Voter {}

impl IsInitialized for Voter {
    fn is_initialized(&self) -> bool {
        self.is_initialized
    }
}

impl Pack for Voter {
    const LEN: usize = 2;

    fn unpack_from_slice(src: &[u8]) -> Result<Self, ProgramError> {
        let src = array_ref![src, 0, 34];
        let (is_initialized, has_voted, temp_token_account_pubkey) = array_refs![src, 1, 1, 32];
        let is_initialized = match is_initialized {
            [0] => false,
            [1] => true,
            _ => return Err(ProgramError::InvalidAccountData),
        };
        let has_voted = match has_voted {
            [0] => false,
            [1] => true,
            _ => return Err(ProgramError::InvalidAccountData),
        };
        Ok(Voter {
            is_initialized,
            has_voted,
            temp_token_account_pubkey: Pubkey::new_from_array(*temp_token_account_pubkey),
        })
    }
    fn pack_into_slice(&self, dst: &mut [u8]) {
        let dst = array_mut_ref![dst, 0, 34];
        let (is_initialized_dst, has_voted_dst, temp_token_account_pubkey_dst) =
            mut_array_refs![dst, 1, 1, 32];
        let &Voter {
            is_initialized,
            has_voted,
            temp_token_account_pubkey,
        } = self;
        is_initialized_dst[0] = is_initialized as u8;
        has_voted_dst[0] = has_voted as u8;
        temp_token_account_pubkey_dst.copy_from_slice(temp_token_account_pubkey.as_ref());
    }
}
