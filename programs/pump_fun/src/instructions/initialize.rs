use anchor_lang::prelude::*;

use std::mem::size_of;

use crate::{ AppStats, APP_STATS_SEED };

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,

    // app state
    #[account(
        init,
        payer = owner,
        space = size_of::<AppStats>() + 8,
        seeds = [APP_STATS_SEED],
        bump
    )]
    pub app_stats: Box<Account<'info, AppStats>>,
    // fee account
    /// CHECK:
    pub fee_account: AccountInfo<'info>,
    pub system_program: Program<'info, System>,
}


/* 
    @dev initialize app state by owner.
    @param fee_larmports: sol amount as fee
*/
pub fn handler(ctx: Context<Initialize>, fee_lamports: u64) -> Result<()> {
    let app_stats: &mut Box<Account<AppStats>> = &mut ctx.accounts.app_stats;
    app_stats.owner = ctx.accounts.owner.key();
    app_stats.fee_account = ctx.accounts.fee_account.key();
    app_stats.fee_lamports = fee_lamports;
    Ok(())
}
