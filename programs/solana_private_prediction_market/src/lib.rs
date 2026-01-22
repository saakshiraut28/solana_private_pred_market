use anchor_lang::prelude::*;
use anchor_lang::solana_program::system_instruction;

pub mod state;
use state::*;

declare_id!("6Q1Tj3ef6vEYkRbNh96pkP6WL6ktqq2YNhBb1ntJLudg");

#[program]
pub mod solana_private_prediction_market {
    use super::*;

    pub fn create_market(
        ctx: Context<CreateMarket>,
        question: String,
        liquidity_param: u64,
        end_time: i64,
    ) -> Result<()> {
        require!(question.len() <= 200, ErrorCode::QuestionTooLong);
        require!(liquidity_param > 0, ErrorCode::InvalidLiquidity);
        require!(end_time > Clock::get()?.unix_timestamp, ErrorCode::InvalidEndTime);

        let market = &mut ctx.accounts.market;
        let clock = Clock::get()?;

        market.creator = ctx.accounts.creator.key();
        market.question = question;
        market.liquidity_param = liquidity_param;
        market.end_time = end_time;
        market.resolved = false;
        market.outcome = false;
        market.total_yes_shares = 0;
        market.total_no_shares = 0;
        market.current_yes_probability = 500_000;
        market.total_liquidity = liquidity_param;
        market.oracle_authority = ctx.accounts.creator.key();
        market.created_at = clock.unix_timestamp;
        market.bump = ctx.bumps.market;

        let transfer_instruction = system_instruction::transfer(
            &ctx.accounts.creator.key(),
            &ctx.accounts.market_vault.key(),
            liquidity_param,
        );

        anchor_lang::solana_program::program::invoke(
            &transfer_instruction,
            &[
                ctx.accounts.creator.to_account_info(),
                ctx.accounts.market_vault.to_account_info(),
            ],
        )?;

        msg!("Market created: {}", market.question);

        Ok(())
    }

    pub fn place_bet(
        ctx: Context<PlaceBet>,
        amount: u64,
        is_yes: bool,
    ) -> Result<()> {
        let market = &mut ctx.accounts.market;
        let user_position = &mut ctx.accounts.user_position;
        let clock = Clock::get()?;

        require!(!market.resolved, ErrorCode::MarketResolved);
        require!(clock.unix_timestamp < market.end_time, ErrorCode::MarketEnded);
        require!(amount > 0, ErrorCode::InvalidAmount);

        let shares = calculate_shares(
            amount,
            is_yes,
            market.total_yes_shares,
            market.total_no_shares,
            market.liquidity_param,
        )?;

        let transfer_instruction = system_instruction::transfer(
            &ctx.accounts.user.key(),
            &ctx.accounts.market_vault.key(),
            amount,
        );

        anchor_lang::solana_program::program::invoke(
            &transfer_instruction,
            &[
                ctx.accounts.user.to_account_info(),
                ctx.accounts.market_vault.to_account_info(),
            ],
        )?;

        if is_yes {
            market.total_yes_shares = market.total_yes_shares
                .checked_add(shares)
                .ok_or(ErrorCode::ArithmeticOverflow)?;
            
            user_position.yes_shares = user_position.yes_shares
                .checked_add(shares)
                .ok_or(ErrorCode::ArithmeticOverflow)?;
        } else {
            market.total_no_shares = market.total_no_shares
                .checked_add(shares)
                .ok_or(ErrorCode::ArithmeticOverflow)?;
            
            user_position.no_shares = user_position.no_shares
                .checked_add(shares)
                .ok_or(ErrorCode::ArithmeticOverflow)?;
        }

        market.total_liquidity = market.total_liquidity
            .checked_add(amount)
            .ok_or(ErrorCode::ArithmeticOverflow)?;
        
        user_position.total_deposited = user_position.total_deposited
            .checked_add(amount)
            .ok_or(ErrorCode::ArithmeticOverflow)?;

        market.current_yes_probability = calculate_probability(
            market.total_yes_shares,
            market.total_no_shares,
            market.liquidity_param,
        )?;

        if user_position.market == Pubkey::default() {
            user_position.market = market.key();
            user_position.user = ctx.accounts.user.key();
            user_position.claimed = false;
            user_position.bump = ctx.bumps.user_position;
        }

        msg!("Bet placed: {} shares of {}", shares, if is_yes { "YES" } else { "NO" });

        Ok(())
    }

    pub fn update_price(
        ctx: Context<UpdatePrice>,
        new_probability: u64,
    ) -> Result<()> {
        let market = &mut ctx.accounts.market;

        require!(!market.resolved, ErrorCode::MarketResolved);
        require!(new_probability <= 1_000_000, ErrorCode::InvalidProbability);

        market.current_yes_probability = new_probability;

        msg!("Price updated to: {}%", new_probability / 10_000);

        Ok(())
    }

    pub fn resolve_market(
        ctx: Context<ResolveMarket>,
        outcome: bool,
    ) -> Result<()> {
        let market = &mut ctx.accounts.market;
        let clock = Clock::get()?;

        require!(!market.resolved, ErrorCode::AlreadyResolved);
        require!(clock.unix_timestamp >= market.end_time, ErrorCode::MarketNotEnded);

        market.resolved = true;
        market.outcome = outcome;

        msg!("Market resolved: {}", if outcome { "YES" } else { "NO" });

        Ok(())
    }

