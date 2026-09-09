//! Bounded, overload-preserving static reachability for exact Swift proof targets.
//! Unknown argument types retain every possible local overload. Each possible
//! overload must have a path to the target; names alone are not proof of identity.
use crate::effect_analysis::{
    function_node_name, matching_delimiter, split_top_level, swift_callable_selector,
    tree_sitter_qualified_name,
};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use tree_sitter::{Node, Parser};

#[derive(Default, Debug)]
pub(crate) struct Index {
    functions: Vec<Function>,
}
#[derive(Debug)]
struct Function {
    path: PathBuf,
    name: String,
    qualified: String,
    selector: String,
    parameters: Vec<Parameter>,
    calls: Vec<Call>,
    locals: BTreeSet<String>,
}
#[derive(Debug)]
struct Parameter {
    label: String,
    name: String,
    ty: String,
    default: bool,
}
#[derive(Debug)]
struct Call {
    name: String,
    arguments: Vec<(String, Option<String>)>,
}

impl Index {
    pub(crate) fn new(files: &[(PathBuf, String)]) -> Self {
        let mut parser = Parser::new();
        if parser
            .set_language(&tree_sitter_swift::LANGUAGE.into())
            .is_err()
        {
            return Self::default();
        }
        let trees = files
            .iter()
            .filter_map(|(path, source)| {
                parser.parse(source, None).map(|tree| (path, source, tree))
            })
            .collect::<Vec<_>>();
        let decoder_unshadowed = trees.iter().all(|(_, source, tree)| {
            let mut safe = true;
            walk(tree.root_node(), &mut |node| {
                if node.kind() == "function_declaration"
                    && function_node_name(node, source).as_deref() == Some("decode")
                {
                    safe = false;
                }
                if node.kind() == "simple_identifier" && text(node, source) == "JSONDecoder" {
                    let constructor = node.parent().is_some_and(|parent| {
                        parent.kind() == "call_expression"
                            && compact(text(parent, source)) == "JSONDecoder()"
                    });
                    if !constructor {
                        safe = false;
                    }
                }
            });
            safe
        });
        let mut functions = Vec::new();
        for (path, source, tree) in &trees {
            walk(tree.root_node(), &mut |node| {
                if node.kind() != "function_declaration"
                    || node.has_error()
                    || enclosing_function(node).is_some()
                {
                    return;
                }
                let Some(body) = node.child_by_field_name("body") else {
                    return;
                };
                let Some(name) = function_node_name(node, source) else {
                    return;
                };
                let Some(selector) = swift_callable_selector(node, source) else {
                    return;
                };
                let declaration = text(node, source);
                let Some(open) = declaration.find('(') else {
                    return;
                };
                let Some(close) = matching_delimiter(declaration, open, '(', ')') else {
                    return;
                };
                let Some(parameters) = parameters(&declaration[open + 1..close]) else {
                    return;
                };
                let mut bindings = BTreeMap::<String, Vec<Option<String>>>::new();
                for parameter in &parameters {
                    bindings
                        .entry(parameter.name.clone())
                        .or_default()
                        .push(Some(parameter.ty.clone()));
                }
                let foundation_decoder = decoder_unshadowed
                    && source
                        .lines()
                        .any(|line| line.trim() == "import Foundation");
                body_walk(body, &mut |child| {
                    if child.kind() == "function_declaration" {
                        if let Some(name) = function_node_name(child, source) {
                            bindings.entry(name).or_default().push(None);
                        }
                        return;
                    }
                    if child.kind() == "property_declaration" {
                        if let Some(name) = child
                            .child_by_field_name("name")
                            .map(|name| text(name, source))
                            .filter(|name| identifier(name))
                        {
                            bindings.entry(name.into()).or_default().push(None);
                        }
                    }
                    if child.kind() == "assignment" {
                        if let Some(name) = child
                            .named_child(0)
                            .map(|name| text(name, source))
                            .filter(|name| identifier(name))
                        {
                            bindings.entry(name.into()).or_default().push(None);
                        }
                    }
                    // Guard bindings unwrap the successful optional value. Pair
                    // only an exact bound identifier followed by '=' and one RHS.
                    if child.kind() == "guard_statement" {
                        for i in 0..child.child_count() {
                            if child.field_name_for_child(i as u32) != Some("bound_identifier") {
                                continue;
                            }
                            let Some(name_node) = child.child(i) else {
                                continue;
                            };
                            let name = text(name_node, source);
                            if !identifier(name) {
                                continue;
                            }
                            let rhs = name_node.next_named_sibling().filter(|rhs| {
                                source
                                    .get(name_node.end_byte()..rhs.start_byte())
                                    .is_some_and(|between| between.trim() == "=")
                            });
                            let ty = rhs.and_then(|rhs| {
                                decoded_type(text(rhs, source), foundation_decoder)
                            });
                            bindings.entry(name.into()).or_default().push(ty);
                        }
                    }
                });
                let locals = bindings.keys().cloned().collect();
                let types = bindings
                    .into_iter()
                    .filter_map(|(name, values)| {
                        (values.len() == 1)
                            .then(|| values.into_iter().next().flatten().map(|ty| (name, ty)))
                            .flatten()
                    })
                    .collect::<BTreeMap<_, _>>();
                let mut calls = Vec::new();
                body_walk(body, &mut |call| {
                    if call.kind() != "call_expression" {
                        return;
                    }
                    let Some(callee) = call.named_child(0) else {
                        return;
                    };
                    let name = text(callee, source);
                    if !identifier(name) && !name.strip_prefix("self.").is_some_and(identifier) {
                        return;
                    }
                    let suffix = source
                        .get(callee.end_byte()..call.end_byte())
                        .unwrap_or("")
                        .trim();
                    if !suffix.starts_with('(')
                        || matching_delimiter(suffix, 0, '(', ')') != Some(suffix.len() - 1)
                    {
                        return;
                    }
                    let arguments = split_top_level(&suffix[1..suffix.len() - 1], ',')
                        .into_iter()
                        .filter(|arg| !arg.trim().is_empty())
                        .map(|argument| {
                            let argument = argument.trim();
                            let (label, value) = argument
                                .split_once(':')
                                .filter(|(label, _)| identifier(label.trim()))
                                .map_or(("_", argument), |(label, value)| {
                                    (label.trim(), value.trim())
                                });
                            (label.to_string(), types.get(value).cloned())
                        })
                        .collect();
                    calls.push(Call {
                        name: name.into(),
                        arguments,
                    });
                });
                functions.push(Function {
                    path: (*path).clone(),
                    qualified: tree_sitter_qualified_name(node, source, &name).replace("::", "."),
                    name,
                    selector,
                    parameters,
                    calls,
                    locals,
                });
            });
        }
        Self { functions }
    }

