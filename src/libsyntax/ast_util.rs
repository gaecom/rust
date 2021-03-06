// Copyright 2012-2014 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use ast::*;
use ast;
use ast_util;
use codemap;
use codemap::Span;
use owned_slice::OwnedSlice;
use parse::token;
use print::pprust;
use visit::Visitor;
use visit;

use std::cell::Cell;
use std::cmp;
use std::strbuf::StrBuf;
use std::u32;

pub fn path_name_i(idents: &[Ident]) -> StrBuf {
    // FIXME: Bad copies (#2543 -- same for everything else that says "bad")
    idents.iter().map(|i| {
        token::get_ident(*i).get().to_strbuf()
    }).collect::<Vec<StrBuf>>().connect("::").to_strbuf()
}

// totally scary function: ignores all but the last element, should have
// a different name
pub fn path_to_ident(path: &Path) -> Ident {
    path.segments.last().unwrap().identifier
}

pub fn local_def(id: NodeId) -> DefId {
    ast::DefId { krate: LOCAL_CRATE, node: id }
}

pub fn is_local(did: ast::DefId) -> bool { did.krate == LOCAL_CRATE }

pub fn stmt_id(s: &Stmt) -> NodeId {
    match s.node {
      StmtDecl(_, id) => id,
      StmtExpr(_, id) => id,
      StmtSemi(_, id) => id,
      StmtMac(..) => fail!("attempted to analyze unexpanded stmt")
    }
}

pub fn variant_def_ids(d: Def) -> Option<(DefId, DefId)> {
    match d {
      DefVariant(enum_id, var_id, _) => {
          Some((enum_id, var_id))
      }
      _ => None
    }
}

pub fn def_id_of_def(d: Def) -> DefId {
    match d {
        DefFn(id, _) | DefStaticMethod(id, _, _) | DefMod(id) |
        DefForeignMod(id) | DefStatic(id, _) |
        DefVariant(_, id, _) | DefTy(id) | DefTyParam(id, _) |
        DefUse(id) | DefStruct(id) | DefTrait(id) | DefMethod(id, _) => {
            id
        }
        DefArg(id, _) | DefLocal(id, _) | DefSelfTy(id)
        | DefUpvar(id, _, _, _) | DefBinding(id, _) | DefRegion(id)
        | DefTyParamBinder(id) | DefLabel(id) => {
            local_def(id)
        }

        DefPrimTy(_) => fail!()
    }
}

pub fn binop_to_str(op: BinOp) -> &'static str {
    match op {
        BiAdd => "+",
        BiSub => "-",
        BiMul => "*",
        BiDiv => "/",
        BiRem => "%",
        BiAnd => "&&",
        BiOr => "||",
        BiBitXor => "^",
        BiBitAnd => "&",
        BiBitOr => "|",
        BiShl => "<<",
        BiShr => ">>",
        BiEq => "==",
        BiLt => "<",
        BiLe => "<=",
        BiNe => "!=",
        BiGe => ">=",
        BiGt => ">"
    }
}

pub fn lazy_binop(b: BinOp) -> bool {
    match b {
      BiAnd => true,
      BiOr => true,
      _ => false
    }
}

pub fn is_shift_binop(b: BinOp) -> bool {
    match b {
      BiShl => true,
      BiShr => true,
      _ => false
    }
}

pub fn unop_to_str(op: UnOp) -> &'static str {
    match op {
      UnBox => "@",
      UnUniq => "box() ",
      UnDeref => "*",
      UnNot => "!",
      UnNeg => "-",
    }
}

pub fn is_path(e: @Expr) -> bool {
    return match e.node { ExprPath(_) => true, _ => false };
}

// Get a string representation of a signed int type, with its value.
// We want to avoid "45int" and "-3int" in favor of "45" and "-3"
pub fn int_ty_to_str(t: IntTy, val: Option<i64>) -> StrBuf {
    let s = match t {
        TyI if val.is_some() => "",
        TyI => "int",
        TyI8 => "i8",
        TyI16 => "i16",
        TyI32 => "i32",
        TyI64 => "i64"
    };

    match val {
        Some(n) => format!("{}{}", n, s).to_strbuf(),
        None => s.to_strbuf()
    }
}

