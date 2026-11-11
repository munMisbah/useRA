//! Modal state machine: forms with text fields, confirmations, and
//! pending operations.

use ratatui::layout::Rect;
use ratatui::Frame;
use std::path::PathBuf;

/// Formats single path or multi-select count for modal titles.
pub fn format_paths_title(paths: &[PathBuf]) -> String {
    if paths.len() == 1 {
        paths[0].display().to_string()
    } else {
        format!("{} selected items", paths.len())
    }
}

/// What the modal is currently asking the user for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModalKind {
    /// Create-user form.
    CreateUser,
    /// Edit form for the selected user (shell only in phase 3 core).
    EditShell(String),
    /// Lock confirmation.
    ConfirmLock(String),
    /// Unlock confirmation.
    ConfirmUnlock(String),
    /// Delete confirmation; bool = remove home as well.
    ConfirmDelete {
        user: String,
        remove_home: bool,
    },
    /// Add-user-to-groups form.
    AddGroups(String),
    /// Set expiry form.
    SetExpiry(String),
    /// Set password form.
    SetPassword(String),
    /// Create-group form.
    CreateGroup,
    /// Rename-group form (value = current name).
    RenameGroup(String),
    /// Delete-group confirmation.
    ConfirmDeleteGroup(String),
    /// Add users to a group (value = group name).
    GroupAddMembers(String),
    /// Remove users from a group (value = group name).
    GroupRemoveMembers(String),
    /// Jump directory modal.
    GoToPath,
    /// Change mode (`chmod`) checkbox grid modal.
    Chmod(ChmodState),
    /// Change owner/group (`chown`) modal.
    Chown {
        paths: Vec<PathBuf>,
        is_dir: bool,
    },
    /// Set POSIX ACL (`setfacl -m`) interactive scope/autocomplete/checkbox modal.
    SetAcl(SetAclState),
    /// Remove POSIX ACL (`setfacl -x` / `-k`) checklist modal from active entries.
    RemoveAcl(RemoveAclState),
    /// Clear all extended ACLs (`setfacl -b`) confirmation.
    ConfirmClearAcl(Vec<PathBuf>),
    /// Explicit preview + YES confirmation for recursive (`-R`) resource mutations.
    /// Paste/clone copied ACL (`setfacl --set-file=-`) confirmation modal.
    PasteAcl(PasteAclState),
    ConfirmRecursive(RecursiveOp),
}

/// Pending recursive operation awaiting explicit YES confirmation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecursiveOp {
    Chmod {
        paths: Vec<PathBuf>,
        mode: String,
    },
    Chown {
        paths: Vec<PathBuf>,
        owner: String,
    },
    SetAcl {
        paths: Vec<PathBuf>,
        specs: Vec<String>,
    },
    RemoveAcl {
        paths: Vec<PathBuf>,
        specs: Vec<String>,
        clear_default: bool,
    },
}

/// Interactive `r/w/x` permission checkbox state for `Owner / Group / Other`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChmodState {
    pub paths: Vec<PathBuf>,
    pub is_dir: bool,
    /// Full 12-bit POSIX mode (`0o7777`); lower 9 bits toggled via checkboxes.
    pub mode: u32,
    /// Focused row: `0` = Owner, `1` = Group, `2` = Other, `3` = Special, `4` = Recursive (when `is_dir`).
    pub row: usize,
    /// Focused permission column: `0` = Read, `1` = Write, `2` = Execute.
    pub col: usize,
    /// Whether `-R` is checked (only editable when `is_dir`).
    pub recursive: bool,
}

impl ChmodState {
    pub fn new(paths: Vec<PathBuf>, mode: u32, is_dir: bool) -> Self {
        Self {
            paths,
            is_dir,
            mode: mode & 0o7777,
            row: 0,
            col: 0,
            recursive: false,
        }
    }

    fn row_count(&self) -> usize {
        if self.is_dir {
            5
        } else {
            4
        }
    }

    fn bit_mask(row: usize, col: usize) -> u32 {
        if row == 3 {
            return match col {
                0 => 0o4000, // SUID
                1 => 0o2000, // SGID
                2 => 0o1000, // Sticky
                _ => 0,
            };
        }
        let shift = match row {
            0 => 6,
            1 => 3,
            2 => 0,
            _ => return 0,
        };
        let bit = match col {
            0 => 0o4,
            1 => 0o2,
            2 => 0o1,
            _ => return 0,
        };
        bit << shift
    }

    /// True if permission bit at `(row, col)` is currently enabled.
    pub fn has_bit(&self, row: usize, col: usize) -> bool {
        let mask = Self::bit_mask(row, col);
        mask != 0 && (self.mode & mask) != 0
    }

    /// Toggles the currently focused checkbox (permission/special bit or Recursive).
    pub fn toggle(&mut self) {
        if self.row == 4 && self.is_dir {
            self.recursive = !self.recursive;
        } else {
            self.mode ^= Self::bit_mask(self.row, self.col);
        }
    }

    /// Jumps to `col` (`0..=2`) on the active row (`0..=3`) and toggles it.
    pub fn toggle_col(&mut self, col: usize) {
        if self.row < 4 && col < 3 {
            self.col = col;
            self.mode ^= Self::bit_mask(self.row, col);
        }
    }

    /// Moves row focus vertically, wrapping around.
    pub fn move_row(&mut self, delta: isize) {
        let len = self.row_count() as isize;
        self.row = (self.row as isize + delta).rem_euclid(len) as usize;
    }

    /// Moves column focus horizontally, wrapping around.
    pub fn move_col(&mut self, delta: isize) {
        if self.row < 4 {
            self.col = (self.col as isize + delta).rem_euclid(3) as usize;
        }
    }

    /// Formatted 4-digit octal string (preserving any existing SUID/SGID/sticky bits).
    pub fn octal_string(&self) -> String {
        format!("{:04o}", self.mode & 0o7777)
    }

    /// Formatted 10-character `drwxr-x---` permission preview.
    pub fn rwx_string(&self) -> String {
        let kind = if self.is_dir {
            crate::models::EntryKind::Directory
        } else {
            crate::models::EntryKind::File
        };
        crate::models::resource::format_mode_rwx(self.mode, kind)
    }
}

/// ACL target scope selector (`User / Group / Mask / Other`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AclTargetScope {
    User,
    Group,
    Mask,
    Other,
}

impl AclTargetScope {
    pub fn cycle(self, delta: isize) -> Self {
        const ALL: [AclTargetScope; 4] = [
            AclTargetScope::User,
            AclTargetScope::Group,
            AclTargetScope::Mask,
            AclTargetScope::Other,
        ];
        let idx = ALL.iter().position(|&s| s == self).unwrap_or(0) as isize;
        ALL[(idx + delta).rem_euclid(ALL.len() as isize) as usize]
    }

    pub const fn label(self) -> &'static str {
        match self {
            AclTargetScope::User => "User (u:)",
            AclTargetScope::Group => "Group (g:)",
            AclTargetScope::Mask => "Mask (m:)",
            AclTargetScope::Other => "Other (o:)",
        }
    }

    pub const fn needs_name(self) -> bool {
        matches!(self, AclTargetScope::User | AclTargetScope::Group)
    }
}

/// Candidate account/group item for Set ACL autocomplete.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CandidateItem {
    pub name: String,
    pub meta: String,
    pub is_regular: bool,
}

/// Execute permission state for POSIX ACLs (`-` / `x` / `X`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecPerm {
    /// No execute permission (`-`).
    None,
    /// Unconditional execute (`x`).
    Always,
    /// Conditional execute (`X`: directories and already-executable files).
    Conditional,
}

impl ExecPerm {
    pub fn cycle(self) -> Self {
        match self {
            ExecPerm::None => ExecPerm::Always,
            ExecPerm::Always => ExecPerm::Conditional,
            ExecPerm::Conditional => ExecPerm::None,
        }
    }

    pub const fn ch(self) -> char {
        match self {
            ExecPerm::None => '-',
            ExecPerm::Always => 'x',
            ExecPerm::Conditional => 'X',
        }
    }
}

/// Interactive `Scope + Autocomplete Name + r/w/x/X + Access/Default + Recursive (-R)` modal for `setfacl -m`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SetAclState {
    pub paths: Vec<PathBuf>,
    pub is_dir: bool,
    pub scope: AclTargetScope,
    pub name: String,
    pub cursor_col: usize,
    pub users: Vec<CandidateItem>,
    pub groups: Vec<CandidateItem>,
    pub existing_acl: crate::models::AclInfo,
    /// True when permissions were auto-populated from an existing ACL entry.
    pub loaded_existing: bool,
    /// Highlighted index inside `filtered_suggestions()`.
    pub sug_idx: usize,
    pub read: bool,
    pub write: bool,
    pub exec: ExecPerm,
    /// Apply non-default access ACL (`u:name:perms`).
    pub apply_access: bool,
    /// Apply default ACL (`d:u:name:perms`, directories only).
    pub apply_default: bool,
    /// Whether to apply recursively `-R` (directories only).
    pub recursive: bool,
    /// Focused row:
    /// `0` = Scope, `1` = Name, `2` = Perms, `3` = Access ACL, `4` = Default (`d:`), `5` = Recursive (`-R`).
    pub row: usize,
    /// Focused permission column (`0=r, 1=w, 2=x/X`) when `row == 2`.
    pub col: usize,
}

