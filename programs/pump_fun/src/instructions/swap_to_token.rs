use anchor_lang::prelude::*;
use anchor_spl::{
    token::{ transfer, Mint, Token, TokenAccount, Transfer },
    token_2022::{ transfer_checked, TransferChecked, ID },
    token_interface,
};

use crate::{ curve::{ to_u128, to_u64 }, error::SwapError, AppStats, SwapPair, APP_STATS_SEED, AUTHORITY_SEED, DENOMINATOR, FEE_PERCENTAGE };

#[derive(Accounts)]
pub struct SwapToToken<'info> {
    #[account(mut)]
    pub swapper: Signer<'info>,

    /// CHECK:
    pub pda: AccountInfo<'info>,

    #[account(token::token_program = token_program_mint)]
    pub mint: Box<InterfaceAccount<'info, token_interface::Mint>>,
    pub wsol: Box<Account<'info, Mint>>,

    // pair account
    #[account(
      mut,
      constraint = pair.mint == mint.key(),
      constraint = pair.token_account == token_account_for_pda.key(),
    )]
    pub pair: Box<Account<'info, SwapPair>>,

    // token account for swapper
    #[account(
      mut,
      token::mint = mint,
      token::token_program = token_program_mint,
    )]
    pub token_account_for_swapper: Box<InterfaceAccount<'info, token_interface::TokenAccount>>,

    // wsol account for swapper
    #[account(
      mut,
      constraint = native_account_for_swapper.is_native() == true,
    )]
    pub native_account_for_swapper: Box<Account<'info, TokenAccount>>,

    // token account for pda
    #[account(
      mut,
      token::token_program = token_program_mint,
      token::authority = pda,
    )]
    pub token_account_for_pda: Box<InterfaceAccount<'info, token_interface::TokenAccount>>,

    // wsol account for pda
    #[account(
      mut,
      token::authority = pda,
      constraint = native_account_for_pda.is_native() == true,
    )]
    pub native_account_for_pda: Box<Account<'info, TokenAccount>>,

    // fee account for fee lamports
    #[account(
        mut, 
        constraint = app_stats.fee_account == fee_account.key()
    )]
    pub fee_account: Box<Account<'info, TokenAccount>>,

    // app state account
    #[account(
        seeds = [APP_STATS_SEED],
        bump
    )]
    pub app_stats: Box<Account<'info, AppStats>>,

    pub token_program: Program<'info, Token>,
    pub token_program_mint: Interface<'info, token_interface::TokenInterface>,
}

impl<'info> SwapToToken<'info> {
    fn to_transfer_context(&self) -> CpiContext<'_, '_, '_, 'info, Transfer<'info>> {
        let cpi_accounts: Transfer = Transfer {
            from: self.token_account_for_pda.to_account_info().clone(),
            to: self.token_account_for_swapper.to_account_info().clone(),
            authority: self.pda.to_account_info().clone(),
        };
        CpiContext::new(self.token_program_mint.to_account_info(), cpi_accounts)
    }

    pub fn transfer_checked_ctx(&self) -> CpiContext<'_, '_, '_, 'info, TransferChecked<'info>> {
        let cpi_program: AccountInfo = self.token_program_mint.to_account_info();
        let cpi_accounts: TransferChecked = TransferChecked {
            from: self.token_account_for_pda.to_account_info(),
            to: self.token_account_for_swapper.to_account_info(),
            authority: self.pda.to_account_info(),
            mint: self.mint.to_account_info(),
        };

        CpiContext::new(cpi_program, cpi_accounts)
    }

    fn to_transfer_native_context(&self) -> CpiContext<'_, '_, '_, 'info, Transfer<'info>> {
        let cpi_accounts: Transfer = Transfer {
            from: self.native_account_for_swapper.to_account_info().clone(),
            to: self.native_account_for_pda.to_account_info().clone(),
            authority: self.swapper.to_account_info().clone(),
        };
        CpiContext::new(self.token_program.to_account_info(), cpi_accounts)
    }

    fn to_transfer_fee_context(&self) -> CpiContext<'_, '_, '_, 'info, Transfer<'info>> {
        let cpi_accounts: Transfer = Transfer {
            from: self.native_account_for_swapper.to_account_info().clone(),
            to: self.fee_account.to_account_info().clone(),
            authority: self.swapper.to_account_info().clone(),
        };
        CpiContext::new(self.token_program.to_account_info(), cpi_accounts)
    }
}

/*
    buy token with sol by investor
    @param: amount_in : sol amount to buy token
*/

pub fn swap_to_token_handler(ctx: Context<SwapToToken>, amount_in: u64) -> Result<()> {
    let fee_amount: u64 = amount_in / DENOMINATOR as u64 * FEE_PERCENTAGE as u64;
    let source_amount: u128 = to_u128(amount_in - fee_amount)?;
    let swap_source_amount: u128 = to_u128(ctx.accounts.native_account_for_pda.amount)?;
    let swap_destination_amount: u128 = to_u128(ctx.accounts.token_account_for_pda.amount)?;
    let (source_amount, destination_amount) = ctx.accounts.pair.curve
        .swap_a_to_b(source_amount, swap_source_amount, swap_destination_amount)
        .ok_or(SwapError::ZeroTradingTokens)?;
    transfer(ctx.accounts.to_transfer_native_context(), to_u64(source_amount)?)?;
    transfer(ctx.accounts.to_transfer_fee_context(), fee_amount)?;
    let seeds: &[&[u8]; 3] = &[
        AUTHORITY_SEED,
        ctx.accounts.mint.to_account_info().key.as_ref(),
        &[ctx.accounts.pair.bump],
    ];
    let signer_seeds: &[&[&[u8]]; 1] = &[&seeds[..]];
    if ctx.accounts.token_program_mint.key() == ID {
        transfer_checked(
            ctx.accounts.transfer_checked_ctx().with_signer(signer_seeds),
            to_u64(destination_amount)?,
            ctx.accounts.mint.decimals
        )?;
    } else {
        transfer(
            ctx.accounts.to_transfer_context().with_signer(signer_seeds),
            to_u64(destination_amount)?
        )?;
    }
    Ok(())
}
