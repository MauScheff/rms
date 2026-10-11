//! Production workflow ownership is expression-based, never pattern-based.
//! This check does not grant purity, effect, or source authority.
use syn::parse::{Parse, ParseStream, Parser};
use syn::visit::{self, Visit};
use syn::{Expr, Item, Member, Path};

struct Ownership<'a> {
    types: &'a [&'a str],
    found: bool,
}

fn progression(name: &str) -> bool {
    matches!(
        name,
        "next_state" | "nextState" | "state_after" | "stateAfter"
    )
}

fn workflow(name: &str) -> bool {
    matches!(
        name,
        "transition_record"
            | "transitionRecord"
            | "drive_machine"
            | "driveMachine"
            | "initial_state"
    )
}

impl Ownership<'_> {
    fn machine_path(&self, path: &Path) -> bool {
        path.segments.iter().any(|segment| {
            self.types.iter().any(|name| {
                let name = name.rsplit("::").next().unwrap_or(name);
                !name.is_empty() && segment.ident == name
            })
        })
    }

    fn write_target(expr: &Expr) -> bool {
        match expr {
            Expr::Field(field) => {
                matches!(&field.member, Member::Named(name) if progression(&name.to_string()))
                    || Self::write_target(&field.base)
            }
            Expr::Path(path) => path
                .path
                .segments
                .last()
                .is_some_and(|part| progression(&part.ident.to_string())),
            Expr::Paren(expr) => Self::write_target(&expr.expr),
            Expr::Group(expr) => Self::write_target(&expr.expr),
            Expr::Unary(expr) => Self::write_target(&expr.expr),
            Expr::Index(expr) => Self::write_target(&expr.expr),
            _ => false,
        }
    }
}

// matches! contains an expression followed by a pattern and optional guard.
// Parsing those positions prevents pattern names from becoming constructors.
struct Matches {
    input: Expr,
    guard: Option<Expr>,
}
impl Parse for Matches {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let expr = input.parse()?;
        input.parse::<syn::Token![,]>()?;
        syn::Pat::parse_multi_with_leading_vert(input)?;
        let guard = if input.peek(syn::Token![if]) {
            input.parse::<syn::Token![if]>()?;
            Some(input.parse()?)
        } else {
            None
        };
        if input.peek(syn::Token![,]) {
            input.parse::<syn::Token![,]>()?;
        }
        Ok(Self { input: expr, guard })
    }
}

