#[deprecated(
    since = "1.15.0",
    note = "Please use `domichain_quic_client::nonblocking::quic_client::QuicClientConnection` instead."
)]
pub use domichain_quic_client::nonblocking::quic_client::QuicClientConnection as QuicTpuConnection;
pub use domichain_quic_client::nonblocking::quic_client::{
    QuicClient, QuicClientCertificate, QuicLazyInitializedEndpoint,
};
