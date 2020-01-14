use std::{
    fmt::{self, Display, Formatter},
    ops::Deref,
    path::PathBuf,
};

use super::traits::FileName;
use crate::prelude::*;

pub type RemoteDestination = HwOptQual<String>;
pub type RemotePattern = HwQual<String>;

pub type HwQual<T> = HwQualBase<usize, T>;
pub type HwOptQual<T> = HwQualBase<Option<usize>, T>;

#[derive(Clone, Debug)]
pub struct HwQualBase<H, N> {
    pub hw: H,
    pub name: N,
}

pub enum CpArg {
    Local(PathBuf),
    Remote(RemotePattern),
}

impl<T: FileName> RemotePath for HwOptQual<T> {
    type Name = T;
    type Qual = HwQual<T>;
    type Unqual = HwOptQual<T>;

    fn hw_name(hw: usize, name: impl Into<Self::Name>) -> Self {
        Self {
            hw: Some(hw),
            name: name.into(),
        }
    }

    fn opt_hw(&self) -> Option<usize> {
        self.hw
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn with_name(&self, name: impl Into<Self::Name>) -> Self {
        Self {
            hw: self.hw,
            name: name.into(),
        }
    }

    fn into_name(self) -> Self::Name {
        self.name
    }
}

fn non_empty<S: Deref<Target = str>>(s: S) -> Option<S> {
    if s.is_empty() {
        None
    } else {
        Some(s)
    }
}

impl<T: FileName> Unqualified for HwOptQual<T> {
    fn opt_hw_name(hw: Option<usize>, name: impl Into<Self::Name>) -> Self {
        Self {
            hw,
            name: name.into(),
        }
    }

    fn unwrap_or(self, default: Self::Qual) -> Self::Qual {
        match (self.hw, non_empty(self.name)) {
            (Some(hw), Some(name)) => Self::Qual::hw_name(hw, name),
            (Some(hw), None) => Self::Qual::hw_name(hw, default.name),
            (None, Some(name)) => Self::Qual::hw_name(default.hw, name),
            (None, None) => panic!("HwOptQual::unwrap_or: got nothing"),
        }
    }
}

impl<T: FileName> RemotePath for HwQual<T> {
    type Name = T;
    type Qual = HwQual<T>;
    type Unqual = HwOptQual<T>;

    fn hw_name(hw: usize, name: impl Into<Self::Name>) -> Self {
        Self {
            hw,
            name: name.into(),
        }
    }

    fn opt_hw(&self) -> Option<usize> {
        Some(self.hw)
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn with_name(&self, name: impl Into<Self::Name>) -> Self {
        Self {
            hw: self.hw,
            name: name.into(),
        }
    }

    fn into_name(self) -> Self::Name {
        self.name
    }
}

impl<T: FileName> Qualified for HwQual<T> {
    fn hw(&self) -> usize {
        self.hw
    }
}

impl CpArg {
    pub fn is_whole_hw(&self) -> bool {
        match self {
            CpArg::Local(_) => false,
            CpArg::Remote(rpat) => rpat.is_whole_hw(),
        }
    }
}

impl<T: Deref<Target = str>> HwQual<T> {
    pub fn is_whole_hw(&self) -> bool {
        self.name.is_empty()
    }
}

impl<T: Display> Display for HwQual<T> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "hw{}:{}", self.hw, self.name)
    }
}

impl<T: Display> Display for HwOptQual<T> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        if let Some(hw) = self.hw {
            write!(f, "hw{}:{}", hw, self.name)
        } else {
            write!(f, ":{}", self.name)
        }
    }
}

impl<T> From<HwQualBase<T, &str>> for HwQualBase<T, String> {
    fn from(path: HwQualBase<T, &str>) -> Self {
        Self {
            hw: path.hw,
            name: path.name.to_owned(),
        }
    }
}

impl<T, U: From<T>> From<HwQual<T>> for HwOptQual<U> {
    fn from(rp: HwQual<T>) -> Self {
        HwOptQual {
            hw: Some(rp.hw),
            name: rp.name.into(),
        }
    }
}
