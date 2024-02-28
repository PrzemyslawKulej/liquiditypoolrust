use rand::thread_rng;
use rand::Rng;

// Data structure definitions representing various values in the liquidity pool.
#[derive(Debug, Clone, Copy)]
struct TokenAmount(u64);
#[derive(Debug, Clone, Copy)]
struct StakedTokenAmount(u64);
#[derive(Debug, Clone, Copy)]
struct LpTokenAmount(u64);
#[derive(Debug, Clone, Copy)]
struct Price(u64);
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
struct Percentage(u64);

// Structure representing the liquidity pool.
struct LpPool {
    price: Price,
    token_amount: TokenAmount,
    st_token_amount: StakedTokenAmount,
    lp_token_amount: LpTokenAmount,
    liquidity_target: TokenAmount,
    min_fee: Percentage,
    max_fee: Percentage,
}

// Error definitions that may occur during operations on the liquidity pool.
#[derive(Debug, PartialEq)]
enum Error {
    InsufficientLiquidity,
    InvalidInput,
}

// Checking if the input is u64 or f64, and making conversion accordingly.
impl From<f64> for Price {
    fn from(value: f64) -> Self {
        Price((value * SCALE as f64) as u64)
    }
}

impl Percentage {
    fn to_f64(&self) -> f64 {
        self.0 as f64 / SCALE as f64
    }
}

impl From<f64> for TokenAmount {
    fn from(value: f64) -> Self {
        TokenAmount((value * SCALE as f64) as u64)
    }
}

impl From<f64> for StakedTokenAmount {
    fn from(value: f64) -> Self {
        StakedTokenAmount((value * SCALE as f64) as u64)
    }
}

impl From<f64> for LpTokenAmount {
    fn from(value: f64) -> Self {
        LpTokenAmount((value * SCALE as f64) as u64)
    }
}

const SCALE: u64 = 1_000_000;

// Methods implementation
impl LpPool {
    // Initialize the liquidity pool with basic parameters.
    pub fn init(
        price: Price,
        min_fee: Percentage,
        max_fee: Percentage,
        liquidity_target: TokenAmount,
    ) -> Result<Self, Error> {
        Ok(LpPool {
            price,
            token_amount: TokenAmount(0),
            st_token_amount: StakedTokenAmount(0),
            lp_token_amount: LpTokenAmount(0),
            liquidity_target,
            min_fee,
            max_fee,
        })
    }

    // Add liquidity to the pool.
    pub fn add_liquidity(&mut self, token_amount: TokenAmount) -> Result<f64, Error> {
        if token_amount.0 <= 0 {
            return Err(Error::InvalidInput);
        }

        let lp_tokens_to_mint_f64 = if self.lp_token_amount.0 > 0 {
            // Calculate proportional LP token minting based on existing ones
            (token_amount.0 as f64 * self.lp_token_amount.0 as f64 / self.token_amount.0 as f64)
                / SCALE as f64
        } else {
            // If the pool is empty, mint LP tokens 1:1, return in f64 format
            token_amount.0 as f64 / SCALE as f64
        };

        self.token_amount.0 += token_amount.0;
        self.lp_token_amount.0 += (lp_tokens_to_mint_f64 * SCALE as f64) as u64; // Scale up to u64 to update state

        Ok(lp_tokens_to_mint_f64)
    }

    // Remove liquidity from the pool
    pub fn remove_liquidity(
        &mut self,
        lp_token_amount: LpTokenAmount,
    ) -> Result<(f64, f64), Error> {
        if lp_token_amount.0 <= 0 || lp_token_amount.0 > self.lp_token_amount.0 {
            return Err(Error::InsufficientLiquidity);
        }

        let lp_tokens_share = lp_token_amount.0 as f64 / self.lp_token_amount.0 as f64;

        let token_amount_to_return_f64 =
            self.token_amount.0 as f64 * lp_tokens_share / SCALE as f64;
        let staked_token_amount_to_return_f64 =
            self.st_token_amount.0 as f64 * lp_tokens_share / SCALE as f64;

        self.token_amount.0 -= (token_amount_to_return_f64 * SCALE as f64) as u64; // Scale up to u64 to update state
        self.st_token_amount.0 -= (staked_token_amount_to_return_f64 * SCALE as f64) as u64; // Scale up to u64 to update state
        self.lp_token_amount.0 -= lp_token_amount.0;

        Ok((
            token_amount_to_return_f64,
            staked_token_amount_to_return_f64,
        ))
    }

    // Swap staked tokens
    pub fn swap(&mut self, staked_token_amount: StakedTokenAmount) -> Result<f64, Error> {
        if staked_token_amount.0 <= 0 {
            return Err(Error::InvalidInput);
        }

        let fee_rate = thread_rng().gen_range(self.min_fee.to_f64()..=self.max_fee.to_f64());

        let staked_amount_f64 = staked_token_amount.0 as f64 / SCALE as f64;
        let price_f64 = self.price.0 as f64 / SCALE as f64; // Assuming `self.price.0` is already scaled

        let tokens_to_receive_f64 =
            staked_amount_f64 * price_f64 * (1.0 - fee_rate as f64 / SCALE as f64);

        let tokens_to_receive_scaled = (tokens_to_receive_f64 * SCALE as f64).round() as u64;

        // Check for available liquidity
        if tokens_to_receive_scaled > self.token_amount.0 {
            return Err(Error::InsufficientLiquidity);
        }

        // Update state
        self.token_amount.0 -= tokens_to_receive_scaled;
        self.st_token_amount.0 += staked_token_amount.0;

        // Scale down the result to return the "natural" value
        Ok(tokens_to_receive_f64)
    }
}

