//! Very reduced Solana program that indirectly validates
//! Ed25519 signatures by using instruction introspection
//!
//! Single-file, made for learning / teaching / example purposes.
//!

use anchor_lang::prelude::*;
use solana_program::instruction::Instruction;
use solana_program::sysvar::instructions::{ID as IX_ID, load_instruction_at_checked};
use solana_program::ed25519_program::{ID as ED25519_ID};

use std::convert::TryInto;

declare_id!("DHxesXA69rUmz5AJ1CnLCQezUzQR5j7KKTwTp1zZPc9j");

/// Main module
#[program]
pub mod signatures {
    use super::*;

    /// External instruction that only gets executed if
    /// an `Ed25519Program.createInstructionWithPublicKey`
    /// instruction was sent in the same transaction.
    pub fn verify(ctx: Context<Verify>, msg: Vec<u8>, sig: [u8; 64], pubkey: [u8; 32]) -> ProgramResult {        
        
        // Get what should be the Ed25519Program instruction
        let ix: Instruction = load_instruction_at_checked(0, &ctx.accounts.ix_sysvar)?;

        // Check that ix is what we expect to have been sent
        utils::sig_verify(&ix, &msg, &sig, &pubkey)?;

        // Do other stuff

        Ok(())
    }
}

/// This mod contains functions that validate that an instruction
/// is constructed the way we expect. In this case, this is for a
/// `web3.Ed25519Program.createInstructionWithPublicKey()` instruction
pub mod utils {
    use super::*;

    /// Verify fields
    pub fn sig_verify(ix: &Instruction, msg: &[u8], sig: &[u8], pubkey: &[u8]) -> ProgramResult {
        if  ix.program_id       != ED25519_ID                   ||  // The program id we expect
            ix.accounts.len()   != 0                            ||  // With no context accounts
            ix.data.len()       != (16 + 64 + 32 + msg.len())       // And data of this size
        {
            return Err(ErrorCode::SigVerificationFailed.into());    // Otherwise, we can already throw err
        }

        check_data(&ix.data, msg, sig, pubkey)?;                    // If that's not the case, check data

        Ok(())
    }

    /// Verify serialized instruction data
    pub fn check_data(data: &[u8], msg: &[u8], sig: &[u8], pubkey: &[u8]) -> ProgramResult {
        // According to this layout used by the Ed25519Program
        // https://github.com/solana-labs/solana-web3.js/blob/d93efdf/src/ed25519-program.ts#L102

        // "Deserializing" byte slices

        let num_signatures                  = &[data[0]];        // Byte  0
        let padding                         = &[data[1]];        // Byte  1
        let signature_offset                = &data[2..=3];      // Bytes 2,3
        let signature_instruction_index     = &data[4..=5];      // Bytes 4,5
        let public_key_offset               = &data[6..=7];      // Bytes 6,7
        let public_key_instruction_index    = &data[8..=9];      // Bytes 8,9
        let message_data_offset             = &data[10..=11];    // Bytes 10,11
        let message_data_size               = &data[12..=13];    // Bytes 12,13
        let message_instruction_index       = &data[14..=15];    // Bytes 14,15

        let data_pubkey                     = &data[16..16+32];  // Bytes 16..16+32
        let data_sig                        = &data[48..48+64];  // Bytes 48..48+64
        let data_msg                        = &data[112..];      // Bytes 112..end

        // Expected values

        let exp_public_key_offset:      u16 = 16; // 2*u8 + 7*u16
        let exp_signature_offset:       u16 = exp_public_key_offset + pubkey.len() as u16;
        let exp_message_data_offset:    u16 = exp_signature_offset + sig.len() as u16;
        let exp_num_signatures:          u8 = 1;
        let exp_message_data_size:      u16 = msg.len().try_into().unwrap();

        // Header and Arg Checks

        // Header
        if  num_signatures                  != &exp_num_signatures.to_le_bytes()        ||
            padding                         != &[0]                                     ||
            signature_offset                != &exp_signature_offset.to_le_bytes()      ||
            signature_instruction_index     != &u16::MAX.to_le_bytes()                  ||
            public_key_offset               != &exp_public_key_offset.to_le_bytes()     ||
            public_key_instruction_index    != &u16::MAX.to_le_bytes()                  ||
            message_data_offset             != &exp_message_data_offset.to_le_bytes()   ||
            message_data_size               != &exp_message_data_size.to_le_bytes()     ||
            message_instruction_index       != &u16::MAX.to_le_bytes()  
        {
            return Err(ErrorCode::SigVerificationFailed.into());
        }

        // Arguments
        if  data_pubkey != pubkey   ||
            data_msg    != msg      ||
            data_sig    != sig
        {
            return Err(ErrorCode::SigVerificationFailed.into());
        }

        Ok(())
    }
}

/// Context accounts
#[derive(Accounts)]
pub struct Verify<'info> {
    pub sender: Signer<'info>,

    /// The address check is needed because otherwise
    /// the supplied Sysvar could be anything else.
    /// The Instruction Sysvar has not been implemented
    /// in the Anchor framework yet, so this is the safe approach.
    #[account(address = IX_ID)]
    pub ix_sysvar: AccountInfo<'info>,
}

/// Custom error codes
#[error]
pub enum ErrorCode {
    #[msg("EC25519 signature verification failed.")]
    SigVerificationFailed,
}