    pub(crate) fn reaches(
        &self,
        start_path: &Path,
        start: &str,
        target_path: Option<&Path>,
        target: &str,
    ) -> bool {
        let roots = self.matches(Some(start_path), start);
        let targets = self.matches(target_path, target);
        roots.len() == 1 && targets.len() == 1 && self.visit(roots[0], targets[0], &BTreeSet::new())
    }
    fn matches(&self, path: Option<&Path>, symbol: &str) -> Vec<usize> {
        self.functions
            .iter()
            .enumerate()
            .filter(|(_, function)| {
                path.is_none_or(|path| function.path == path)
                    && if symbol.contains('(') {
                        symbol == format!("{}{}", function.qualified, function.selector)
                            || (function.qualified == function.name
                                && symbol == format!("{}{}", function.name, function.selector))
                    } else {
                        symbol == function.qualified || symbol == function.name
                    }
            })
            .map(|(index, _)| index)
            .collect()
    }
    fn visit(&self, index: usize, target: usize, seen: &BTreeSet<usize>) -> bool {
        if index == target {
            return true;
        }
        if seen.len() >= 128 || seen.contains(&index) {
            return false;
        }
        let mut seen = seen.clone();
        seen.insert(index);
        let current = &self.functions[index];
        current.calls.iter().any(|call| {
            if current.locals.contains(&call.name) {
                return false;
            }
            let name = call.name.strip_prefix("self.").unwrap_or(&call.name);
            let owner = current.qualified.rsplit_once('.').map(|(owner, _)| owner);
            let same_owner = self.functions.iter().any(|function| {
                function.path == current.path
                    && function.name == name
                    && function.qualified.rsplit_once('.').map(|(owner, _)| owner) == owner
            });
            let candidates = self
                .functions
                .iter()
                .enumerate()
                .filter(|(_, function)| {
                    function.name == name
                        && if same_owner {
                            function.path == current.path
                                && function.qualified.rsplit_once('.').map(|(owner, _)| owner)
                                    == owner
                        } else {
                            !call.name.starts_with("self.") && function.qualified == function.name
                        }
                })
                .filter(|(_, function)| {
                    call.arguments.len() <= function.parameters.len()
                        && function.parameters[call.arguments.len()..]
                            .iter()
                            .all(|parameter| parameter.default)
                        && call.arguments.iter().zip(&function.parameters).all(
                            |((label, ty), parameter)| {
                                label == &parameter.label
                                    && ty.as_ref().is_none_or(|ty| ty == &parameter.ty)
                            },
                        )
                })
                .map(|(index, _)| index)
                .collect::<Vec<_>>();
            !candidates.is_empty()
                && candidates
                    .into_iter()
                    .all(|candidate| self.visit(candidate, target, &seen))
        })
    }
}

