//! This module contains the top level netlink header code and attribute parsing. Every netlink
//! message will be encapsulated in a top level `Nlmsghdr`.
//!
//! `Nlmsghdr` is the structure representing a header that all netlink protocols require to be
//! passed to the correct destination.
//!
//! # Design decisions
//!
//! Payloads for `Nlmsghdr` can be any type that implements the `Nl` trait.

use crate::{
    consts::{
        alignto,
        nl::{NlType, NlTypeWrapper, NlmFFlags, Nlmsg},
    },
    err::{DeError, NlError, Nlmsgerr, SerError},
    parse::packet_length_u32,
    types::{DeBuffer, DeBufferOps, NlBuffer, NlBufferOps, SerBuffer, SerBufferOps},
    Nl,
};

impl<T, P> Nl for NlBuffer<T, P>
where
    T: NlType,
    P: Nl,
{
    fn serialize<'a>(&self, mut mem: SerBuffer<'a>) -> Result<SerBuffer<'a>, SerError<'a>> {
        let mut pos = 0;
        for nlhdr in self.iter() {
            let (mem_tmp, pos_tmp) = drive_serialize!(nlhdr, mem, pos);
            mem = mem_tmp;
            pos = pos_tmp;
        }
        Ok(drive_serialize!(END mem, pos))
    }

    fn deserialize(mem: DeBuffer) -> Result<Self, DeError> {
        let mut nlhdrs = NlBuffer::new();
        let mut pos = 0;
        while pos < mem.len() {
            let packet_len = packet_length_u32(mem.as_ref(), pos);
            let (nlhdr, pos_tmp) = drive_deserialize!(
                Nlmsghdr<T, P>, mem, pos, alignto(packet_len)
            );
            pos = pos_tmp;
            nlhdrs.push(nlhdr);
        }
        drive_deserialize!(END mem, pos);
        Ok(nlhdrs)
    }

    fn type_size() -> Option<usize> {
        None
    }

    fn size(&self) -> usize {
        self.iter().fold(0, |acc, nlhdr| acc + nlhdr.size())
    }
}

/// An enum representing either the desired payload as requested
/// by the payload type parameter or an ACK received at the end
/// of a message or stream of messages.
#[derive(Debug, PartialEq)]
pub enum NlPayload<P> {
    /// Represents an ACK returned by netlink.
    Ack(Nlmsgerr<NlTypeWrapper>),
    /// Represents an application level error returned by netlink.
    Err(Nlmsgerr<NlTypeWrapper>),
    /// Represents the requested payload.
    Payload(P),
}

impl<P> NlPayload<P> {
    /// Get the payload of the netlink packet and return `None`
    /// if the contained data in the payload is actually an ACK.
    pub fn get_payload(&self) -> Option<&P> {
        match self {
            NlPayload::Payload(ref p) => Some(p),
            _ => None,
        }
    }
}

impl<P> Nl for NlPayload<P>
where
    P: Nl,
{
    fn serialize<'a>(&self, mem: SerBuffer<'a>) -> Result<SerBuffer<'a>, SerError<'a>> {
        match *self {
            NlPayload::Ack(ref e) => e.serialize(mem),
            NlPayload::Err(ref e) => e.serialize(mem),
            NlPayload::Payload(ref p) => p.serialize(mem),
        }
    }

    fn deserialize(_: DeBuffer) -> Result<Self, DeError> {
        Err(DeError::new(
            "Cannot deserialize payload type without knowing the \
            netlink packet type.",
        ))
    }

    fn size(&self) -> usize {
        match *self {
            NlPayload::Ack(ref e) => e.size(),
            NlPayload::Err(ref e) => e.size(),
            NlPayload::Payload(ref p) => p.size(),
        }
    }

    fn type_size() -> Option<usize> {
        None
    }
}

/// Top level netlink header and payload
#[derive(Debug, PartialEq)]
pub struct Nlmsghdr<T, P> {
    /// Length of the netlink message
    pub nl_len: u32,
    /// Type of the netlink message
    pub nl_type: T,
    /// Flags indicating properties of the request or response
    pub nl_flags: NlmFFlags,
    /// Sequence number for netlink protocol
    pub nl_seq: u32,
    /// ID of the netlink destination for requests and source for responses
    pub nl_pid: u32,
    /// Payload of netlink message
    pub nl_payload: NlPayload<P>,
}

impl<T, P> Nlmsghdr<T, P>
where
    T: NlType,
    P: Nl,
{
    /// Create a new top level netlink packet with a payload
    pub fn new(
        nl_len: Option<u32>,
        nl_type: T,
        nl_flags: NlmFFlags,
        nl_seq: Option<u32>,
        nl_pid: Option<u32>,
        nl_payload: NlPayload<P>,
    ) -> Self {
        let mut nl = Nlmsghdr {
            nl_type,
            nl_flags,
            nl_seq: nl_seq.unwrap_or(0),
            nl_pid: nl_pid.unwrap_or(0),
            nl_payload,
            nl_len: 0,
        };
        nl.nl_len = nl_len.unwrap_or(nl.size() as u32);
        nl
    }

    /// Get the payload if there is one or return an error.
    pub fn get_payload(&self) -> Result<&P, NlError> {
        match self.nl_payload {
            NlPayload::Payload(ref p) => Ok(p),
            _ => Err(NlError::new("This packet does not have a payload.")),
        }
    }

    /// Get the size of the netlink header.
    #[inline]
    pub fn header_size() -> usize {
        u32::type_size().expect("constant size") * 3
            + NlmFFlags::type_size().expect("constant size")
            + T::type_size().expect("constant size")
    }
}

