//! # Type safety for the weary netlink user
//! 
//! ## Rationale
//! 
//! The `libc` crate currently provides an interface for sockets but
//! the constants to configure the socket to do anything useful in netlink
//! are not included in the crate because they live in `/usr/include/linux/netlink.h` and friends.
//! As a result, doing anything with netlink in Rust is currently a bit of a headache.
//! 
//! This crate aims to define the necessary constants and wrap them in types to both take
//! advantage of the Rust type system and also avoid the need to pop open `.h` files
//! to find the information necessary to construct netlink messages.
//! 
//! ## Notes
//! 
//! This crate is currently under heavy development.
//! 
//! The `cc` crate is a build dependency to provide as much of a natively cross distribution
//! approach as possible regarding `#define`s in C. It is used to compile a C file that includes
//! the appropriate headers and exports them to the corresponding `stdint.h` types in C.

extern crate libc;
extern crate byteorder;

/// C constants defined as types
pub mod ffi;
/// Wrapper for `libc` sockets
pub mod socket;
/// Top-level netlink header
pub mod nlhdr;
/// Genetlink (generic netlink) header and attribute helpers
pub mod genlhdr;
/// Error module
pub mod err;

use std::io::{Cursor,Read,Write};
use std::mem;

use byteorder::{NativeEndian,ReadBytesExt,WriteBytesExt};

use ffi::alignto;
use err::{SerError,DeError};

pub struct NlSerState(Cursor<Vec<u8>>);

impl NlSerState {
    pub fn new() -> Self {
        NlSerState(Cursor::new(Vec::new()))
    }

    pub fn into_inner(self) -> Vec<u8> {
        self.0.into_inner()
    }
}

pub struct NlDeState<'a>(Cursor<&'a [u8]>);

impl<'a> NlDeState<'a> {
    pub fn new(s: &'a [u8]) -> Self {
        NlDeState(Cursor::new(s))
    }
}

pub trait Nl: Sized + Default {
    type Input: Default;

    fn serialize(&mut self, &mut NlSerState) -> Result<(), SerError>;
    fn deserialize_with(&mut NlDeState, Self::Input) -> Result<Self, DeError>;
    fn deserialize(state: &mut NlDeState) -> Result<Self, DeError> {
        Self::deserialize_with(state, Self::Input::default())
    }
    fn size(&self) -> usize;
    fn asize(&self) -> usize {
        alignto(self.size())
    }
}

impl Nl for u8 {
    type Input = ();

    fn serialize(&mut self, state: &mut NlSerState) -> Result<(), SerError> {
        try!(state.0.write_u8(*self));
        Ok(())
    }

    fn deserialize_with(state: &mut NlDeState, _input: Self::Input)
                        -> Result<Self, DeError> {
        Ok(try!(state.0.read_u8()))
    }

    fn size(&self) -> usize {
        mem::size_of::<u8>()
    }
}

impl Nl for u16 {
    type Input = ();

    fn serialize(&mut self, state: &mut NlSerState) -> Result<(), SerError> {
        try!(state.0.write_u16::<NativeEndian>(*self));
        Ok(())
    }

    fn deserialize_with(state: &mut NlDeState, _input: Self::Input)
                        -> Result<Self, DeError> {
        Ok(try!(state.0.read_u16::<NativeEndian>()))
    }

    fn size(&self) -> usize {
        mem::size_of::<u16>()
    }
}

impl Nl for u32 {
    type Input = ();

    fn serialize(&mut self, state: &mut NlSerState) -> Result<(), SerError> {
        try!(state.0.write_u32::<NativeEndian>(*self));
        Ok(())
    }

    fn deserialize_with(state: &mut NlDeState, _input: Self::Input)
                        -> Result<Self, DeError> {
        Ok(try!(state.0.read_u32::<NativeEndian>()))
    }

    fn size(&self) -> usize {
        mem::size_of::<u32>()
    }
}

impl Nl for Vec<u8> {
    type Input = usize;

    fn serialize(&mut self, state: &mut NlSerState) -> Result<(), SerError> {
        try!(state.0.write(self.as_slice()));
        Ok(())
    }

    fn deserialize_with(state: &mut NlDeState, input: Self::Input)
                        -> Result<Self, DeError> {
        let mut v = Vec::with_capacity(input);
        try!(state.0.by_ref().take(input as u64).read_to_end(&mut v));
        Ok(v)
    }

    fn size(&self) -> usize {
        self.len()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_nl_u8() {
        let mut v: u8 = 5;
        let s: &[u8; 1] = &[5];
        let mut state = NlSerState::new();
        <u8 as Nl>::serialize(&mut v, &mut state).unwrap();
        assert_eq!(s, state.into_inner().as_slice());

        let s: &[u8; 1] = &[5];
        let mut state = NlDeState::new(s);
        let v = <u8 as Nl>::deserialize(&mut state).unwrap();
        assert_eq!(v, 5)
    }

    #[test]
    fn test_nl_u16() {
        let mut v: u16 = 6000;
        let s: &mut [u8] = &mut [0; 2];
        {
            let mut c = Cursor::new(&mut *s);
            c.write_u16::<NativeEndian>(6000).unwrap();
        }
        let mut state = NlSerState::new();
        <u16 as Nl>::serialize(&mut v, &mut state).unwrap();
        assert_eq!(s, state.into_inner().as_slice());

        let s: &mut [u8] = &mut [0; 2];
        {
            let mut c = Cursor::new(&mut *s);
            c.write_u16::<NativeEndian>(6000).unwrap();
        }
        let mut state = NlDeState::new(&*s);
        let v = <u16 as Nl>::deserialize(&mut state).unwrap();
        assert_eq!(v, 6000)
    }

    #[test]
    fn test_nl_u32() {
        let mut v: u32 = 600000;
        let s: &mut [u8] = &mut [0; 4];
        {
            let mut c = Cursor::new(&mut *s);
            c.write_u32::<NativeEndian>(600000).unwrap();
        }
        let mut state = NlSerState::new();
        <u32 as Nl>::serialize(&mut v, &mut state).unwrap();
        assert_eq!(s, state.into_inner().as_slice());

        let s: &mut [u8] = &mut [0; 4];
        {
            let mut c = Cursor::new(&mut *s);
            c.write_u32::<NativeEndian>(600000).unwrap();
        }
        let mut state = NlDeState::new(&*s);
        let v = <u32 as Nl>::deserialize(&mut state).unwrap();
        assert_eq!(v, 600000)
    }

    #[test]
    fn test_nl_vec() {
        let mut v = vec![1, 2, 3, 4, 5, 6, 7, 8, 9];
        let mut state = NlSerState::new();
        <Vec<u8> as Nl>::serialize(&mut v, &mut state).unwrap();
        assert_eq!(vec![1, 2, 3, 4, 5, 6, 7, 8, 9], state.into_inner().as_slice());

        let s = &[1, 2, 3, 4, 5, 6, 7, 8, 9];
        let mut state = NlDeState::new(s);
        let v = <Vec<u8> as Nl>::deserialize_with(&mut state, s.len()).unwrap();
        assert_eq!(v, vec![1, 2, 3, 4, 5, 6, 7, 8, 9])
    }
}
