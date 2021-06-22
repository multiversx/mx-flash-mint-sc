#![no_std]
#![allow(clippy::too_many_arguments)]

elrond_wasm::imports!();
elrond_wasm::derive_imports!();

type Nonce = u64;

use core::iter::FromIterator;
const PERCENT_BASE_PRECISION: u64 = 1_000;

#[derive(TopEncode, TopDecode, NestedEncode, NestedDecode, PartialEq, TypeAbi)]
pub struct LoanServiceSettings<BigUint: BigUintApi> {
    minimum_loan_amount: BigUint,
    maximum_loan_amount: BigUint,
    fee_percent: u64,
    fee_transfer_gas_limit: u64,
    fee_destination_addr: Address,
    fee_destination_func: BoxedBytes,
}

#[derive(TopEncode, TopDecode, PartialEq, TypeAbi)]
pub struct ScCall<BigUint: BigUintApi> {
    payment_token_id: TokenIdentifier,
    payment_amount: BigUint,
    address: Address,
    function: BoxedBytes,
    gas_limit: u64,
    arguments: Vec<BoxedBytes>,
}

#[derive(TopEncode, TopDecode, PartialEq, TypeAbi)]
pub struct OngoingLoan<BigUint: BigUintApi> {
    loan_token_id: TokenIdentifier,
    payback_amount: BigUint,
}

#[elrond_wasm_derive::contract]
pub trait FlashMintProvider {
    #[init]
    fn init(&self) {}

    #[payable("*")]
    #[endpoint(acceptPayback)]
    fn accept_payback(
        &self,
        #[payment_token] token_id: TokenIdentifier,
        #[payment_amount] amount: Self::BigUint,
        #[payment_nonce] nonce: Nonce,
    ) -> SCResult<()> {
        require!(self.is_ongoing_flash_loan(), "Flash loan not ongoing");
        require!(nonce == 0, "Can only receive ESDT tokens");
        let mut ongoing_loan = self.ongoing_flash_loan().get();
        require!(
            ongoing_loan.loan_token_id == token_id,
            "Payment token differs from lend token"
        );

        if amount > 0 {
            ongoing_loan.payback_amount += amount;
            self.ongoing_flash_loan().set(&ongoing_loan);
        }
        Ok(())
    }

    #[endpoint(flashLoan)]
    fn flash_loan(
        &self,
        loan_token_id: TokenIdentifier,
        loan_amount: Self::BigUint,
        address: Address,
        function: BoxedBytes,
        gas_limit: u64,
        #[var_args] arguments: MultiArgVec<BoxedBytes>,
    ) -> SCResult<()> {
        require!(loan_amount != 0, "Loan amount cannot be zero");
        require!(
            self.token_loan_service_settings()
                .contains_key(&loan_token_id),
            "Token not configured"
        );
        let loan_service_settings = self
            .token_loan_service_settings()
            .get(&loan_token_id)
            .unwrap();
        require!(
            loan_amount >= loan_service_settings.minimum_loan_amount,
            "Requested amount is lower than minimum configured"
        );
        require!(
            loan_amount <= loan_service_settings.maximum_loan_amount,
            "Requested amount is higher than maximum configured"
        );
        self.require_local_burn_and_mint_roles_set(&loan_token_id)?;
        require!(
            !self.is_ongoing_flash_loan(),
            "Flash loan is already ongoing"
        );

        self.mint(&loan_token_id, &loan_amount);
        self.ongoing_flash_loan().set(&OngoingLoan {
            loan_token_id: loan_token_id.clone(),
            payback_amount: Self::BigUint::zero(),
        });

        let sc_call = ScCall {
            payment_token_id: loan_token_id.clone(),
            payment_amount: loan_amount.clone(),
            address,
            function,
            gas_limit,
            arguments: arguments.into_vec(),
        };
        self.execute_sc_call(&sc_call)?;
        let received_amount = self.ongoing_flash_loan().get().payback_amount;
        self.require_paid_back_loan(&loan_amount, &received_amount, &loan_service_settings)?;

        self.ongoing_flash_loan().clear();
        self.burn(&loan_token_id, &loan_amount);

        self.send_fees(
            &loan_token_id,
            &(received_amount - loan_amount),
            &loan_service_settings,
        )
    }