pub fn int_ty_max(t: IntTy) -> u64 {
    match t {
        TyI8 => 0x80u64,
        TyI16 => 0x8000u64,
        TyI | TyI32 => 0x80000000u64, // actually ni about TyI
        TyI64 => 0x8000000000000000u64
    }
}

// Get a string representation of an unsigned int type, with its value.
// We want to avoid "42uint" in favor of "42u"
pub fn uint_ty_to_str(t: UintTy, val: Option<u64>) -> StrBuf {
    let s = match t {
        TyU if val.is_some() => "u",
        TyU => "uint",
        TyU8 => "u8",
        TyU16 => "u16",
        TyU32 => "u32",
        TyU64 => "u64"
    };

    match val {
        Some(n) => format!("{}{}", n, s).to_strbuf(),
        None => s.to_strbuf()
    }
}

pub fn uint_ty_max(t: UintTy) -> u64 {
    match t {
        TyU8 => 0xffu64,
        TyU16 => 0xffffu64,
        TyU | TyU32 => 0xffffffffu64, // actually ni about TyU
        TyU64 => 0xffffffffffffffffu64
    }
}

pub fn float_ty_to_str(t: FloatTy) -> StrBuf {
    match t {
        TyF32 => "f32".to_strbuf(),
        TyF64 => "f64".to_strbuf(),
        TyF128 => "f128".to_strbuf(),
    }
}

pub fn is_call_expr(e: @Expr) -> bool {
    match e.node { ExprCall(..) => true, _ => false }
}

pub fn block_from_expr(e: @Expr) -> P<Block> {
    P(Block {
        view_items: Vec::new(),
        stmts: Vec::new(),
        expr: Some(e),
        id: e.id,
        rules: DefaultBlock,
        span: e.span
    })
}

pub fn ident_to_path(s: Span, identifier: Ident) -> Path {
    ast::Path {
        span: s,
        global: false,
        segments: vec!(
            ast::PathSegment {
                identifier: identifier,
                lifetimes: Vec::new(),
                types: OwnedSlice::empty(),
            }
        ),
    }
}

pub fn ident_to_pat(id: NodeId, s: Span, i: Ident) -> @Pat {
    @ast::Pat { id: id,
                node: PatIdent(BindByValue(MutImmutable), ident_to_path(s, i), None),
                span: s }
}

pub fn name_to_dummy_lifetime(name: Name) -> Lifetime {
    Lifetime { id: DUMMY_NODE_ID,
               span: codemap::DUMMY_SP,
               name: name }
}

pub fn is_unguarded(a: &Arm) -> bool {
    match a.guard {
      None => true,
      _    => false
    }
}

pub fn unguarded_pat(a: &Arm) -> Option<Vec<@Pat> > {
    if is_unguarded(a) {
        Some(/* FIXME (#2543) */ a.pats.clone())
    } else {
        None
    }
}

/// Generate a "pretty" name for an `impl` from its type and trait.
/// This is designed so that symbols of `impl`'d methods give some
/// hint of where they came from, (previously they would all just be
/// listed as `__extensions__::method_name::hash`, with no indication
/// of the type).
pub fn impl_pretty_name(trait_ref: &Option<TraitRef>, ty: &Ty) -> Ident {
    let mut pretty = pprust::ty_to_str(ty);
    match *trait_ref {
        Some(ref trait_ref) => {
            pretty.push_char('.');
            pretty.push_str(pprust::path_to_str(&trait_ref.path).as_slice());
        }
        None => {}
    }
    token::gensym_ident(pretty.as_slice())
}

pub fn public_methods(ms: Vec<@Method> ) -> Vec<@Method> {
    ms.move_iter().filter(|m| {
        match m.vis {
            Public => true,
            _   => false
        }
    }).collect()
}

