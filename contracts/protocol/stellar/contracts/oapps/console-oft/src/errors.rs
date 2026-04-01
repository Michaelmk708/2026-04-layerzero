use common_macros::contract_error;

#[contract_error]
pub enum OFTError {
    /// The function is disabled.
    Disabled,
}
