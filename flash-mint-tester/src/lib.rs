#![no_std]
#![allow(clippy::too_many_arguments)]

elrond_wasm::imports!();

const FINISH_EXEC_GAS_THRESHOLD: u64 = 100_000;
const PAYBACK_FUNCTION_NAME: &[u8] = b"acceptPayback";

#[elrond_wasm_derive::contract]
pub trait FlashMintTester {
    #[init]
    fn init(&self) {}

    #[payable("*")]
    #[endpoint(testArbitrage)]
    fn test_arbitrage(
        &self,
        #[payment_token] payment_token: TokenIdentifier,
        #[payment_amount] _amount: Self::BigUint,
        amount_to_return: Self::BigUint,
        _token_id: TokenIdentifier,
        _boxed_bytes: BoxedBytes,
        _big_uint: Self::BigUint,
        _u64: u64,
    ) {
        let _ = self.send().direct_esdt_execute(
            &self.blockchain().get_caller(),
            &payment_token,
            &amount_to_return,
            self.blockchain().get_gas_left() - FINISH_EXEC_GAS_THRESHOLD,
            PAYBACK_FUNCTION_NAME,
            &ArgBuffer::new(),
        );
    }
}
