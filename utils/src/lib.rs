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

//! Utilities for using the core lib conveniently
use std::marker::PhantomData;
use codec::{Codec, Encode};
use sp_keystore::SyncCryptoStorePtr;
use sp_runtime::generic::{SignedPayload, UncheckedExtrinsic};
use sp_runtime::{CryptoTypeId, KeyTypeId, MultiSignature};
use sp_runtime::app_crypto::CryptoTypePublicPair;
use sp_runtime::traits::SignedExtension;
use sp_runtime::transaction_validity::TransactionValidity;
use sp_core::sr25519;
use sp_inherents::CreateInherentDataProviders;
use sp_keystore::SyncCryptoStore;

pub struct Signer {
    key_store: SyncCryptoStorePtr,
    crypto: CryptoTypeId,
    key_type: KeyTypeId,
}

impl Signer {
    pub fn new(key_store: SyncCryptoStorePtr, crypto: CryptoTypeId, key_type: KeyTypeId) -> Self {
        Self {
            key_type,
            key_store,
            crypto
        }
    }

    pub fn signed_ext<Address: Codec + Clone, Call: Encode + Clone, Extra: SignedExtension<AccountId = Address> + Clone>(&self, call: Call, signed: Address, extra: Extra) -> Result<UncheckedExtrinsic<Address, Call, MultiSignature, Extra>, ()> {
        let sig = self.signature(call.clone(), signed.clone(), extra.clone()).map_err(|_| ())?;
        // TODO; Only Sr25519 currently supported
        Ok(UncheckedExtrinsic::new_signed(call, signed, MultiSignature::Sr25519(sr25519::Signature::try_from(sig.as_slice()).unwrap()), extra ))
    }

    pub fn signature<Address: Codec, Call: Encode, Extra: SignedExtension<AccountId = Address>>(&self, call: Call, signer: Address, extra: Extra) -> Result<Vec<u8>, ()> {
        let payload = SignedPayload::new(call, extra).map_err(|_| ())?;
        let crypto_pair = CryptoTypePublicPair(self.crypto, signer.encode());
        SyncCryptoStore::sign_with(&(*self.key_store), self.key_type, &crypto_pair, payload.encode().as_slice()).map(|sig| sig.unwrap()).map_err(|_| ())
    }
}