// Part of the Crubit project, under the Apache License v2.0 with LLVM
// Exceptions. See /LICENSE for license information.
// SPDX-License-Identifier: Apache-2.0 WITH LLVM-exception

//! Generates a zngur (https://github.com/HKalbasi/zngur) `.zng` file for a C++ target.
//!
//! With `--zng_out`, crubit is only a front end: Clang parses the target's headers into crubit's
//! IR, and this module describes the target's API in a `.zng` file, using zngur's support for
//! calling C++ from Rust: `c++::` types (held by value when crubit knows they're movable with
//! `memcpy`, otherwise only referred to), their fields, member functions and constructors, free
//! functions, and `std::unique_ptr` instantiations. zngur then generates the Rust bindings and
//! their C++ glue from the file, which a developer can amend by hand or extend with `merge`d files.
//!
//! Items that can't be expressed in the `.zng` file (yet) are listed in `// skipped:` comments,
//! with the reason. Types of dependency targets come from those targets' `.zng` files, which the
//! file `import`s.
//!
//! C++ references follow crubit's own rule: a reference with lifetime information (an explicit
//! `$(a)` annotation, the lifetimes that crubit infers for constructors, or the defaults of the
//! `assume_lifetimes` feature) is a reference to a C++ object in the `.zng` file (`&'a const T` for
//! a `const T&`, and `&'a unique var T` for a `T&`, which matches crubit's `&mut`), with its
//! lifetime declared on the function. Any other reference is a raw one (`const T&` / `T&`, a raw
//! pointer in Rust), and the function is `unsafe` if it takes one.

use arc_anyhow::{anyhow, bail, Result};
use crubit_feature::CrubitFeature;
use database::BindingsGenerator;
use ir::{
    AccessSpecifier, CcType, CcTypeVariant, Func, Item, ItemId, PointerTypeKind, Primitive, Record,
    ReferenceQualification, SafetyAnnotation, TemplateSpecializationKind, UnqualifiedIdentifier,
    IR,
};
use lifetime_defaults_transform::lifetime_defaults_transform_func;
use std::collections::{HashMap, HashSet};
use std::fmt::Write as _;
use std::path::PathBuf;
use zngur_def::printing::IDLPrinter;
use zngur_def::{
    AdditionalIncludes, CppConstness, CppFieldAccess, CppIndirection, CppTypePath, LayoutPolicy,
    Lifetime, ModuleImport, Mutability, PrimitiveRustType, RustType, TriviallyRelocatable,
    ZngurCppConstructor, ZngurCppField, ZngurCppFn, ZngurCppMethod, ZngurMethodReceiver, ZngurSpec,
    ZngurType, ZngurWellknownTrait,
};

/// The `.zng` file of a dependency target, which the generated file imports.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ZngImport {
    /// The dependency target, e.g. `//foo:bar`.
    pub target: String,
    /// The path of the `.zng` file, for crubit to read which types it declares.
    pub zng_file: PathBuf,
    /// The path to write in the `import` statement (relative to the generated file).
    pub import_path: String,
}

/// Options of the generated `.zng` file.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ZngOptions {
    /// The `#crate` of the generated module: the Rust crate that holds zngur's bindings.
    pub crate_name: String,
    /// The `.zng` files of dependency targets (including support modules, like the home of
    /// `std::unique_ptr`).
    pub imports: Vec<ZngImport>,
    /// Hand-written `.zng` files for the generated file to `merge` (paths relative to it).
    pub merges: Vec<String>,
    /// Whether to omit information that varies between runs or machines.
    pub is_golden_test: bool,
}

/// An item that crubit imported, but that isn't in the `.zng` file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkippedItem {
    /// The C++ path of the item, e.g. `ns::Foo` or `ns::Foo::method`.
    pub path: String,
    /// Why the item was skipped.
    pub reason: String,
}

