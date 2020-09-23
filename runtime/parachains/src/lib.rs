// Copyright 2020 Parity Technologies (UK) Ltd.
// This file is part of Polkadot.

// Polkadot is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Polkadot is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Polkadot.  If not, see <http://www.gnu.org/licenses/>.

//! Runtime modules for parachains code.
//!
//! It is crucial to include all the modules from this crate in the runtime, in
//! particular the `Initializer` module, as it is responsible for initializing the state
//! of the other modules.

#![cfg_attr(not(feature = "std"), no_std)]

use sp_std::result;
use sp_runtime::traits::BadOrigin;
use primitives::v1::Id as ParaId;
use codec::{Decode, Encode};

pub mod configuration;
pub mod inclusion;
pub mod inclusion_inherent;
pub mod initializer;
pub mod paras;
pub mod router;
pub mod scheduler;
pub mod validity;

pub mod runtime_api_impl;

mod util;

#[cfg(test)]
mod mock;

/// Origin for the parachains.
#[derive(PartialEq, Eq, Clone, Encode, Decode)]
#[cfg_attr(feature = "std", derive(Debug))]
pub enum Origin {
	/// It comes from a parachain.
	Parachain(ParaId),
}

/// Ensure that the origin `o` represents a parachain.
/// Returns `Ok` with the parachain ID that effected the extrinsic or an `Err` otherwise.
pub fn ensure_parachain<OuterOrigin>(o: OuterOrigin) -> result::Result<ParaId, BadOrigin>
	where OuterOrigin: Into<result::Result<Origin, OuterOrigin>>
{
	match o.into() {
		Ok(Origin::Parachain(id)) => Ok(id),
		_ => Err(BadOrigin),
	}
}

#[doc(hidden)]
pub mod dummy {
	// There is no way to register an origin type in `construct_runtime` without a pallet the origin belongs
	// to.
	//
	// However, in this case, the origin doesn't belong to a particular pallet, but rather is meant to
	// be shared among multiple pallets. Instead of choosing an arbitrary Module to attach the origin
	// to, we declare a dummy module that doesn't do anything and just serves as a home base for the
	// origin.
	//
	// ideally, though, the `construct_runtime` should support a free-standing origin.

	pub use super::Origin;

	pub trait Trait: frame_system::Trait {}
	frame_support::decl_module! {
		pub struct Module<T: Trait> for enum Call where origin: <T as frame_system::Trait>::Origin {}
	}
}