impl SetAclState {
    pub fn new(
        paths: Vec<PathBuf>,
        scope: AclTargetScope,
        prefill_name: &str,
        is_dir: bool,
        users: Vec<CandidateItem>,
        groups: Vec<CandidateItem>,
        existing_acl: crate::models::AclInfo,
    ) -> Self {
        let name = prefill_name.to_string();
        let cursor_col = name.len();
        let start_row = if !name.is_empty() || !scope.needs_name() {
            2
        } else {
            1
        };
        let mut st = Self {
            paths,
            is_dir,
            scope,
            name,
            cursor_col,
            users,
            groups,
            existing_acl,
            loaded_existing: false,
            sug_idx: 0,
            read: true,
            write: true,
            exec: if is_dir {
                ExecPerm::Conditional
            } else {
                ExecPerm::None
            },
            apply_access: true,
            apply_default: false,
            recursive: false,
            row: start_row,
            col: 0,
        };
        st.sync_existing_acl(false);
        st
    }

    fn row_count(&self) -> usize {
        if self.is_dir {
            6
        } else {
            3
        }
    }

    fn reset_default_perms(&mut self) {
        self.read = true;
        self.write = true;
        self.exec = if self.is_dir {
            ExecPerm::Conditional
        } else {
            ExecPerm::None
        };
        self.apply_access = true;
        self.apply_default = false;
        self.loaded_existing = false;
    }

    /// Populates `r/w/x` and `Access`/`Default` flags if the selected target already has an ACL.
    fn sync_existing_acl(&mut self, reset_when_missing: bool) {
        use crate::models::AclScope;
        let trimmed = self.name.trim();
        if self.scope.needs_name() && trimmed.is_empty() {
            if reset_when_missing {
                self.reset_default_perms();
            }
            return;
        }
        let target_scope = match self.scope {
            AclTargetScope::User => AclScope::User,
            AclTargetScope::Group => AclScope::Group,
            AclTargetScope::Mask => AclScope::Mask,
            AclTargetScope::Other => AclScope::Other,
        };
        let matches_entry = |e: &&crate::models::AclEntry| -> bool {
            if e.scope != target_scope {
                return false;
            }
            if self.scope.needs_name() {
                e.qualifier == trimmed
            } else {
                e.qualifier.is_empty()
            }
        };
        let access = self
            .existing_acl
            .entries
            .iter()
            .find(|e| !e.is_default && matches_entry(e));
        let default_e = self
            .existing_acl
            .entries
            .iter()
            .find(|e| e.is_default && matches_entry(e));

        if let Some(primary) = access.or(default_e) {
            self.read = primary.perms.contains('r');
            self.write = primary.perms.contains('w');
            self.exec = if primary.perms.contains('X') {
                ExecPerm::Conditional
            } else if primary.perms.contains('x') {
                ExecPerm::Always
            } else {
                ExecPerm::None
            };
            if self.is_dir {
                self.apply_access = access.is_some();
                self.apply_default = default_e.is_some();
            } else {
                self.apply_access = true;
                self.apply_default = false;
            }
            self.loaded_existing = true;
        } else if reset_when_missing {
            self.reset_default_perms();
        } else {
            self.loaded_existing = false;
        }
    }

    pub fn cycle_scope(&mut self, delta: isize) {
        self.scope = self.scope.cycle(delta);
        self.sug_idx = 0;
        if !self.scope.needs_name() {
            self.name.clear();
            self.cursor_col = 0;
        }
        self.sync_existing_acl(true);
    }

    pub fn move_row(&mut self, delta: isize) {
        let len = self.row_count() as isize;
        let mut next = (self.row as isize + delta).rem_euclid(len) as usize;
        if next == 1 && !self.scope.needs_name() {
            next = (next as isize + delta.signum()).rem_euclid(len) as usize;
        }
        self.row = next;
    }

    pub fn move_col(&mut self, delta: isize) {
        if self.row == 0 {
            self.cycle_scope(delta);
        } else if self.row == 1 {
            let cols: Vec<usize> = (0..=self.name.len())
                .filter(|i| self.name.is_char_boundary(*i))
                .collect();
            let pos = cols.iter().position(|&c| c == self.cursor_col).unwrap_or(0);
            let next = (pos as isize + delta).clamp(0, cols.len() as isize - 1) as usize;
            self.cursor_col = cols[next];
        } else if self.row == 2 {
            self.col = (self.col as isize + delta).rem_euclid(3) as usize;
        }
    }

    pub fn push_char(&mut self, c: char) {
        if self.row == 1 && self.scope.needs_name() {
            let col = self.cursor_col.min(self.name.len());
            self.name.insert(col, c);
            self.cursor_col = col + c.len_utf8();
            self.sug_idx = 0;
            self.sync_existing_acl(false);
        }
    }

    pub fn backspace(&mut self) {
        if self.row == 1 && self.scope.needs_name() {
            let col = self.cursor_col.min(self.name.len());
            if col > 0 {
                let prev = self.name[..col]
                    .chars()
                    .last()
                    .map(|c| c.len_utf8())
                    .unwrap_or(1);
                self.name.remove(col - prev);
                self.cursor_col = col - prev;
                self.sug_idx = 0;
                self.sync_existing_acl(false);
            }
        }
    }

    pub fn delete(&mut self) {
        if self.row == 1 && self.scope.needs_name() {
            let col = self.cursor_col.min(self.name.len());
            if col < self.name.len() {
                self.name.remove(col);
                self.sug_idx = 0;
                self.sync_existing_acl(false);
            }
            self.cursor_col = col;
        }
    }

    /// Suggestions appear only when >= 1 char is typed and not yet a unique exact match.
    pub fn filtered_suggestions(&self) -> Vec<&CandidateItem> {
        if self.row != 1 || !self.scope.needs_name() {
            return Vec::new();
        }
        let q = self.name.trim().to_lowercase();
        if q.is_empty() {
            return Vec::new();
        }
        let pool = match self.scope {
            AclTargetScope::User => &self.users,
            AclTargetScope::Group => &self.groups,
            _ => return Vec::new(),
        };
        let mut matches: Vec<&CandidateItem> = pool
            .iter()
            .filter(|c| c.name.to_lowercase().contains(&q))
            .collect();
        matches.sort_by(|a, b| {
            let a_pref = a.name.to_lowercase().starts_with(&q);
            let b_pref = b.name.to_lowercase().starts_with(&q);
            b_pref
                .cmp(&a_pref)
                .then_with(|| b.is_regular.cmp(&a.is_regular))
                .then_with(|| a.name.cmp(&b.name))
        });
        matches.truncate(5);
        if matches.len() == 1 && matches[0].name == self.name.trim() {
            return Vec::new();
        }
        matches
    }

    pub fn move_suggestion(&mut self, delta: isize) {
        let len = self.filtered_suggestions().len();
        if len > 0 {
            self.sug_idx = (self.sug_idx as isize + delta).rem_euclid(len as isize) as usize;
        }
    }

    pub fn accept_suggestion(&mut self) -> bool {
        let list = self.filtered_suggestions();
        if list.is_empty() {
            return false;
        }
        let idx = self.sug_idx.min(list.len() - 1);
        self.name = list[idx].name.clone();
        self.cursor_col = self.name.len();
        self.sug_idx = 0;
        self.sync_existing_acl(true);
        true
    }

    pub fn toggle(&mut self) {
        match self.row {
            0 => self.cycle_scope(1),
            2 => match self.col {
                0 => self.read = !self.read,
                1 => self.write = !self.write,
                2 => self.exec = self.exec.cycle(),
                _ => {}
            },
            3 if self.is_dir => self.apply_access = !self.apply_access,
            4 if self.is_dir => self.apply_default = !self.apply_default,
            5 if self.is_dir => self.recursive = !self.recursive,
            _ => {}
        }
    }

    pub fn toggle_read(&mut self) {
        if self.row == 2 {
            self.col = 0;
            self.read = !self.read;
        }
    }

    pub fn toggle_write(&mut self) {
        if self.row == 2 {
            self.col = 1;
            self.write = !self.write;
        }
    }

    pub fn set_exec(&mut self, kind: ExecPerm) {
        if self.row == 2 {
            self.col = 2;
            self.exec = if self.exec == kind {
                ExecPerm::None
            } else {
                kind
            };
        }
    }

