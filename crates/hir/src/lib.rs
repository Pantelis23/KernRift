use std::collections::BTreeSet;

use krir::{CallEdge, Ctx, Eff, Function, FunctionAttrs, KrirModule, KrirOp};
use parser::{FnAst, ModuleAst, RawAttr, Stmt, split_csv};

pub fn lower_to_krir(ast: &ModuleAst) -> Result<KrirModule, Vec<String>> {
    let mut errors = Vec::new();
    let mut functions = Vec::new();
    let mut names = BTreeSet::new();

    for item in &ast.items {
        if !names.insert(item.name.clone()) {
            errors.push(format!("duplicate symbol '{}'", item.name));
        }
    }

    for item in &ast.items {
        match lower_function(item) {
            Ok(function) => functions.push(function),
            Err(errs) => errors.extend(errs),
        }
    }

    let mut call_edges = Vec::new();
    let fn_names: BTreeSet<_> = functions.iter().map(|f| f.name.clone()).collect();

    for function in &functions {
        if function.is_extern {
            continue;
        }
        for op in &function.ops {
            if let KrirOp::Call { callee } = op {
                if !fn_names.contains(callee) {
                    errors.push(format!(
                        "undefined symbol '{}': add extern declaration with facts (@ctx/@eff)",
                        callee
                    ));
                    continue;
                }
                call_edges.push(CallEdge {
                    caller: function.name.clone(),
                    callee: callee.clone(),
                });
            }
        }
    }

    if !errors.is_empty() {
        return Err(errors);
    }

    let mut module = KrirModule {
        module_caps: ast.module_caps.clone(),
        functions,
        call_edges,
    };
    module.canonicalize();
    Ok(module)
}

fn lower_function(item: &FnAst) -> Result<Function, Vec<String>> {
    let mut errors = Vec::new();
    let mut ctx_ok = BTreeSet::new();
    let mut eff_used = BTreeSet::new();
    let mut caps_req = BTreeSet::new();
    let mut attrs = FunctionAttrs::default();
    let mut saw_ctx = false;
    let mut saw_eff = false;
    let mut saw_caps = false;

    for attr in &item.attrs {
        let name = attr.name.to_ascii_lowercase();
        match name.as_str() {
            "ctx" => {
                saw_ctx = true;
                match parse_ctx_attr(attr) {
                    Ok(values) => ctx_ok.extend(values),
                    Err(msg) => errors.push(format!("{} for '{}'", msg, item.name)),
                }
            }
            "eff" => {
                saw_eff = true;
                match parse_eff_attr(attr) {
                    Ok(values) => eff_used.extend(values),
                    Err(msg) => errors.push(format!("{} for '{}'", msg, item.name)),
                }
            }
            "caps" => {
                saw_caps = true;
                if let Some(raw) = &attr.args {
                    caps_req.extend(split_csv(raw));
                }
            }
            "irq" => {
                saw_ctx = true;
                ctx_ok.clear();
                ctx_ok.insert(Ctx::Irq);
            }
            "noirq" => {
                saw_ctx = true;
                ctx_ok.clear();
                ctx_ok.insert(Ctx::Boot);
                ctx_ok.insert(Ctx::Thread);
            }
            "alloc" => {
                saw_eff = true;
                eff_used.insert(Eff::Alloc);
            }
            "block" => {
                saw_eff = true;
                eff_used.insert(Eff::Block);
            }
            "preempt_off" => {
                saw_eff = true;
                eff_used.insert(Eff::PreemptOff);
            }
            "noyield" => attrs.noyield = true,
            "critical" => attrs.critical = true,
            "leaf" => attrs.leaf = true,
            "hotpath" => attrs.hotpath = true,
            "lock_budget" => match parse_lock_budget(attr) {
                Ok(v) => attrs.lock_budget = Some(v),
                Err(msg) => errors.push(format!("{} for '{}'", msg, item.name)),
            },
            "module_caps" => {}
            other => {
                errors.push(format!(
                    "unknown attribute '@{}' on function '{}'",
                    other, item.name
                ));
            }
        }
    }

    if item.is_extern {
        if !saw_ctx {
            errors.push(format!(
                "extern '{}' must declare @ctx(...) facts explicitly",
                item.name
            ));
        }
        if !saw_eff {
            errors.push(format!(
                "extern '{}' must declare @eff(...) facts explicitly",
                item.name
            ));
        }
    } else {
        if !saw_ctx {
            // Default is conservative and excludes IRQ/NMI.
            ctx_ok.insert(Ctx::Boot);
            ctx_ok.insert(Ctx::Thread);
        }
        if !saw_eff {
            // Default is no declared effects.
        }
        if !saw_caps {
            // Default is no required capabilities.
        }
    }

    let mut ops = Vec::new();
    for stmt in &item.body {
        match stmt {
            Stmt::Call(callee) => ops.push(KrirOp::Call {
                callee: callee.clone(),
            }),
            Stmt::YieldPoint => {
                ops.push(KrirOp::YieldPoint);
                eff_used.insert(Eff::Yield);
            }
            Stmt::AllocPoint => {
                ops.push(KrirOp::AllocPoint);
                eff_used.insert(Eff::Alloc);
            }
            Stmt::BlockPoint => {
                ops.push(KrirOp::BlockPoint);
                eff_used.insert(Eff::Block);
            }
            Stmt::Acquire(lock_class) => ops.push(KrirOp::Acquire {
                lock_class: lock_class.clone(),
            }),
            Stmt::Release(lock_class) => ops.push(KrirOp::Release {
                lock_class: lock_class.clone(),
            }),
            Stmt::MmioRead => {
                ops.push(KrirOp::MmioRead);
                eff_used.insert(Eff::Mmio);
            }
            Stmt::MmioWrite => {
                ops.push(KrirOp::MmioWrite);
                eff_used.insert(Eff::Mmio);
            }
        }
    }

    if !errors.is_empty() {
        return Err(errors);
    }

    Ok(Function {
        name: item.name.clone(),
        is_extern: item.is_extern,
        ctx_ok: ctx_ok.into_iter().collect(),
        eff_used: eff_used.into_iter().collect(),
        caps_req: caps_req.into_iter().collect(),
        attrs,
        ops,
    })
}

