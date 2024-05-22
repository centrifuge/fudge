// TODO: Remove before release
#![allow(dead_code)]
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

pub mod builder;
pub mod digest;
pub mod inherent;
pub mod provider;

#[cfg(test)]
mod tests;

pub mod types {
	pub type Bytes = Vec<u8>;

	#[derive(Clone, Debug)]
	pub struct StoragePair {
		key: Bytes,
		value: Option<Bytes>,
	}
}