//Tests

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_init() {
        // Tests if the liquidity pool can be initialized successfully.
        let lp_pool = LpPool::init(
            Price(1500000),
            Percentage(90000),
            Percentage(900000),
            TokenAmount(90000000),
        );
        assert!(lp_pool.is_ok());
    }

    #[test]
    fn test_add_liquidity() {
        // Tests adding liquidity to the pool and expects it to succeed.
        let mut lp_pool = LpPool::init(
            Price(1500000),
            Percentage(90000),
            Percentage(900000),
            TokenAmount(90000000),
        )
        .unwrap();
        let result = lp_pool.add_liquidity(TokenAmount(100000000));
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 100.0); // Verify the amount of liquidity added matches expectation.
    }

    #[test]
    fn test_remove_liquidity_insufficient() {
        // Tests removing more liquidity than available and expects it to fail.
        let mut lp_pool = LpPool::init(
            Price(1500000),
            Percentage(90000),
            Percentage(900000),
            TokenAmount(90000000),
        )
        .unwrap();
        lp_pool.add_liquidity(TokenAmount(100000000)).unwrap();
        let result = lp_pool.remove_liquidity(LpTokenAmount(200000000)); // Attempt to remove more than available
        assert!(result.is_err());
        assert_eq!(result, Err(Error::InsufficientLiquidity));
    }

    #[test]
    fn test_remove_liquidity_valid() {
        // Tests valid removal of liquidity and expects it to succeed.
        let mut lp_pool = LpPool::init(
            Price(1500000),
            Percentage(90000),
            Percentage(900000),
            TokenAmount(90000000),
        )
        .unwrap();
        lp_pool.add_liquidity(TokenAmount(100000000)).unwrap();
        let result = lp_pool.remove_liquidity(LpTokenAmount(100000000));
        assert!(result.is_ok());
    }

    #[test]
    fn test_swap_invalid_input() {
        // Tests swapping with an invalid input amount (0) and expects it to fail.
        let mut lp_pool = LpPool::init(
            Price(1500000),
            Percentage(90000),
            Percentage(900000),
            TokenAmount(90000000),
        )
        .unwrap();
        let result = lp_pool.swap(StakedTokenAmount(0)); // Attempt to swap with an amount of 0
        assert!(result.is_err());
        assert_eq!(result, Err(Error::InvalidInput));
    }

    #[test]
    fn test_swap_insufficient_liquidity() {
        // Tests swapping with an amount that exceeds available liquidity and expects it to fail.
        let mut lp_pool = LpPool::init(
            Price(1500000),
            Percentage(90000),
            Percentage(900000),
            TokenAmount(90000000),
        )
        .unwrap();
        let result = lp_pool.swap(StakedTokenAmount(100000000000)); // Attempt to swap with a large amount exceeding liquidity
        assert!(result.is_err());
        assert_eq!(result, Err(Error::InsufficientLiquidity));
    }

    #[test]
    fn test_swap_valid() {
        // Tests a valid swap operation and expects it to succeed.
        let mut lp_pool = LpPool::init(
            Price(1500000),
            Percentage(90000),
            Percentage(900000),
            TokenAmount(90000000),
        )
        .unwrap();
        lp_pool.add_liquidity(TokenAmount(100000000)).unwrap();
        let result = lp_pool.swap(StakedTokenAmount(50000000)); // Valid swap operation
        assert!(result.is_ok());
    }
}

fn main() {
    // Initialize the pool with example values
    let min_fee = Percentage((0.9 * SCALE as f64) as u64);
    let max_fee = Percentage((9.0 * SCALE as f64) as u64);
    let mut lp_pool =
        LpPool::init(Price::from(1.5), min_fee, max_fee, TokenAmount::from(90.0)).unwrap();

    // Add liquidity to the pool
    let add_liquidity_result = lp_pool.add_liquidity(TokenAmount::from(100.0)).unwrap();
    println!("Liquidity added: {:?}", add_liquidity_result); 

    // Swap tokens
    let swap_result = lp_pool.swap(StakedTokenAmount::from(6.0)).unwrap();
    println!("Tokens received from swap: {:?}", swap_result); 

    // Add more liquidity
    let add_more_liquidity_result = lp_pool.add_liquidity(TokenAmount::from(10.0)).unwrap();
    println!(
        "Additional liquidity added: {:?}",
        add_more_liquidity_result
    ); 

    // Another token swap
    let second_swap_result = lp_pool.swap(StakedTokenAmount::from(30.0)).unwrap();
    println!("Tokens received from second swap: {:?}", second_swap_result); 

    // Remove liquidity from the pool
    let remove_liquidity_result = lp_pool
        .remove_liquidity(LpTokenAmount::from(109.9991))
        .unwrap();
    println!("Liquidity removed: {:?}", remove_liquidity_result); 
}
