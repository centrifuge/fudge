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

use std::{error::Error, sync::Arc};

use sc_client_api::{
	AuxStore, Backend as BackendT, BlockBackend, BlockOf, HeaderBackend, UsageProvider,
};
use sc_consensus::BlockImport;
use sc_service::TaskManager;
use sc_transaction_pool_api::TransactionPool;
use sp_api::{ApiExt, CallApiAt, ProvideRuntimeApi};
use sp_block_builder::BlockBuilder;
use sp_core::traits::CodeExecutor;
use sp_runtime::traits::{Block as BlockT, BlockIdTo};
use sp_transaction_pool::runtime_api::TaggedTransactionQueue;

pub mod backend;
pub mod externalities;
pub mod initiator;
pub mod state;

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
		+ sc_block_builder::BlockBuilderProvider<Self::Backend, Block, Self::Client>;
	type Backend: 'static + BackendT<Block>;
	type Pool: 'static + TransactionPool<Block = Block>;
	type Executor: 'static + CodeExecutor + Clone;
	type Error: 'static + Error;

	fn init(
		self,
	) -> Result<
		(
			Arc<Self::Client>,
			Arc<Self::Backend>,
			Arc<Self::Pool>,
			Self::Executor,
			TaskManager,
		),
		Self::Error,
	>;
}

pub trait BackendProvider<Block> {
	type Backend: 'static + BackendT<Block>;
	type Error: 'static + Error;

	fn provide(&self) -> Result<Arc<Self::Backend>, Self::Error>;
}
