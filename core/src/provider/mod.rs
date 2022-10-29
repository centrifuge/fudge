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
use sc_service::TaskManager;
pub use state::{DbOpen, StateProvider};

mod externalities;
mod initiator;
mod state;

pub trait Initiator {
	type Client;
	type Backend;
	type Executor;

	fn init(self) -> Result<(Self::Client, Self::Backend, Self::Executor, TaskManager), ()>;
}

pub trait GenesisState {}

pub trait BackendProvider {}