// extract a TypeMethod from a TraitMethod. if the TraitMethod is
// a default, pull out the useful fields to make a TypeMethod
pub fn trait_method_to_ty_method(method: &TraitMethod) -> TypeMethod {
    match *method {
        Required(ref m) => (*m).clone(),
        Provided(ref m) => {
            TypeMethod {
                ident: m.ident,
                attrs: m.attrs.clone(),
                fn_style: m.fn_style,
                decl: m.decl,
                generics: m.generics.clone(),
                explicit_self: m.explicit_self,
                id: m.id,
                span: m.span,
            }
        }
    }
}

pub fn split_trait_methods(trait_methods: &[TraitMethod])
    -> (Vec<TypeMethod> , Vec<@Method> ) {
    let mut reqd = Vec::new();
    let mut provd = Vec::new();
    for trt_method in trait_methods.iter() {
        match *trt_method {
            Required(ref tm) => reqd.push((*tm).clone()),
            Provided(m) => provd.push(m)
        }
    };
    (reqd, provd)
}

pub fn struct_field_visibility(field: ast::StructField) -> Visibility {
    match field.node.kind {
        ast::NamedField(_, v) | ast::UnnamedField(v) => v
    }
}

/// Maps a binary operator to its precedence
pub fn operator_prec(op: ast::BinOp) -> uint {
  match op {
      // 'as' sits here with 12
      BiMul | BiDiv | BiRem     => 11u,
      BiAdd | BiSub             => 10u,
      BiShl | BiShr             =>  9u,
      BiBitAnd                  =>  8u,
      BiBitXor                  =>  7u,
      BiBitOr                   =>  6u,
      BiLt | BiLe | BiGe | BiGt =>  4u,
      BiEq | BiNe               =>  3u,
      BiAnd                     =>  2u,
      BiOr                      =>  1u
  }
}

/// Precedence of the `as` operator, which is a binary operator
/// not appearing in the prior table.
pub static as_prec: uint = 12u;

pub fn empty_generics() -> Generics {
    Generics {lifetimes: Vec::new(),
              ty_params: OwnedSlice::empty()}
}

// ______________________________________________________________________
// Enumerating the IDs which appear in an AST

#[deriving(Encodable, Decodable)]
pub struct IdRange {
    pub min: NodeId,
    pub max: NodeId,
}

impl IdRange {
    pub fn max() -> IdRange {
        IdRange {
            min: u32::MAX,
            max: u32::MIN,
        }
    }

    pub fn empty(&self) -> bool {
        self.min >= self.max
    }

    pub fn add(&mut self, id: NodeId) {
        self.min = cmp::min(self.min, id);
        self.max = cmp::max(self.max, id + 1);
    }
}

pub trait IdVisitingOperation {
    fn visit_id(&self, node_id: NodeId);
}

pub struct IdVisitor<'a, O> {
    pub operation: &'a O,
    pub pass_through_items: bool,
    pub visited_outermost: bool,
}

impl<'a, O: IdVisitingOperation> IdVisitor<'a, O> {
    fn visit_generics_helper(&self, generics: &Generics) {
        for type_parameter in generics.ty_params.iter() {
            self.operation.visit_id(type_parameter.id)
        }
        for lifetime in generics.lifetimes.iter() {
            self.operation.visit_id(lifetime.id)
        }
    }
}

impl<'a, O: IdVisitingOperation> Visitor<()> for IdVisitor<'a, O> {
    fn visit_mod(&mut self,
                 module: &Mod,
                 _: Span,
                 node_id: NodeId,
                 env: ()) {
        self.operation.visit_id(node_id);
        visit::walk_mod(self, module, env)
    }