impl<T, P> Nl for Nlmsghdr<T, P>
where
    T: NlType,
    P: Nl,
{
    fn serialize<'a>(&self, mem: SerBuffer<'a>) -> Result<SerBuffer<'a>, SerError<'a>> {
        Ok(serialize! {
            PAD self;
            mem;
            self.nl_len;
            self.nl_type;
            self.nl_flags;
            self.nl_seq;
            self.nl_pid;
            self.nl_payload
        })
    }

    fn deserialize(mem: DeBuffer) -> Result<Self, DeError> {
        let (nl_len, pos) = drive_deserialize!(u32, mem, 0);
        let (nl_type, pos) = drive_deserialize!(T, mem, pos);
        let (nl_flags, pos) = drive_deserialize!(NlmFFlags, mem, pos);
        let (nl_seq, pos) = drive_deserialize!(u32, mem, pos);
        let (nl_pid, pos) = drive_deserialize!(u32, mem, pos);
        let nl_type_int: u16 = nl_type.into();
        let (nl_payload, pos) = if nl_type_int == Nlmsg::Error.into() {
            let (nl_payload, pos) = drive_deserialize!(Nlmsgerr<NlTypeWrapper>, mem, pos);
            if nl_payload.error == 0 {
                (NlPayload::Ack(nl_payload), pos)
            } else {
                (NlPayload::Err(nl_payload), pos)
            }
        } else {
            let (nl_payload, pos) = drive_deserialize!(
                P,
                mem,
                pos,
                (nl_len as usize)
                    .checked_sub(Self::header_size())
                    .ok_or(DeError::UnexpectedEOB)?
            );
            (NlPayload::Payload(nl_payload), pos)
        };
        let pos = drive_deserialize!(
            STRIP mem,
            pos,
            alignto(nl_len as usize) - nl_len as usize
        );
        drive_deserialize!(END mem, pos);
        Ok(Nlmsghdr {
            nl_len,
            nl_type,
            nl_flags,
            nl_seq,
            nl_pid,
            nl_payload,
        })
    }

    fn size(&self) -> usize {
        Self::header_size() + self.nl_payload.size()
    }

    fn type_size() -> Option<usize> {
        P::type_size().map(|sz| sz + Self::header_size())
    }
}

/// Signifies an empty payload for a netlink packet.
#[derive(Debug, PartialEq)]
pub struct NlEmpty;

impl Nl for NlEmpty {
    #[inline]
    fn serialize<'a>(&self, mut mem: SerBuffer<'a>) -> Result<SerBuffer<'a>, SerError<'a>> {
        for i in 0..mem.len() {
            mem.as_mut()[i] = 0;
        }
        Ok(mem)
    }

    #[inline]
    fn deserialize(mem: DeBuffer) -> Result<Self, DeError> {
        for i in 0..mem.len() {
            if mem.as_ref()[i] != 0 {
                return Err(DeError::new("Expected an empty buffer or a zeroed buffer"));
            }
        }
        Ok(NlEmpty)
    }

    #[inline]
    fn size(&self) -> usize {
        0
    }

    #[inline]
    fn type_size() -> Option<usize> {
        None
    }
}

#[cfg(test)]
mod test {
    use super::*;

    use std::io::Cursor;

    use byteorder::{NativeEndian, WriteBytesExt};

    use crate::consts::nl::{NlmF, Nlmsg};

    #[test]
    fn test_nlmsghdr_serialize() {
        let nl = Nlmsghdr::<Nlmsg, NlEmpty>::new(
            None,
            Nlmsg::Noop,
            NlmFFlags::empty(),
            None,
            None,
            NlPayload::Payload(NlEmpty),
        );
        let mut mem = SerBuffer::new(Some(nl.asize()));
        mem = nl.serialize(mem).unwrap();
        let mut s = [0u8; 16];
        {
            let mut c = Cursor::new(&mut s as &mut [u8]);
            c.write_u32::<NativeEndian>(16).unwrap();
            c.write_u16::<NativeEndian>(1).unwrap();
        };
        assert_eq!(&s, mem.as_ref())
    }

    #[test]
    fn test_nlmsghdr_deserialize() {
        let mut s = [0u8; 16];
        {
            let mut c = Cursor::new(&mut s as &mut [u8]);
            c.write_u32::<NativeEndian>(16).unwrap();
            c.write_u16::<NativeEndian>(1).unwrap();
            c.write_u16::<NativeEndian>(NlmF::Ack.into()).unwrap();
        }
        let nl = Nlmsghdr::<Nlmsg, NlEmpty>::deserialize(DeBuffer::from(&s as &[u8])).unwrap();
        assert_eq!(
            Nlmsghdr::<Nlmsg, NlEmpty>::new(
                None,
                Nlmsg::Noop,
                NlmFFlags::new(&[NlmF::Ack]),
                None,
                None,
                NlPayload::Payload(NlEmpty),
            ),
            nl
        );
    }
}
