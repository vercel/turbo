use std::hash::BuildHasherDefault;

use indexmap::IndexSet;
use rustc_hash::FxHasher;
use swc_core::ecma::{
    ast::{
        BlockStmtOrExpr, Constructor, Expr, Function, Id, Ident, MemberProp, Pat, PatOrExpr,
        PropName,
    },
    visit::{noop_visit_type, visit_obj_and_computed, Visit, VisitWith},
};

#[derive(Default)]
pub(crate) struct IdentUsageCollector {
    vars: Vars,
    ignore_nested: bool,
    is_read: bool,
}

impl Visit for IdentUsageCollector {
    fn visit_block_stmt_or_expr(&mut self, n: &BlockStmtOrExpr) {
        if self.ignore_nested {
            return;
        }

        n.visit_children_with(self);
    }

    fn visit_constructor(&mut self, n: &Constructor) {
        if self.ignore_nested {
            return;
        }

        n.visit_children_with(self);
    }

    fn visit_expr(&mut self, e: &Expr) {
        let old = self.is_read;
        self.is_read = true;
        e.visit_children_with(self);
        self.is_read = old;
    }

    fn visit_function(&mut self, n: &Function) {
        if self.ignore_nested {
            return;
        }

        n.visit_children_with(self);
    }

    fn visit_ident(&mut self, n: &Ident) {
        if self.is_read {
            if self.reads.iter().any(|id| id == &n.to_id()) {
                return;
            }
            self.reads.push(n.to_id());
        } else {
            if self.writes.iter().any(|id| id == &n.to_id()) {
                return;
            }
            self.writes.push(n.to_id());
        }
    }

    fn visit_member_prop(&mut self, n: &MemberProp) {
        if let MemberProp::Computed(..) = n {
            n.visit_children_with(self);
        }
    }

    fn visit_pat(&mut self, p: &Pat) {
        let old = self.is_read;
        self.is_read = false;
        p.visit_children_with(self);
        self.is_read = old;
    }

    fn visit_pat_or_expr(&mut self, n: &PatOrExpr) {
        if let PatOrExpr::Expr(e) = n {
            let old = self.is_read;
            self.is_read = false;
            // Skip `visit_expr`
            e.visit_children_with(self);
            self.is_read = old;
        } else {
            n.visit_children_with(self);
        }
    }

    fn visit_prop_name(&mut self, n: &PropName) {
        if let PropName::Computed(..) = n {
            n.visit_children_with(self);
        }
    }

    noop_visit_type!();

    visit_obj_and_computed!();
}

#[derive(Default)]
pub(crate) struct CapturedIdCollector {
    vars: Vars,
    is_nested: bool,
    is_read: bool,
}

impl Visit for CapturedIdCollector {
    fn visit_block_stmt_or_expr(&mut self, n: &BlockStmtOrExpr) {
        let old = self.is_nested;
        self.is_nested = true;
        n.visit_children_with(self);
        self.is_nested = old;
    }

    fn visit_constructor(&mut self, n: &Constructor) {
        let old = self.is_nested;
        self.is_nested = true;
        n.visit_children_with(self);
        self.is_nested = old;
    }

    fn visit_expr(&mut self, e: &Expr) {
        let old = self.is_read;
        self.is_read = true;
        e.visit_children_with(self);
        self.is_read = old;
    }

    fn visit_function(&mut self, n: &Function) {
        let old = self.is_nested;
        self.is_nested = true;
        n.visit_children_with(self);
        self.is_nested = old;
    }

    fn visit_ident(&mut self, n: &Ident) {
        if self.is_nested {
            if self.is_read {
                if self.reads.iter().any(|id| id == &n.to_id()) {
                    return;
                }
                self.reads.push(n.to_id());
            } else {
                if self.writes.iter().any(|id| id == &n.to_id()) {
                    return;
                }
                self.writes.push(n.to_id());
            }
        }
    }

    fn visit_pat(&mut self, p: &Pat) {
        let old = self.is_read;
        self.is_read = false;
        p.visit_children_with(self);
        self.is_read = old;
    }

    fn visit_pat_or_expr(&mut self, n: &PatOrExpr) {
        if let PatOrExpr::Expr(e) = n {
            let old = self.is_read;
            self.is_read = false;
            // Skip `visit_expr`
            e.visit_children_with(self);
            self.is_read = old;
        } else {
            n.visit_children_with(self);
        }
    }

    fn visit_prop_name(&mut self, n: &PropName) {
        if let PropName::Computed(..) = n {
            n.visit_children_with(self);
        }
    }

    noop_visit_type!();

    visit_obj_and_computed!();
}

#[derive(Debug, Default)]
pub(super) struct Vars {
    pub read: IndexSet<Id, BuildHasherDefault<FxHasher>>,
    pub write: IndexSet<Id, BuildHasherDefault<FxHasher>>,
}

/// Returns `(read, write)`
pub(crate) fn ids_captured_by<N>(n: &N) -> Vars
where
    N: VisitWith<CapturedIdCollector>,
{
    let mut v = CapturedIdCollector {
        is_nested: false,
        ..Default::default()
    };
    n.visit_with(&mut v);
    v.vars
}

/// Returns `(read, write)`
pub(crate) fn ids_used_by<N>(n: &N) -> Vars
where
    N: VisitWith<IdentUsageCollector>,
{
    let mut v = IdentUsageCollector {
        ignore_nested: false,
        ..Default::default()
    };
    n.visit_with(&mut v);
    v.vars
}

/// Returns `(read, write)`
pub(crate) fn ids_used_by_ignoring_nested<N>(n: &N) -> Vars
where
    N: VisitWith<IdentUsageCollector>,
{
    let mut v = IdentUsageCollector {
        ignore_nested: true,
        ..Default::default()
    };
    n.visit_with(&mut v);
    v.vars
}
