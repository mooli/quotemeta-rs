use quotemeta::quotemeta;

fn main() {
    for path in std::env::args_os().skip(1) {
        println!("cat {}", quotemeta(path));
    }
}
