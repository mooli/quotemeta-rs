//! Shell-quoting, à la Perl's `quotemeta` function.
//!
//! This crate currently provides a single [`quotemeta`] function which shell-escapes a filename or
//! other data. It is anticipated that it may expand to include fine-tuning of the escaping
//! strategy, but for now it will return the input as-is if there are no troublesome characters,
//! otherwise single-quoted if it is printable ASCII without single-quotes, otherwise it'll break
//! out the big guns of ["ANSI-C
//! Quoted"](https://www.gnu.org/software/bash/manual/html_node/ANSI_002dC-Quoting.html#ANSI_002dC-Quoting)
//! for input which contains control codes or UTF-8 text.
//!

//// -- start of boilerplate that's generally pasted into the top of new projects -- ////
#![cfg_attr(feature="clippy-insane", warn(
    //// Turn on the "allow" lints currently listed by `rustc -W help` (as of 2019-11-06) into warn
    //// lints, unless they're not useful:
    absolute_paths_not_starting_with_crate, anonymous_parameters,
    // box_pointers, //// obsolete
    deprecated_in_future,
    // elided_lifetimes_in_paths,  //// suggests adding dubious <'_> noise everywhere
    explicit_outlives_requirements, indirect_structural_match, keyword_idents,
    macro_use_extern_crate, meta_variable_misuse,
    missing_copy_implementations,  //// too noisy; enable and inspect before release
    missing_debug_implementations, //// too noisy; enable and inspect before release
    missing_docs,                  //// too noisy; enable and inspect before release
    // missing_doc_code_examples,     //// too noisy; enable and inspect before release
    non_ascii_idents,
    private_doc_tests,          //// broken; still complains if "private" item is pub-used
    single_use_lifetimes,       //// gets confused too easily by macros
    trivial_casts, trivial_numeric_casts,
    unreachable_pub,            //// too noisy; enable and inspect before release
    unsafe_code,
    unstable_features, //// silly; explicit use of #![feature] already indicates opt-in
    unused_extern_crates, unused_import_braces, unused_labels, unused_lifetimes,
    unused_qualifications, unused_results, variant_size_differences,
    //// Ditto for clippy lint categories (see https://github.com/rust-lang/rust-clippy):
    clippy::all, clippy::pedantic, clippy::nursery,
    clippy::cargo,
    clippy::restriction
))]
#![cfg_attr(feature="clippy-insane", allow(
    // //// turn off individual noisy/buggy clippy lints:
    // clippy::doc_markdown,
    // clippy::use_self,             //// gets easily confused by macros
    // clippy::cast_possible_truncation,
    // clippy::missing_const_for_fn,
    // clippy::similar_names,
    // clippy::pub_enum_variant_names,
    // //// from clippy::restriction:
    clippy::implicit_return,    //// bad style
    // clippy::integer_arithmetic, clippy::integer_division, //// uh-huh
    // clippy::float_arithmetic,
    clippy::missing_docs_in_private_items, //// too noisy; enable and inspect before release
    clippy::missing_inline_in_public_items, //// just moans about all public items
    // // clippy::multiple_inherent_impl,      //// breaks with e.g. derive macros
    // clippy::shadow_reuse,                //// e.g. `let foo = bar(foo)`
    // clippy::shadow_same,                 //// e.g. `let foo = &foo`
    // clippy::mem_forget,                  //// triggered by no_panic macro
    // clippy::non_ascii_literal,
    // clippy::option_expect_used, clippy::result_expect_used, //// .expect() used for bug assertions
    // clippy::panic,                                          //// panic!() used for bug assertions
    // clippy::empty_line_after_outer_attr,                    //// gets easily confused
    // clippy::wildcard_enum_match_arm,
))]
//// #[no_panic] generates code which triggers clippy::mem_forget
//#![cfg_attr(all(feature = "clippy-insane", feature = "no-panic"), allow(clippy::mem_forget))]
//// -- end of boilerplate that's generally pasted into the top of new projects -- ////

