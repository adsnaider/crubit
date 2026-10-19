// Part of the Crubit project, under the Apache License v2.0 with LLVM
// Exceptions. See /LICENSE for license information.
// SPDX-License-Identifier: Apache-2.0 WITH LLVM-exception

//! Generates a zngur (https://github.com/HKalbasi/zngur) `.zng` file for a Rust crate.
//!
//! With `--zng-out`, crubit is only a front end: it analyzes the crate with rustc, and describes
//! its API in a `.zng` file. zngur then generates the C++ bindings (and the Rust glue) from that
//! file, which a developer can amend by hand, or extend with `merge`d files.
//!
//! The emitter decides what to include on its own, from rustc's view of the crate and crubit's
//! analysis queries (public paths, layouts, `#[do_not_bind]`, ...), but not crubit's code
//! generators. Items that can't be expressed in the `.zng` file (yet) are listed in `// skipped:`
//! comments, with the reason.
//!
//! Items are named by their absolute Rust paths (`::my_crate::Foo`), so the zngur bindings live in
//! a separate crate that depends on `my_crate`, and C++ code refers to them as
//! `rust::my_crate::Foo`. Types of dependency crates come from those crates' `.zng` files, which
//! the file `import`s.

#![feature(rustc_private)]
#![deny(rustc::internal)]

extern crate rustc_abi;
extern crate rustc_hir;
extern crate rustc_middle;
extern crate rustc_span;

use arc_anyhow::{anyhow, bail, Result};
use code_gen_utils::unkeyword_cpp_ident;
use database::BindingsGenerator;
use generate_bindings::{field_def_is_pub_and_stable, normalize_ty, should_receive_bindings};
use query_compiler::{
    does_type_implement_trait, get_layout, has_non_lifetime_generics, is_copy,
    liberate_and_deanonymize_late_bound_regions,
};
use rustc_abi::Layout;
use rustc_hir::def::DefKind;
use rustc_middle::ty::{self, Ty};
use rustc_span::def_id::DefId;
use rustc_span::sym;
use std::borrow::Cow;
use std::collections::{HashMap, HashSet};
use std::fmt::Write as _;
use std::path::PathBuf;
use zngur_def::printing::IDLPrinter;
use zngur_def::{
    LayoutPolicy, Lifetime, ModuleImport, Mutability, PrimitiveRustType, RustPathAndGenerics,
    RustType, TriviallyRelocatable, ZngurField, ZngurFn, ZngurMethod, ZngurMethodDetails,
    ZngurMethodReceiver, ZngurSpec, ZngurType, ZngurWellknownTrait,
};

/// The `.zng` file of a dependency crate, which the generated file imports.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ZngImport {
    /// The name of the dependency crate.
    pub crate_name: String,
    /// The path of the `.zng` file, for crubit to read which types it declares.
    pub zng_file: PathBuf,
    /// The path to write in the `import` statement (relative to the generated file).
    pub import_path: String,
}

/// Options of the generated `.zng` file.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ZngOptions {
    /// The `.zng` files of dependency crates.
    pub imports: Vec<ZngImport>,
    /// Hand-written `.zng` files for the generated file to `merge` (paths relative to it).
    pub merges: Vec<String>,
}

/// An item that crubit could bind, but that isn't in the `.zng` file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkippedItem {
    /// The Rust path of the item, e.g. `::my_crate::Bar` or `::my_crate::Bar::method`.
    pub path: String,
    /// Why the item was skipped.
    pub reason: String,
}

/// The result of [`generate_zng_spec`].
pub struct ZngOutput {
    pub spec: ZngurSpec,
    pub skipped: Vec<SkippedItem>,
}

