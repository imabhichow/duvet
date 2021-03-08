use crate::{attribute::Attribute, db::Db, entity, fs, types};
use anyhow::Result;
use proc_macro2::Span;
use syn::{
    spanned::Spanned,
    visit::{self, Visit},
};

#[derive(Default)]
pub struct RustSrc {
    config: Config,
}

#[derive(Default)]
pub struct Config {
    // TODO
}

impl RustSrc {
    pub fn annotate(&self, db: &Db) -> Result<()> {
        for file in db.fs().iter() {
            let (file, _path) = file?;
            let content = db.fs().open(file)?;
            match syn::parse_file(&content) {
                Ok(ast) => {
                    let entity = db.entities().create()?;
                    db.entities().set_attribute(entity, types::CODE, ())?;
                    let mut visitor = Visitor {
                        config: &self.config,
                        db,
                        mode: (entity, types::CODE),
                        entity,
                        file,
                    };
                    visitor.visit_file(&ast);
                }
                Err(_err) => {
                    // TODO add syntax error
                }
            }
        }

        Ok(())
    }
}

struct Visitor<'a> {
    config: &'a Config,
    db: &'a Db,
    mode: (entity::Id, Attribute<()>),
    entity: entity::Id,
    file: fs::Id,
}

impl<'a> Visitor<'a> {
    fn on_span(&self, span: Span) {
        let start = span.start();
        let end = span.end();

        // if it's empty don't record anything
        if start == end {
            return;
        }

        let offsets = self
            .db
            .fs()
            .map_line_column(
                self.file,
                ((start.line - 1) as _, start.column as _),
                ((end.line - 1) as _, end.column as _),
            )
            .unwrap();

        self.db
            .regions()
            .insert(self.file, offsets, self.entity)
            .unwrap();
    }

    fn on_grouped_span(&self, span: Span) {
        let start = span.start();
        let end = span.end();

        // if it's empty don't record anything
        if start == end {
            return;
        }

        let open_offset = self
            .db
            .fs()
            .map_line_column(
                self.file,
                ((start.line - 1) as _, (start.column) as _),
                ((start.line - 1) as _, (start.column + 1) as _),
            )
            .unwrap();

        self.db
            .regions()
            .insert(self.file, open_offset, self.entity)
            .unwrap();

        let close_offset = self
            .db
            .fs()
            .map_line_column(
                self.file,
                ((end.line - 1) as _, (end.column - 1) as _),
                ((end.line - 1) as _, (end.column) as _),
            )
            .unwrap();

        self.db
            .regions()
            .insert(self.file, close_offset, self.entity)
            .unwrap();
    }

    fn on_attrs(&mut self, _attrs: &[syn::Attribute]) -> (entity::Id, Attribute<()>) {
        let id = self.mode.0;

        if self.mode.1 == types::TEST {
            return (id, types::TEST);
        }

        // TODO parse the attributes and figure out if we're in test mode
        (id, types::CODE)
    }
}

macro_rules! span {
    ($visitor:ident $(, $other:expr)* $(,)?) => {
        $(
            $visitor.on_span($other.span());
        )*
    };
}

impl<'a, 'ast> Visit<'ast> for Visitor<'a> {
    fn visit_arm(&mut self, i: &'ast syn::Arm) {
        let mode = self.on_attrs(&i.attrs);
        visit::visit_arm(self, i);
        self.mode = mode;
    }

    fn visit_bare_fn_arg(&mut self, i: &'ast syn::BareFnArg) {
        let mode = self.on_attrs(&i.attrs);
        visit::visit_bare_fn_arg(self, i);
        self.mode = mode;
    }

    fn visit_expr_array(&mut self, i: &'ast syn::ExprArray) {
        let mode = self.on_attrs(&i.attrs);
        self.on_grouped_span(i.bracket_token.span);
        visit::visit_expr_array(self, i);
        self.mode = mode;
    }

    fn visit_expr_assign(&mut self, i: &'ast syn::ExprAssign) {
        let mode = self.on_attrs(&i.attrs);
        span!(self, i.eq_token);
        visit::visit_expr_assign(self, i);
        self.mode = mode;
    }

    fn visit_expr_assign_op(&mut self, i: &'ast syn::ExprAssignOp) {
        let mode = self.on_attrs(&i.attrs);
        span!(self, i.op);
        visit::visit_expr_assign_op(self, i);
        self.mode = mode;
    }

    fn visit_expr_async(&mut self, i: &'ast syn::ExprAsync) {
        let mode = self.on_attrs(&i.attrs);
        span!(self, i.async_token, i.capture);
        visit::visit_expr_async(self, i);
        self.mode = mode;
    }

    fn visit_expr_await(&mut self, i: &'ast syn::ExprAwait) {
        let mode = self.on_attrs(&i.attrs);
        span!(self, i.dot_token, i.await_token);
        visit::visit_expr_await(self, i);
        self.mode = mode;
    }

