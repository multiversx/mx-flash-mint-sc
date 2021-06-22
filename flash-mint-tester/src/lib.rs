#![no_std]
#![allow(clippy::too_many_arguments)]

elrond_wasm::imports!();
elrond_wasm::derive_imports!();

const PAYBACK_FUNCTION_NAME: &[u8] = b"acceptPayback";
const MULTI_PAIR_SWAP_FUNCTION_NAME: &[u8] = b"multiPairSwap";

#[elrond_wasm_derive::contract]
pub trait FlashMintTester {
    #[init]
    fn init(&self) {}

    #[payable("*")]
    #[endpoint(testEcho)]
    fn test_echo(
        &self,
        #[payment_token] payment_token: TokenIdentifier,
        #[payment_amount] _amount: Self::BigUint,
        amount_to_return: Self::BigUint,
        _token_id: TokenIdentifier,
        _boxed_bytes: BoxedBytes,
        _big_uint: Self::BigUint,
        _u64: u64,
    ) -> SCResult<()> {
        let _ = self.send().direct_esdt_execute(
            &self.blockchain().get_caller(),
            &payment_token,
            &amount_to_return,
            self.blockchain().get_gas_left(),
            PAYBACK_FUNCTION_NAME,
            &ArgBuffer::new(),
        )?;
        Ok(())
    }

    #[payable("*")]
    #[endpoint(testArbitrage)]
    fn test_arbitrage(
        &self,
        #[payment_token] payment_token: TokenIdentifier,
        #[payment_amount] amount: Self::BigUint,
        roter_address: Address,
        user_address: Address,
        payback_amount: Self::BigUint,
        #[var_args] swap_operations_args: MultiArgVec<BoxedBytes>,
    ) -> SCResult<()> {
        let mut arg_buffer = ArgBuffer::new();
        for arg in swap_operations_args.into_vec() {
            arg_buffer.push_argument_bytes(arg.as_slice());
        }

        let balance_before = &self.blockchain().get_esdt_balance(
            &self.blockchain().get_sc_address(),
            &payment_token,
            0,
        ) - &amount;

        let _ = self.send().direct_esdt_execute(
            &roter_address,
            &payment_token,
            &amount,
            self.blockchain().get_gas_left() / 2,
            MULTI_PAIR_SWAP_FUNCTION_NAME,
            &arg_buffer,
        )?;

        let _ = self.send().direct_esdt_execute(
            &self.blockchain().get_caller(),
            &payment_token,
            &payback_amount,
            self.blockchain().get_gas_left(),
            &PAYBACK_FUNCTION_NAME,
            &ArgBuffer::new(),
        )?;

        let balance_after = self.blockchain().get_esdt_balance(
            &self.blockchain().get_sc_address(),
            &payment_token,
            0,
        );

        let _ = self.send().direct_esdt_execute(
            &user_address,
            &payment_token,
            &(balance_after - balance_before),
            self.blockchain().get_gas_left(),
            &[],
            &ArgBuffer::new(),
        )?;
        Ok(())
    }
}