    /// Builds the `setfacl -m` spec list (access and/or default).
    pub fn build_specs(&self) -> Vec<String> {
        let target = match self.scope {
            AclTargetScope::User => {
                let n = self.name.trim();
                if n.is_empty() {
                    return Vec::new();
                }
                format!("u:{n}")
            }
            AclTargetScope::Group => {
                let n = self.name.trim();
                if n.is_empty() {
                    return Vec::new();
                }
                format!("g:{n}")
            }
            AclTargetScope::Mask => "m:".to_string(),
            AclTargetScope::Other => "o:".to_string(),
        };
        let perms = format!(
            "{}{}{}",
            if self.read { 'r' } else { '-' },
            if self.write { 'w' } else { '-' },
            self.exec.ch()
        );
        if !self.is_dir {
            return vec![format!("{target}:{perms}")];
        }
        let mut specs = Vec::new();
        if self.apply_access {
            specs.push(format!("{target}:{perms}"));
        }
        if self.apply_default {
            specs.push(format!("d:{target}:{perms}"));
        }
        specs
    }
}

/// What an item in `RemoveAclState` removes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RemoveAclTarget {
    Spec(String),
    ClearDefault,
}

/// One removable ACL entry shown in the `X` checklist modal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoveAclItem {
    pub label: String,
    pub target: RemoveAclTarget,
    pub selected: bool,
}

/// Interactive checklist populated from active `AclInfo` entries on the selected path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoveAclState {
    pub paths: Vec<PathBuf>,
    pub is_dir: bool,
    pub items: Vec<RemoveAclItem>,
    pub cursor: usize,
    pub recursive: bool,
}

impl RemoveAclState {
    pub fn from_acls(
        paths: Vec<PathBuf>,
        is_dir: bool,
        acls: &[crate::models::AclInfo],
        preselect_user: Option<&str>,
        preselect_group: Option<&str>,
    ) -> Self {
        let mut items: Vec<RemoveAclItem> = Vec::new();
        let mut seen_specs = std::collections::HashSet::new();
        let mut any_default = false;
        for acl in acls {
            for entry in &acl.entries {
                if entry.is_default {
                    any_default = true;
                }
                if entry.qualifier.is_empty() {
                    continue;
                }
                let scope_ch = match entry.scope {
                    crate::models::AclScope::User => "u",
                    crate::models::AclScope::Group => "g",
                    _ => continue,
                };
                let def_prefix = if entry.is_default { "d:" } else { "" };
                let spec = format!("{def_prefix}{scope_ch}:{}", entry.qualifier);
                if !seen_specs.insert(spec.clone()) {
                    continue;
                }
                let preselected = match entry.scope {
                    crate::models::AclScope::User => {
                        preselect_user == Some(entry.qualifier.as_str())
                    }
                    crate::models::AclScope::Group => {
                        preselect_group == Some(entry.qualifier.as_str())
                    }
                    _ => false,
                };
                let label = if acls.len() == 1 {
                    entry.display_line()
                } else {
                    let d = if entry.is_default { "default:" } else { "" };
                    format!("{d}{}:{}", entry.scope.short(), entry.qualifier)
                };
                items.push(RemoveAclItem {
                    label,
                    target: RemoveAclTarget::Spec(spec),
                    selected: preselected,
                });
            }
        }
        if any_default {
            items.push(RemoveAclItem {
                label: "All default ACLs (setfacl -k)".to_string(),
                target: RemoveAclTarget::ClearDefault,
                selected: false,
            });
        }
        Self {
            paths,
            is_dir,
            items,
            cursor: 0,
            recursive: false,
        }
    }

    fn row_count(&self) -> usize {
        self.items.len() + if self.is_dir { 1 } else { 0 }
    }

    pub fn move_row(&mut self, delta: isize) {
        let len = self.row_count();
        if len > 0 {
            self.cursor = (self.cursor as isize + delta).rem_euclid(len as isize) as usize;
        }
    }

    pub fn toggle(&mut self) {
        if self.cursor < self.items.len() {
            self.items[self.cursor].selected = !self.items[self.cursor].selected;
        } else if self.is_dir && self.cursor == self.items.len() {
            self.recursive = !self.recursive;
        }
    }

    pub fn selected_specs(&self) -> Vec<String> {
        self.items
            .iter()
            .filter(|i| i.selected)
            .filter_map(|i| match &i.target {
                RemoveAclTarget::Spec(s) => Some(s.clone()),
                RemoveAclTarget::ClearDefault => None,
            })
            .collect()
    }

    pub fn wants_clear_default(&self) -> bool {
        self.items
            .iter()
            .any(|i| i.selected && i.target == RemoveAclTarget::ClearDefault)
    }
}

/// Confirmation modal state for cloning a copied ACL onto target path(s).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PasteAclState {
    pub paths: Vec<PathBuf>,
    pub is_dir: bool,
    pub source_label: String,
    pub acl_text: String,
    pub rule_count: usize,
    pub recursive: bool,
    pub confirm: String,
    pub cursor_col: usize,
    /// Row `0` = Recursive (`-R`) when `is_dir`; confirm input is `1` (or `0` when file).
    pub row: usize,
}

impl PasteAclState {
    pub fn new(
        paths: Vec<PathBuf>,
        is_dir: bool,
        source_label: String,
        acl_text: String,
        rule_count: usize,
    ) -> Self {
        Self {
            paths,
            is_dir,
            source_label,
            acl_text,
            rule_count,
            recursive: false,
            confirm: String::new(),
            cursor_col: 0,
            row: if is_dir { 1 } else { 0 },
        }
    }

    fn confirm_row(&self) -> usize {
        if self.is_dir {
            1
        } else {
            0
        }
    }

    pub fn move_row(&mut self, delta: isize) {
        if self.is_dir {
            self.row = (self.row as isize + delta).rem_euclid(2) as usize;
        }
    }

    pub fn toggle_recursive(&mut self) {
        if self.is_dir && self.row == 0 {
            self.recursive = !self.recursive;
        }
    }

    pub fn move_col(&mut self, delta: isize) {
        if self.row == self.confirm_row() {
            let cols: Vec<usize> = (0..=self.confirm.len())
                .filter(|i| self.confirm.is_char_boundary(*i))
                .collect();
            let pos = cols.iter().position(|&c| c == self.cursor_col).unwrap_or(0);
            let next = (pos as isize + delta).clamp(0, cols.len() as isize - 1) as usize;
            self.cursor_col = cols[next];
        }
    }

    pub fn push_char(&mut self, c: char) {
        if self.row == self.confirm_row() {
            let col = self.cursor_col.min(self.confirm.len());
            self.confirm.insert(col, c);
            self.cursor_col = col + c.len_utf8();
        }
    }

    pub fn backspace(&mut self) {
        if self.row == self.confirm_row() {
            let col = self.cursor_col.min(self.confirm.len());
            if col > 0 {
                let prev = self.confirm[..col]
                    .chars()
                    .last()
                    .map(|c| c.len_utf8())
                    .unwrap_or(1);
                self.confirm.remove(col - prev);
                self.cursor_col = col - prev;
            }
        }
    }

    pub fn delete(&mut self) {
        if self.row == self.confirm_row() {
            let col = self.cursor_col.min(self.confirm.len());
            if col < self.confirm.len() {
                self.confirm.remove(col);
            }
            self.cursor_col = col;
        }
    }

    pub fn is_confirmed(&self) -> bool {
        self.confirm.trim() == "YES"
    }
}

/// One labeled text input with its current buffer.
#[derive(Debug, Clone)]
pub struct Field {
    /// Prompt label.
    pub label: &'static str,
    /// Current input buffer.
    pub value: String,
    /// Whether this field masks input characters.
    pub masked: bool,
}

impl Field {
    pub fn new(label: &'static str) -> Self {
        Field {
            label,
            value: String::new(),
            masked: false,
        }
    }

    pub fn with_value(label: &'static str, value: impl Into<String>) -> Self {
        Field {
            label,
            value: value.into(),
            masked: false,
        }
    }

    pub fn masked(label: &'static str) -> Self {
        Field {
            label,
            value: String::new(),
            masked: true,
        }
    }
}

/// Active modal: kind, its fields, cursor position, and any inline
/// validation error to display.
#[derive(Debug, Default)]
pub struct Modal {
    /// What is being asked.
    pub kind: Option<ModalKind>,
    /// Editable fields, in order.
    pub fields: Vec<Field>,
    /// Index of the focused field.
    pub cursor_field: usize,
    /// Byte offset of the caret inside the focused field.
    pub cursor_col: usize,
    /// Inline validation error shown above the input line.
    pub error: Option<String>,
}

