//! A one-off compile used only to ask "what would this tag's call look like
//! closed" — never stored, never diagnosed, never shown to the editor as the
//! file's own state.
//!
//! An element still being typed — `<Button v`, with no `>` anywhere yet — is a
//! genuine parse error (see `luaux::compile`'s own tests), so it produces no
//! generated Luau at all, and [`crate::server`]'s derived positions
//! (`props_table`, `member_position`) have nothing to look inside. But nothing
//! *before* the cursor needs to change to answer "what are this component's
//! props": splicing a `/>` in right there is always syntactically valid — a
//! bare attribute name is legal LuauX shorthand for a boolean prop — and
//! leaves every earlier byte, and so every earlier run in a fresh map,
//! exactly where it was.

use crate::sourcemap::SourceMap;
use luaux::Config;

/// The patched source, its compile, and its own fresh map — all valid only
/// together, since the map describes exactly this `source`/`output` pair and
/// nothing else.
pub struct Synthetic {
    pub source: String,
    pub output: String,
    pub map: SourceMap,
}

/// Inserts `/>` at `insert_at` and recompiles the patched text from scratch.
///
/// `None` if even the patched text does not compile — a component that does
/// not resolve is still not going to resolve once closed, and this is one
/// more attempt, not a second way to guess.
pub fn close_and_compile(source: &str, insert_at: usize, config: &Config) -> Option<Synthetic> {
    let mut patched = String::with_capacity(source.len() + 2);
    patched.push_str(source.get(..insert_at)?);
    patched.push_str("/>");
    patched.push_str(source.get(insert_at..)?);

    let compiled =
        luaux::compile::compile_recovering(&patched, crate::backend(config), config.clone())
            .ok()?;
    let map = crate::map_builder::build(&patched, &compiled.output, config);

    Some(Synthetic { source: patched, output: compiled.output, map })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn closes_an_unclosed_tag_and_still_compiles() {
        let config = Config::with_create("create");
        let source = "local create = f()\nlocal function Row(p) return p end\nlocal e = <Row Na";

        let synthetic = close_and_compile(source, source.len(), &config).expect("compile");
        assert!(synthetic.output.contains("Row({"), "{}", synthetic.output);
    }

    #[test]
    fn nothing_before_the_insertion_point_changes() {
        let config = Config::with_create("create");
        let source = "local create = f()\nlocal function Row(p) return p end\nlocal a = <Row/>\nlocal e = <Row Na";
        let insert_at = source.len();

        let synthetic = close_and_compile(source, insert_at, &config).expect("compile");

        // The earlier, already-closed `Row` still compiled to its own call,
        // undisturbed by the patch further down.
        assert_eq!(synthetic.output.matches("Row({})").count(), 1, "{}", synthetic.output);
    }
}
