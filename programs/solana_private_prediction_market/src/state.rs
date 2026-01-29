use anchor_lang::prelude::*;

#[account]
pub struct Market {
    pub creator: Pubkey,                    
    pub question: String,                   
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
    pub const LEN: usize = 8_usize              
        .checked_add(32).unwrap()                
        .checked_add(4 + 200).unwrap()           
        .checked_add(8).unwrap()               
        .checked_add(8).unwrap()                 
        .checked_add(1).unwrap()                 
        .checked_add(1).unwrap()                 
        .checked_add(32).unwrap()                
        .checked_add(32).unwrap()              
        .checked_add(32).unwrap()                
        .checked_add(8).unwrap()                 
        .checked_add(32).unwrap()               
        .checked_add(8).unwrap()                
        .checked_add(1).unwrap();   
}

#[account]
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
    pub const LEN: usize = 8_usize              
        .checked_add(32).unwrap()               
        .checked_add(32).unwrap()               
        .checked_add(32).unwrap()              
        .checked_add(32).unwrap()               
        .checked_add(8).unwrap()                 
        .checked_add(1).unwrap()                 
        .checked_add(1).unwrap();     
}