impl Modal {
    /// Opens a create-user form.
    pub fn create_user() -> Self {
        Modal {
            kind: Some(ModalKind::CreateUser),
            fields: vec![
                Field::new("Username"),
                Field::new("Full name (optional)"),
                Field::new("Home dir (default /home/<name>)"),
                Field::new("Shell [/bin/bash]"),
                Field::new("Supplementary groups (comma-sep, optional)"),
                Field::masked("Password (optional)"),
            ],
            ..Modal::default()
        }
    }

    /// Opens the edit-shell form prefilled with the current shell.
    pub fn edit_shell(user: String, current: &str) -> Self {
        Modal {
            kind: Some(ModalKind::EditShell(user)),
            fields: vec![Field::with_value("New shell", current)],
            ..Modal::default()
        }
    }

    /// Opens a yes/no confirmation with a message line.
    pub fn confirm(kind: ModalKind, message: &str) -> Self {
        let mut modal = Modal {
            kind: Some(kind),
            fields: vec![Field::new("Type YES to confirm")],
            ..Modal::default()
        };
        modal.error = Some(message.to_string());
        modal
    }

    /// Opens the add-groups form.
    pub fn add_groups(user: String) -> Self {
        Modal {
            kind: Some(ModalKind::AddGroups(user)),
            fields: vec![Field::new("Groups (comma-sep)")],
            ..Modal::default()
        }
    }

    /// Opens the expiry form (empty clears the expiry).
    pub fn set_expiry(user: String) -> Self {
        Modal {
            kind: Some(ModalKind::SetExpiry(user)),
            fields: vec![Field::new("Expiry YYYY-MM-DD (empty=clear)")],
            ..Modal::default()
        }
    }

    /// Opens the set-password form for a user.
    pub fn set_password(user: String) -> Self {
        Modal {
            kind: Some(ModalKind::SetPassword(user)),
            fields: vec![
                Field::masked("New password"),
                Field::masked("Confirm password"),
            ],
            ..Modal::default()
        }
    }

    /// Opens a create-group form (name + optional GID).
    pub fn create_group() -> Self {
        Modal {
            kind: Some(ModalKind::CreateGroup),
            fields: vec![Field::new("Group name"), Field::new("GID (optional)")],
            ..Modal::default()
        }
    }

    /// Opens a rename-group form prefilled with the current name.
    pub fn rename_group(current: String) -> Self {
        Modal {
            kind: Some(ModalKind::RenameGroup(current.clone())),
            fields: vec![Field::with_value("New name", current)],
            ..Modal::default()
        }
    }

    /// Opens the add-members form (comma-separated usernames).
    pub fn group_add_members(group: String) -> Self {
        Modal {
            kind: Some(ModalKind::GroupAddMembers(group)),
            fields: vec![Field::new("Users to add (comma-sep)")],
            ..Modal::default()
        }
    }

    /// Opens the remove-members form prefilled with current members.
    pub fn group_remove_members(group: String, members: &[String]) -> Self {
        Modal {
            kind: Some(ModalKind::GroupRemoveMembers(group)),
            fields: vec![Field::with_value(
                "Users to remove (comma-sep)",
                members.join(", "),
            )],
            ..Modal::default()
        }
    }

    /// Opens the directory jump modal prefilled with `current`.
    pub fn goto_path(current: &str) -> Self {
        Modal {
            kind: Some(ModalKind::GoToPath),
            fields: vec![Field::with_value("Directory path", current)],
            cursor_col: current.len(),
            ..Modal::default()
        }
    }

    /// Opens the interactive `chmod` checkbox modal initialized from `current_mode`.
    pub fn chmod(paths: Vec<PathBuf>, current_mode: u32, is_dir: bool) -> Self {
        Modal {
            kind: Some(ModalKind::Chmod(ChmodState::new(
                paths,
                current_mode,
                is_dir,
            ))),
            ..Modal::default()
        }
    }

    /// Opens the `chown` modal; directories include an optional recursive toggle.
    pub fn chown(paths: Vec<PathBuf>, current_owner: &str, is_dir: bool) -> Self {
        let mut fields = vec![Field::with_value("Owner[:Group]", current_owner)];
        if is_dir {
            fields.push(Field::new("Recursive? (yes/no)"));
        }
        Modal {
            kind: Some(ModalKind::Chown { paths, is_dir }),
            cursor_col: current_owner.len(),
            fields,
            ..Modal::default()
        }
    }

    /// Opens the interactive `setfacl -m` modal with Scope selector + 1+ char autocomplete.
    pub fn set_acl(
        paths: Vec<PathBuf>,
        scope: AclTargetScope,
        prefill_name: &str,
        is_dir: bool,
        users: Vec<CandidateItem>,
        groups: Vec<CandidateItem>,
        existing_acl: crate::models::AclInfo,
    ) -> Self {
        Modal {
            kind: Some(ModalKind::SetAcl(SetAclState::new(
                paths,
                scope,
                prefill_name,
                is_dir,
                users,
                groups,
                existing_acl,
            ))),
            ..Modal::default()
        }
    }

    /// Opens the `Remove ACL` checklist populated from active entries.
    pub fn remove_acl(state: RemoveAclState) -> Self {
        Modal {
            kind: Some(ModalKind::RemoveAcl(state)),
            ..Modal::default()
        }
    }

    /// Opens the `Paste ACL` confirmation dialog.
    pub fn paste_acl(state: PasteAclState) -> Self {
        Modal {
            kind: Some(ModalKind::PasteAcl(state)),
            ..Modal::default()
        }
    }

    /// Opens the recursive preview + YES confirmation dialog.
    pub fn confirm_recursive(op: RecursiveOp, preview_msg: &str) -> Self {
        Self::confirm(ModalKind::ConfirmRecursive(op), preview_msg)
    }

    /// Whether a modal is open.
    pub fn is_open(&self) -> bool {
        self.kind.is_some()
    }

    /// Closes the modal.
    pub fn close(&mut self) {
        *self = Modal::default();
    }

    /// Current focused field, if any.
    fn active_field_mut(&mut self) -> Option<&mut Field> {
        self.fields.get_mut(self.cursor_field)
    }

    /// Types a character into the focused field at the caret.
    pub fn push_char(&mut self, c: char) {
        let col = self.cursor_col.min(self.field_value_len());
        if let Some(field) = self.fields.get_mut(self.cursor_field) {
            field.value.insert(col, c);
        }
        self.cursor_col = col + c.len_utf8();
    }

    /// Backspace at the caret.
    pub fn backspace(&mut self) {
        let col = self.cursor_col.min(self.field_value_len());
        if let Some(field) = self.fields.get_mut(self.cursor_field) {
            if col > 0 {
                let prev = field.value[..col]
                    .chars()
                    .last()
                    .map(|c| c.len_utf8())
                    .unwrap_or(1);
                field.value.remove(col - prev);
                self.cursor_col = col - prev;
            }
        } else {
            self.cursor_col = col;
        }
    }

    /// Deletes the char under the caret.
    pub fn delete(&mut self) {
        let col = self.cursor_col.min(self.field_value_len());
        if let Some(field) = self.fields.get_mut(self.cursor_field) {
            if col < field.value.len() {
                field.value.remove(col);
            }
        }
        self.cursor_col = col;
    }

    /// Moves the caret left/right within the field.
    pub fn move_cursor(&mut self, delta: isize) {
        if let Some(field) = self.active_field_mut() {
            let cols: Vec<usize> = (0..=field.value.len())
                .filter(|i| field.value.is_char_boundary(*i))
                .collect();
            let pos = cols.iter().position(|&c| c == self.cursor_col).unwrap_or(0);
            let next = (pos as isize + delta).clamp(0, cols.len() as isize - 1) as usize;
            self.cursor_col = cols[next];
        }
    }

    /// Moves to the next (or first) field; wraps around.
    pub fn next_field(&mut self) {
        if self.fields.is_empty() {
            return;
        }
        self.cursor_field = (self.cursor_field + 1) % self.fields.len();
        self.cursor_col = self.field_value_len();
    }

    /// Moves to the previous field.
    pub fn previous_field(&mut self) {
        if self.fields.is_empty() {
            return;
        }
        self.cursor_field = (self.cursor_field + self.fields.len() - 1) % self.fields.len();
        self.cursor_col = self.field_value_len();
    }

    fn field_value_len(&self) -> usize {
        self.fields
            .get(self.cursor_field)
            .map(|f| f.value.len())
            .unwrap_or(0)
    }
}

/// Character input handling for the focused field.
pub fn handle_input(modal: &mut Modal, code: ratatui::crossterm::event::KeyCode) {
    match code {
        ratatui::crossterm::event::KeyCode::Left => modal.move_cursor(-1),
        ratatui::crossterm::event::KeyCode::Right => modal.move_cursor(1),
        ratatui::crossterm::event::KeyCode::Backspace => modal.backspace(),
        ratatui::crossterm::event::KeyCode::Delete => modal.delete(),
        _ => {}
    }
}

