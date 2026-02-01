#[cfg(not(any(feature = "platform-linux")))]
compile_error!("Please select at least one platform feature flag.");

#[cfg(not(any(feature = "display-protocol-x11")))]
compile_error!("Please select at least one display protocol feature flag.");

pub mod error;
pub mod windowing;
