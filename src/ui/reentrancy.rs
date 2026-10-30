// WaveTune - Unofficial multi-brand wireless headset control center for Linux
// Copyright (C) 2026 onlymash <fiepi.dev@gmail.com>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, version 3.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: GPL-3.0-only

use std::cell::Cell;
use std::rc::Rc;

/// RAII guard that resets the re-entrancy flag when dropped.
#[derive(Debug)]
pub struct UpdatingGuard {
    flag: Rc<Cell<bool>>,
    was_updating: bool,
}

impl Drop for UpdatingGuard {
    fn drop(&mut self) {
        if !self.was_updating {
            self.flag.set(false);
        }
    }
}

/// A cloneable, thread-local re-entrancy flag for GTK UI callbacks.
///
/// Prevents recursive signal emission loops when widgets are programmatically
/// updated from background events or state synchronization.
#[derive(Clone, Default, Debug)]
pub struct ReentrancyFlag(Rc<Cell<bool>>);

impl ReentrancyFlag {
    pub fn new() -> Self {
        Self(Rc::new(Cell::new(false)))
    }

    /// Returns `true` if a programmatic UI update is currently in progress.
    pub fn is_updating(&self) -> bool {
        self.0.get()
    }

    /// Enters a programmatic update scope and returns an RAII guard.
    /// When the guard is dropped, the flag is restored.
    pub fn enter(&self) -> UpdatingGuard {
        let was_updating = self.0.get();
        self.0.set(true);
        UpdatingGuard {
            flag: Rc::clone(&self.0),
            was_updating,
        }
    }

    /// Executes `f` within the guarded update context.
    #[allow(dead_code)]
    pub fn guarded<R>(&self, f: impl FnOnce() -> R) -> R {
        let _guard = self.enter();
        f()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reentrancy_guard_lifecycle() {
        let flag = ReentrancyFlag::new();
        assert!(!flag.is_updating());

        {
            let _guard = flag.enter();
            assert!(flag.is_updating());
        }

        assert!(!flag.is_updating());
    }

    #[test]
    fn test_nested_guards_restore_only_at_root() {
        let flag = ReentrancyFlag::new();
        {
            let _guard1 = flag.enter();
            assert!(flag.is_updating());
            {
                let _guard2 = flag.enter();
                assert!(flag.is_updating());
            }
            assert!(flag.is_updating());
        }
        assert!(!flag.is_updating());
    }

    #[test]
    fn test_guarded_closure() {
        let flag = ReentrancyFlag::new();
        let val = flag.guarded(|| {
            assert!(flag.is_updating());
            42
        });
        assert_eq!(val, 42);
        assert!(!flag.is_updating());
    }
}