/// Renders the active form/confirm modal.
pub fn render(frame: &mut Frame, modal: &Modal, area: Rect) {
    use ratatui::style::Styled;
    use ratatui::text::{Line, Span};
    use ratatui::widgets::{Block, Borders, Clear, Paragraph};

    // Dim overlay.
    frame.render_widget(
        Block::new().style(ratatui::style::Style::new().bg(crate::ui::theme::BG)),
        area,
    );

    let (title, _warn) = match &modal.kind {
        Some(ModalKind::CreateUser) => ("Create User", false),
        Some(ModalKind::EditShell(user)) => (&format!("Edit Shell — {user}")[..], false),
        Some(ModalKind::ConfirmLock(user)) => (&format!("Lock — {user}")[..], true),
        Some(ModalKind::ConfirmUnlock(user)) => (&format!("Unlock — {user}")[..], true),
        Some(ModalKind::ConfirmDelete { user, remove_home }) => (
            &format!(
                "Delete — {user} (home {})",
                if *remove_home { "removed" } else { "kept" }
            )[..],
            true,
        ),
        Some(ModalKind::AddGroups(user)) => (&format!("Add Groups — {user}")[..], false),
        Some(ModalKind::SetExpiry(user)) => (&format!("Set Expiry — {user}")[..], false),
        Some(ModalKind::SetPassword(user)) => (&format!("Set Password — {user}")[..], false),
        Some(ModalKind::CreateGroup) => ("Create Group", false),
        Some(ModalKind::RenameGroup(group)) => (&format!("Rename Group — {group}")[..], false),
        Some(ModalKind::ConfirmDeleteGroup(group)) => {
            (&format!("Delete Group — {group}")[..], true)
        }
        Some(ModalKind::GroupAddMembers(group)) => (&format!("Add Members — {group}")[..], false),
        Some(ModalKind::GroupRemoveMembers(group)) => {
            (&format!("Remove Members — {group}")[..], false)
        }
        Some(ModalKind::GoToPath) => ("Go To Directory", false),
        Some(ModalKind::Chmod(state)) => {
            render_chmod(frame, modal, state, area);
            return;
        }
        Some(ModalKind::Chown { paths, .. }) => (
            &format!("Owner (chown) — {}", format_paths_title(paths))[..],
            false,
        ),
        Some(ModalKind::SetAcl(state)) => {
            render_set_acl(frame, modal, state, area);
            return;
        }
        Some(ModalKind::RemoveAcl(state)) => {
            render_remove_acl(frame, modal, state, area);
            return;
        }
        Some(ModalKind::PasteAcl(state)) => {
            render_paste_acl(frame, modal, state, area);
            return;
        }
        Some(ModalKind::ConfirmClearAcl(paths)) => (
            &format!("Clear ACLs — {}", format_paths_title(paths))[..],
            true,
        ),
        Some(ModalKind::ConfirmRecursive(_)) => ("Confirm Recursive Operation", true),
        None => return,
    };

    // Size: message line (if any) + fields + hints, bordered.
    let msg_rows = if modal.error.is_some() { 2 } else { 0 };
    let height = (modal.fields.len() as u16 + msg_rows + 3).min(area.height.saturating_sub(2));
    let width = 64_u16.min(area.width.saturating_sub(2));

    let mut lines: Vec<Line> = Vec::new();
    if let Some(err) = &modal.error {
        let style = if modal.kind.as_ref().is_some_and(|k| {
            matches!(
                k,
                ModalKind::ConfirmLock(_)
                    | ModalKind::ConfirmUnlock(_)
                    | ModalKind::ConfirmDelete { .. }
                    | ModalKind::ConfirmClearAcl(_)
                    | ModalKind::ConfirmRecursive(_)
            )
        }) {
            crate::ui::theme::warning()
        } else {
            crate::ui::theme::error()
        };
        lines.push(Line::from(format!(" {err}").set_style(style)));
        lines.push(Line::from(""));
    }
    for (i, field) in modal.fields.iter().enumerate() {
        let focused = i == modal.cursor_field;
        let caret = if focused { "▸ " } else { "  " };
        let label = Span::styled(format!("{caret}{}: ", field.label), crate::ui::theme::dim());
        let value_style = if focused {
            crate::ui::theme::base()
        } else {
            crate::ui::theme::dim()
        };
        // Render caret inside the value on the focused field.
        if focused {
            let char_count = field.value.chars().count();
            let display_chars: Vec<char> = if field.masked {
                vec!['•'; char_count]
            } else {
                field.value.chars().collect()
            };
            let col = modal.cursor_col.min(field.value.len());
            let chars_before = field.value[..col].chars().count();
            let before: String = display_chars[..chars_before].iter().collect();
            let after: String = display_chars[chars_before..].iter().collect();
            lines.push(Line::from(vec![
                label,
                Span::raw(before),
                Span::styled("▏", crate::ui::theme::accent()),
                Span::styled(after, value_style),
            ]));
        } else {
            let display_str: String = if field.masked {
                "•".repeat(field.value.chars().count())
            } else {
                field.value.clone()
            };
            lines.push(Line::from(vec![
                label,
                Span::styled(display_str, value_style),
            ]));
        }
    }
    lines.push(
        Line::from("  Enter submit · Tab next field · Esc cancel")
            .set_style(crate::ui::theme::dim()),
    );

    let box_area = super::modal::centered(area, width, height);
    let block = Block::new()
        .border_type(ratatui::widgets::BorderType::Rounded)
        .borders(Borders::ALL)
        .title(format!(" {title} ").set_style(crate::ui::theme::title()))
        .border_style(
            if title.starts_with("Delete")
                || title.starts_with("Lock")
                || title.starts_with("Unlock")
                || title.starts_with("Clear ACLs")
                || title.starts_with("Confirm Recursive")
            {
                crate::ui::theme::error()
            } else {
                crate::ui::theme::border_focused()
            },
        );

    frame.render_widget(Clear, box_area);
    frame.render_widget(Paragraph::new(lines).block(block), box_area);
}

/// Renders the interactive 3x3 Read/Write/Execute checkbox matrix for `chmod`.
fn render_chmod(frame: &mut Frame, modal: &Modal, state: &ChmodState, area: Rect) {
    use ratatui::style::Styled;
    use ratatui::text::{Line, Span};
    use ratatui::widgets::{Block, Borders, Clear, Paragraph};

    let title = format!("Mode (chmod) — {}", format_paths_title(&state.paths));
    let mut lines: Vec<Line> = Vec::new();

    if let Some(err) = &modal.error {
        lines.push(Line::from(
            format!(" {err}").set_style(crate::ui::theme::error()),
        ));
        lines.push(Line::from(""));
    } else {
        lines.push(Line::from(""));
    }

    lines.push(Line::from(
        "             Read (4)     Write (2)     Exec (1)".set_style(crate::ui::theme::dim()),
    ));

    for (row_idx, row_label) in ["Owner", "Group", "Other"].iter().enumerate() {
        let row_focused = state.row == row_idx;
        let prefix = if row_focused { "▸ " } else { "  " };
        let label_style = if row_focused {
            crate::ui::theme::title()
        } else {
            crate::ui::theme::dim()
        };
        let mut spans = vec![Span::styled(
            format!("{prefix}{row_label:<8}   "),
            label_style,
        )];
        for (col_idx, perm_char) in ['r', 'w', 'x'].iter().enumerate() {
            let checked = state.has_bit(row_idx, col_idx);
            let cell_focused = row_focused && state.col == col_idx;
            let box_str = if checked {
                format!("[x] {perm_char}")
            } else {
                format!("[ ] {perm_char}")
            };
            let style = if cell_focused {
                crate::ui::theme::accent()
            } else if checked {
                crate::ui::theme::success()
            } else {
                crate::ui::theme::dim()
            };
            spans.push(Span::styled(box_str, style));
            if col_idx < 2 {
                spans.push(Span::raw("        "));
            }
        }
        lines.push(Line::from(spans));
    }

    let sp_focused = state.row == 3;
    let sp_prefix = if sp_focused { "▸ " } else { "  " };
    let sp_label_style = if sp_focused {
        crate::ui::theme::title()
    } else {
        crate::ui::theme::dim()
    };
    let mut sp_spans = vec![Span::styled(
        format!("{sp_prefix}Special    "),
        sp_label_style,
    )];
    for (col_idx, label_name) in ["SUID", "SGID", "Sticky"].iter().enumerate() {
        let checked = state.has_bit(3, col_idx);
        let cell_focused = sp_focused && state.col == col_idx;
        let box_str = if checked {
            format!("[x] {label_name}")
        } else {
            format!("[ ] {label_name}")
        };
        let style = if cell_focused {
            crate::ui::theme::accent()
        } else if checked {
            crate::ui::theme::warning()
        } else {
            crate::ui::theme::dim()
        };
        sp_spans.push(Span::styled(box_str, style));
        if col_idx < 2 {
            sp_spans.push(Span::raw("     "));
        }
    }
    lines.push(Line::from(sp_spans));

    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled("  Octal: ", crate::ui::theme::dim()),
        Span::styled(state.octal_string(), crate::ui::theme::title()),
        Span::styled(
            format!("  ({})", state.rwx_string()),
            crate::ui::theme::base(),
        ),
    ]));

    if state.is_dir {
        let rec_focused = state.row == 4;
        let prefix = if rec_focused { "▸ " } else { "  " };
        let mark = if state.recursive { "[x]" } else { "[ ]" };
        let style = if rec_focused {
            crate::ui::theme::accent()
        } else if state.recursive {
            crate::ui::theme::warning()
        } else {
            crate::ui::theme::dim()
        };
        lines.push(Line::from(Span::styled(
            format!("{prefix}{mark} Recursive (-R)"),
            style,
        )));
    }

    lines.push(Line::from(""));
    lines.push(
        Line::from("  ↑↓/←→ move · Space or r/w/x/s/g/t toggle · Enter apply · Esc cancel")
            .set_style(crate::ui::theme::dim()),
    );

    let width = 68_u16.min(area.width.saturating_sub(2));
    let height = (lines.len() as u16 + 2).min(area.height.saturating_sub(2));
    let box_area = super::modal::centered(area, width, height);
    let block = Block::new()
        .border_type(ratatui::widgets::BorderType::Rounded)
        .borders(Borders::ALL)
        .title(format!(" {title} ").set_style(crate::ui::theme::title()))
        .border_style(crate::ui::theme::border_focused());

    frame.render_widget(Clear, box_area);
    frame.render_widget(Paragraph::new(lines).block(block), box_area);
}

