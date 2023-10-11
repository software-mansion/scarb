use std::ops::Deref;
use std::{fmt, path};

use anyhow::Result;
use camino::{Utf8Path, Utf8PathBuf};
use once_cell::sync::OnceCell;
use tracing::trace;

use crate::internal::fsx;

pub struct LazyDirectoryCreator<'p> {
    path: Utf8PathBuf,
    creation_lock: OnceCell<()>,
    parent: Option<Calf<'p, LazyDirectoryCreator<'p>>>,
    is_output_dir: bool,
}

impl<'p> LazyDirectoryCreator<'p> {
    pub fn new(path: impl Into<Utf8PathBuf>) -> Self {
        Self {
            path: path.into(),
            creation_lock: OnceCell::new(),
            parent: None,
            is_output_dir: false,
        }
    }

    pub fn new_output_dir(path: impl Into<Utf8PathBuf>) -> Self {
        Self {
            is_output_dir: true,
            ..Self::new(path)
        }
    }

    pub fn child(&'p self, path: impl AsRef<Utf8Path>) -> Self {
        Self {
            path: self.path.join(path),
            creation_lock: OnceCell::new(),
            parent: Some(Calf::Borrowed(self)),
            is_output_dir: false,
        }
    }

    pub fn into_child(self, path: impl AsRef<Utf8Path>) -> Self {
        Self {
            path: self.path.join(path),
            creation_lock: OnceCell::new(),
            parent: Some(Calf::Owned(Box::new(self))),
            is_output_dir: false,
        }
    }

    pub fn as_unchecked(&self) -> &Utf8Path {
        &self.path
    }

    pub fn as_existent(&self) -> Result<&Utf8Path> {
        self.ensure_created()?;
        Ok(&self.path)
    }

    pub const fn is_output_dir(&self) -> bool {
        self.is_output_dir
    }

    fn ensure_created(&self) -> Result<()> {
        if let Some(parent) = &self.parent {
            parent.ensure_created()?;
        }

        self.creation_lock
            .get_or_try_init(|| {
                trace!(
                    "creating directory {}; output_dir={}",
                    &self.path,
                    self.is_output_dir
                );

                if self.is_output_dir {
                    create_output_dir::create_output_dir(self.path.as_std_path())
                } else {
                    fsx::create_dir_all(&self.path)
                }
            })
            .copied()
    }
}

impl<'p> fmt::Display for LazyDirectoryCreator<'p> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_unchecked())
    }
}

impl<'p> fmt::Debug for LazyDirectoryCreator<'p> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let base = self
            .parent
            .as_deref()
            .map(|p| p.path.as_path())
            .unwrap_or_else(|| Utf8Path::new(""));
        if let Ok(stem) = self.path.strip_prefix(base) {
            write!(f, "<{base}>{}{stem}", path::MAIN_SEPARATOR)?;
        } else {
            write!(f, "{}", self.path)?;
            if let Some(parent) = &self.parent {
                write!(f, r#", parent: "{}""#, parent.path)?;
            }
        }

        if self.creation_lock.get().is_some() {
            write!(f, ", created")?;
        }

        if self.is_output_dir {
            write!(f, ", output")?;
        }

        Ok(())
    }
}

enum Calf<'a, T> {
    Borrowed(&'a T),
    Owned(Box<T>),
}

impl<'a, T: 'a> Deref for Calf<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        match self {
            Calf::Borrowed(t) => t,
            Calf::Owned(t) => t,
        }
    }
}
