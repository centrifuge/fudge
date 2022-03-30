///! FUDGE - FUlly Decoupled Generic Environment for Substrate-based Chains
///!
///! Generally only this dependency is needed in order to use FUDGE.
///! Developers who want to use the more raw apis and types are
///! referred to the fudge-core repository.
// Re-export everything nicely as a single crate
pub use fudge_companion::companion;

pub mod primitives {
	pub use fudge_core::FudgeParaChain;
	pub use polkadot_parachain::primitives::Id as ParaId;

	pub enum Chain {
		Relay,
		Para(u32),
	}
}