/// Renders the interactive POSIX ACL (`setfacl -m`) modal with Scope + 1+ char autocomplete + checkboxes.
fn render_set_acl(frame: &mut Frame, modal: &Modal, state: &SetAclState, area: Rect) {
    use ratatui::style::Styled;
    use ratatui::text::{Line, Span};
    use ratatui::widgets::{Block, Borders, Clear, Paragraph};

    let title = format!("Set ACL — {}", format_paths_title(&state.paths));
    let mut lines: Vec<Line> = Vec::new();

    if let Some(err) = &modal.error {
        lines.push(Line::from(
            format!(" {err}").set_style(crate::ui::theme::error()),
        ));
        lines.push(Line::from(""));
    } else {
        lines.push(Line::from(""));
    }

    // Row 0: Scope selector (`◀ User (u:) ▶`)
    let scope_focused = state.row == 0;
    let s_prefix = if scope_focused { "▸ " } else { "  " };
    lines.push(Line::from(vec![
        Span::styled(
            format!("{s_prefix}Scope  : "),
            if scope_focused {
                crate::ui::theme::title()
            } else {
                crate::ui::theme::dim()
            },
        ),
        Span::styled(
            format!("◀ {} ▶", state.scope.label()),
            if scope_focused {
                crate::ui::theme::accent()
            } else {
                crate::ui::theme::base()
            },
        ),
        Span::styled("  (←/→ change)", crate::ui::theme::dim()),
    ]));

    // Row 1: Name input (with 1+ char filtered suggestions below)
    if state.scope.needs_name() {
        let name_focused = state.row == 1;
        let n_prefix = if name_focused { "▸ " } else { "  " };
        let n_label = Span::styled(
            format!("{n_prefix}Name   : "),
            if name_focused {
                crate::ui::theme::title()
            } else {
                crate::ui::theme::dim()
            },
        );
        if name_focused {
            let col = state.cursor_col.min(state.name.len());
            let (before, after) = state.name.split_at(col);
            lines.push(Line::from(vec![
                n_label,
                Span::styled(before.to_string(), crate::ui::theme::base()),
                Span::styled("▏", crate::ui::theme::accent()),
                Span::styled(after.to_string(), crate::ui::theme::base()),
                if state.name.is_empty() {
                    Span::styled("  (type 1+ chars for list)", crate::ui::theme::dim())
                } else {
                    Span::raw("")
                },
            ]));
        } else {
            lines.push(Line::from(vec![
                n_label,
                Span::styled(state.name.clone(), crate::ui::theme::base()),
            ]));
        }

        let suggestions = state.filtered_suggestions();
        if !suggestions.is_empty() {
            let active_idx = state.sug_idx.min(suggestions.len() - 1);
            for (idx, cand) in suggestions.iter().enumerate() {
                let is_sel = idx == active_idx;
                let mark = if is_sel { "  ▸ " } else { "    " };
                lines.push(Line::from(vec![
                    Span::styled(
                        format!("{mark}{}", cand.name),
                        if is_sel {
                            crate::ui::theme::accent()
                        } else {
                            crate::ui::theme::base()
                        },
                    ),
                    Span::styled(format!("  ({})", cand.meta), crate::ui::theme::dim()),
                ]));
            }
        }
    } else {
        lines.push(Line::from(Span::styled(
            "  Name   : (not needed for Mask/Other)",
            crate::ui::theme::dim(),
        )));
    }

    // Row 2: r/w/x/X checkboxes
    let perm_focused = state.row == 2;
    let p_prefix = if perm_focused { "▸ " } else { "  " };
    let mut perm_spans = vec![Span::styled(
        format!("{p_prefix}Perms  : "),
        if perm_focused {
            crate::ui::theme::title()
        } else {
            crate::ui::theme::dim()
        },
    )];
    let cells = [
        (
            if state.read { "[x] r" } else { "[ ] r" },
            state.read,
            false,
        ),
        (
            if state.write { "[x] w" } else { "[ ] w" },
            state.write,
            false,
        ),
        (
            match state.exec {
                ExecPerm::None => "[ ] x",
                ExecPerm::Always => "[x] x",
                ExecPerm::Conditional => "[X] X (dir/+x)",
            },
            state.exec != ExecPerm::None,
            state.exec == ExecPerm::Conditional,
        ),
    ];
    for (col_idx, (label, is_on, is_cond)) in cells.iter().enumerate() {
        let cell_focused = perm_focused && state.col == col_idx;
        let style = if cell_focused {
            crate::ui::theme::accent()
        } else if *is_cond {
            crate::ui::theme::warning()
        } else if *is_on {
            crate::ui::theme::success()
        } else {
            crate::ui::theme::dim()
        };
        perm_spans.push(Span::styled(*label, style));
        if col_idx < 2 {
            perm_spans.push(Span::raw("    "));
        }
    }
    lines.push(Line::from(perm_spans));

    if state.is_dir {
        let acc_focused = state.row == 3;
        let a_prefix = if acc_focused { "▸ " } else { "  " };
        let a_mark = if state.apply_access { "[x]" } else { "[ ]" };
        let a_style = if acc_focused {
            crate::ui::theme::accent()
        } else if state.apply_access {
            crate::ui::theme::success()
        } else {
            crate::ui::theme::dim()
        };
        lines.push(Line::from(Span::styled(
            format!("{a_prefix}{a_mark} Apply Access ACL (this path)"),
            a_style,
        )));

        let def_focused = state.row == 4;
        let prefix = if def_focused { "▸ " } else { "  " };
        let mark = if state.apply_default { "[x]" } else { "[ ]" };
        let style = if def_focused {
            crate::ui::theme::accent()
        } else if state.apply_default {
            crate::ui::theme::warning()
        } else {
            crate::ui::theme::dim()
        };
        lines.push(Line::from(Span::styled(
            format!("{prefix}{mark} Apply Default ACL (d:) — new files inherit"),
            style,
        )));

        let rec_focused = state.row == 5;
        let r_prefix = if rec_focused { "▸ " } else { "  " };
        let r_mark = if state.recursive { "[x]" } else { "[ ]" };
        let r_style = if rec_focused {
            crate::ui::theme::accent()
        } else if state.recursive {
            crate::ui::theme::warning()
        } else {
            crate::ui::theme::dim()
        };
        lines.push(Line::from(Span::styled(
            format!("{r_prefix}{r_mark} Recursive (-R)"),
            r_style,
        )));
    }

    lines.push(Line::from(""));
    let specs = state.build_specs();
    let spec_text = if specs.is_empty() {
        if state.scope.needs_name() && state.name.trim().is_empty() {
            "(select or type name)".to_string()
        } else {
            "(enable Access and/or Default ACL)".to_string()
        }
    } else {
        specs.join(",")
    };
    let loaded_note = if state.loaded_existing {
        "  (loaded existing ACL)"
    } else {
        ""
    };
    lines.push(Line::from(vec![
        Span::styled("  Spec: ", crate::ui::theme::dim()),
        Span::styled(
            spec_text,
            if specs.is_empty() {
                crate::ui::theme::dim()
            } else {
                crate::ui::theme::title()
            },
        ),
        Span::styled(loaded_note, crate::ui::theme::success()),
    ]));
    lines.push(
        Line::from("  ↑↓/Tab move · Enter pick/apply · Space/r/w/x/X toggle · Esc cancel")
            .set_style(crate::ui::theme::dim()),
    );

    let width = 74_u16.min(area.width.saturating_sub(2));
    let height = (lines.len() as u16 + 2).min(area.height.saturating_sub(2));
    let box_area = super::modal::centered(area, width, height);
    let block = Block::new()
        .border_type(ratatui::widgets::BorderType::Rounded)
        .borders(Borders::ALL)
        .title(format!(" {title} ").set_style(crate::ui::theme::title()))
        .border_style(crate::ui::theme::border_focused());

    frame.render_widget(Clear, box_area);
    frame.render_widget(Paragraph::new(lines).block(block), box_area);
}