#[cfg(unix)] use std::os::unix::ffi::OsStrExt;
use std::path::Path;

fn quotemeta_inner(s: &[u8]) -> String {
    let (mut single_quoted, mut c_quoted) = (false, false);
    let s = s
        .iter()
        .map(|&c| match c {
            // These characters are safe to use without quoting or escaping.
            b'+'
            | b','
            | b'-'
            | b'.'
            | b'/'
            | b'0' ..= b'9'
            | b':'
            | b'='
            | b'@'
            | b'A' ..= b'Z'
            | b'_'
            | b'a' ..= b'z' => char::from(c).to_string(),
            // Control and high-bit-set characters require C-quoting and \ooo-escaping.
            0 ..= 31 | 127 ..= 255 => {
                c_quoted = true;
                format!(r"\{:03o}", c)
            }
            // A single quote or backslash must be C-quoted and backslash-escaped. Technically, we
            // can get away with just single-quoting backslashes, but they then must _not_ be
            // backslash-escaped. Since we don't know if a subsequent character might need to be
            // C-quoted, we play it safe.
            b'\'' | b'\\' => {
                c_quoted = true;
                format!(r"\{}", char::from(c))
            }
            // Other characters are safe provided they are at least single-quoted.
            _ => {
                single_quoted = true;
                char::from(c).to_string()
            }
        })
        .collect();
    match (c_quoted, single_quoted) {
        (true, _) => format!("$'{}'", s),
        (false, true) => format!("'{}'", s),
        (false, false) => s,
    }
}

/// Shell-quotes the given [`Path`].
///
/// This takes any `AsRef<Path>`, so accepts `&str`/`String`, `&Path`/`PathBuf`, `OsStr`/`OsString`,
/// and so on.
///
/// ```
/// use quotemeta::quotemeta;
///
/// // "Boring" Unix paths do not need to be quoted.
/// assert_eq!(&quotemeta("/bin/cat"), "/bin/cat");
/// // Spaces etc are single-quoted.
/// assert_eq!(&quotemeta("Hello, world"), "'Hello, world'");
/// // Unicode gets C-quoted.
/// assert_eq!(&quotemeta("\u{1f980}"), r"$'\360\237\246\200'");
/// ```
pub fn quotemeta(s: impl AsRef<Path>) -> String {
    quotemeta_inner(s.as_ref().as_os_str().as_bytes())
}

#[cfg(test)]
mod tests {
    use crate::quotemeta;
    #[cfg(unix)] use std::os::unix::ffi::OsStrExt;
    use std::{
        ffi::{OsStr, OsString},
        path::{Path, PathBuf},
    };

    #[test]
    fn test_quotemeta() {
        assert_eq!(&quotemeta(""), "");
        assert_eq!(&quotemeta("test"), "test");
        assert_eq!(&quotemeta("Hello, world!"), "'Hello, world!'");
        assert_eq!(&quotemeta("isn't"), r"$'isn\'t'");
        assert_eq!(&quotemeta(r"isn\t"), r"$'isn\\t'");
        // Octal escapes are unambiguous.
        assert_eq!(&quotemeta("\n3"), r"$'\0123'");
        // Valid UTF-8
        assert_eq!(&quotemeta("\u{a3}"), r"$'\302\243'");
        // Invalid UTF-8 (in this case, Latin-1.)
        assert_eq!(&quotemeta(OsStr::from_bytes(&[0xa3])), r"$'\243'");
    }

    // merely a compilation test to ensure that we accept the given types.
    #[test]
    fn test_types() {
        let _ = quotemeta("");
        let _ = quotemeta("".to_string());
        let _ = quotemeta(Path::new(""));
        let _ = quotemeta(PathBuf::new());
        let _ = quotemeta(OsStr::new(""));
        let _ = quotemeta(OsString::new());
    }
}
