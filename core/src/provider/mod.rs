// Copyright 2021 Centrifuge Foundation (centrifuge.io).
//
// This file is part of the FUDGE project.
// FUDGE is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version (see http://www.gnu.org/licenses).
// Centrifuge is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

pub use externalities::ExternalitiesProvider;
pub use initiator::Init;
use sc_client_api::{
	AuxStore, Backend as BackendT, BlockBackend, BlockOf, CallExecutor, HeaderBackend,
	UsageProvider,
};
use sc_consensus::BlockImport;
use sc_executor::RuntimeVersionOf;
use sc_service::TaskManager;
use sc_transaction_pool_api::TransactionPool;
use sp_api::{ApiExt, CallApiAt, ProvideRuntimeApi};
use sp_core::traits::CodeExecutor;
use sp_runtime::traits::{Block as BlockT, BlockIdTo};
pub use state::{DbOpen, StateProvider};

mod externalities;
mod initiator;
mod state;

pub trait Initiator<Block: BlockT>
where
	<Self::Client as ProvideRuntimeApi<Block>>::Api: BlockBuilder<Block>
		+ ApiExt<Block, StateBackend = <Self::Backend as BackendT<Block>>::State>
		+ TaggedTransactionQueue<Block>,
{
	type Client: 'static
		+ ProvideRuntimeApi<Block>
		+ BlockOf
		+ BlockBackend<Block>
		+ BlockIdTo<Block>
		+ Send
		+ Sync
		+ AuxStore
		+ UsageProvider<Block>
		+ HeaderBackend<Block>
		+ BlockImport<Block>
		+ CallApiAt<Block>
		+ sc_block_builder::BlockBuilderProvider<B, Block, C>;
	type Backend: 'static + BackendT<Block>;
	type Pool: 'static + TransactionPool<Block = Block>;
	type Executor: 'static + CodeExecutor + Clone;

	fn init(
		self,
	) -> Result<
		(
			Self::Client,
			Self::Backend,
			Self::Pool,
			Self::Executor,
			TaskManager,
		),
		(),
	>;
}

pub trait GenesisState {}

pub trait BackendProvider {}
