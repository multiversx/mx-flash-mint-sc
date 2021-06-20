#[test]
fn complete_setup_mandos_go() {
    elrond_wasm_debug::mandos_go("mandos/complete_setup.scen.json");
}

#[test]
fn flash_loan_fail_mandos_go() {
    elrond_wasm_debug::mandos_go("mandos/flash_loan_fail.scen.json");
}

#[test]
fn flash_loan_success_mandos_go() {
    elrond_wasm_debug::mandos_go("mandos/flash_loan_success.scen.json");
}