/// Generates the contents of the `.zng` file for the current target of `db.ir()`. `db` is crubit's
/// bindings database (see `generate_bindings::new_database`), which applies the lifetime rules of
/// the target's features.
pub fn generate_zng(db: &BindingsGenerator<'_>, options: &ZngOptions) -> Result<String> {
    let ir = db.ir();
    let ZngSpec { spec, skipped, allows_safe_raw_pointer } = generate_zng_spec(db, options)?;

    // Writing to a `String` can't fail, so the `writeln!` results are ignored below.
    let mut out = String::new();
    let _ = writeln!(out, "// Automatically @generated zngur IDL for the following C++ target:");
    let _ = writeln!(out, "// {}", ir.current_target());
    if !skipped.is_empty() {
        let _ = writeln!(out, "//");
        let _ = writeln!(out, "// Items that are not (yet) supported in this file:");
        for SkippedItem { path, reason } in &skipped {
            let reason = reason.split_whitespace().collect::<Vec<_>>().join(" ");
            let _ = writeln!(out, "// skipped: {path}: {reason}");
        }
    }
    // zngur's `#allow`s apply to the whole file, wherever they are.
    if allows_safe_raw_pointer {
        let _ = writeln!(out);
        let _ = writeln!(
            out,
            "// Some functions are `CRUBIT_UNSAFE_MARK_SAFE`, but take raw pointers."
        );
        let _ = writeln!(out, "#allow(safe_raw_pointer)");
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

/// The description of a C++ target for zngur.
pub struct ZngSpec {
    pub spec: ZngurSpec,
    /// The items that aren't in `spec`, and why.
    pub skipped: Vec<SkippedItem>,
    /// Whether the file must silence zngur's `safe_raw_pointer` warning (`#allow`), because a
    /// `safe` function takes raw pointers.
    pub allows_safe_raw_pointer: bool,
}

/// Builds the `ZngurSpec` for the current target of `db.ir()`, and the list of skipped items.
pub fn generate_zng_spec(db: &BindingsGenerator<'_>, options: &ZngOptions) -> Result<ZngSpec> {
    let ir = db.ir();
    let crate_name = if options.crate_name.is_empty() {
        ir.crate_name(ir.current_target())
            .map(|name| name.to_string())
            .unwrap_or_else(|| ir.current_target().target_name_escaped())
    } else {
        options.crate_name.clone()
    };
    let mut emitter = Emitter {
        db,
        ir,
        declared: HashMap::new(),
        imported_types: HashMap::new(),
        imported_builtins: HashSet::new(),
        unique_ptrs: vec![],
        skipped: vec![],
        uses_bool: false,
        allows_safe_raw_pointer: false,
    };
    let mut spec = ZngurSpec { crate_name: Some(crate_name), ..ZngurSpec::default() };
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
    // The generated C++ code needs the target's headers.
    let includes: String =
        ir.public_headers().map(|header| format!("#include \"{}\"\n", header.name())).collect();
    if !includes.is_empty() {
        spec.additional_includes = AdditionalIncludes(format!("\n{includes}"));
    }
    emitter.add_items(&mut spec);
    let allows_safe_raw_pointer = emitter.allows_safe_raw_pointer;
    let mut skipped = emitter.skipped;
    skipped.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(ZngSpec { spec, skipped, allows_safe_raw_pointer })
}

/// Returns the path of `path` relative to the directory `dir`, as written in `.zng` files: with
/// `/` separators, and starting with `./` or `../`.
pub fn zng_relative_path(dir: &std::path::Path, path: &std::path::Path) -> Result<String> {
    // E.g. the parent of `foo.zng`.
    let dir = if dir.as_os_str().is_empty() { std::path::Path::new(".") } else { dir };
    let dir = std::path::absolute(dir)?;
    let path = std::path::absolute(path)?;
    let dir: Vec<_> = dir.components().collect();
    let path: Vec<_> = path.components().collect();
    let common = dir.iter().zip(&path).take_while(|(a, b)| a == b).count();
    let mut parts = vec![];
    if common == dir.len() {
        parts.push(".".to_string());
    }
    parts.extend(std::iter::repeat_n("..".to_string(), dir.len() - common));
    parts.extend(path[common..].iter().map(|c| c.as_os_str().to_string_lossy().into_owned()));
    Ok(parts.join("/"))
}

/// Where a C++ type is used, which decides how exactly it must match its zngur type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Position {
    /// A parameter or return value: arguments are converted as in any C++ call, and return types
    /// only need the same size (zngur checks it), so `long long` can be an `i64`.
    Value,
    /// A field or a pointee: the C++ type must be exactly the zngur type's C++ type (zngur checks
    /// fields with a `static_assert`, and pointers don't convert).
    Exact,
}

struct Emitter<'a, 'pb> {
    db: &'a BindingsGenerator<'pb>,
    ir: &'a IR<'pb>,
    /// Records of the current target that are declared in the `.zng` file, and their C++ paths.
    declared: HashMap<ItemId, Vec<String>>,
    /// Target -> the C++ types that its (imported) `.zng` file declares.
    imported_types: HashMap<String, HashSet<RustType>>,
    /// The builtin types (e.g. `bool`) that imported files declare.
    imported_builtins: HashSet<PrimitiveRustType>,
    /// `std::unique_ptr` instantiations that the file declares (their element types), in order.
    unique_ptrs: Vec<(RustType, ir::SizeAlign)>,
    skipped: Vec<SkippedItem>,
    /// Whether `bool` is used, and so needs to be declared.
    uses_bool: bool,
    /// Whether a `safe` function takes raw pointers (`CRUBIT_UNSAFE_MARK_SAFE`), so the file must
    /// silence zngur's `safe_raw_pointer` warning.
    allows_safe_raw_pointer: bool,
}

impl<'a, 'pb> Emitter<'a, 'pb> {
    fn skip(&mut self, path: impl Into<String>, reason: impl std::fmt::Display) {
        self.skipped.push(SkippedItem { path: path.into(), reason: reason.to_string() });
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
        let types = self.imported_types.entry(import.target.clone()).or_default();
        for ty in parsed.spec.types.into_iter().chain(parsed.spec.cpp_templates) {
            if let RustType::Primitive(p) = &ty.ty {
                self.imported_builtins.insert(p.clone());
            }
            types.insert(ty.ty);
        }
        Ok(())
    }

    /// Whether `ty` is declared by any imported file.
    fn is_imported(&self, ty: &RustType) -> bool {
        self.imported_types.values().any(|types| types.contains(ty))
    }

    /// The C++ path of a declaration (`["ns", "Name"]`), from its enclosing namespaces.
    fn cc_path(&self, item: &Item<'pb>) -> Result<Vec<String>> {
        let name = item.cc_name_as_str().ok_or_else(|| anyhow!("the item has no name"))?;
        let mut path = vec![name.to_string()];
        let mut enclosing = item.enclosing_item_id();
        while let Some(id) = enclosing {
            let parent = self.ir.get_decl(id).ok_or_else(|| anyhow!("unknown enclosing item"))?;
            match parent {
                Item::Namespace(ns) => path.push(ns.cc_name().as_str().to_string()),
                Item::Record(_) => bail!("nested types are not supported yet"),
                _ => bail!("the item is nested in an unsupported kind of item"),
            }
            enclosing = parent.enclosing_item_id();
        }
        path.reverse();
        Ok(path)
    }

    fn add_items(&mut self, spec: &mut ZngurSpec) {
        let ir = self.ir;
        let current = ir.current_target().clone();

        // Find the items of the target, through namespaces.
        let mut records: Vec<(Vec<String>, std::rc::Rc<Record<'pb>>)> = vec![];
        let mut funcs: Vec<(Vec<String>, std::rc::Rc<Func<'pb>>)> = vec![];
        let mut stack: Vec<Item<'pb>> =
            ir.top_level_items_in_target(&current).iter().rev().cloned().collect();
        while let Some(item) = stack.pop() {
            match &item {
                Item::Namespace(ns) => {
                    stack.extend(ns.children().iter().rev().cloned());
                    continue;
                }
                Item::Comment(_) | Item::UseMod(_) => continue,
                Item::UnsupportedItem(unsupported) => {
                    let reason = unsupported
                        .errors()
                        .iter()
                        .map(|e| e.message.to_string())
                        .collect::<Vec<_>>()
                        .join("; ");
                    self.skip(
                        unsupported.name().to_string(),
                        format!("crubit can't import this item: {reason}"),
                    );
                    continue;
                }
                _ => {}
            }
            let display = item.cc_name_as_str().unwrap_or("<unnamed>").to_string();
            let path = match self.cc_path(&item) {
                Ok(path) => path,
                Err(err) => {
                    self.skip(display, format!("{err:#}"));
                    continue;
                }
            };
            let display = path.join("::");
            match item {
                Item::Record(record) => {
                    // Instantiations of class templates (like `std::unique_ptr<T>`) are declared
                    // where they're used, if supported.
                    if record.template_specialization().is_some() {
                        continue;
                    }
                    if let Err(err) = self.check_record(&path, &record) {
                        self.skip(display, format!("{err:#}"));
                        continue;
                    }
                    self.declared.insert(record.id(), path.clone());
                    records.push((path, record));
                }
                Item::Func(func) => funcs.push((path, func)),
                Item::Enum(_) => self.skip(display, "enums are not supported yet"),
                Item::TypeAlias(_) => self.skip(display, "type aliases are not supported yet"),
                Item::Constant(_) => self.skip(display, "constants are not supported yet"),
                Item::GlobalVar(_) => self.skip(display, "global variables are not supported yet"),
                Item::IncompleteRecord(_) => {
                    self.skip(display, "forward declarations are not supported yet")
                }
                Item::ExistingRustType(_) => {
                    self.skip(display, "types mapped to existing Rust types are not supported")
                }
                Item::Namespace(_)
                | Item::Comment(_)
                | Item::UseMod(_)
                | Item::UnsupportedItem(_) => {
                    unreachable!()
                }
            }
        }

        // Then the records' members, now that signatures can refer to any record.
        for (path, record) in &records {
            let ty = self.zngur_type(path, record);
            spec.types.push(ty);
        }
        for (path, func) in &funcs {
            match self.free_function(path, func) {
                Ok(func) => spec.cpp_funcs.push(func),
                Err(err) => self.skip(path.join("::"), format!("{err:#}")),
            }
        }
        // Overloaded free functions get distinct Rust names.
        let mut names: HashMap<(Vec<String>, String), usize> = HashMap::new();
        for func in &mut spec.cpp_funcs {
            let count =
                names.entry((func.namespace.clone(), func.rust_name().to_string())).or_default();
            *count += 1;
            if *count > 1 {
                func.rust_name = Some(format!("{}_{count}", func.rust_name()));
            }
        }

        let unique_ptrs = std::mem::take(&mut self.unique_ptrs);
        for (element, size_align) in unique_ptrs {
            spec.types.push(unique_ptr_type(element, size_align));
        }
        if self.uses_bool && !self.imported_builtins.contains(&PrimitiveRustType::Bool) {
            spec.types.insert(
                0,
                ZngurType {
                    ty: RustType::Primitive(PrimitiveRustType::Bool),
                    layout: Some(LayoutPolicy::StackAllocated { size: 1, align: 1 }),
                    wellknown_traits: vec![ZngurWellknownTrait::Copy],
                    ..empty_zngur_type(RustType::UNIT)
                },
            );
        }
    }

    /// Checks that a record of the current target can be declared.
    fn check_record(&self, path: &[String], record: &Record<'pb>) -> Result<()> {
        if record.template_specialization().is_some() {
            bail!("class template specializations are not supported yet");
        }
        if record.is_union() {
            bail!("unions are not supported yet");
        }
        if let Some(segment) = path.iter().find(|s| is_rust_keyword(s)) {
            bail!("`{segment}` is a Rust keyword, which zngur can't use as a module or type name");
        }
        Ok(())
    }

    fn zngur_type(&mut self, path: &[String], record: &Record<'pb>) -> ZngurType {
        let display = path.join("::");
        let size_align = record.size_align();
        // Rust holds the object by value if crubit knows that moving it with `memcpy` is fine.
        // (zngur's own check, `rust::is_trivially_relocatable`, only knows trivially copyable
        // types.) Otherwise, Rust only refers to it, and C++ still checks the layout.
        let by_value = record.is_rust_movable() && !record.is_abstract();
        let mut ty =
            empty_zngur_type(RustType::Cpp(CppTypePath { path: path.to_vec(), generics: vec![] }));
        ty.layout = Some(LayoutPolicy::StackAllocated {
            size: size_align.size(),
            align: size_align.alignment(),
        });
        if by_value {
            ty.trivially_relocatable = TriviallyRelocatable::Unchecked;
        }

        let mut rust_names: HashSet<String> = HashSet::new();
        for field in record.fields() {
            let Some(name) = field.cpp_identifier() else { continue };
            let name = name.as_str().to_string();
            if field.access() != AccessSpecifier::Public {
                continue;
            }
            let field_path = format!("{display}::{name}");
            if field.is_bitfield() {
                self.skip(field_path, "field: bitfields are not supported by zngur");
                continue;
            }
            let field_ty = field.type_();
            if matches!(field_ty.variant(), CcTypeVariant::Pointer(p) if matches!(p.kind(), PointerTypeKind::LValueRef | PointerTypeKind::RValueRef))
            {
                self.skip(field_path, "field: fields of reference type are not supported by zngur");
                continue;
            }
            let zngur_ty = match self.zngur_ty(field_ty, Position::Exact) {
                Ok(ty) => ty,
                Err(err) => {
                    self.skip(field_path, format!("field: {err:#}"));
                    continue;
                }
            };
            // A shared reference to the `const` field, and a unique one to the non-`const` field,
            // which matches crubit's `&mut`.
            let const_name = unique_name(&mut rust_names, rust_name(&name));
            ty.cpp_fields.push(ZngurCppField {
                rust_name: (const_name != name).then_some(const_name),
                name: name.clone(),
                ty: zngur_ty.clone(),
                access: CppFieldAccess::Ref { unique: false, constness: CppConstness::Const },
            });
            if !field_ty.is_const() {
                let mut_name = unique_name(&mut rust_names, rust_name(&format!("{name}_mut")));
                ty.cpp_fields.push(ZngurCppField {
                    rust_name: Some(mut_name),
                    name,
                    ty: zngur_ty,
                    access: CppFieldAccess::Ref { unique: true, constness: CppConstness::Var },
                });
            }
        }

        let mut constructors: Vec<(bool, ZngurCppConstructor)> = vec![];
        for child in record.children() {
            let func = match child {
                Item::Func(func) => func,
                Item::Record(_)
                | Item::Enum(_)
                | Item::TypeAlias(_)
                | Item::IncompleteRecord(_) => {
                    let name = child.cc_name_as_str().unwrap_or("<unnamed>");
                    self.skip(format!("{display}::{name}"), "nested types are not supported yet");
                    continue;
                }
                _ => continue,
            };
            let method_path = format!("{display}::{}", func_display_name(func));
            match func.cc_name() {
                UnqualifiedIdentifier::Constructor => {
                    if !by_value {
                        // zngur only constructs objects that Rust holds by value.
                        continue;
                    }
                    // Rust moves objects itself (with `memcpy`), so it doesn't need move
                    // constructors.
                    let is_move_constructor = matches!(
                        func.params().get(1).map(|p| p.type_().variant()),
                        Some(CcTypeVariant::Pointer(p)) if p.kind() == PointerTypeKind::RValueRef
                    ) && func.params().len() == 2;
                    if is_move_constructor {
                        continue;
                    }
                    match self.constructor(func) {
                        Ok(constructor) => {
                            constructors.push((is_copy_constructor(func, record.id()), constructor))
                        }
                        Err(err) => self.skip(method_path, format!("{err:#}")),
                    }
                }
                UnqualifiedIdentifier::Identifier(name) => match self.method(func) {
                    Ok(mut method) => {
                        let name = unique_name(&mut rust_names, rust_name(name.as_str()));
                        if name != method.cpp_name {
                            method.rust_name = Some(name);
                        }
                        ty.cpp_methods.push(method);
                    }
                    Err(err) => self.skip(method_path, format!("{err:#}")),
                },
                // Destructors run when Rust drops the object; Rust assigns objects itself.
                UnqualifiedIdentifier::Destructor => {}
                UnqualifiedIdentifier::Operator(op) if op.name() == "=" => {}
                UnqualifiedIdentifier::Operator(_) | UnqualifiedIdentifier::ConversionOperator => {
                    self.skip(method_path, "operators are not supported yet")
                }
            }
        }
        // The copy constructor is `new_copy`, and the others are `new`, `new_2`, ... in declaration
        // order (of those that are emitted). crubit lists implicit constructors (often the copy
        // constructor) first, which shouldn't take the short names.
        for (is_copy, constructor) in &mut constructors {
            if !*is_copy {
                constructor.rust_name = unique_name(&mut rust_names, "new".to_string());
            }
        }
        for (is_copy, constructor) in &mut constructors {
            if *is_copy {
                constructor.rust_name = unique_name(&mut rust_names, "new_copy".to_string());
            }
        }
        ty.cpp_constructors.extend(constructors.into_iter().map(|(_, c)| c));
        ty
    }

    fn method(&mut self, func: &Func<'pb>) -> Result<ZngurCppMethod> {
        let UnqualifiedIdentifier::Identifier(name) = func.cc_name() else {
            unreachable!("only called for named member functions")
        };
        self.check_func(func)?;
        let lowered = self.lowered(func)?;
        let mut sig = Signature::new(self.ir, func, &lowered, /* infer= */ false);
        let mut params = 0..func.params().len();
        let (receiver, receiver_constness, receiver_lifetime, raw_receiver) =
            match func.instance_method_metadata() {
                Some(metadata) => {
                    if metadata.reference() == ReferenceQualification::RValue {
                        bail!("`&&`-qualified member functions are not supported yet");
                    }
                    // The first parameter is `this`.
                    params.start = 1;
                    let constness =
                        if metadata.is_const() { CppConstness::Const } else { CppConstness::Var };
                    match self.this_lifetime(func, &lowered, &mut sig)? {
                        // `&'a const self` (a `const` member function) or `&'a unique var self`.
                        Some(lifetime) => {
                            let m = match constness {
                                CppConstness::Const => Mutability::Not,
                                CppConstness::Var => Mutability::Mut,
                            };
                            (ZngurMethodReceiver::Ref(m), constness, Some(lifetime), None)
                        }
                        // A raw receiver: `const self&` or `self&`.
                        None => (
                            ZngurMethodReceiver::Ref(Mutability::Not),
                            constness,
                            None,
                            Some(CppIndirection::Reference),
                        ),
                    }
                }
                None => (ZngurMethodReceiver::Static, CppConstness::Var, None, None),
            };
        let inputs = params.map(|i| self.sig_ty(&mut sig, Some(i))).collect::<Result<Vec<_>>>()?;
        let output = self.sig_ty(&mut sig, None)?;
        let is_safe = self.is_safe(func, raw_receiver.is_some(), &inputs);
        Ok(ZngurCppMethod {
            cpp_name: name.as_str().to_string(),
            lifetimes: sig.declared,
            template_args: vec![],
            rust_name: None,
            receiver,
            receiver_constness,
            receiver_lifetime,
            raw_receiver,
            inputs,
            output,
            is_safe,
        })
    }

    fn constructor(&mut self, func: &Func<'pb>) -> Result<ZngurCppConstructor> {
        self.check_func(func)?;
        let lowered = self.lowered(func)?;
        // Like crubit, constructors infer the lifetimes of their references.
        let mut sig = Signature::new(self.ir, func, &lowered, /* infer= */ true);
        // The first parameter is `this`.
        let inputs = (1..func.params().len())
            .map(|i| self.sig_ty(&mut sig, Some(i)))
            .collect::<Result<Vec<_>>>()?;
        Ok(ZngurCppConstructor {
            // Named by the caller.
            rust_name: String::new(),
            is_safe: self.is_safe(func, false, &inputs),
            lifetimes: sig.declared,
            inputs,
        })
    }

    fn free_function(&mut self, path: &[String], func: &Func<'pb>) -> Result<ZngurCppFn> {
        let UnqualifiedIdentifier::Identifier(name) = func.cc_name() else {
            bail!("operators are not supported yet");
        };
        self.check_func(func)?;
        if path.len() < 2 {
            bail!("functions in the global namespace are not supported by zngur");
        }
        if let Some(segment) = path[..path.len() - 1].iter().find(|s| is_rust_keyword(s)) {
            bail!("`{segment}` is a Rust keyword, which zngur can't use as a module name");
        }
        let lowered = self.lowered(func)?;
        let mut sig = Signature::new(self.ir, func, &lowered, /* infer= */ false);
        let inputs = (0..func.params().len())
            .map(|i| self.sig_ty(&mut sig, Some(i)))
            .collect::<Result<Vec<_>>>()?;
        let output = self.sig_ty(&mut sig, None)?;
        let cpp_name = name.as_str().to_string();
        let rust_name = rust_name(&cpp_name);
        Ok(ZngurCppFn {
            namespace: path[..path.len() - 1].to_vec(),
            template_args: vec![],
            rust_name: (rust_name != cpp_name).then_some(rust_name),
            cpp_name,
            lifetimes: sig.declared,
            is_safe: self.is_safe(func, false, &inputs),
            inputs,
            output,
        })
    }

    /// Checks the properties of a function that zngur can't express.
    fn check_func(&self, func: &Func<'pb>) -> Result<()> {
        if func.is_variadic() {
            bail!("variadic functions are not supported by zngur");
        }
        if func.is_consteval() {
            bail!("`consteval` functions can't be called at runtime");
        }
        Ok(())
    }

    /// Whether the owning target of `func` has `feature`.
    fn has_feature(&self, func: &Func<'pb>, feature: CrubitFeature) -> bool {
        self.ir.target_crubit_features(func.owning_target()).contains(feature)
    }

    /// `func` with the default lifetimes of the `assume_lifetimes` feature filled in, if its
    /// target has the feature (`None` otherwise).
    fn lowered(&self, func: &Func<'pb>) -> Result<Option<Func<'pb>>> {
        if !self.has_feature(func, CrubitFeature::AssumeLifetimes) {
            return Ok(None);
        }
        lifetime_defaults_transform_func(self.db, func)
            .map(Some)
            .map_err(|err| anyhow!("can't assign lifetimes to the references: {err:#}"))
    }

    /// The lifetime of `this` (`params[0]`), if it has lifetime information: from the
    /// `assume_lifetimes` defaults, an explicit annotation, or `assume_this_lifetimes` (like
    /// crubit). `None` makes the receiver raw.
    fn this_lifetime(
        &self,
        func: &Func<'pb>,
        lowered: &Option<Func<'pb>>,
        sig: &mut Signature,
    ) -> Result<Option<Lifetime>> {
        let CcTypeVariant::Pointer(this) = func.params()[0].type_().variant() else {
            bail!("the `this` parameter isn't a pointer");
        };
        let name = match lowered {
            Some(lowered) => match lowered.params()[0].type_().explicit_lifetimes() {
                [] => LifetimeInfo::Elided,
                [name] if name.as_ref() == "unknown" => LifetimeInfo::None,
                [name] => LifetimeInfo::Named(name.to_string()),
                _ => bail!("`this` may only have one lifetime"),
            },
            None => match this.lifetime() {
                Some(id) => LifetimeInfo::Named(self.lifetime_name(id)?),
                None if self.has_feature(func, CrubitFeature::AssumeThisLifetimes) => {
                    LifetimeInfo::Elided
                }
                None => LifetimeInfo::None,
            },
        };
        Ok(sig.resolve(name, /* is_return= */ false))
    }

    fn lifetime_name(&self, id: ir::LifetimeId) -> Result<String> {
        // crubit's `STATIC_LIFETIME_ID`, for `$static`, which isn't in the IR's lifetimes.
        if id == ir::LifetimeId(-1) {
            return Ok("static".to_string());
        }
        Ok(self
            .ir
            .get_lifetime(id)
            .ok_or_else(|| anyhow!("no known lifetime with id {id:?}"))?
            .name()
            .to_string())
    }

    /// The zngur type of the parameter `param` of the function of `sig` (its return type if
    /// `None`), where C++ references with lifetime information are references to C++ objects.
    fn sig_ty(&mut self, sig: &mut Signature, param: Option<usize>) -> Result<RustType> {
        let (ty, lowered) = match param {
            Some(i) => (sig.func.params()[i].type_(), sig.lowered.map(|f| f.params()[i].type_())),
            None => (sig.func.return_type(), sig.lowered.map(|f| f.return_type())),
        };
        // `sig` borrows the IR, not `self`, so the types outlive this call.
        let (ty, lowered) = (ty.clone(), lowered.cloned());
        reject_lifetime_params(&ty)?;
        if let Some(lowered) = &lowered {
            reject_lifetime_params(lowered)?;
        }
        let CcTypeVariant::Pointer(pointer) = ty.variant() else {
            return self.zngur_ty(&ty, Position::Value);
        };
        if pointer.kind() != PointerTypeKind::LValueRef {
            return self.zngur_ty(&ty, Position::Value);
        }
        let pointee = pointer.pointee_type();
        if pointee.is_unit_type() {
            bail!("`void` pointers are not supported yet");
        }
        let info = match &lowered {
            // `assume_lifetimes`: the defaults are on the lowered type, which is a pointer if no
            // lifetime could be assigned.
            Some(lowered) => match lowered.variant() {
                CcTypeVariant::Pointer(p) if p.kind() == PointerTypeKind::LValueRef => {
                    match lowered.explicit_lifetimes() {
                        [] => LifetimeInfo::Elided,
                        [name] if name.as_ref() == "unknown" => LifetimeInfo::None,
                        [name] => LifetimeInfo::Named(name.to_string()),
                        _ => bail!("references may only have one lifetime"),
                    }
                }
                _ => LifetimeInfo::None,
            },
            None => match pointer.lifetime() {
                Some(id) => LifetimeInfo::Named(self.lifetime_name(id)?),
                None if sig.infer => LifetimeInfo::Elided,
                None => LifetimeInfo::None,
            },
        };
        let pointee_ty = self.zngur_ty(pointee, Position::Exact)?;
        Ok(match sig.resolve(info, /* is_return= */ param.is_none()) {
            // A reference to a C++ object: `&'a const T`, or `&'a unique var T` (crubit's `&mut`).
            Some(lifetime) => {
                let (m, constness) = if pointee.is_const() {
                    (Mutability::Not, CppConstness::Const)
                } else {
                    (Mutability::Mut, CppConstness::Var)
                };
                RustType::Ref(
                    m,
                    Some(lifetime),
                    Box::new(RustType::cpp_object(constness, pointee_ty)),
                )
            }
            // A raw `const T&` / `T&`.
            None => {
                let m = if pointee.is_const() { Mutability::Not } else { Mutability::Mut };
                RustType::cpp_ptr(m, CppIndirection::Reference, pointee_ty)
            }
        })
    }

    /// Whether a function is `safe`, like in crubit's own bindings: `CRUBIT_UNSAFE` functions
    /// aren't, `CRUBIT_UNSAFE_MARK_SAFE` ones are, and the others are unless they take raw
    /// pointers (a raw receiver, C++ pointers, or C++ references without lifetimes), which may be
    /// null or invalid. (References to C++ objects refer to valid objects, and returned raw
    /// pointers don't make a function unsafe.) A `safe` function that takes raw pointers makes
    /// the file silence zngur's `safe_raw_pointer` warning.
    fn is_safe(&mut self, func: &Func<'pb>, raw_receiver: bool, inputs: &[RustType]) -> bool {
        let takes_raw_ptr = raw_receiver || inputs.iter().any(contains_raw_ptr);
        let is_safe = match func.safety_annotation() {
            SafetyAnnotation::Unannotated => !takes_raw_ptr,
            SafetyAnnotation::Unsafe => false,
            SafetyAnnotation::DisableUnsafe => true,
        };
        if is_safe && takes_raw_ptr {
            self.allows_safe_raw_pointer = true;
        }
        is_safe
    }

    /// Converts a C++ type to a zngur type. Fails if zngur can't express the type, or it isn't
    /// declared in the file (or in an imported file).
    fn zngur_ty(&mut self, ty: &CcType, position: Position) -> Result<RustType> {
        use PrimitiveRustType::*;
        let primitive = |p| Ok(RustType::Primitive(p));
        match ty.variant() {
            CcTypeVariant::Primitive(p) => match p {
                Primitive::Void => Ok(RustType::UNIT),
                Primitive::Bool => {
                    self.uses_bool = true;
                    primitive(Bool)
                }
                Primitive::Float => primitive(Float(32)),
                Primitive::Double => primitive(Float(64)),
                Primitive::SignedChar | Primitive::Int8T | Primitive::StdInt8T => primitive(Int(8)),
                Primitive::UnsignedChar | Primitive::Uint8T | Primitive::StdUint8T => {
                    primitive(Uint(8))
                }
                Primitive::Short | Primitive::Int16T | Primitive::StdInt16T => primitive(Int(16)),
                Primitive::UnsignedShort | Primitive::Uint16T | Primitive::StdUint16T => {
                    primitive(Uint(16))
                }
                Primitive::Int | Primitive::Int32T | Primitive::StdInt32T => primitive(Int(32)),
                Primitive::UnsignedInt | Primitive::Uint32T | Primitive::StdUint32T => {
                    primitive(Uint(32))
                }
                // `int64_t` is `long` on LP64 targets (Linux, macOS), so `long long` is a
                // different type, which only converts as a value.
                Primitive::Long | Primitive::Int64T | Primitive::StdInt64T => primitive(Int(64)),
                Primitive::UnsignedLong | Primitive::Uint64T | Primitive::StdUint64T => {
                    primitive(Uint(64))
                }
                Primitive::LongLong if position == Position::Value => primitive(Int(64)),
                Primitive::UnsignedLongLong if position == Position::Value => primitive(Uint(64)),
                Primitive::SizeT | Primitive::StdSizeT => primitive(Usize),
                Primitive::UintptrT | Primitive::StdUintptrT if position == Position::Value => {
                    primitive(Usize)
                }
                Primitive::Char => bail!("`char` is not supported yet (its signedness varies)"),
                p => bail!("`{}` is not supported by zngur", quote::quote!(#p)),
            },
            CcTypeVariant::Pointer(pointer) => {
                let indirection = match pointer.kind() {
                    PointerTypeKind::LValueRef => CppIndirection::Reference,
                    PointerTypeKind::Nullable | PointerTypeKind::NonNull => CppIndirection::Pointer,
                    PointerTypeKind::RValueRef => bail!("rvalue references are not supported yet"),
                    PointerTypeKind::Owned => bail!("owned pointers are not supported yet"),
                };
                let pointee = pointer.pointee_type();
                if pointee.is_unit_type() {
                    bail!("`void` pointers are not supported yet");
                }
                if matches!(pointee.variant(), CcTypeVariant::Pointer(_)) {
                    bail!("pointers to pointers are not supported yet");
                }
                let mutability = if pointee.is_const() { Mutability::Not } else { Mutability::Mut };
                // References in signatures are handled by `sig_ty`; others are raw.
                Ok(RustType::cpp_ptr(
                    mutability,
                    indirection,
                    self.zngur_ty(pointee, Position::Exact)?,
                ))
            }
            CcTypeVariant::FuncPointer { .. } => bail!("function pointers are not supported yet"),
            CcTypeVariant::Error(err) => bail!("{}", err.message),
            CcTypeVariant::Decl { id, .. } => {
                let item = self.ir.get_decl(*id).ok_or_else(|| anyhow!("unknown type"))?.clone();
                match &item {
                    Item::TypeAlias(alias) => self.zngur_ty(alias.underlying_type(), position),
                    Item::Record(record) => self.record_ty(&item, record),
                    Item::Enum(_) => bail!("enums are not supported yet"),
                    Item::IncompleteRecord(_) => {
                        bail!("forward-declared types are not supported yet")
                    }
                    _ => bail!(
                        "type `{}` is not supported yet",
                        item.cc_name_as_str().unwrap_or("?")
                    ),
                }
            }
        }
    }

    fn record_ty(&mut self, item: &Item<'pb>, record: &Record<'pb>) -> Result<RustType> {
        if let Some(path) = self.declared.get(&record.id()) {
            return Ok(RustType::Cpp(CppTypePath { path: path.clone(), generics: vec![] }));
        }
        if let Some(specialization) = record.template_specialization() {
            return match specialization.kind() {
                TemplateSpecializationKind::StdUniquePtr { raw_element_type } => {
                    self.unique_ptr_ty(raw_element_type, record.size_align())
                }
                _ => bail!(
                    "class template specialization `{}` is not supported yet",
                    record.cc_name()
                ),
            };
        }
        let path = self.cc_path(item)?;
        let ty = RustType::Cpp(CppTypePath { path: path.clone(), generics: vec![] });
        let target = record.owning_target();
        if self.ir.is_current_target(target) {
            bail!("type `{}` isn't in this file (see why it's skipped)", path.join("::"));
        }
        match self.imported_types.get(target.as_str()) {
            Some(types) if types.contains(&ty) => Ok(ty),
            Some(_) => bail!("type `{}` isn't in the `.zng` file of `{target}`", path.join("::")),
            None => bail!(
                "type `{}` is from `{target}`, whose `.zng` file isn't imported",
                path.join("::")
            ),
        }
    }

    /// `std::unique_ptr<T>`: declared in this file if `T` is a record of this target (like Rust's
    /// orphan rule, the instantiation belongs next to `T`), otherwise it must be imported.
    fn unique_ptr_ty(&mut self, element: &CcType, size_align: ir::SizeAlign) -> Result<RustType> {
        let element_ty = self.zngur_ty(element, Position::Exact)?;
        let ty = RustType::Cpp(CppTypePath {
            path: vec!["std".to_string(), "unique_ptr".to_string()],
            generics: vec![element_ty.clone()],
        });
        if self.is_imported(&ty) {
            return Ok(ty);
        }
        let is_local = matches!(element.variant(), CcTypeVariant::Decl { id, .. } if self.declared.contains_key(id));
        if !is_local {
            bail!(
                "`std::unique_ptr<{}>` isn't declared in an imported `.zng` file (instantiations \
                 belong in the file of their element type, or in `cpp_std.zng` for builtins)",
                element_ty
            );
        }
        if !self.unique_ptrs.iter().any(|(e, _)| *e == element_ty) {
            self.unique_ptrs.push((element_ty, size_align));
        }
        Ok(ty)
    }
}

/// `std::unique_ptr<element>`: Rust holds it by value (zngur knows it's trivially relocatable),
/// and `get()` gives the object. If a module that Rust imports declares
/// `type<T> c++::std::unique_ptr<T>`, it's the home of the template, and these members are the
/// same as its defaults.
fn unique_ptr_type(element: RustType, size_align: ir::SizeAlign) -> ZngurType {
    let ty = RustType::Cpp(CppTypePath {
        path: vec!["std".to_string(), "unique_ptr".to_string()],
        generics: vec![element.clone()],
    });
    ZngurType {
        layout: Some(LayoutPolicy::StackAllocated {
            size: size_align.size(),
            align: size_align.alignment(),
        }),
        trivially_relocatable: TriviallyRelocatable::Checked,
        cpp_methods: vec![ZngurCppMethod {
            cpp_name: "get".to_string(),
            lifetimes: vec![Lifetime("a".to_string())],
            template_args: vec![],
            rust_name: None,
            receiver: ZngurMethodReceiver::Ref(Mutability::Not),
            receiver_constness: CppConstness::Const,
            receiver_lifetime: Some(Lifetime("a".to_string())),
            raw_receiver: None,
            inputs: vec![],
            output: RustType::cpp_ptr(Mutability::Mut, CppIndirection::Pointer, element),
            is_safe: true,
        }],
        ..empty_zngur_type(ty)
    }
}

/// What crubit knows about the lifetime of a C++ reference.
enum LifetimeInfo {
    /// It has no lifetime information, so it's a raw reference.
    None,
    /// It's a reference whose lifetime is elided (e.g. inferred for a constructor).
    Elided,
    /// It's a reference with this lifetime (without the `'`).
    Named(String),
}

/// The lifetimes of a function's signature, which are always named and declared on the function,
/// so that the `.zng` file never relies on elision.
struct Signature<'s, 'pb> {
    func: &'s Func<'pb>,
    /// `func` with the `assume_lifetimes` defaults, if they apply.
    lowered: Option<&'s Func<'pb>>,
    /// Whether references without annotations get lifetimes (constructors).
    infer: bool,
    /// The declared lifetimes, in order of first use.
    declared: Vec<Lifetime>,
    /// The lifetime names that the signature already uses, which fresh names avoid.
    reserved: HashSet<String>,
    next_fresh: usize,
}

impl<'s, 'pb> Signature<'s, 'pb> {
    fn new(ir: &IR<'pb>, func: &'s Func<'pb>, lowered: &'s Option<Func<'pb>>, infer: bool) -> Self {
        let mut reserved = HashSet::new();
        let funcs = std::iter::once(func).chain(lowered.as_ref());
        for f in funcs {
            for ty in f.params().iter().map(|p| p.type_()).chain([f.return_type()]) {
                collect_lifetime_names(ir, ty, &mut reserved);
            }
        }
        Signature {
            func,
            lowered: lowered.as_ref(),
            infer,
            declared: vec![],
            reserved,
            next_fresh: 0,
        }
    }

    /// The lifetime of a reference with lifetime information `info`, declaring it on the function,
    /// or `None` if the reference is raw. An elided lifetime gets a fresh name, except in the
    /// return type, where it can't be named, so the reference is raw.
    fn resolve(&mut self, info: LifetimeInfo, is_return: bool) -> Option<Lifetime> {
        let name = match info {
            LifetimeInfo::None => return None,
            LifetimeInfo::Elided if is_return => return None,
            LifetimeInfo::Elided => self.fresh(),
            // crubit's names, including the ones it makes up (`__this`, `__param_0`, ...).
            LifetimeInfo::Named(name) => name,
        };
        let lifetime = Lifetime(name);
        if !lifetime.is_builtin() && !self.declared.contains(&lifetime) {
            self.declared.push(lifetime.clone());
        }
        Some(lifetime)
    }

    /// A lifetime name that the signature doesn't use: `a`, `b`, ..., then `a1`, `b1`, ....
    fn fresh(&mut self) -> String {
        loop {
            let n = self.next_fresh;
            self.next_fresh += 1;
            let letter = char::from(b'a' + (n % 26) as u8);
            let name = match n / 26 {
                0 => letter.to_string(),
                round => format!("{letter}{round}"),
            };
            if self.reserved.insert(name.clone()) {
                return name;
            }
        }
    }
}

/// Adds the lifetime names that `ty` uses (crubit's explicit lifetimes, and annotated ones) to
/// `names`.
fn collect_lifetime_names(ir: &IR<'_>, ty: &CcType, names: &mut HashSet<String>) {
    names.extend(ty.explicit_lifetimes().iter().map(|name| name.to_string()));
    if let CcTypeVariant::Pointer(pointer) = ty.variant() {
        if let Some(name) = pointer.lifetime().and_then(|id| ir.get_lifetime(id)) {
            names.insert(name.name().to_string());
        }
        collect_lifetime_names(ir, pointer.pointee_type(), names);
    }
}

/// Fails for C++ types with lifetime parameters, which zngur doesn't support (e.g. views
/// annotated with lifetimes), anywhere in `ty` (as lowered by `assume_lifetimes`).
fn reject_lifetime_params(ty: &CcType) -> Result<()> {
    match ty.variant() {
        CcTypeVariant::Decl { .. } if !ty.explicit_lifetimes().is_empty() => {
            bail!("C++ types with lifetime parameters are not supported by zngur yet")
        }
        CcTypeVariant::Pointer(pointer) => reject_lifetime_params(pointer.pointee_type()),
        _ => Ok(()),
    }
}

/// Whether `ty` is or contains a raw pointer: a C++ pointer or a C++ reference without a lifetime
/// (e.g. `T*`, `const T&` or `&'a unique var T*`). zngur warns about `safe` functions that take one.
fn contains_raw_ptr(ty: &RustType) -> bool {
    match ty {
        RustType::CppPtr(..) | RustType::Raw(..) => true,
        RustType::Ref(_, _, inner) | RustType::CppConst(inner) | RustType::CppVar(inner) => {
            contains_raw_ptr(inner)
        }
        _ => false,
    }
}

/// A name for error messages.
fn func_display_name(func: &Func<'_>) -> String {
    match func.cc_name() {
        UnqualifiedIdentifier::Identifier(name) => name.as_str().to_string(),
        UnqualifiedIdentifier::Constructor => "(constructor)".to_string(),
        UnqualifiedIdentifier::Destructor => "(destructor)".to_string(),
        UnqualifiedIdentifier::Operator(op) => format!("operator{}", op.name()),
        UnqualifiedIdentifier::ConversionOperator => "(conversion operator)".to_string(),
    }
}

/// The Rust name for a C++ name: the same, unless it's a Rust keyword (`type` -> `type_`).
fn rust_name(name: &str) -> String {
    if is_rust_keyword(name) {
        format!("{name}_")
    } else {
        name.to_string()
    }
}

/// Whether `func` is a copy constructor of the record `record_id`: `T(const T&)`.
fn is_copy_constructor(func: &Func<'_>, record_id: ItemId) -> bool {
    let [_, other] = func.params() else {
        return false;
    };
    let CcTypeVariant::Pointer(ptr) = other.type_().variant() else {
        return false;
    };
    ptr.kind() == PointerTypeKind::LValueRef
        && ptr.pointee_type().is_const()
        && matches!(ptr.pointee_type().variant(), CcTypeVariant::Decl { id, .. } if *id == record_id)
}

/// Appends `_N` to `name` until it isn't in `names`, and adds it.
fn unique_name(names: &mut HashSet<String>, name: String) -> String {
    let mut candidate = name.clone();
    let mut n = 1;
    while names.contains(&candidate) {
        n += 1;
        candidate = format!("{name}_{n}");
    }
    names.insert(candidate.clone());
    candidate
}

/// Whether `name` is a Rust keyword (strict or reserved, in any edition).
fn is_rust_keyword(name: &str) -> bool {
    matches!(
        name,
        "as" | "break"
            | "const"
            | "continue"
            | "crate"
            | "else"
            | "enum"
            | "extern"
            | "false"
            | "fn"
            | "for"
            | "if"
            | "impl"
            | "in"
            | "let"
            | "loop"
            | "match"
            | "mod"
            | "move"
            | "mut"
            | "pub"
            | "ref"
            | "return"
            | "self"
            | "Self"
            | "static"
            | "struct"
            | "super"
            | "trait"
            | "true"
            | "type"
            | "unsafe"
            | "use"
            | "where"
            | "while"
            | "async"
            | "await"
            | "dyn"
            | "abstract"
            | "become"
            | "box"
            | "do"
            | "final"
            | "macro"
            | "override"
            | "priv"
            | "typeof"
            | "unsized"
            | "virtual"
            | "yield"
            | "try"
            | "gen"
    )
}

/// Gives a closure the signature of a zngur `ReportSink`.
fn report_sink<F>(f: F) -> F
where
    F: for<'r, 'c> FnMut(bool, &zngur_parser::ParseReport<'r>, zngur_parser::SourceCache<'c>),
{
    f
}

fn empty_zngur_type(ty: RustType) -> ZngurType {
    ZngurType {
        ty,
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
