//! Helpers for aliasing mutable [Index] storage in the stream layer.
//!
//! [crate::Stream] must be able to update and query an index while reading bytes.
//! Depending on the caller's ownership model, the index can be either:
//!
//! - `Direct(&mut Index)`: exclusive borrow, no runtime borrow checks.
//! - `Shared(Rc<RefCell<Index>>)`: shared aliasing inside a single thread.
//!
//! The shared variant is intended for the case where multiple owners need to
//! query the same index, while the direct variant is optimized for a single
//! exclusive stream owner.
use crate::Index;
use std::{
    cell::{Ref, RefCell, RefMut},
    ops,
    rc::Rc,
};

#[derive(Debug)]
/// A mutable index reference used by [crate::Stream].
///
/// This enum abstracts over two index ownership patterns:
///
/// - `Direct(&mut Index)`: the stream owns an exclusive mutable borrow of the index.
/// - `Shared(Rc<RefCell<Index>>)`: the stream borrows the index through runtime-checked
///   interior mutability, allowing aliasing within a single thread.
///
/// Use `Direct` when the stream is the only owner of the index. Use `Shared`
/// when the same index must be accessed from multiple aliasing locations.
pub enum IndexRef<'idx> {
    Direct(&'idx mut Index),
    Shared(Rc<RefCell<Index>>),
}

/// A read-only index guard returned by [`IndexRef::get`].
///
/// This type abstracts over either a plain reference or a [`RefCell`] borrow.
pub enum Guard<'a> {
    Raw(&'a Index),
    RefCell(Ref<'a, Index>),
}

impl<'a> ops::Deref for Guard<'a> {
    type Target = Index;
    fn deref(&self) -> &Self::Target {
        match self {
            Guard::Raw(r) => r,
            Guard::RefCell(r) => r.deref(),
        }
    }
}

/// A mutable index guard returned by [`IndexRef::get_mut`].
///
/// This type abstracts over either a plain mutable reference or a mutable
/// [`RefCell`] borrow.
pub enum MutGuard<'a> {
    Raw(&'a mut Index),
    RefCell(RefMut<'a, Index>),
}

impl<'a> ops::Deref for MutGuard<'a> {
    type Target = Index;
    fn deref(&self) -> &Self::Target {
        match self {
            MutGuard::Raw(r) => r,
            MutGuard::RefCell(r) => r.deref(),
        }
    }
}

impl<'a> ops::DerefMut for MutGuard<'a> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        match self {
            MutGuard::Raw(r) => r,
            MutGuard::RefCell(r) => r.deref_mut(),
        }
    }
}

impl<'idx> IndexRef<'idx> {
    /// Get a read-only view of the underlying index.
    ///
    /// For `Shared`, this performs a [`RefCell::borrow`] and will panic if a
    /// mutable borrow is already active.
    pub fn get(&self) -> Guard<'_> {
        match self {
            IndexRef::Direct(index) => Guard::Raw(&**index),
            IndexRef::Shared(ref_cell) => {
                let r = ref_cell.borrow();
                Guard::RefCell(r)
            }
        }
    }

    /// Get a mutable view of the underlying index.
    ///
    /// For `Shared`, this performs a [`RefCell::borrow_mut`] and will panic if
    /// any other borrow is active.
    pub fn get_mut(&mut self) -> MutGuard<'_> {
        match self {
            IndexRef::Direct(index) => MutGuard::Raw(*index),
            IndexRef::Shared(ref_cell) => {
                let r = ref_cell.borrow_mut();
                MutGuard::RefCell(r)
            }
        }
    }
}
