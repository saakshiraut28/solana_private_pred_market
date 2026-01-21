use anchor_lang::prelude::*;

// Market account - stores market information
#[account]
#[derive(InitSpace)]
pub struct Market {
    pub creator: Pubkey,         
    #[max_len(200)]     
    pub question: String,             
    #[max_len(50)]
    pub kalshi_market_id: String,     
    pub liquidity_param: u64,         
    pub end_time: i64,                
    pub resolved: bool,               
    pub outcome: bool,                
    
    pub total_yes_shares: u64,        
    pub total_no_shares: u64,         
    
    pub current_yes_probability: u64, 
    pub total_liquidity: u64,         
    
    pub oracle_authority: Pubkey,     
    
    pub created_at: i64,
    pub bump: u8,                     
}

impl Market {
    pub const LEN: usize = 8usize
        .checked_add(32).unwrap()          // creator
        .checked_add(4 + 200).unwrap()     // question
        .checked_add(4 + 50).unwrap()      // kalshi_market_id
        .checked_add(8).unwrap()           // liquidity_param
        .checked_add(8).unwrap()           // end_time
        .checked_add(1).unwrap()           // resolved
        .checked_add(1).unwrap()           // outcome
        .checked_add(8).unwrap()           // total_yes_shares
        .checked_add(8).unwrap()           // total_no_shares
        .checked_add(8).unwrap()           // current_yes_probability
        .checked_add(8).unwrap()           // total_liquidity
        .checked_add(32).unwrap()          // oracle_authority
        .checked_add(8).unwrap()           // created_at
        .checked_add(1).unwrap();          // bump
}


// User position - stores individual user's bets
#[account]
#[derive(InitSpace)]
pub struct UserPosition {
    pub market: Pubkey,               
    pub user: Pubkey,                 
    pub yes_shares: u64,              
    pub no_shares: u64,               
    pub total_deposited: u64,         
    pub claimed: bool,                
    pub bump: u8,
}

impl UserPosition {
    pub const LEN: usize = 8usize
    .checked_add(32).unwrap()
    .checked_add(32).unwrap()
    .checked_add(8).unwrap()
    .checked_add(8).unwrap()
    .checked_add(8).unwrap()
    .checked_add(1).unwrap()
    .checked_add(1).unwrap();
}