/// Renders the Paste/Clone ACL confirmation modal (`setfacl --set-file=-`).
fn render_paste_acl(frame: &mut Frame, modal: &Modal, state: &PasteAclState, area: Rect) {
    use ratatui::style::Styled;
    use ratatui::text::{Line, Span};
    use ratatui::widgets::{Block, Borders, Clear, Paragraph};

    let title = format!("Paste ACL — {}", format_paths_title(&state.paths));
    let mut lines: Vec<Line> = Vec::new();

    if let Some(err) = &modal.error {
        lines.push(Line::from(
            format!(" {err}").set_style(crate::ui::theme::error()),
        ));
        lines.push(Line::from(""));
    } else {
        lines.push(Line::from(""));
    }

    lines.push(Line::from(vec![
        Span::styled("  Source : ", crate::ui::theme::dim()),
        Span::styled(
            format!("{} ({} rules)", state.source_label, state.rule_count),
            crate::ui::theme::title(),
        ),
    ]));
    lines.push(Line::from(vec![
        Span::styled("  Target : ", crate::ui::theme::dim()),
        Span::styled(format_paths_title(&state.paths), crate::ui::theme::base()),
    ]));
    lines.push(Line::from(
        "  Replaces all existing extended ACLs on target(s)."
            .set_style(crate::ui::theme::warning()),
    ));
    lines.push(Line::from(""));

    if state.is_dir {
        let rec_focused = state.row == 0;
        let prefix = if rec_focused { "▸ " } else { "  " };
        let mark = if state.recursive { "[x]" } else { "[ ]" };
        let style = if rec_focused {
            crate::ui::theme::accent()
        } else if state.recursive {
            crate::ui::theme::warning()
        } else {
            crate::ui::theme::dim()
        };
        lines.push(Line::from(Span::styled(
            format!("{prefix}{mark} Recursive (-R)"),
            style,
        )));
    }

    let conf_focused = state.row == state.confirm_row();
    let c_prefix = if conf_focused { "▸ " } else { "  " };
    let c_label = Span::styled(
        format!("{c_prefix}Type YES to confirm: "),
        if conf_focused {
            crate::ui::theme::title()
        } else {
            crate::ui::theme::dim()
        },
    );
    if conf_focused {
        let col = state.cursor_col.min(state.confirm.len());
        let (before, after) = state.confirm.split_at(col);
        lines.push(Line::from(vec![
            c_label,
            Span::styled(before.to_string(), crate::ui::theme::base()),
            Span::styled("▏", crate::ui::theme::accent()),
            Span::styled(after.to_string(), crate::ui::theme::base()),
        ]));
    } else {
        lines.push(Line::from(vec![
            c_label,
            Span::styled(state.confirm.clone(), crate::ui::theme::base()),
        ]));
    }

    lines.push(Line::from(""));
    lines.push(
        Line::from("  ↑↓/Tab move · Space toggle -R · Enter paste · Esc cancel")
            .set_style(crate::ui::theme::dim()),
    );

    let width = 70_u16.min(area.width.saturating_sub(2));
    let height = (lines.len() as u16 + 2).min(area.height.saturating_sub(2));
    let box_area = super::modal::centered(area, width, height);
    let block = Block::new()
        .border_type(ratatui::widgets::BorderType::Rounded)
        .borders(Borders::ALL)
        .title(format!(" {title} ").set_style(crate::ui::theme::title()))
        .border_style(crate::ui::theme::warning());

    frame.render_widget(Clear, box_area);
    frame.render_widget(Paragraph::new(lines).block(block), box_area);
}

