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

pub use para_parachain::Inherent as FudgeInherentParaParachain;
pub use relay_parachain::{
	DummyInherent as FudgeDummyInherentRelayParachain, Inherent as FudgeInherentRelayParachain,
};
pub use sp_inherents::CreateInherentDataProviders;
pub use timestamp::CurrTimeProvider as FudgeInherentTimestamp;

mod para_parachain;
mod relay_parachain;
mod timestamp;

pub trait ArgsProvider<ExtraArgs> {
	fn extra() -> ExtraArgs;
}

impl ArgsProvider<()> for () {
	fn extra() -> () {
		()
	}
}
