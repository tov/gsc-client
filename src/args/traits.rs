use std::ops::Deref;
use crate::prelude::*;

pub trait FileName: Sized + Default + Deref<Target = str> + Into<String> { }

impl<T> FileName for T
where T: Sized + Default + Deref<Target = str> + Into<String> { }

pub trait RemotePath: Sized {
    type Name: FileName;
    type Qual: Qualified;
    type Unqual: Unqualified;

    fn hw_name(hw: usize, name: impl Into<Self::Name>) -> Self;

    fn opt_hw(&self) -> Option<usize>;
    fn name(&self) -> &str;

    fn with_name(&self, name: impl Into<Self::Name>) -> Self;
    fn into_name(self) -> Self::Name;

    fn just_hw(hw: usize) -> Self {
        Self::hw_name(hw, Self::Name::default())
    }
}

pub trait Unqualified: RemotePath<Unqual = Self> {
    fn opt_hw_name(hw: Option<usize>, name: impl Into<Self::Name>) -> Self;
    fn unwrap_or(self, default: Self::Qual) -> Self::Qual;

    fn just_name(name: impl Into<Self::Name>) -> Self {
        Self::opt_hw_name(None, name)
    }

    fn as_ref(&self) -> HwOptQual<&str> {
        HwOptQual {
            hw: self.opt_hw(),
            name: self.name(),
        }
    }
}

pub trait Qualified: RemotePath<Qual = Self> {
    fn hw(&self) -> usize;

    fn as_ref(&self) -> HwQual<&str> {
        HwQual {
            hw: self.hw(),
            name: self.name(),
        }
    }
}

