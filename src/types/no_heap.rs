pub use heapless::Vec;

/// A buffer of bytes that, when used, can avoid unnecessary allocations.
pub type Buffer = Vec<u8>;

/// A buffer to serialize into
pub type SerBuffer<'a> = &'a mut [u8];

/// A buffer to deserialize from
pub type DeBuffer<'a> = &'a [u8];

/// A buffer to hold data read from sockets
pub type SockBuffer = Vec<u8>;

/// A buffer of netlink messages.
pub type NlBuffer<T, P> = Vec<Nlmsghdr<T, P>>;

/// A buffer of generic netlink attributes.
pub type GenlBuffer<T, P> = Vec<Nlattr<T, P>>;

/// A buffer of rtnetlink attributes.
pub type RtBuffer<T, P> = Vec<Rtattr<T, P>>;