    #[endpoint(configureTokenLoanServiceSettings)]
    fn configure_per_token_loan_service_settings(
        &self,
        token_id: TokenIdentifier,
        minimum_loan_amount: Self::BigUint,
        maximum_loan_amount: Self::BigUint,
        fee_percent: u64,
        fee_transfer_gas_limit: u64,
        fee_destination_addr: Address,
        fee_destination_func: BoxedBytes,
    ) -> SCResult<()> {
        self.require_owner()?;
        require!(
            token_id.is_valid_esdt_identifier(),
            "Not a valid ESDT identifier"
        );
        require!(
            fee_percent < PERCENT_BASE_PRECISION,
            "Fee percent above maximum allowed"
        );
        require!(maximum_loan_amount > 0, "Maximum amount cannot be zero");
        require!(
            minimum_loan_amount <= maximum_loan_amount,
            "Minimum amount larger than maximum amount"
        );

        let loan_service_settings = LoanServiceSettings {
            minimum_loan_amount,
            maximum_loan_amount,
            fee_percent,
            fee_transfer_gas_limit,
            fee_destination_addr,
            fee_destination_func,
        };
        self.token_loan_service_settings()
            .insert(token_id, loan_service_settings);
        Ok(())
    }

    #[endpoint(removeTokenLoanService)]
    fn remove_token_loan_service(&self, token_id: TokenIdentifier) -> SCResult<()> {
        self.require_owner()?;
        self.token_loan_service_settings().remove(&token_id);
        Ok(())
    }

    fn send_fees(
        &self,
        token_id: &TokenIdentifier,
        amount: &Self::BigUint,
        loan_service_settings: &LoanServiceSettings<Self::BigUint>,
    ) -> SCResult<()> {
        if amount > &0 {
            let sc_call = ScCall {
                payment_token_id: token_id.clone(),
                payment_amount: amount.clone(),
                address: loan_service_settings.fee_destination_addr.clone(),
                function: loan_service_settings.fee_destination_func.clone(),
                gas_limit: loan_service_settings.fee_transfer_gas_limit,
                arguments: Vec::new(),
            };
            self.execute_sc_call(&sc_call)
        } else {
            Ok(())
        }
    }

    fn execute_sc_call(&self, sc_call: &ScCall<Self::BigUint>) -> SCResult<()> {
        let mut arg_buffer = ArgBuffer::new();
        for arg in sc_call.arguments.clone() {
            arg_buffer.push_argument_bytes(arg.as_slice());
        }

        self.send().direct_esdt_execute(
            &sc_call.address,
            &sc_call.payment_token_id,
            &sc_call.payment_amount,
            sc_call.gas_limit,
            sc_call.function.as_slice(),
            &arg_buffer,
        )?;
        Ok(())
    }

    fn mint(&self, token_id: &TokenIdentifier, amount: &Self::BigUint) {
        self.send().esdt_local_mint(token_id, amount);
    }

    fn burn(&self, token_id: &TokenIdentifier, amount: &Self::BigUint) {
        self.send().esdt_local_burn(token_id, amount);
    }

    fn is_ongoing_flash_loan(&self) -> bool {
        !self.ongoing_flash_loan().is_empty()
    }

    fn require_paid_back_loan(
        &self,
        lend_amount: &Self::BigUint,
        received_amount: &Self::BigUint,
        token_loan_service_settings: &LoanServiceSettings<Self::BigUint>,
    ) -> SCResult<()> {
        require!(
            received_amount
                >= &(lend_amount
                    * &Self::BigUint::from(
                        token_loan_service_settings.fee_percent + PERCENT_BASE_PRECISION
                    )
                    / Self::BigUint::from(PERCENT_BASE_PRECISION)),
            "Did not pay back loan"
        );
        Ok(())
    }

    fn require_owner(&self) -> SCResult<()> {
        only_owner!(self, "Permission denied");
        Ok(())
    }

    fn require_local_burn_and_mint_roles_set(&self, token_id: &TokenIdentifier) -> SCResult<()> {
        let roles = self.blockchain().get_esdt_local_roles(token_id);
        require!(
            roles.contains(&EsdtLocalRole::Mint),
            "Local Mint role not set"
        );
        require!(
            roles.contains(&EsdtLocalRole::Burn),
            "Local Burn role not set"
        );
        Ok(())
    }

    #[view(getTokenLoanServiceSettingsList)]
    fn get_token_loan_service_settings_list(
        &self,
    ) -> MultiResultVec<(TokenIdentifier, LoanServiceSettings<Self::BigUint>)> {
        MultiResultVec::from_iter(
            self.token_loan_service_settings()
                .iter()
                .collect::<Vec<(TokenIdentifier, LoanServiceSettings<Self::BigUint>)>>(),
        )
    }

    #[view(getLoanServiceSettings)]
    fn get_loan_service_settings(
        &self,
        token: TokenIdentifier,
    ) -> Option<LoanServiceSettings<Self::BigUint>> {
        self.token_loan_service_settings().get(&token)
    }

    #[storage_mapper("token_loan_service_settings")]
    fn token_loan_service_settings(
        &self,
    ) -> MapMapper<Self::Storage, TokenIdentifier, LoanServiceSettings<Self::BigUint>>;

    #[storage_mapper("ongoing_flash_loan")]
    fn ongoing_flash_loan(&self) -> SingleValueMapper<Self::Storage, OngoingLoan<Self::BigUint>>;
}