    fn visit_expr_binary(&mut self, i: &'ast syn::ExprBinary) {
        let mode = self.on_attrs(&i.attrs);
        span!(self, i.op);
        visit::visit_expr_binary(self, i);
        self.mode = mode;
    }

    fn visit_expr_block(&mut self, i: &'ast syn::ExprBlock) {
        let mode = self.on_attrs(&i.attrs);
        visit::visit_expr_block(self, i);
        self.mode = mode;
    }

    fn visit_expr_box(&mut self, i: &'ast syn::ExprBox) {
        let mode = self.on_attrs(&i.attrs);
        span!(self, i.box_token);
        visit::visit_expr_box(self, i);
        self.mode = mode;
    }

    fn visit_expr_break(&mut self, i: &'ast syn::ExprBreak) {
        let mode = self.on_attrs(&i.attrs);
        span!(self, i.break_token, i.label);
        visit::visit_expr_break(self, i);
        self.mode = mode;
    }

    fn visit_expr_call(&mut self, i: &'ast syn::ExprCall) {
        let mode = self.on_attrs(&i.attrs);
        self.on_grouped_span(i.paren_token.span);
        visit::visit_expr_call(self, i);
        self.mode = mode;
    }

    fn visit_expr_cast(&mut self, i: &'ast syn::ExprCast) {
        let mode = self.on_attrs(&i.attrs);
        span!(self, i.as_token);
        visit::visit_expr_cast(self, i);
        self.mode = mode;
    }

    fn visit_expr_closure(&mut self, i: &'ast syn::ExprClosure) {
        let mode = self.on_attrs(&i.attrs);
        visit::visit_expr_closure(self, i);
        self.mode = mode;
    }

    fn visit_expr_continue(&mut self, i: &'ast syn::ExprContinue) {
        let mode = self.on_attrs(&i.attrs);
        span!(self, i.continue_token, i.label);
        visit::visit_expr_continue(self, i);
        self.mode = mode;
    }

    fn visit_expr_field(&mut self, i: &'ast syn::ExprField) {
        let mode = self.on_attrs(&i.attrs);
        span!(self, i.dot_token, i.member);
        visit::visit_expr(self, &i.base);
        self.mode = mode;
    }

    fn visit_expr_for_loop(&mut self, i: &'ast syn::ExprForLoop) {
        let mode = self.on_attrs(&i.attrs);
        span!(self, i.label, i.for_token);
        visit::visit_expr_for_loop(self, i);
        self.mode = mode;
    }

    fn visit_expr_group(&mut self, i: &'ast syn::ExprGroup) {
        let mode = self.on_attrs(&i.attrs);
        visit::visit_expr_group(self, i);
        self.mode = mode;
    }

    fn visit_expr_if(&mut self, i: &'ast syn::ExprIf) {
        let mode = self.on_attrs(&i.attrs);
        visit::visit_expr_if(self, i);
        self.mode = mode;
    }

    fn visit_expr_index(&mut self, i: &'ast syn::ExprIndex) {
        let mode = self.on_attrs(&i.attrs);
        self.on_grouped_span(i.bracket_token.span);
        visit::visit_expr_index(self, i);
        self.mode = mode;
    }

    fn visit_expr_let(&mut self, i: &'ast syn::ExprLet) {
        let mode = self.on_attrs(&i.attrs);
        span!(self, i.let_token);
        visit::visit_expr_let(self, i);
        self.mode = mode;
    }

    fn visit_expr_lit(&mut self, i: &'ast syn::ExprLit) {
        let mode = self.on_attrs(&i.attrs);
        visit::visit_expr_lit(self, i);
        self.mode = mode;
    }

    fn visit_expr_loop(&mut self, i: &'ast syn::ExprLoop) {
        let mode = self.on_attrs(&i.attrs);
        span!(self, i.label, i.loop_token);
        visit::visit_expr_loop(self, i);
        self.mode = mode;
    }

    fn visit_expr_macro(&mut self, i: &'ast syn::ExprMacro) {
        let mode = self.on_attrs(&i.attrs);
        visit::visit_expr_macro(self, i);
        self.mode = mode;
    }

    fn visit_expr_match(&mut self, i: &'ast syn::ExprMatch) {
        let mode = self.on_attrs(&i.attrs);
        visit::visit_expr_match(self, i);
        self.mode = mode;
    }

    fn visit_expr_method_call(&mut self, i: &'ast syn::ExprMethodCall) {
        let mode = self.on_attrs(&i.attrs);
        span!(self, i.dot_token, i.method, i.turbofish);
        visit::visit_expr_method_call(self, i);
        self.mode = mode;
    }