    fn visit_view_item(&mut self, view_item: &ViewItem, env: ()) {
        if !self.pass_through_items {
            if self.visited_outermost {
                return;
            } else {
                self.visited_outermost = true;
            }
        }
        match view_item.node {
            ViewItemExternCrate(_, _, node_id) => {
                self.operation.visit_id(node_id)
            }
            ViewItemUse(ref view_path) => {
                match view_path.node {
                    ViewPathSimple(_, _, node_id) |
                    ViewPathGlob(_, node_id) => {
                        self.operation.visit_id(node_id)
                    }
                    ViewPathList(_, ref paths, node_id) => {
                        self.operation.visit_id(node_id);
                        for path in paths.iter() {
                            self.operation.visit_id(path.node.id)
                        }
                    }
                }
            }
        }
        visit::walk_view_item(self, view_item, env);
        self.visited_outermost = false;
    }

    fn visit_foreign_item(&mut self, foreign_item: &ForeignItem, env: ()) {
        self.operation.visit_id(foreign_item.id);
        visit::walk_foreign_item(self, foreign_item, env)
    }

    fn visit_item(&mut self, item: &Item, env: ()) {
        if !self.pass_through_items {
            if self.visited_outermost {
                return
            } else {
                self.visited_outermost = true
            }
        }

        self.operation.visit_id(item.id);
        match item.node {
            ItemEnum(ref enum_definition, _) => {
                for variant in enum_definition.variants.iter() {
                    self.operation.visit_id(variant.node.id)
                }
            }
            _ => {}
        }

        visit::walk_item(self, item, env);

        self.visited_outermost = false
    }

    fn visit_local(&mut self, local: &Local, env: ()) {
        self.operation.visit_id(local.id);
        visit::walk_local(self, local, env)
    }

    fn visit_block(&mut self, block: &Block, env: ()) {
        self.operation.visit_id(block.id);
        visit::walk_block(self, block, env)
    }

    fn visit_stmt(&mut self, statement: &Stmt, env: ()) {
        self.operation.visit_id(ast_util::stmt_id(statement));
        visit::walk_stmt(self, statement, env)
    }

    fn visit_pat(&mut self, pattern: &Pat, env: ()) {
        self.operation.visit_id(pattern.id);
        visit::walk_pat(self, pattern, env)
    }

    fn visit_expr(&mut self, expression: &Expr, env: ()) {
        self.operation.visit_id(expression.id);
        visit::walk_expr(self, expression, env)
    }

    fn visit_ty(&mut self, typ: &Ty, env: ()) {
        self.operation.visit_id(typ.id);
        match typ.node {
            TyPath(_, _, id) => self.operation.visit_id(id),
            _ => {}
        }
        visit::walk_ty(self, typ, env)
    }

    fn visit_generics(&mut self, generics: &Generics, env: ()) {
        self.visit_generics_helper(generics);
        visit::walk_generics(self, generics, env)
    }

    fn visit_fn(&mut self,
                function_kind: &visit::FnKind,
                function_declaration: &FnDecl,
                block: &Block,
                span: Span,
                node_id: NodeId,
                env: ()) {
        if !self.pass_through_items {
            match *function_kind {
                visit::FkMethod(..) if self.visited_outermost => return,
                visit::FkMethod(..) => self.visited_outermost = true,
                _ => {}
            }
        }

        self.operation.visit_id(node_id);

        match *function_kind {
            visit::FkItemFn(_, generics, _, _) |
            visit::FkMethod(_, generics, _) => {
                self.visit_generics_helper(generics)
            }
            visit::FkFnBlock => {}
        }

        for argument in function_declaration.inputs.iter() {
            self.operation.visit_id(argument.id)
        }

        visit::walk_fn(self,
                        function_kind,
                        function_declaration,
                        block,
                        span,
                        node_id,
                        env);

        if !self.pass_through_items {
            match *function_kind {
                visit::FkMethod(..) => self.visited_outermost = false,
                _ => {}
            }
        }
    }

    fn visit_struct_field(&mut self, struct_field: &StructField, env: ()) {
        self.operation.visit_id(struct_field.node.id);
        visit::walk_struct_field(self, struct_field, env)
    }