/// Generates the contents of the `.zng` file for the crate that `db` generates bindings for.
pub fn generate_zng(db: &BindingsGenerator, options: &ZngOptions) -> Result<String> {
    let ZngOutput { spec, skipped } = generate_zng_spec(db, options)?;
    let tcx = db.tcx();

    // Writing to a `String` can't fail, so the `writeln!` results are ignored below.
    let mut out = String::new();
    let _ = writeln!(out, "// Automatically @generated zngur IDL for the following Rust crate:");
    let _ = writeln!(out, "// {}", tcx.crate_name(db.source_crate_num()));
    if !db.is_golden_test() {
        // Layouts are only valid for the compiler and target that computed them. zngur checks
        // them at compile time, but the stamp makes a mismatch easier to diagnose.
        let _ = writeln!(out, "// rustc: {}", tcx.sess.cfg_version);
        let _ = writeln!(out, "// target: {}", tcx.sess.target.llvm_target);
    }
    if !skipped.is_empty() {
        let _ = writeln!(out, "//");
        let _ = writeln!(out, "// Items that are not (yet) supported in this file:");
        for SkippedItem { path, reason } in &skipped {
            let reason = reason.split_whitespace().collect::<Vec<_>>().join(" ");
            let _ = writeln!(out, "// skipped: {path}: {reason}");
        }
    }
    let mut idl = Vec::<u8>::new();
    IDLPrinter::new(&mut idl)
        .write_idl(&spec)
        .map_err(|err| anyhow!("Failed to print the .zng file: {err}"))?;
    let idl =
        std::str::from_utf8(&idl).map_err(|err| anyhow!("The .zng file is not UTF-8: {err}"))?;
    if !idl.is_empty() {
        let _ = writeln!(out);
        out.push_str(idl);
    }
    if !options.merges.is_empty() {
        let _ = writeln!(out);
        let _ = writeln!(out, "// Hand-written additions.");
        for merge in &options.merges {
            let _ = writeln!(out, "merge \"{merge}\";");
        }
    }
    Ok(out)
}

/// Builds the `ZngurSpec` for the crate that `db` generates bindings for.
///
/// Items are visited in order of their Rust path, so the output is deterministic.
pub fn generate_zng_spec(db: &BindingsGenerator, options: &ZngOptions) -> Result<ZngOutput> {
    let mut emitter = Emitter {
        db,
        declared: HashMap::new(),
        imported_types: HashMap::new(),
        imported_builtins: HashSet::new(),
        skipped: vec![],
        uses_bool: false,
        uses_char: false,
        uses_str: false,
        fn_lifetimes: vec![],
    };
    let mut spec = ZngurSpec::default();
    for import in &options.imports {
        emitter.read_import(import)?;
        spec.imported_modules.push(ModuleImport {
            path: import.import_path.clone().into(),
            crate_name: None,
            cpp_types: vec![],
            cpp_by_value_types: vec![],
            cpp_template_homes: vec![],
        });
    }
    emitter.add_items(&mut spec);
    let mut skipped = emitter.skipped;
    // Items are skipped at different stages (e.g. types before functions), so sort the list to
    // make it easy to scan. The sort is stable, so entries for the same path keep their order.
    skipped.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(ZngOutput { spec, skipped })
}

struct Emitter<'a, 'tcx> {
    db: &'a BindingsGenerator<'tcx>,
    /// ADTs of the source crate that are declared in the `.zng` file, and their paths.
    declared: HashMap<DefId, Vec<String>>,
    /// Crate name -> the Rust types that its (imported) `.zng` file declares.
    imported_types: HashMap<String, HashSet<RustType>>,
    /// The builtin types (`bool`, `char`, `str`) that imported files declare.
    imported_builtins: HashSet<PrimitiveRustType>,
    skipped: Vec<SkippedItem>,
    /// Whether `bool` is used, and so needs to be declared.
    uses_bool: bool,
    /// Whether `char` is used, and so needs to be declared.
    uses_char: bool,
    /// Whether `str` is used, and so needs to be declared.
    uses_str: bool,
    /// The lifetimes of the function being described, in order of first use.
    fn_lifetimes: Vec<Lifetime>,
}

