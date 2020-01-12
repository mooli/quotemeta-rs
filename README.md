Shell-quoting, à la Perl's `quotemeta` function.

One idiom when writing simple shell tools in Rust (or Perl or whatever) is to spit out a shell
script or shell fragment for later perusal and/or piping directly into a shell. However, a
simple `println!` generates an incorrect and potentially insecure script if the filename happens
to contain shell metacharacters. This includes the not exactly uncommon space character. Perl
includes a `quotemeta` function which _usually_ does the job. Let's do the job properly!

(Actually, you can't even `println!` a plain `Path`/`PathBuf` because they don't implement
`Display`.)

This crate provides a function which quotes and escapes filenames (or other data) such that it
can be interpolated. For example:

```
use quotemeta::quotemeta;

fn main() {
    for path in std::env::args_os().skip(1) {
        println!("cat {}", quotemeta(path));
    }
}
```

This will work even if the filename contains a carriage return, or is invalid UTF-8.