    pub fn claim_winnings(ctx: Context<ClaimWinnings>) -> Result<()> {
        let market = &ctx.accounts.market;
        let user_position = &mut ctx.accounts.user_position;

        require!(market.resolved, ErrorCode::NotResolved);
        require!(!user_position.claimed, ErrorCode::AlreadyClaimed);

        let winning_shares = if market.outcome {
            user_position.yes_shares
        } else {
            user_position.no_shares
        };

        let total_winning_shares = if market.outcome {
            market.total_yes_shares
        } else {
            market.total_no_shares
        };

        require!(winning_shares > 0, ErrorCode::NoWinnings);
        require!(total_winning_shares > 0, ErrorCode::InvalidMarketState);

        let payout = (winning_shares as u128)
            .checked_mul(market.total_liquidity as u128)
            .ok_or(ErrorCode::ArithmeticOverflow)?
            .checked_div(total_winning_shares as u128)
            .ok_or(ErrorCode::ArithmeticOverflow)?;
        
        let payout = u64::try_from(payout)
            .map_err(|_| ErrorCode::ArithmeticOverflow)?;

        **ctx.accounts.market_vault.try_borrow_mut_lamports()? = ctx.accounts.market_vault
            .lamports()
            .checked_sub(payout)
            .ok_or(ErrorCode::InsufficientFunds)?;
        
        **ctx.accounts.user.try_borrow_mut_lamports()? = ctx.accounts.user
            .lamports()
            .checked_add(payout)
            .ok_or(ErrorCode::ArithmeticOverflow)?;

        user_position.claimed = true;

        msg!("Winnings claimed: {} lamports", payout);

        Ok(())
    }
}

fn calculate_shares(
    amount: u64,
    _is_yes: bool,
    _total_yes: u64,
    _total_no: u64,
    _liquidity_param: u64,
) -> Result<u64> {
    // will complete later after research
    Ok(amount)
}

fn calculate_probability(
    yes_shares: u64,
    no_shares: u64,
    _liquidity_param: u64,
) -> Result<u64> {
    if yes_shares == 0 && no_shares == 0 {
        return Ok(500_000);
    }

    let total = yes_shares
        .checked_add(no_shares)
        .ok_or(ErrorCode::ArithmeticOverflow)?;
    
    let probability = (yes_shares as u128)
        .checked_mul(1_000_000)
        .ok_or(ErrorCode::ArithmeticOverflow)?
        .checked_div(total as u128)
        .ok_or(ErrorCode::ArithmeticOverflow)?;
    
    let probability = u64::try_from(probability)
        .map_err(|_| ErrorCode::ArithmeticOverflow)?;

    Ok(probability)
}

#[derive(Accounts)]
#[instruction(question: String)]
pub struct CreateMarket<'info> {
    #[account(
        init,
        payer = creator,
        space = Market::LEN,
        seeds = [
            b"market",
            creator.key().as_ref(),
        ],
        bump
    )]
    pub market: Account<'info, Market>,

    /// CHECK: PDA vault for holding funds
    #[account(
        mut,
        seeds = [b"vault", market.key().as_ref()],
        bump
    )]
    pub market_vault: AccountInfo<'info>,

    #[account(mut)]
    pub creator: Signer<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct PlaceBet<'info> {
    #[account(mut)]
    pub market: Account<'info, Market>,

    /// CHECK: Market vault
    #[account(
        mut,
        seeds = [b"vault", market.key().as_ref()],
        bump
    )]
    pub market_vault: AccountInfo<'info>,

    #[account(
        init_if_needed,
        payer = user,
        space = UserPosition::LEN,
        seeds = [
            b"position",
            market.key().as_ref(),
            user.key().as_ref()
        ],
        bump
    )]
    pub user_position: Account<'info, UserPosition>,

    #[account(mut)]
    pub user: Signer<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct UpdatePrice<'info> {
    #[account(
        mut,
        has_one = oracle_authority
    )]
    pub market: Account<'info, Market>,

    pub oracle_authority: Signer<'info>,
}

#[derive(Accounts)]
pub struct ResolveMarket<'info> {
    #[account(
        mut,
        has_one = oracle_authority
    )]
    pub market: Account<'info, Market>,

    pub oracle_authority: Signer<'info>,
}

#[derive(Accounts)]
pub struct ClaimWinnings<'info> {
    #[account(mut)]
    pub market: Account<'info, Market>,

    /// CHECK: Vault PDA
    #[account(
        mut,
        seeds = [b"vault", market.key().as_ref()],
        bump
    )]
    pub market_vault: AccountInfo<'info>,

    #[account(
        mut,
        has_one = market,
        has_one = user
    )]
    pub user_position: Account<'info, UserPosition>,

    #[account(mut)]
    pub user: Signer<'info>,
}

#[error_code]
pub enum ErrorCode {
    #[msg("Question exceeds 200 characters")]
    QuestionTooLong,
    
    #[msg("Market ID exceeds 50 characters")]
    IdTooLong,
    
    #[msg("Invalid liquidity parameter")]
    InvalidLiquidity,
    
    #[msg("Invalid end time")]
    InvalidEndTime,
    
    #[msg("Market already resolved")]
    MarketResolved,
    
    #[msg("Market has ended")]
    MarketEnded,
    
    #[msg("Invalid bet amount")]
    InvalidAmount,
    
    #[msg("Invalid probability value")]
    InvalidProbability,
    
    #[msg("Market already resolved")]
    AlreadyResolved,
    
    #[msg("Market not ended yet")]
    MarketNotEnded,
    
    #[msg("Market not resolved")]
    NotResolved,
    
    #[msg("Already claimed")]
    AlreadyClaimed,
    
    #[msg("No winnings to claim")]
    NoWinnings,
    
    #[msg("Arithmetic overflow or underflow")]
    ArithmeticOverflow,
    
    #[msg("Insufficient funds in vault")]
    InsufficientFunds,
    
    #[msg("Invalid market state")]
    InvalidMarketState,
}