impl<'a, 'tcx> Emitter<'a, 'tcx> {
    fn skip(&mut self, path: String, reason: impl std::fmt::Display) {
        self.skipped.push(SkippedItem { path, reason: reason.to_string() });
    }

    /// Records the types that an imported `.zng` file declares.
    fn read_import(&mut self, import: &ZngImport) -> Result<()> {
        let mut errors = String::new();
        let parsed = zngur_parser::ParsedZngFile::parse(
            &import.zng_file,
            zngur_parser::cfg::NullCfg,
            &mut report_sink(|fatal, report, sources| {
                if fatal {
                    let mut buf = vec![];
                    let _ = report.write_for_stdout(sources, &mut buf);
                    errors.push_str(&String::from_utf8_lossy(&buf));
                }
            }),
        );
        if parsed.errors > 0 {
            bail!("Failed to parse `{}`:\n{errors}", import.zng_file.display());
        }
        let types = self.imported_types.entry(import.crate_name.clone()).or_default();
        for ty in parsed.spec.types {
            if let RustType::Primitive(p) = &ty.ty {
                self.imported_builtins.insert(p.clone());
            }
            types.insert(ty.ty);
        }
        Ok(())
    }

    /// Returns the C++ name that crubit uses for the Rust identifier `name`: the same name,
    /// unless it is reserved in C++ (e.g. `operator`), in which case `_` is appended.
    fn cpp_name<'n>(&self, name: &'n str) -> Cow<'n, str> {
        unkeyword_cpp_ident(name, self.db.crate_features(self.db.source_crate_num()))
    }

    /// The absolute Rust path of an item (`["my_crate", "module", "Item"]`), or an error if it
    /// can't be named.
    fn item_path(&self, def_id: DefId) -> Result<Vec<String>> {
        let name = self.db.symbol_canonical_name(def_id)?;
        if !name.rs_mod_path.nested_records.is_empty() {
            bail!("items nested in types are not supported yet");
        }
        Ok(std::iter::once(name.krate.to_string())
            .chain(name.rs_mod_path.namespaces.iter().map(|ns| ns.to_string()))
            .chain([name.unqualified.rs_name.to_string()])
            .collect())
    }

    fn add_items(&mut self, spec: &mut ZngurSpec) {
        let db = self.db;
        let tcx = db.tcx();
        let source_crate = db.source_crate_num();

        let mut items: Vec<(Vec<String>, DefId)> = db
            .public_paths_by_def_id(source_crate)
            .into_keys()
            .filter_map(|def_id| {
                let name = db.symbol_canonical_name(def_id).ok()?;
                // Re-exports of items from other crates are bound by those crates.
                if name.krate_num != source_crate {
                    return None;
                }
                Some((self.item_path(def_id).ok()?, def_id))
            })
            .collect();
        items.sort_by_cached_key(|(path, def_id)| (path.clone(), tcx.def_path_str(*def_id)));

        // First, find the ADTs, so that signatures can refer to any of them regardless of order.
        let mut adts: Vec<(Vec<String>, DefId)> = vec![];
        let mut fns: Vec<(Vec<String>, DefId)> = vec![];
        for (path, def_id) in items {
            let display_path = display_path(&path);
            if !matches!(tcx.def_kind(def_id), DefKind::Struct | DefKind::Enum | DefKind::Fn) {
                // Modules, traits, constants, etc. Only modules are silently ignored.
                if tcx.def_kind(def_id) != DefKind::Mod {
                    let descr = tcx.def_kind(def_id).descr(def_id);
                    self.skip(display_path, format!("{descr} items are not supported yet"));
                }
                continue;
            }
            if let Err(err) = should_receive_bindings(db, def_id) {
                self.skip(display_path, format!("{err:#}"));
                continue;
            }
            // Crubit renames C++-reserved identifiers, but zngur can only rename methods.
            if let Some(segment) = path[1..].iter().find(|s| self.cpp_name(s) != s.as_str()) {
                let reason = format!("`{segment}` is reserved in C++, and zngur can't rename it");
                self.skip(display_path, reason);
                continue;
            }
            // zngur can't spell raw identifiers (`r#move`).
            if let Some(segment) = path.iter().find(|s| is_rust_keyword(s)) {
                let reason = format!("`{segment}` is a Rust keyword, which zngur can't spell");
                self.skip(display_path, reason);
                continue;
            }
            match tcx.def_kind(def_id) {
                DefKind::Struct | DefKind::Enum => {
                    if has_non_lifetime_generics(tcx, def_id) {
                        self.skip(display_path, "generic types are not supported yet");
                        continue;
                    }
                    // zngur can't spell lifetimes, e.g. in the types of fields.
                    if !tcx.generics_of(def_id).own_params.is_empty() {
                        self.skip(
                            display_path,
                            "types with lifetime parameters are not supported yet",
                        );
                        continue;
                    }
                    match self.adt_layout(def_id) {
                        Ok(_) => {
                            self.declared.insert(def_id, path.clone());
                            adts.push((path, def_id));
                        }
                        Err(err) => self.skip(display_path, format!("{err:#}")),
                    }
                }
                _ => fns.push((path, def_id)),
            }
        }

        for (path, def_id) in &adts {
            let ty = self.zngur_type(path, *def_id);
            spec.types.push(ty);
        }
        // Keep each module's functions together, so that they're printed in a single `mod` block.
        // The sort is stable, so ties keep the `def_path_str` order from above.
        fns.sort_by(|(a, _), (b, _)| {
            let a = a.split_last().map(|(name, module)| (module, name));
            let b = b.split_last().map(|(name, module)| (module, name));
            a.cmp(&b)
        });
        for (path, def_id) in fns {
            match self.zngur_fn(&path, def_id) {
                Ok(func) => spec.funcs.push(func),
                Err(err) => self.skip(display_path(&path), format!("{err:#}")),
            }
        }

        // Unlike the other primitives, `bool`, `char` and `str` are zngur types that must be
        // declared, once: if an imported file declares one, it's declared there.
        let mut builtins = vec![];
        let primitive_decl = |primitive, size| ZngurType {
            ty: RustType::Primitive(primitive),
            layout: Some(LayoutPolicy::StackAllocated { size, align: size }),
            wellknown_traits: vec![ZngurWellknownTrait::Copy],
            ..empty_zngur_type()
        };
        if self.uses_bool && !self.imported_builtins.contains(&PrimitiveRustType::Bool) {
            builtins.push(primitive_decl(PrimitiveRustType::Bool, 1));
        }
        if self.uses_char && !self.imported_builtins.contains(&PrimitiveRustType::Char) {
            builtins.push(primitive_decl(PrimitiveRustType::Char, 4));
        }
        if self.uses_str && !self.imported_builtins.contains(&PrimitiveRustType::Str) {
            builtins.push(str_decl());
        }
        spec.types.splice(0..0, builtins);
    }

    /// The layout of a (non-generic) ADT, or an error if zngur can't hold it by value.
    fn adt_layout(&self, def_id: DefId) -> Result<Layout<'tcx>> {
        let tcx = self.db.tcx();
        let layout = get_layout(tcx, type_of(tcx, def_id))?;
        if layout.is_unsized() {
            bail!("unsized types are not supported yet");
        }
        if layout.is_uninhabited() {
            bail!("uninhabited types can't be used");
        }
        Ok(layout)
    }

    fn zngur_type(&mut self, path: &[String], def_id: DefId) -> ZngurType {
        let tcx = self.db.tcx();
        let self_ty = type_of(tcx, def_id);
        let layout = self.adt_layout(def_id).expect("checked when the type was declared");

        let is_copy = is_copy(tcx, self_ty);
        // zngur adds `Drop` to every type that isn't `Copy` (or unsized), so do the same here
        // to keep the spec identical to what zngur would parse from the printed file.
        let mut wellknown_traits =
            vec![if is_copy { ZngurWellknownTrait::Copy } else { ZngurWellknownTrait::Drop }];
        if let Some(debug) = tcx.get_diagnostic_item(sym::Debug)
            && does_type_implement_trait(tcx, self_ty, debug, [])
        {
            wellknown_traits.push(ZngurWellknownTrait::Debug);
        }

        let fields = match *self_ty.kind() {
            ty::Adt(adt_def, args) if adt_def.is_struct() => {
                let mut fields = vec![];
                for (index, field) in adt_def.non_enum_variant().fields.iter().enumerate() {
                    // Private fields aren't part of the API.
                    if field_def_is_pub_and_stable(tcx, field).is_err() {
                        continue;
                    }
                    let field_name = field.name.to_string();
                    let field_path = format!("{}::{field_name}", display_path(path));
                    if self.cpp_name(&field_name) != field_name.as_str() {
                        self.skip(
                            field_path,
                            "field: the name is reserved in C++, and zngur can't rename fields",
                        );
                        continue;
                    }
                    let field_ty = normalize_ty(tcx, tcx.param_env(field.did), field.ty(tcx, args));
                    // zngur checks the field's type, which would need a lifetime (`&'static T`).
                    if field_ty.is_ref() {
                        self.skip(
                            field_path,
                            "field: fields of reference type are not supported yet",
                        );
                        continue;
                    }
                    match self.zngur_ty(field_ty) {
                        Ok(ty) => fields.push(ZngurField {
                            name: field_name,
                            ty,
                            offset: Some(layout.fields().offset(index).bytes() as usize),
                        }),
                        Err(err) => self.skip(field_path, format!("field: {err:#}")),
                    }
                }
                fields
            }
            _ => vec![],
        };

        let ty = RustType::Adt(rust_path(path));
        let mut methods = self.methods(path, def_id, self_ty);
        self.trait_methods(&ty, self_ty, is_copy, &mut methods);
        // In C++, fields and member functions share a namespace, and zngur can only rename the
        // member functions: append `_` to their names until they're unique (like crubit does
        // for fields).
        let cpp_name = |m: &ZngurMethodDetails| m.cpp_name.clone().unwrap_or(m.data.name.clone());
        for index in 0..methods.len() {
            let mut name = cpp_name(&methods[index]);
            if !fields.iter().any(|f| f.name == name) {
                continue;
            }
            while fields.iter().any(|f| f.name == name)
                || methods.iter().enumerate().any(|(i, m)| i != index && cpp_name(m) == name)
            {
                name.push('_');
            }
            methods[index].cpp_name = Some(name);
        }

        ZngurType {
            ty,
            layout: Some(LayoutPolicy::StackAllocated {
                size: layout.size().bytes() as usize,
                align: layout.align().abi.bytes() as usize,
            }),
            wellknown_traits,
            fields,
            methods,
            ..empty_zngur_type()
        }
    }

    /// Returns the inherent methods of `self_ty` that zngur can bind.
    fn methods(
        &mut self,
        type_path: &[String],
        def_id: DefId,
        self_ty: Ty<'tcx>,
    ) -> Vec<ZngurMethodDetails> {
        let db = self.db;
        let tcx = db.tcx();
        let mut methods = vec![];
        for impl_id in tcx.inherent_impls(def_id).iter().copied() {
            for item in tcx.associated_items(impl_id).in_definition_order() {
                if !matches!(item.kind, ty::AssocKind::Fn { .. }) {
                    continue;
                }
                if !tcx.visibility(item.def_id).is_public()
                    || !tcx.lookup_stability(item.def_id).is_none_or(|s| s.is_stable())
                {
                    continue;
                }
                let path = format!("{}::{}", display_path(type_path), tcx.item_name(item.def_id));
                if let Err(err) = should_receive_bindings(db, item.def_id) {
                    self.skip(path, format!("{err:#}"));
                    continue;
                }
                match self.zngur_method(item, self_ty) {
                    Ok(method) => {
                        let cpp_name = self.cpp_name(&method.name);
                        let cpp_name =
                            (cpp_name != method.name.as_str()).then(|| cpp_name.into_owned());
                        methods.push(ZngurMethodDetails {
                            data: method,
                            use_path: None,
                            deref: None,
                            cpp_name,
                        })
                    }
                    Err(err) => self.skip(path, format!("{err:#}")),
                }
            }
        }
        methods
    }

    /// Adds `clone` (for types that are `Clone` but not `Copy`) and `default` methods, which call
    /// the trait implementations (zngur brings the `use` path, the trait, into scope).
    fn trait_methods(
        &self,
        ty: &RustType,
        self_ty: Ty<'tcx>,
        is_copy: bool,
        methods: &mut Vec<ZngurMethodDetails>,
    ) {
        let tcx = self.db.tcx();
        let mut add = |name: &str, receiver, use_path: &str| {
            // An inherent method with the same name takes precedence.
            if methods.iter().any(|m| m.data.name == name) {
                return;
            }
            methods.push(ZngurMethodDetails {
                data: ZngurMethod {
                    name: name.to_string(),
                    lifetimes: vec![],
                    generics: vec![],
                    receiver,
                    receiver_lifetime: None,
                    inputs: vec![],
                    output: ty.clone(),
                    is_safe: true,
                },
                use_path: Some(use_path.to_string()),
                deref: None,
                cpp_name: None,
            });
        };
        if !is_copy
            && let Some(clone) = tcx.lang_items().clone_trait()
            && does_type_implement_trait(tcx, self_ty, clone, [])
        {
            add("clone", ZngurMethodReceiver::Ref(Mutability::Not), "::core::clone::Clone");
        }
        if let Some(default) = tcx.get_diagnostic_item(sym::Default)
            && does_type_implement_trait(tcx, self_ty, default, [])
        {
            add("default", ZngurMethodReceiver::Static, "::core::default::Default");
        }
    }

    fn zngur_method(&mut self, item: &ty::AssocItem, self_ty: Ty<'tcx>) -> Result<ZngurMethod> {
        let tcx = self.db.tcx();
        let sig = self.fn_sig(item.def_id)?;
        self.begin_fn();
        let mut inputs = sig.inputs().iter().copied();
        let (receiver, receiver_lifetime) = if item.is_method() {
            let self_param = inputs.next().expect("methods have a `self` parameter");
            match *self_param.kind() {
                _ if self_param == self_ty => (ZngurMethodReceiver::Move, None),
                ty::Ref(region, pointee, mutbl) if pointee == self_ty => (
                    ZngurMethodReceiver::Ref(mutability(mutbl.is_mut())),
                    Some(self.lifetime(region)?),
                ),
                _ => bail!("`self: {self_param}` receivers are not supported yet"),
            }
        } else {
            (ZngurMethodReceiver::Static, None)
        };
        let inputs = inputs.map(|ty| self.zngur_ty(ty)).collect::<Result<Vec<_>>>()?;
        let output = self.zngur_ty(sig.output())?;
        Ok(ZngurMethod {
            name: tcx.item_name(item.def_id).to_string(),
            lifetimes: std::mem::take(&mut self.fn_lifetimes),
            generics: vec![],
            receiver,
            receiver_lifetime,
            inputs,
            output,
            // zngur's parser sets this for every Rust method; it only matters for C++ methods.
            is_safe: true,
        })
    }

    fn zngur_fn(&mut self, path: &[String], def_id: DefId) -> Result<ZngurFn> {
        let sig = self.fn_sig(def_id)?;
        self.begin_fn();
        let inputs =
            sig.inputs().iter().map(|ty| self.zngur_ty(*ty)).collect::<Result<Vec<_>>>()?;
        let output = self.zngur_ty(sig.output())?;
        Ok(ZngurFn {
            path: rust_path(path),
            lifetimes: std::mem::take(&mut self.fn_lifetimes),
            inputs,
            output,
        })
    }

    /// Starts describing a function.
    fn begin_fn(&mut self) {
        self.fn_lifetimes.clear();
    }

    /// The lifetime of a reference in a function's signature, which it adds to the function's
    /// lifetimes. Anonymous lifetimes are named by `liberate_and_deanonymize_late_bound_regions`
    /// (`'__anon1`, ...).
    fn lifetime(&mut self, region: ty::Region<'tcx>) -> Result<Lifetime> {
        let name = region
            .get_name(self.db.tcx())
            .ok_or_else(|| anyhow!("lifetime `{region}` is not supported"))?;
        let name = name.as_str();
        let name = name.strip_prefix('\'').unwrap_or(name).to_string();
        let lifetime = Lifetime(name);
        if !lifetime.is_builtin() && !self.fn_lifetimes.contains(&lifetime) {
            self.fn_lifetimes.push(lifetime.clone());
        }
        Ok(lifetime)
    }

    /// Returns the signature of a function, or an error if zngur can't express the function.
    fn fn_sig(&self, def_id: DefId) -> Result<ty::FnSig<'tcx>> {
        let tcx = self.db.tcx();
        if has_non_lifetime_generics(tcx, def_id) {
            bail!("generic functions are not supported yet");
        }
        let sig = liberate_and_deanonymize_late_bound_regions(
            tcx,
            tcx.fn_sig(def_id).instantiate_identity(),
            def_id,
        );
        if sig.safety() != rustc_hir::Safety::Safe {
            bail!("unsafe functions are not supported yet");
        }
        Ok(sig)
    }

    /// Converts a Rust type to a zngur type. Fails if the type can't be spelled in the `.zng`
    /// file, or isn't declared in it (or in an imported file).
    fn zngur_ty(&mut self, ty: Ty<'tcx>) -> Result<RustType> {
        use PrimitiveRustType::*;
        let primitive = |p| Ok(RustType::Primitive(p));
        match *ty.kind() {
            ty::Bool => {
                self.uses_bool = true;
                primitive(Bool)
            }
            ty::Char => {
                self.uses_char = true;
                primitive(Char)
            }
            ty::Str => {
                self.uses_str = true;
                primitive(Str)
            }
            ty::Int(int_ty) => match int_ty {
                ty::IntTy::I8 => primitive(Int(8)),
                ty::IntTy::I16 => primitive(Int(16)),
                ty::IntTy::I32 => primitive(Int(32)),
                ty::IntTy::I64 => primitive(Int(64)),
                ty::IntTy::I128 | ty::IntTy::Isize => bail!("`{ty}` is not supported by zngur"),
            },
            ty::Uint(uint_ty) => match uint_ty {
                ty::UintTy::U8 => primitive(Uint(8)),
                ty::UintTy::U16 => primitive(Uint(16)),
                ty::UintTy::U32 => primitive(Uint(32)),
                ty::UintTy::U64 => primitive(Uint(64)),
                ty::UintTy::Usize => primitive(Usize),
                ty::UintTy::U128 => bail!("`{ty}` is not supported by zngur"),
            },
            ty::Float(float_ty) => match float_ty {
                ty::FloatTy::F32 => primitive(Float(32)),
                ty::FloatTy::F64 => primitive(Float(64)),
                _ => bail!("`{ty}` is not supported by zngur"),
            },
            ty::Tuple(tys) if tys.is_empty() => Ok(RustType::UNIT),
            // `str` is only usable behind a reference.
            ty::Ref(region, pointee, mutbl) => {
                let lifetime = self.lifetime(region)?;
                Ok(RustType::Ref(
                    mutability(mutbl.is_mut()),
                    Some(lifetime),
                    Box::new(self.zngur_ty(pointee)?),
                ))
            }
            ty::RawPtr(pointee, mutbl) => {
                Ok(RustType::Raw(mutability(mutbl.is_mut()), Box::new(self.zngur_ty(pointee)?)))
            }
            ty::Adt(adt_def, args) if args.is_empty() => {
                if let Some(path) = self.declared.get(&adt_def.did()) {
                    return Ok(RustType::Adt(rust_path(path)));
                }
                let krate = adt_def.did().krate;
                if krate == self.db.source_crate_num() {
                    bail!("type `{ty}` isn't in this file (see why it's skipped)");
                }
                let crate_name = self.db.tcx().crate_name(krate).to_string();
                let Some(types) = self.imported_types.get(&crate_name) else {
                    bail!("type `{ty}` is from `{crate_name}`, whose `.zng` file isn't imported");
                };
                let path = self
                    .item_path(adt_def.did())
                    .map_err(|err| anyhow!("type `{ty}` can't be named: {err:#}"))?;
                let zngur_ty = RustType::Adt(rust_path(&path));
                if !types.contains(&zngur_ty) {
                    bail!("type `{ty}` isn't in the `.zng` file of `{crate_name}`");
                }
                Ok(zngur_ty)
            }
            ty::Adt(..) => bail!("generic type `{ty}` is not supported yet"),
            _ => bail!("type `{ty}` is not supported yet"),
        }
    }
}