    fn visit_struct_def(&mut self,
                        struct_def: &StructDef,
                        ident: ast::Ident,
                        generics: &ast::Generics,
                        id: NodeId,
                        _: ()) {
        self.operation.visit_id(id);
        struct_def.ctor_id.map(|ctor_id| self.operation.visit_id(ctor_id));
        visit::walk_struct_def(self, struct_def, ident, generics, id, ());
    }

    fn visit_trait_method(&mut self, tm: &ast::TraitMethod, _: ()) {
        match *tm {
            ast::Required(ref m) => self.operation.visit_id(m.id),
            ast::Provided(ref m) => self.operation.visit_id(m.id),
        }
        visit::walk_trait_method(self, tm, ());
    }
}

pub fn visit_ids_for_inlined_item<O: IdVisitingOperation>(item: &InlinedItem,
                                                          operation: &O) {
    let mut id_visitor = IdVisitor {
        operation: operation,
        pass_through_items: true,
        visited_outermost: false,
    };

    visit::walk_inlined_item(&mut id_visitor, item, ());
}

struct IdRangeComputingVisitor {
    result: Cell<IdRange>,
}

impl IdVisitingOperation for IdRangeComputingVisitor {
    fn visit_id(&self, id: NodeId) {
        let mut id_range = self.result.get();
        id_range.add(id);
        self.result.set(id_range)
    }
}

pub fn compute_id_range_for_inlined_item(item: &InlinedItem) -> IdRange {
    let visitor = IdRangeComputingVisitor {
        result: Cell::new(IdRange::max())
    };
    visit_ids_for_inlined_item(item, &visitor);
    visitor.result.get()
}

pub fn compute_id_range_for_fn_body(fk: &visit::FnKind,
                                    decl: &FnDecl,
                                    body: &Block,
                                    sp: Span,
                                    id: NodeId)
                                    -> IdRange
{
    /*!
     * Computes the id range for a single fn body,
     * ignoring nested items.
     */

    let visitor = IdRangeComputingVisitor {
        result: Cell::new(IdRange::max())
    };
    let mut id_visitor = IdVisitor {
        operation: &visitor,
        pass_through_items: false,
        visited_outermost: false,
    };
    id_visitor.visit_fn(fk, decl, body, sp, id, ());
    visitor.result.get()
}

pub fn is_item_impl(item: @ast::Item) -> bool {
    match item.node {
        ItemImpl(..) => true,
        _            => false
    }
}

pub fn walk_pat(pat: &Pat, it: |&Pat| -> bool) -> bool {
    if !it(pat) {
        return false;
    }

    match pat.node {
        PatIdent(_, _, Some(p)) => walk_pat(p, it),
        PatStruct(_, ref fields, _) => {
            fields.iter().advance(|f| walk_pat(f.pat, |p| it(p)))
        }
        PatEnum(_, Some(ref s)) | PatTup(ref s) => {
            s.iter().advance(|&p| walk_pat(p, |p| it(p)))
        }
        PatUniq(s) | PatRegion(s) => {
            walk_pat(s, it)
        }
        PatVec(ref before, ref slice, ref after) => {
            before.iter().advance(|&p| walk_pat(p, |p| it(p))) &&
                slice.iter().advance(|&p| walk_pat(p, |p| it(p))) &&
                after.iter().advance(|&p| walk_pat(p, |p| it(p)))
        }
        PatWild | PatWildMulti | PatLit(_) | PatRange(_, _) | PatIdent(_, _, _) |
        PatEnum(_, _) => {
            true
        }
    }
}

pub trait EachViewItem {
    fn each_view_item(&self, f: |&ast::ViewItem| -> bool) -> bool;
}

struct EachViewItemData<'a> {
    callback: |&ast::ViewItem|: 'a -> bool,
}

impl<'a> Visitor<()> for EachViewItemData<'a> {
    fn visit_view_item(&mut self, view_item: &ast::ViewItem, _: ()) {
        let _ = (self.callback)(view_item);
    }
}

