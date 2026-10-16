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

use arc_anyhow::{anyhow, bail, Result};
use ir::{
    AccessSpecifier, CcType, CcTypeVariant, Func, Item, ItemId, PointerTypeKind, Primitive, Record,
    ReferenceQualification, SafetyAnnotation, TemplateSpecializationKind, UnqualifiedIdentifier,
    IR,
};
use std::collections::{HashMap, HashSet};
use std::fmt::Write as _;
use std::path::PathBuf;
use zngur_def::printing::IDLPrinter;
use zngur_def::{
    AdditionalIncludes, CppIndirection, CppTypePath, LayoutPolicy, ModuleImport, Mutability,
    PrimitiveRustType, RustType, TriviallyRelocatable, ZngurCppConstructor, ZngurCppField,
    ZngurCppFn, ZngurCppMethod, ZngurMethodReceiver, ZngurSpec, ZngurType, ZngurWellknownTrait,
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

/// Generates the contents of the `.zng` file for the current target of `ir`.
pub fn generate_zng(ir: &IR, options: &ZngOptions) -> Result<String> {
    let (spec, skipped) = generate_zng_spec(ir, options)?;

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

/// Builds the `ZngurSpec` for the current target of `ir`, and the list of skipped items.
pub fn generate_zng_spec(ir: &IR, options: &ZngOptions) -> Result<(ZngurSpec, Vec<SkippedItem>)> {
    let crate_name = if options.crate_name.is_empty() {
        ir.crate_name(ir.current_target())
            .map(|name| name.to_string())
            .unwrap_or_else(|| ir.current_target().target_name_escaped())
    } else {
        options.crate_name.clone()
    };
    let mut emitter = Emitter {
        ir,
        declared: HashMap::new(),
        imported_types: HashMap::new(),
        imported_builtins: HashSet::new(),
        unique_ptrs: vec![],
        skipped: vec![],
        uses_bool: false,
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
    let mut skipped = emitter.skipped;
    skipped.sort_by(|a, b| a.path.cmp(&b.path));
    Ok((spec, skipped))
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
            let const_name = unique_name(&mut rust_names, rust_name(&name));
            ty.cpp_fields.push(ZngurCppField {
                rust_name: (const_name != name).then_some(const_name),
                name: name.clone(),
                ty: zngur_ty.clone(),
                mutability: Mutability::Not,
            });
            if !field_ty.is_const() {
                let mut_name = unique_name(&mut rust_names, rust_name(&format!("{name}_mut")));
                ty.cpp_fields.push(ZngurCppField {
                    rust_name: Some(mut_name),
                    name,
                    ty: zngur_ty,
                    mutability: Mutability::Mut,
                });
            }
        }

        let mut constructor_inputs: Vec<Vec<RustType>> = vec![];
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
                            // E.g. `T(const T&)` and `T(const T*)` would be the same Rust trait.
                            if constructor_inputs.contains(&constructor.inputs) {
                                self.skip(
                                    method_path,
                                    "another constructor has the same Rust parameter types",
                                );
                                continue;
                            }
                            constructor_inputs.push(constructor.inputs.clone());
                            ty.cpp_constructors.push(constructor);
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
        ty
    }

    fn method(&mut self, func: &Func<'pb>) -> Result<ZngurCppMethod> {
        let UnqualifiedIdentifier::Identifier(name) = func.cc_name() else {
            unreachable!("only called for named member functions")
        };
        self.check_func(func)?;
        let mut params = func.params();
        let receiver = match func.instance_method_metadata() {
            Some(metadata) => {
                if metadata.reference() == ReferenceQualification::RValue {
                    bail!("`&&`-qualified member functions are not supported yet");
                }
                // The first parameter is `this`.
                params = &params[1..];
                if metadata.is_const() {
                    ZngurMethodReceiver::Ref(Mutability::Not)
                } else {
                    ZngurMethodReceiver::Ref(Mutability::Mut)
                }
            }
            None => ZngurMethodReceiver::Static,
        };
        let inputs = params
            .iter()
            .map(|p| self.zngur_ty(p.type_(), Position::Value))
            .collect::<Result<Vec<_>>>()?;
        let output = self.zngur_ty(func.return_type(), Position::Value)?;
        // `this` is a `CppRef`, which refers to a valid object, so it doesn't make the member
        // function unsafe.
        let is_safe = self.is_safe(func, &inputs);
        Ok(ZngurCppMethod {
            cpp_name: name.as_str().to_string(),
            template_args: vec![],
            rust_name: None,
            receiver,
            inputs,
            output,
            is_safe,
        })
    }

    fn constructor(&mut self, func: &Func<'pb>) -> Result<ZngurCppConstructor> {
        self.check_func(func)?;
        // The first parameter is `this`.
        let inputs = func.params()[1..]
            .iter()
            .map(|p| self.zngur_ty(p.type_(), Position::Value))
            .collect::<Result<Vec<_>>>()?;
        Ok(ZngurCppConstructor { is_safe: self.is_safe(func, &inputs), inputs })
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
        let inputs = func
            .params()
            .iter()
            .map(|p| self.zngur_ty(p.type_(), Position::Value))
            .collect::<Result<Vec<_>>>()?;
        let output = self.zngur_ty(func.return_type(), Position::Value)?;
        let cpp_name = name.as_str().to_string();
        let rust_name = rust_name(&cpp_name);
        Ok(ZngurCppFn {
            namespace: path[..path.len() - 1].to_vec(),
            template_args: vec![],
            rust_name: (rust_name != cpp_name).then_some(rust_name),
            cpp_name,
            is_safe: self.is_safe(func, &inputs),
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

    /// Whether a function can be `safe`: it isn't annotated as unsafe, and takes no C++ pointers,
    /// which may be null or invalid. (C++ references, including `this`, are `CppRef`s, which refer
    /// to valid objects.)
    fn is_safe(&self, func: &Func<'pb>, inputs: &[RustType]) -> bool {
        func.safety_annotation() != SafetyAnnotation::Unsafe && !inputs.iter().any(contains_cpp_ptr)
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
                Ok(RustType::CppPtr(
                    mutability,
                    indirection,
                    Box::new(self.zngur_ty(pointee, Position::Exact)?),
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
            template_args: vec![],
            rust_name: None,
            receiver: ZngurMethodReceiver::Ref(Mutability::Not),
            inputs: vec![],
            output: RustType::CppPtr(Mutability::Mut, CppIndirection::Pointer, Box::new(element)),
            is_safe: true,
        }],
        ..empty_zngur_type(ty)
    }
}

/// Whether `ty` is or contains a C++ pointer (e.g. `T*` or `T*&`).
fn contains_cpp_ptr(ty: &RustType) -> bool {
    match ty {
        RustType::CppPtr(_, CppIndirection::Pointer, _) => true,
        RustType::CppPtr(_, CppIndirection::Reference, inner) => contains_cpp_ptr(inner),
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
