# Flash Mint Smart Contract

This Smart Contract is a general purpose Flash Loan Provider with the particularity that it does not need any funds to be added by providers, instead it Mints the requested tokens and lets the user use them as he wishes by the time the transaction ends.

It can be used for any number of tokens, the owner being the one that can configure the per token Loan Service Settings (including for example the fee percent).

## Endpoints

### `flashLoan`

This is the endpoint that a user can use to loan tokens and configure an action to be executed with the tokens. The arguments are:

        loan_token_id: TokenIdentifier,
        loan_amount: Self::BigUint,
        address: Address,
        function: BoxedBytes,
        gas_limit: u64,
        #[var_args] arguments: MultiArgVec<BoxedBytes>,

By calling this endpoint, the following flow of actions with happen (assuming input values are correct):

- The contract will mint `loan_amount` of `loan_token_id`

- The contract will transfer the minted tokens and will execute `function` at given `address` with given `gas_limit` and `arguments`

- The contract will expect the payback to be received via `acceptPay` endpoint. The contract will only accept funds while a flash loan is ongoing and will only accept `loan_token_id` tokens.

- The contract will check if the loan was paid back (the initial amount + a configured fee). The whole transaction will be reverted if this check fails. For example, if a user requests a loan of 100 tokenA and the fee percent for tokenA is 10%, the contract will verify it's balance and will expect to have at least 110 tokenA.

- The contract will send the fees to a configured address.

This function is not re-entrant, meaning that one cannot call it multiple times in a nested way.

### `configureTokenLoanServiceSettings`

This endpoint can be used only by the owner in order to configure the Loan Service Settings for a speficic Token. The arguments are:

        token_id: TokenIdentifier,
        minimum_loan_amount: Self::BigUint,
        maximum_loan_amount: Self::BigUint,
        fee_percent: u64,
        fee_transfer_gas_limit: u64,
        fee_destination_addr: Address,
        fee_destination_func: BoxedBytes,

- Any Flash Mint Loans with the `token_id` will respect the boundry configured [`minimum_loan_amount`, `maximum_loan_amount`].

- Any Successful Flash Mint Loans with the `token_id` will produce a fee of the configured `fee_percent` and will transfer the fee tokens them and execute `fee_destination_func` at `fee_destination_addr` with `fee_transfer_gas_limit`.

After calling this function, the owner must make sure the contract has `EsdtLocalMint` and `EsdtLocalBurn` roles for the configured `token_id`.

### `removeTokenLoanService`

This endpoint can be used only by the owner in order to delete the settings for a specific token, hence will not allow Flash Mint Loans for it.

### `getTokenLoanServiceSettingsList` and `getLoanServiceSettings`

This view function can be used by anyone. Their purpose is to allow the user check the tokens that are available for Flash Mint Lending and what are the Loan Service Settings (most important what is the fee_percent).
