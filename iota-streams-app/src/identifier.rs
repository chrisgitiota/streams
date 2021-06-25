use iota_streams_core::{
    err,
    Errors::{
        IdentifierGenerationFailure,
        BadOneof,
    },
    prelude::{
        digest::generic_array::GenericArray,
        Vec,
    },
    psk::{
        self,
        PskId,
        PSKID_SIZE,
    },
    sponge::prp::PRP,
    Result,
};

use iota_streams_core_edsig::signature::ed25519;

use iota_streams_ddml::{
    command::*,
    io,
    types::*,
};

use crate::{
    message::*,
};

#[derive(Clone, Copy, Hash, PartialEq, Eq)]
pub enum Identifier {
    EdPubKey(ed25519::PublicKeyWrap),
    PskId(PskId),
}

impl Identifier {
    pub fn to_bytes(&self) -> Vec<u8> {
        match self {
            Identifier::EdPubKey(id) => id.0.as_bytes().to_vec(),
            Identifier::PskId(id) => id.to_vec(),
        }
    }

    pub fn from_bytes(bytes: &[u8]) -> iota_streams_core::Result<Self> {
        match bytes.len() {
            ed25519::PUBLIC_KEY_LENGTH => Ok(Identifier::EdPubKey(ed25519::PublicKey::from_bytes(bytes)?.into())),
            PSKID_SIZE => Ok(Identifier::PskId(GenericArray::clone_from_slice(bytes))),
            _ => err(IdentifierGenerationFailure)
        }
    }

    pub fn get_pk(&self) -> Option<&ed25519::PublicKey> {
        if let Identifier::EdPubKey(pk) = self { Some(&pk.0) } else { None }
    }
}

impl From<ed25519::PublicKey> for Identifier {
    fn from(pk: ed25519::PublicKey) -> Self {
        Identifier::EdPubKey(pk.into())
    }
}

impl From<&PskId> for Identifier {
    fn from(pskid: &PskId) -> Self {
        Identifier::PskId((*pskid).into())
    }
}

impl<F: PRP> ContentSizeof<F> for Identifier {
    fn sizeof<'c>(&self, ctx: &'c mut sizeof::Context<F>) -> Result<&'c mut sizeof::Context<F>> {
        match self {
            &Identifier::EdPubKey(pk) => {
                let oneof = Uint8(0);
                ctx.absorb(&oneof)?
                    .absorb(&pk.0)?;
                Ok(ctx)
            },
            &Identifier::PskId(pskid) => {
                let oneof = Uint8(1);
                ctx.absorb(&oneof)?
                    .mask(<&NBytes<psk::PskIdSize>>::from(&pskid))?;
                Ok(ctx)
            },
        }
    }
}

impl<F: PRP, Store> ContentWrap<F, Store> for Identifier
{
    fn wrap<'c, OS: io::OStream>(
        &self,
        _store: &Store,
        ctx: &'c mut wrap::Context<F, OS>,
    ) -> Result<&'c mut wrap::Context<F, OS>> {
        match self {
            &Identifier::EdPubKey(pk) => {
                let oneof = Uint8(0);
                ctx.absorb(&oneof)?
                    .absorb(&pk.0)?;
                Ok(ctx)
            },
            &Identifier::PskId(pskid) => {
                let oneof = Uint8(1);
                ctx.absorb(&oneof)?
                    .mask(<&NBytes<psk::PskIdSize>>::from(&pskid))?;
                Ok(ctx)
            }
        }
    }
}

impl<F: PRP, Store> ContentUnwrap<F, Store> for Identifier
{
    fn unwrap<'c, IS: io::IStream>(
        &mut self,
        _store: &Store,
        ctx: &'c mut unwrap::Context<F, IS>,
    ) -> Result<&'c mut unwrap::Context<F, IS>> {
        let mut oneof = Uint8(0);
        ctx.absorb(&mut oneof)?;
        match oneof.0 {
            0 => {
                let mut pk = ed25519::PublicKey::default();
                ctx.absorb(&mut pk)?;
                *self = Identifier::EdPubKey(ed25519::PublicKeyWrap(pk));
                Ok(ctx)
            },
            1 => {
                let mut pskid = PskId::default();
                ctx.mask(<&mut NBytes<psk::PskIdSize>>::from(&mut pskid))?;
                *self = Identifier::PskId(pskid);
                Ok(ctx)
            },
            _ => {
                err(BadOneof)
            },
        }
    }
}

pub fn unwrap_new<'c, F: PRP, Store, IS: io::IStream>(
    _store: &Store,
    ctx: &'c mut unwrap::Context<F, IS>
) -> Result<(Identifier, &'c mut unwrap::Context<F, IS>)> {
    let mut oneof = Uint8(0);
    ctx.absorb(&mut oneof)?;
    match oneof.0 {
        0 => {
            let mut pk = ed25519::PublicKey::default();
            ctx.absorb(&mut pk)?;
            let identifier = Identifier::EdPubKey(ed25519::PublicKeyWrap(pk));
            Ok((identifier, ctx))
        },
        1 => {
            let mut pskid = PskId::default();
            ctx.mask(<&mut NBytes<psk::PskIdSize>>::from(&mut pskid))?;
            let identifier = Identifier::PskId(pskid);
            Ok((identifier, ctx))
        },
        _ => {
            err(BadOneof)
        },
    }
}