/// Renders the Remove ACL checklist modal populated from active entries.
fn render_remove_acl(frame: &mut Frame, modal: &Modal, state: &RemoveAclState, area: Rect) {
    use ratatui::style::Styled;
    use ratatui::text::{Line, Span};
    use ratatui::widgets::{Block, Borders, Clear, Paragraph};

    let title = format!("Remove ACL — {}", format_paths_title(&state.paths));
    let mut lines: Vec<Line> = Vec::new();

    if let Some(err) = &modal.error {
        lines.push(Line::from(
            format!(" {err}").set_style(crate::ui::theme::error()),
        ));
        lines.push(Line::from(""));
    } else {
        lines.push(Line::from(""));
    }

    for (idx, item) in state.items.iter().enumerate() {
        let focused = state.cursor == idx;
        let prefix = if focused { "▸ " } else { "  " };
        let mark = if item.selected { "[x]" } else { "[ ]" };
        let style = if focused {
            crate::ui::theme::accent()
        } else if item.selected {
            crate::ui::theme::error()
        } else {
            crate::ui::theme::base()
        };
        lines.push(Line::from(Span::styled(
            format!("{prefix}{mark} {}", item.label),
            style,
        )));
    }

    if state.is_dir {
        let rec_focused = state.cursor == state.items.len();
        let prefix = if rec_focused { "▸ " } else { "  " };
        let mark = if state.recursive { "[x]" } else { "[ ]" };
        let style = if rec_focused {
            crate::ui::theme::accent()
        } else if state.recursive {
            crate::ui::theme::warning()
        } else {
            crate::ui::theme::dim()
        };
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            format!("{prefix}{mark} Recursive (-R)"),
            style,
        )));
    }

    lines.push(Line::from(""));
    lines.push(
        Line::from("  ↑↓ move · Space toggle · Enter remove · Esc cancel")
            .set_style(crate::ui::theme::dim()),
    );

    let width = 66_u16.min(area.width.saturating_sub(2));
    let height = (lines.len() as u16 + 2).min(area.height.saturating_sub(2));
    let box_area = super::modal::centered(area, width, height);
    let block = Block::new()
        .border_type(ratatui::widgets::BorderType::Rounded)
        .borders(Borders::ALL)
        .title(format!(" {title} ").set_style(crate::ui::theme::title()))
        .border_style(crate::ui::theme::border_focused());

    frame.render_widget(Clear, box_area);
    frame.render_widget(Paragraph::new(lines).block(block), box_area);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn form() -> Modal {
        Modal::create_user()
    }

    #[test]
    fn create_user_form_has_six_fields() {
        let m = form();
        assert!(m.is_open());
        assert_eq!(m.kind, Some(ModalKind::CreateUser));
        assert_eq!(m.fields.len(), 6);
        assert!(m.fields[5].masked);
        assert_eq!(m.cursor_field, 0);
    }

    #[test]
    fn set_password_form_has_two_masked_fields() {
        let m = Modal::set_password("alice".into());
        assert!(m.is_open());
        assert_eq!(m.kind, Some(ModalKind::SetPassword("alice".into())));
        assert_eq!(m.fields.len(), 2);
        assert!(m.fields[0].masked);
        assert!(m.fields[1].masked);
    }

    #[test]
    fn typing_and_backspace_edit_at_caret() {
        let mut m = form();
        for c in "deploy".chars() {
            m.push_char(c);
        }
        assert_eq!(m.fields[0].value, "deploy");
        assert_eq!(m.cursor_col, 6);
        m.backspace();
        assert_eq!(m.fields[0].value, "deplo");
        assert_eq!(m.cursor_col, 5);
        m.delete();
        assert_eq!(m.fields[0].value, "deplo");
    }

    #[test]
    fn cursor_moves_within_field() {
        let mut m = form();
        for c in "abc".chars() {
            m.push_char(c);
        }
        m.move_cursor(-2);
        m.push_char('X');
        assert_eq!(m.fields[0].value, "aXbc");
    }

    #[test]
    fn field_navigation_wraps() {
        let mut m = form();
        m.push_char('x');
        m.next_field();
        assert_eq!(m.cursor_field, 1);
        m.previous_field();
        assert_eq!(m.cursor_field, 0);
        // value length clamps caret to end of previous field.
        assert_eq!(m.cursor_col, 1);
    }

    #[test]
    fn close_resets_state() {
        let mut m = form();
        m.close();
        assert!(!m.is_open());
        assert!(m.fields.is_empty());
    }

    #[test]
    fn confirm_modal_requires_yes() {
        let m = Modal::confirm(
            ModalKind::ConfirmDelete {
                user: "x".into(),
                remove_home: false,
            },
            "Delete user x?",
        );
        assert_eq!(m.fields.len(), 1);
        assert_eq!(m.error.as_deref(), Some("Delete user x?"));
    }

    #[test]
    fn edit_shell_prefills_current() {
        let m = Modal::edit_shell("root".into(), "/bin/bash");
        assert_eq!(m.fields[0].value, "/bin/bash");
        assert_eq!(m.kind, Some(ModalKind::EditShell("root".into())));
    }

    #[test]
    fn delete_user_reports_missing_target() {
        // Unprivileged: userdel fails, exercising the failure path.
        let res = crate::services::user_service::delete_user("no-such-user-xyz", false);
        assert!(!res.ok);
    }

    #[test]
    fn chmod_checkbox_grid_toggles_bits_and_preserves_special_bits() {
        // SGID 0o2750 = drwxr-s---
        let mut state = ChmodState::new(vec![PathBuf::from("/srv/app")], 0o2750, true);
        assert!(state.has_bit(0, 0)); // Owner r
        assert!(state.has_bit(0, 1)); // Owner w
        assert!(state.has_bit(0, 2)); // Owner x
        assert!(state.has_bit(1, 0)); // Group r
        assert!(!state.has_bit(1, 1)); // Group w off
        assert!(state.has_bit(1, 2)); // Group x
        assert!(!state.has_bit(2, 0)); // Other r off

        // Move to Group Write and toggle on -> 0o2770
        state.move_row(1);
        state.move_col(1);
        state.toggle();
        assert_eq!(state.octal_string(), "2770");
        assert_eq!(state.rwx_string(), "drwxrws---");

        // Quick toggle Group Exec off via 'x' col (2) -> 0o2760
        state.toggle_col(2);
        assert_eq!(state.octal_string(), "2760");

        // Move to row 3 (Special: SUID/SGID/Sticky) and toggle SGID off (col 1) -> 0760
        state.move_row(2);
        assert_eq!(state.row, 3);
        state.toggle_col(1);
        assert_eq!(state.octal_string(), "0760");
        // Sticky on (col 2) -> 1760
        state.toggle_col(2);
        assert_eq!(state.octal_string(), "1760");

        // Move down to row 4 (Recursive) and toggle
        state.move_row(1);
        assert_eq!(state.row, 4);
        state.toggle();
        assert!(state.recursive);

        // Files do not expose Recursive row (4 rows: 0..=3)
        let mut file_state = ChmodState::new(vec![PathBuf::from("/srv/app/.env")], 0o0600, false);
        file_state.move_row(-1);
        assert_eq!(file_state.row, 3);
    }

    #[test]
    fn set_acl_scope_autocomplete_existing_sync_and_x_modes() {
        use crate::models::{AclEntry, AclInfo, AclScope};
        use ratatui::backend::TestBackend;
        use ratatui::Terminal;

        let users = vec![
            CandidateItem {
                name: "deploy".into(),
                meta: "UID 1001".into(),
                is_regular: true,
            },
            CandidateItem {
                name: "debian-tor".into(),
                meta: "UID 112".into(),
                is_regular: false,
            },
        ];
        let groups = vec![CandidateItem {
            name: "devs".into(),
            meta: "GID 1005".into(),
            is_regular: false,
        }];
        let existing = AclInfo {
            available: true,
            entries: vec![
                AclEntry {
                    is_default: false,
                    scope: AclScope::User,
                    qualifier: "deploy".into(),
                    perms: "r-x".into(),
                    effective: None,
                },
                AclEntry {
                    is_default: true,
                    scope: AclScope::User,
                    qualifier: "deploy".into(),
                    perms: "r-x".into(),
                    effective: None,
                },
            ],
            error: None,
        };

        let mut st = SetAclState::new(
            vec![PathBuf::from("/var/www/app")],
            AclTargetScope::User,
            "",
            true,
            users.clone(),
            groups.clone(),
            existing.clone(),
        );
        // Empty name -> NO dropdown yet!
        assert_eq!(st.row, 1);
        assert!(st.filtered_suggestions().is_empty());

        // Type 'd' -> dropdown appears, regular user 'deploy' ranked first
        st.push_char('d');
        let sugs = st.filtered_suggestions();
        assert_eq!(sugs.len(), 2);
        assert_eq!(sugs[0].name, "deploy");

        // Accept suggestion -> fills 'deploy' AND loads existing r-x + Access & Default flags!
        assert!(st.accept_suggestion());
        assert_eq!(st.name, "deploy");
        assert!(st.loaded_existing);
        assert!(st.read);
        assert!(!st.write);
        assert_eq!(st.exec, ExecPerm::Always);
        assert!(st.apply_access);
        assert!(st.apply_default);
        assert_eq!(st.build_specs(), vec!["u:deploy:r-x", "d:u:deploy:r-x"]);

        // Move to Perms (row 2), turn Write on and Exec to Conditional 'X' -> rwX on both Access & Default!
        st.move_row(1);
        st.toggle_write();
        st.set_exec(ExecPerm::Conditional);
        assert_eq!(st.build_specs(), vec!["u:deploy:rwX", "d:u:deploy:rwX"]);

        // Switch to Mask scope -> Name row skipped, Mask spec built cleanly
        st.row = 0;
        st.cycle_scope(2);
        assert_eq!(st.scope, AclTargetScope::Mask);
        assert_eq!(st.build_specs(), vec!["m::rwX"]);
        st.move_row(1);
        assert_eq!(st.row, 2); // skipped row 1 (Name)

        let modal = Modal::set_acl(
            vec![
                PathBuf::from("/var/www/app"),
                PathBuf::from("/var/www/uploads"),
            ],
            AclTargetScope::User,
            "de",
            true,
            users,
            groups,
            existing,
        );
        let mut term = Terminal::new(TestBackend::new(80, 24)).unwrap();
        term.draw(|frame| render(frame, &modal, frame.area()))
            .unwrap();
    }

    #[test]
    fn remove_acl_checklist_collects_specs_and_renders() {
        use crate::models::{AclEntry, AclInfo, AclScope};
        use ratatui::backend::TestBackend;
        use ratatui::Terminal;

        let acl = AclInfo {
            available: true,
            entries: vec![
                AclEntry {
                    is_default: false,
                    scope: AclScope::User,
                    qualifier: "deploy".into(),
                    perms: "rwx".into(),
                    effective: None,
                },
                AclEntry {
                    is_default: true,
                    scope: AclScope::Group,
                    qualifier: "devs".into(),
                    perms: "r-x".into(),
                    effective: None,
                },
            ],
            error: None,
        };
        let mut st = RemoveAclState::from_acls(
            vec![PathBuf::from("/var/www")],
            true,
            &[acl],
            Some("deploy"),
            None,
        );
        // Preselected 'deploy', plus 'd:g:devs' and 'All default ACLs'
        assert_eq!(st.items.len(), 3);
        assert!(st.items[0].selected);
        assert_eq!(st.selected_specs(), vec!["u:deploy".to_string()]);
        assert!(!st.wants_clear_default());

        // Toggle 'All default ACLs' on
        st.move_row(2);
        st.toggle();
        assert!(st.wants_clear_default());

        let modal = Modal::remove_acl(st);
        let mut term = Terminal::new(TestBackend::new(80, 24)).unwrap();
        term.draw(|frame| render(frame, &modal, frame.area()))
            .unwrap();
    }

    #[test]
    fn paste_acl_modal_requires_yes_and_renders() {
        use ratatui::backend::TestBackend;
        use ratatui::Terminal;

        let mut st = PasteAclState::new(
            vec![PathBuf::from("/var/www/dst")],
            true,
            "src".into(),
            "user::rwx\ngroup::r-x\nother::---\n".into(),
            3,
        );
        assert!(!st.is_confirmed());
        assert_eq!(st.row, 1);
        for c in "YES".chars() {
            st.push_char(c);
        }
        assert!(st.is_confirmed());
        st.move_row(-1);
        assert_eq!(st.row, 0);
        st.toggle_recursive();
        assert!(st.recursive);

        let modal = Modal::paste_acl(st);
        let mut term = Terminal::new(TestBackend::new(80, 24)).unwrap();
        term.draw(|frame| render(frame, &modal, frame.area()))
            .unwrap();
    }

    #[test]
    fn chmod_checkbox_modal_renders_without_panic() {
        use ratatui::backend::TestBackend;
        use ratatui::Terminal;
        let modal = Modal::chmod(vec![PathBuf::from("/var/www/app")], 0o0750, true);
        let mut term = Terminal::new(TestBackend::new(80, 24)).unwrap();
        term.draw(|frame| render(frame, &modal, frame.area()))
            .unwrap();
    }
}