    fn visit_expr_paren(&mut self, i: &'ast syn::ExprParen) {
        let mode = self.on_attrs(&i.attrs);
        self.on_grouped_span(i.paren_token.span);
        visit::visit_expr_paren(self, i);
        self.mode = mode;
    }

    fn visit_expr_path(&mut self, i: &'ast syn::ExprPath) {
        let mode = self.on_attrs(&i.attrs);
        if let Some(qself) = i.qself.as_ref() {
            span!(
                self,
                qself.lt_token,
                qself.ty,
                qself.as_token,
                qself.gt_token,
                i.path
            );
        } else {
            span!(self, i.path);
        }
        self.mode = mode;
    }

    fn visit_expr_range(&mut self, i: &'ast syn::ExprRange) {
        let mode = self.on_attrs(&i.attrs);
        visit::visit_expr_range(self, i);
        self.mode = mode;
    }

    fn visit_expr_reference(&mut self, i: &'ast syn::ExprReference) {
        let mode = self.on_attrs(&i.attrs);
        span!(self, i.and_token, i.mutability);
        visit::visit_expr_reference(self, i);
        self.mode = mode;
    }

    fn visit_expr_repeat(&mut self, i: &'ast syn::ExprRepeat) {
        let mode = self.on_attrs(&i.attrs);
        self.on_grouped_span(i.bracket_token.span);
        visit::visit_expr_repeat(self, i);
        self.mode = mode;
    }

    fn visit_expr_return(&mut self, i: &'ast syn::ExprReturn) {
        let mode = self.on_attrs(&i.attrs);
        span!(self, i.return_token);
        visit::visit_expr_return(self, i);
        self.mode = mode;
    }

    fn visit_expr_struct(&mut self, i: &'ast syn::ExprStruct) {
        let mode = self.on_attrs(&i.attrs);
        span!(self, i.path);
        visit::visit_expr_struct(self, i);
        self.mode = mode;
    }

    fn visit_expr_try(&mut self, i: &'ast syn::ExprTry) {
        let mode = self.on_attrs(&i.attrs);
        span!(self, i.question_token);
        visit::visit_expr_try(self, i);
        self.mode = mode;
    }

    fn visit_expr_try_block(&mut self, i: &'ast syn::ExprTryBlock) {
        let mode = self.on_attrs(&i.attrs);
        span!(self, i.try_token);
        visit::visit_expr_try_block(self, i);
        self.mode = mode;
    }

    fn visit_expr_tuple(&mut self, i: &'ast syn::ExprTuple) {
        let mode = self.on_attrs(&i.attrs);
        self.on_grouped_span(i.paren_token.span);
        visit::visit_expr_tuple(self, i);
        self.mode = mode;
    }

    fn visit_expr_type(&mut self, i: &'ast syn::ExprType) {
        let mode = self.on_attrs(&i.attrs);
        visit::visit_expr(self, &i.expr);
        self.mode = mode;
    }

    fn visit_expr_unary(&mut self, i: &'ast syn::ExprUnary) {
        let mode = self.on_attrs(&i.attrs);
        span!(self, i.op);
        visit::visit_expr(self, &i.expr);
        self.mode = mode;
    }

    fn visit_expr_unsafe(&mut self, i: &'ast syn::ExprUnsafe) {
        let mode = self.on_attrs(&i.attrs);
        // TODO add unsafe entity
        span!(self, i.unsafe_token);
        visit::visit_expr_unsafe(self, i);
        self.mode = mode;
    }

    fn visit_expr_while(&mut self, i: &'ast syn::ExprWhile) {
        let mode = self.on_attrs(&i.attrs);
        span!(self, i.label, i.while_token);
        visit::visit_expr_while(self, i);
        self.mode = mode;
    }

    fn visit_expr_yield(&mut self, i: &'ast syn::ExprYield) {
        let mode = self.on_attrs(&i.attrs);
        span!(self, i.yield_token);
        visit::visit_expr_yield(self, i);
        self.mode = mode;
    }

    fn visit_field_pat(&mut self, i: &'ast syn::FieldPat) {
        let mode = self.on_attrs(&i.attrs);
        span!(self, i.member);
        visit::visit_field_pat(self, i);
        self.mode = mode;
    }

    fn visit_field_value(&mut self, i: &'ast syn::FieldValue) {
        let mode = self.on_attrs(&i.attrs);
        span!(self, i.member);
        visit::visit_field_value(self, i);
        self.mode = mode;
    }

    fn visit_impl_item_const(&mut self, _: &'ast syn::ImplItemConst) {
        // ignore
    }

    fn visit_impl_item_macro(&mut self, _: &'ast syn::ImplItemMacro) {
        // ignore
        // TODO should this be ignored?
    }