fn parse_ctx_attr(attr: &RawAttr) -> Result<Vec<Ctx>, String> {
    let Some(args) = attr.args.as_deref() else {
        return Err("@ctx(...) requires a context list".to_string());
    };

    let mut out = Vec::new();
    for token in split_csv(args) {
        let ctx = match token.trim().to_ascii_lowercase().as_str() {
            "boot" => Ctx::Boot,
            "thread" => Ctx::Thread,
            "irq" => Ctx::Irq,
            "nmi" => Ctx::Nmi,
            _ => return Err(format!("unknown context '{}'", token)),
        };
        out.push(ctx);
    }
    Ok(out)
}

fn parse_eff_attr(attr: &RawAttr) -> Result<Vec<Eff>, String> {
    let Some(args) = attr.args.as_deref() else {
        return Err("@eff(...) requires an effect list".to_string());
    };

    let mut out = Vec::new();
    for token in split_csv(args) {
        let eff = match token.trim().to_ascii_lowercase().as_str() {
            "alloc" => Eff::Alloc,
            "block" => Eff::Block,
            "preempt_off" => Eff::PreemptOff,
            "ioport" => Eff::Ioport,
            "mmio" => Eff::Mmio,
            "dma_map" => Eff::DmaMap,
            "yield" => Eff::Yield,
            _ => return Err(format!("unknown effect '{}'", token)),
        };
        out.push(eff);
    }
    Ok(out)
}

fn parse_lock_budget(attr: &RawAttr) -> Result<u64, String> {
    let Some(args) = attr.args.as_deref() else {
        return Err("@lock_budget(N) requires a number".to_string());
    };
    let trimmed = args.trim();
    trimmed
        .parse::<u64>()
        .map_err(|_| format!("invalid lock budget '{}'", trimmed))
}

#[cfg(test)]
mod tests {
    use super::lower_to_krir;
    use parser::parse_module;
    use proptest::prelude::*;

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(128))]

        #[test]
        fn lower_to_krir_never_panics_on_random_parseable_input(bytes in proptest::collection::vec(any::<u8>(), 0..256)) {
            let input = String::from_utf8_lossy(&bytes).to_string();
            let parsed = std::panic::catch_unwind(|| parse_module(&input));
            prop_assert!(parsed.is_ok());

            let parse_outcome = match parsed {
                Ok(value) => value,
                Err(_) => return Ok(()),
            };
            if let Ok(ast) = parse_outcome {
                let lowered = std::panic::catch_unwind(|| lower_to_krir(&ast));
                prop_assert!(lowered.is_ok());
            }
        }
    }
}