impl EachViewItem for ast::Crate {
    fn each_view_item(&self, f: |&ast::ViewItem| -> bool) -> bool {
        let mut visit = EachViewItemData {
            callback: f,
        };
        visit::walk_crate(&mut visit, self, ());
        true
    }
}

pub fn view_path_id(p: &ViewPath) -> NodeId {
    match p.node {
        ViewPathSimple(_, _, id) | ViewPathGlob(_, id)
        | ViewPathList(_, _, id) => id
    }
}

/// Returns true if the given struct def is tuple-like; i.e. that its fields
/// are unnamed.
pub fn struct_def_is_tuple_like(struct_def: &ast::StructDef) -> bool {
    struct_def.ctor_id.is_some()
}

/// Returns true if the given pattern consists solely of an identifier
/// and false otherwise.
pub fn pat_is_ident(pat: @ast::Pat) -> bool {
    match pat.node {
        ast::PatIdent(..) => true,
        _ => false,
    }
}

// are two paths equal when compared unhygienically?
// since I'm using this to replace ==, it seems appropriate
// to compare the span, global, etc. fields as well.
pub fn path_name_eq(a : &ast::Path, b : &ast::Path) -> bool {
    (a.span == b.span)
    && (a.global == b.global)
    && (segments_name_eq(a.segments.as_slice(), b.segments.as_slice()))
}

// are two arrays of segments equal when compared unhygienically?
pub fn segments_name_eq(a : &[ast::PathSegment], b : &[ast::PathSegment]) -> bool {
    if a.len() != b.len() {
        false
    } else {
        for (idx,seg) in a.iter().enumerate() {
            if (seg.identifier.name != b[idx].identifier.name)
                // FIXME #7743: ident -> name problems in lifetime comparison?
                || (seg.lifetimes != b[idx].lifetimes)
                // can types contain idents?
                || (seg.types != b[idx].types) {
                return false;
            }
        }
        true
    }
}

// Returns true if this literal is a string and false otherwise.
pub fn lit_is_str(lit: @Lit) -> bool {
    match lit.node {
        LitStr(..) => true,
        _ => false,
    }
}

pub fn get_inner_tys(ty: P<Ty>) -> Vec<P<Ty>> {
    match ty.node {
        ast::TyRptr(_, mut_ty) | ast::TyPtr(mut_ty) => {
            vec!(mut_ty.ty)
        }
        ast::TyBox(ty)
        | ast::TyVec(ty)
        | ast::TyUniq(ty)
        | ast::TyFixedLengthVec(ty, _) => vec!(ty),
        ast::TyTup(ref tys) => tys.clone(),
        _ => Vec::new()
    }
}


#[cfg(test)]
mod test {
    use ast::*;
    use super::*;
    use owned_slice::OwnedSlice;

    fn ident_to_segment(id : &Ident) -> PathSegment {
        PathSegment {identifier:id.clone(),
                     lifetimes: Vec::new(),
                     types: OwnedSlice::empty()}
    }

    #[test] fn idents_name_eq_test() {
        assert!(segments_name_eq(
            [Ident{name:3,ctxt:4}, Ident{name:78,ctxt:82}]
                .iter().map(ident_to_segment).collect::<Vec<PathSegment>>().as_slice(),
            [Ident{name:3,ctxt:104}, Ident{name:78,ctxt:182}]
                .iter().map(ident_to_segment).collect::<Vec<PathSegment>>().as_slice()));
        assert!(!segments_name_eq(
            [Ident{name:3,ctxt:4}, Ident{name:78,ctxt:82}]
                .iter().map(ident_to_segment).collect::<Vec<PathSegment>>().as_slice(),
            [Ident{name:3,ctxt:104}, Ident{name:77,ctxt:182}]
                .iter().map(ident_to_segment).collect::<Vec<PathSegment>>().as_slice()));
    }
}