fn parameters(source: &str) -> Option<Vec<Parameter>> {
    split_top_level(source, ',')
        .into_iter()
        .filter(|parameter| !parameter.trim().is_empty())
        .map(|parameter| {
            let (names, ty) = parameter.split_once(':')?;
            let names = names.split_whitespace().collect::<Vec<_>>();
            let label = *names.first()?;
            let name = *names.last()?;
            if !identifier(label) || !identifier(name) {
                return None;
            }
            let (ty, default) = ty.split_once('=').map_or((ty, false), |(ty, _)| (ty, true));
            Some(Parameter {
                label: label.into(),
                name: name.into(),
                ty: compact(ty),
                default,
            })
        })
        .collect()
}
fn decoded_type(source: &str, foundation: bool) -> Option<String> {
    if !foundation {
        return None;
    }
    let source = compact(source);
    let call = source.strip_prefix("try?JSONDecoder().decode")?;
    if !call.starts_with('(') || matching_delimiter(call, 0, '(', ')') != Some(call.len() - 1) {
        return None;
    }
    let arguments = split_top_level(&call[1..call.len() - 1], ',');
    if arguments.len() != 2 || !arguments[1].starts_with("from:") {
        return None;
    }
    let ty = arguments[0].strip_suffix(".self")?;
    identifier(ty).then(|| ty.into())
}
fn text<'a>(node: Node<'_>, source: &'a str) -> &'a str {
    node.utf8_text(source.as_bytes()).unwrap_or("")
}
fn compact(source: &str) -> String {
    source.chars().filter(|c| !c.is_whitespace()).collect()
}
fn identifier(source: &str) -> bool {
    !source.is_empty()
        && source.chars().enumerate().all(|(i, c)| {
            c == '_'
                || if i == 0 {
                    c.is_ascii_alphabetic()
                } else {
                    c.is_ascii_alphanumeric()
                }
        })
}
fn walk(node: Node<'_>, visitor: &mut impl FnMut(Node<'_>)) {
    visitor(node);
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        walk(child, visitor);
    }
}
fn body_walk(node: Node<'_>, visitor: &mut impl FnMut(Node<'_>)) {
    visitor(node);
    if node.kind() == "function_declaration" {
        return;
    }
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        body_walk(child, visitor);
    }
}
fn enclosing_function(node: Node<'_>) -> Option<Node<'_>> {
    let mut parent = node.parent();
    while let Some(node) = parent {
        if node.kind() == "function_declaration" {
            return Some(node);
        }
        parent = node.parent();
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    const SOURCE: &str = "import Foundation\nfunc parse(_ data: Data, expected: Int) -> Int {\n guard let wire = try? JSONDecoder().decode(Wire.self, from: data) else { return -1 }\n return parse(wire, expected: expected)\n}\nfunc parse(_ wire: Wire, expected: Int) -> Int { 1 }\nfunc helper(_ cases: Cases, file: String = \"test\") { parse(cases.data, expected: cases.expected) }\nfunc proof() { helper(makeCases()) }\n";
    #[test]
    fn exact_overload_proof_follows_typed_decode_and_keeps_unknown_alternatives() {
        let files = vec![(PathBuf::from("Parser.swift"), SOURCE.into())];
        let index = Index::new(&files);
        assert!(
            index.reaches(
                Path::new("Parser.swift"),
                "proof",
                Some(Path::new("Parser.swift")),
                "parse(_:Wire,expected:Int)"
            ),
            "{index:#?}"
        );
        assert!(!index.reaches(
            Path::new("Parser.swift"),
            "proof",
            None,
            "parse(_:Missing,expected:Int)"
        ));
        for source in [
            SOURCE.replace("return parse(wire, expected: expected)", "return 0"),
            SOURCE.replace("Wire.self", "Other.self"),
            format!("{SOURCE}\nfunc parse(_ other: Other, expected: Int) -> Int {{ 0 }}"),
            SOURCE.replace(
                "func proof() { helper(makeCases()) }",
                "func proof() { let text = \"helper(makeCases())\" }",
            ),
            SOURCE.replace(
                "func proof() { helper(makeCases()) }",
                "func proof(helper: Callback) { helper(makeCases()) }",
            ),
            format!("{SOURCE}\nfunc decode() {{}}"),
        ] {
            let index = Index::new(&[(PathBuf::from("Parser.swift"), source.clone())]);
            assert!(
                !index.reaches(
                    Path::new("Parser.swift"),
                    "proof",
                    None,
                    "parse(_:Wire,expected:Int)"
                ),
                "{source}: {index:#?}"
            );
        }
    }
}