impl<'ast> Visit<'ast> for Ownership<'_> {
    fn visit_item(&mut self, item: &'ast Item) {
        if !super::has_cfg_test_attr(super::item_attrs(item)) {
            visit::visit_item(self, item);
        }
    }

    fn visit_impl_item(&mut self, item: &'ast syn::ImplItem) {
        let attrs = match item {
            syn::ImplItem::Fn(item) => &item.attrs,
            syn::ImplItem::Const(item) => &item.attrs,
            syn::ImplItem::Type(item) => &item.attrs,
            syn::ImplItem::Macro(item) => &item.attrs,
            _ => return,
        };
        if !super::has_cfg_test_attr(attrs) {
            visit::visit_impl_item(self, item);
        }
    }

    fn visit_trait_item(&mut self, item: &'ast syn::TraitItem) {
        let attrs = match item {
            syn::TraitItem::Fn(item) => &item.attrs,
            syn::TraitItem::Const(item) => &item.attrs,
            syn::TraitItem::Type(item) => &item.attrs,
            syn::TraitItem::Macro(item) => &item.attrs,
            _ => return,
        };
        if !super::has_cfg_test_attr(attrs) {
            visit::visit_trait_item(self, item);
        }
    }

    fn visit_pat(&mut self, _: &'ast syn::Pat) {}

    fn visit_local(&mut self, local: &'ast syn::Local) {
        if !super::has_cfg_test_attr(&local.attrs) {
            visit::visit_local(self, local);
        }
    }

    fn visit_arm(&mut self, arm: &'ast syn::Arm) {
        if !super::has_cfg_test_attr(&arm.attrs) {
            visit::visit_arm(self, arm);
        }
    }

    fn visit_expr_block(&mut self, expr: &'ast syn::ExprBlock) {
        if !super::has_cfg_test_attr(&expr.attrs) {
            visit::visit_expr_block(self, expr);
        }
    }

    fn visit_signature(&mut self, signature: &'ast syn::Signature) {
        self.found |= workflow(&signature.ident.to_string());
        visit::visit_signature(self, signature);
    }

    fn visit_expr_path(&mut self, expr: &'ast syn::ExprPath) {
        self.found |= self.machine_path(&expr.path);
        visit::visit_expr_path(self, expr);
    }

    fn visit_expr_struct(&mut self, expr: &'ast syn::ExprStruct) {
        self.found |= self.machine_path(&expr.path)
            || expr.fields.iter().any(|field| matches!(&field.member, Member::Named(name) if progression(&name.to_string())));
        visit::visit_expr_struct(self, expr);
    }

    fn visit_expr_call(&mut self, expr: &'ast syn::ExprCall) {
        if let Expr::Path(path) = expr.func.as_ref() {
            self.found |= path
                .path
                .segments
                .last()
                .is_some_and(|part| workflow(&part.ident.to_string()));
        }
        visit::visit_expr_call(self, expr);
    }

    fn visit_expr_method_call(&mut self, expr: &'ast syn::ExprMethodCall) {
        self.found |= workflow(&expr.method.to_string());
        visit::visit_expr_method_call(self, expr);
    }

    fn visit_expr_assign(&mut self, expr: &'ast syn::ExprAssign) {
        self.found |= Self::write_target(&expr.left);
        visit::visit_expr_assign(self, expr);
    }

    fn visit_expr_binary(&mut self, expr: &'ast syn::ExprBinary) {
        if matches!(
            expr.op,
            syn::BinOp::AddAssign(_)
                | syn::BinOp::SubAssign(_)
                | syn::BinOp::MulAssign(_)
                | syn::BinOp::DivAssign(_)
                | syn::BinOp::RemAssign(_)
                | syn::BinOp::BitXorAssign(_)
                | syn::BinOp::BitAndAssign(_)
                | syn::BinOp::BitOrAssign(_)
                | syn::BinOp::ShlAssign(_)
                | syn::BinOp::ShrAssign(_)
        ) {
            self.found |= Self::write_target(&expr.left);
        }
        visit::visit_expr_binary(self, expr);
    }

    fn visit_macro(&mut self, mac: &'ast syn::Macro) {
        if mac
            .path
            .segments
            .last()
            .is_some_and(|part| part.ident == "matches")
        {
            if let Ok(parsed) = syn::parse2::<Matches>(mac.tokens.clone()) {
                self.visit_expr(&parsed.input);
                if let Some(guard) = &parsed.guard {
                    self.visit_expr(guard);
                }
                return;
            }
        }
        if let Ok(exprs) = syn::punctuated::Punctuated::<Expr, syn::Token![,]>::parse_terminated
            .parse2(mac.tokens.clone())
        {
            for expr in &exprs {
                self.visit_expr(expr);
            }
        } else {
            // Unknown macro grammar cannot establish an exemption. Inspect
            // identifiers, not strings/comments, and retain a conservative gate.
            fn identifiers(tokens: proc_macro2::TokenStream, names: &mut Vec<String>) {
                for token in tokens {
                    match token {
                        proc_macro2::TokenTree::Ident(name) => names.push(name.to_string()),
                        proc_macro2::TokenTree::Group(group) => identifiers(group.stream(), names),
                        _ => {}
                    }
                }
            }
            let mut names = Vec::new();
            identifiers(mac.tokens.clone(), &mut names);
            self.found |= names.iter().any(|name| {
                workflow(name)
                    || progression(name)
                    || self
                        .types
                        .iter()
                        .any(|ty| !ty.is_empty() && ty.rsplit("::").next() == Some(name.as_str()))
            });
        }
    }
}

pub(super) fn owns_workflow(source: &str, types: &[&str]) -> Option<bool> {
    let file = syn::parse_file(source).ok()?;
    if super::has_cfg_test_attr(&file.attrs) {
        return Some(false);
    }
    let mut visitor = Ownership {
        types,
        found: false,
    };
    visitor.visit_file(&file);
    Some(visitor.found)
}