    fn visit_impl_item_method(&mut self, i: &'ast syn::ImplItemMethod) {
        let parent = self.entity;
        self.entity = self.db.entities().create().unwrap();

        // TODO associate a function name

        self.db
            .entities()
            .set_attribute(self.entity, types::FUNCTION, ())
            .unwrap();

        let parent_mode = self.on_attrs(&i.attrs);

        visit::visit_impl_item_method(self, i);

        self.entity = parent;
        self.mode = parent_mode;
    }

    fn visit_impl_item_type(&mut self, _: &'ast syn::ImplItemType) {
        // ignore
    }

    fn visit_index(&mut self, i: &'ast syn::Index) {
        self.on_span(i.span);
    }

    fn visit_item_fn(&mut self, i: &'ast syn::ItemFn) {
        let mode = self.on_attrs(&i.attrs);

        let parent = self.entity;
        self.entity = self.db.entities().create().unwrap();

        // TODO associate a function name

        self.db
            .entities()
            .set_attribute(self.entity, types::FUNCTION, ())
            .unwrap();

        visit::visit_item_fn(self, i);

        self.entity = parent;
        self.mode = mode;
    }

    fn visit_item_impl(&mut self, i: &'ast syn::ItemImpl) {
        let mode = self.on_attrs(&i.attrs);

        // TODO annotate unsafe impl

        visit::visit_item_impl(self, i);
        self.mode = mode;
    }

    fn visit_item_macro(&mut self, i: &'ast syn::ItemMacro) {
        let mode = self.on_attrs(&i.attrs);

        // TODO annotate macro

        visit::visit_item_macro(self, i);
        self.mode = mode;
    }

    fn visit_item_macro2(&mut self, i: &'ast syn::ItemMacro2) {
        let mode = self.on_attrs(&i.attrs);

        // TODO annotate macro

        visit::visit_item_macro2(self, i);
        self.mode = mode;
    }

    fn visit_item_mod(&mut self, i: &'ast syn::ItemMod) {
        let mode = self.on_attrs(&i.attrs);

        // TODO annotate module

        visit::visit_item_mod(self, i);
        self.mode = mode;
    }

    fn visit_item_trait(&mut self, i: &'ast syn::ItemTrait) {
        // TODO skip if fmt::Debug
        let mode = self.on_attrs(&i.attrs);
        visit::visit_item_trait(self, i);
        self.mode = mode;
    }

    // ignores
    fn visit_item_const(&mut self, _i: &'ast syn::ItemConst) {}
    fn visit_item_enum(&mut self, _i: &'ast syn::ItemEnum) {}
    fn visit_item_extern_crate(&mut self, _i: &'ast syn::ItemExternCrate) {}
    fn visit_item_foreign_mod(&mut self, _i: &'ast syn::ItemForeignMod) {}
    fn visit_item_static(&mut self, _i: &'ast syn::ItemStatic) {}
    fn visit_item_struct(&mut self, _i: &'ast syn::ItemStruct) {}
    fn visit_item_trait_alias(&mut self, _i: &'ast syn::ItemTraitAlias) {}
    fn visit_item_type(&mut self, _i: &'ast syn::ItemType) {}
    fn visit_item_union(&mut self, _i: &'ast syn::ItemUnion) {}
    fn visit_item_use(&mut self, _i: &'ast syn::ItemUse) {}

    fn visit_lit(&mut self, i: &'ast syn::Lit) {
        // TODO collect all of the literals in the codebase?
        span!(self, i);
    }

    fn visit_local(&mut self, i: &'ast syn::Local) {
        let mode = self.on_attrs(&i.attrs);
        span!(self, i.let_token);
        visit::visit_local(self, i);
        self.mode = mode;
    }

    fn visit_macro(&mut self, i: &'ast syn::Macro) {
        // TODO change mode when panic, unreachable, assert_eq, debug_assert_eq, dbg, etc
        span!(self, i.path, i.bang_token);
        visit::visit_macro(self, i);
    }

    fn visit_trait_item_const(&mut self, _: &'ast syn::TraitItemConst) {
        // ignore
    }

    fn visit_trait_item_macro(&mut self, _: &'ast syn::TraitItemMacro) {
        // ignore
        // TODO should this be ignored?
    }

    fn visit_trait_item_method(&mut self, i: &'ast syn::TraitItemMethod) {
        let mode = self.on_attrs(&i.attrs);

        let parent = self.entity;
        self.entity = self.db.entities().create().unwrap();

        // TODO associate a function name

        self.db
            .entities()
            .set_attribute(self.entity, types::FUNCTION, ())
            .unwrap();

        visit::visit_trait_item_method(self, i);

        self.entity = parent;
        self.mode = mode;
    }

    fn visit_trait_item_type(&mut self, _: &'ast syn::TraitItemType) {
        // ignore
    }

    fn visit_type(&mut self, _: &'ast syn::Type) {
        // ignore
    }
}
