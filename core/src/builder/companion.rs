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

//! A builder that "mimic" a relay-chain-parachain setup and eases
//! usage of the underlying Parachain- and RelayChain-Builder.

use frame_support::traits::Get;
use sc_client_api::{
	blockchain::ProvideCache, AuxStore, Backend as BackendT, BlockOf, HeaderBackend, UsageProvider,
};
use sc_client_db::Backend;
use sc_consensus::{BlockImport, BlockImportParams, ForkChoiceStrategy};
use sc_executor::RuntimeVersionOf;
use sc_service::{Configuration, SpawnTaskHandle, TFullClient};
use sp_api::{ApiExt, CallApiAt, ConstructRuntimeApi, Core as CoreApi, ProvideRuntimeApi};
use sp_block_builder::BlockBuilder;
use sp_consensus::{BlockOrigin, CanAuthorWith, Error as ConsensusError};
use sp_core::{
	traits::{CodeExecutor, ReadRuntimeVersion},
	Pair,
};
use sp_inherents::{CreateInherentDataProviders, InherentDataProvider};
use sp_runtime::{generic::BlockId, traits::Block as BlockT};
use sp_std::time::Duration;
use sp_std::{marker::PhantomData, sync::Arc};
