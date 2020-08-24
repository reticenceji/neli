use crate::{
    err::{DeError, SerError},
    nl::Nlmsghdr,
    nlattr::Nlattr,
    rtnl::Rtattr,
    types::{SockBufferRef, SockBufferRefMut},
};

/// Trait defining operations for byte buffers.
pub trait BufferOps: AsRef<[u8]> + AsMut<[u8]> {
    /// Create a new empty buffer.
    fn new() -> Self;

    /// Initialize the internal buffer from a slice.
    fn from_slice(slice: &[u8]) -> Self;

    /// Extend the internal buffer from the provided slice.
    fn extend_from_slice(&mut self, slice: &[u8]);

    /// Get the current length of the buffer.
    fn len(&self) -> usize;

    /// Get if current buffer is empty.
    fn is_empty(&self) -> bool;
}

/// Trait defining operations for a buffer used for reading data from
/// a socket.
pub trait SockBufferOps {
    /// Create a new buffer with enough allocated memory to store
    /// netlink messages read from a socket.
    fn new() -> Self;

    /// Get the internal buffer as an immutable slice with an
    /// immutable borrow.
    fn get_ref(&self) -> Option<SockBufferRef>;

    /// Get the internal buffer as a mutable slice with an
    /// immutable borrow.
    fn get_mut(&self) -> Option<SockBufferRefMut>;
}

/// Trait defining operations for serialization buffers.
pub trait SerBufferOps<'a>: AsRef<[u8]> + AsMut<[u8]> + Sized {
    /// Create a new `SerBuffer` with optional size hint.
    fn new(size_hint: Option<usize>) -> Self;

    /// Split out a segment of the serialization buffer. Error if the indicies are
    /// out of bounds.
    fn split(
        self,
        start: usize,
        end: usize,
    ) -> Result<(Option<Self>, Self, Option<Self>), SerError<'a>>;

    /// Rejoin the serialization buffer and error if invalid arguments were provided.
    fn join(&mut self, start: Option<Self>, end: Option<Self>) -> Result<(), SerError<'a>>;

    /// Get the current length of the buffer.
    fn len(&self) -> usize;

    /// Get if current buffer is empty.
    fn is_empty(&self) -> bool;
}

/// Trait defining operations for deserialization buffers.
pub trait DeBufferOps<'a>: AsRef<[u8]> + From<&'a [u8]> + Sized {
    /// Get a subslice of the internal deserialization buffer.
    fn slice(&self, start: usize, end: usize) -> Result<Self, DeError>;

    /// Get the current length of the buffer.
    fn len(&self) -> usize;

    /// Get if current buffer is empty.
    fn is_empty(&self) -> bool;
}

/// Trait defining operations for buffers of netlink packets.
pub trait NlBufferOps<T, P>: AsRef<[Nlmsghdr<T, P>]> {
    /// Create an empty buffer.
    fn new() -> Self;

    /// Add a netlink packet to the end of the buffer.
    fn push(&mut self, msg: Nlmsghdr<T, P>);
}

/// Trait defining operations for buffer of generic netlink
/// attributes.
pub trait GenlBufferOps<T, P>: AsRef<[Nlattr<T, P>]> {
    /// Create an empty buffer.
    fn new() -> Self;

    /// Add a netlink attribute to the end of the buffer.
    fn push(&mut self, attr: Nlattr<T, P>);
}

/// Trait defining operations for buffer of routing netlink
/// attributes.
pub trait RtBufferOps<T, P>: AsRef<[Rtattr<T, P>]> {
    /// Create an empty buffer.
    fn new() -> Self;

    /// Add a routing attribute to the end of the buffer.
    fn push(&mut self, attr: Rtattr<T, P>);
}

/// Trait defining operations on a buffer of flags.
pub trait FlagBufferOps<'a, T>: From<&'a [T]>
where
    T: 'a,
{
    /// Iterator over flags.
    type Iter: Iterator;

    /// Create an empty set of flags.
    fn empty() -> Self;

    /// Returns `true` if this set of flags contains the given element.
    fn contains(&self, elem: &T) -> bool;

    /// Set a bit flag.
    fn set(&mut self, flag: T);

    /// Unset a bit flag.
    fn unset(&mut self, flag: &T);

    /// Return an iterator over all flags.
    fn iter(&'a self) -> Self::Iter;
}