/// Whether `name` is a Rust keyword (in any edition), which can only be used as a raw identifier.
fn is_rust_keyword(name: &str) -> bool {
    rustc_span::Symbol::intern(name).is_reserved(|| rustc_span::edition::Edition::Edition2024)
}

/// The type of a (non-generic) ADT.
fn type_of<'tcx>(tcx: ty::TyCtxt<'tcx>, def_id: DefId) -> Ty<'tcx> {
    normalize_ty(tcx, tcx.param_env(def_id), tcx.type_of(def_id).instantiate_identity())
}

/// Gives a closure the signature of a zngur `ReportSink`.
fn report_sink<F>(f: F) -> F
where
    F: for<'r, 'c> FnMut(bool, &zngur_parser::ParseReport<'r>, zngur_parser::SourceCache<'c>),
{
    f
}

/// `str`, with the methods that C++ needs to read it.
fn str_decl() -> ZngurType {
    let method = |name: &str, output| ZngurMethodDetails {
        data: ZngurMethod {
            name: name.to_string(),
            lifetimes: vec![],
            generics: vec![],
            receiver: ZngurMethodReceiver::Ref(Mutability::Not),
            receiver_lifetime: None,
            inputs: vec![],
            output,
            is_safe: true,
        },
        use_path: None,
        deref: None,
        cpp_name: None,
    };
    ZngurType {
        ty: RustType::Primitive(PrimitiveRustType::Str),
        wellknown_traits: vec![ZngurWellknownTrait::Unsized],
        methods: vec![
            method(
                "as_ptr",
                RustType::Raw(
                    Mutability::Not,
                    Box::new(RustType::Primitive(PrimitiveRustType::Uint(8))),
                ),
            ),
            method("len", RustType::Primitive(PrimitiveRustType::Usize)),
        ],
        ..empty_zngur_type()
    }
}

fn display_path(path: &[String]) -> String {
    rust_path(path).to_string()
}

fn mutability(is_mut: bool) -> Mutability {
    if is_mut {
        Mutability::Mut
    } else {
        Mutability::Not
    }
}

fn rust_path(path: &[String]) -> RustPathAndGenerics {
    RustPathAndGenerics {
        path: path.to_vec(),
        lifetimes: vec![],
        generics: vec![],
        named_generics: vec![],
    }
}

fn empty_zngur_type() -> ZngurType {
    ZngurType {
        ty: RustType::UNIT,
        lifetimes: vec![],
        layout: None,
        trivially_relocatable: TriviallyRelocatable::No,
        wellknown_traits: vec![],
        exhaustive: true,
        methods: vec![],
        cpp_methods: vec![],
        cpp_constructors: vec![],
        cpp_fields: vec![],
        constructor: None,
        variants: vec![],
        fields: vec![],
    }
}
