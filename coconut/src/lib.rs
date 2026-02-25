pub use traversal_generator_derive::generate_traversal;

pub struct NoTrav<T>(pub T);

pub mod parser {
    pub fn c_preprocessor<P: AsRef<std::path::Path>, P2: AsRef<std::path::Path>>(p: P, system_dir: P2) -> Option<Vec<u8>> {
        use std::process::*;
        let mut sd = std::ffi::OsString::from("-I".to_string());
        sd.push(system_dir.as_ref().as_os_str());
        let o = Command::new("cpp")
            .arg("-E")
            .arg("-P")
            .arg(sd)
            .arg(p.as_ref())
            .stderr(Stdio::inherit())
            .output()
            .expect("Failed to run cpp (c preprocessor)");
        if !o.status.success() {return None;}
        Some(o.stdout)
    }

    pub fn print_errors(tree: &tree_sitter::Tree, bytes: &[u8]) -> bool {
        let mut has_errors = false;
        let mut cursor = tree.walk();
        loop {
            let node = cursor.node();

            if node.is_error() || node.is_missing() {
                eprintln!("\x1b[31mSyntax error:\x1b[0m");
                let range = node.byte_range();
                let mut i = bytes[..range.start].iter().rposition(|c| *c == b'\n').map(|i|i+1).unwrap_or(0);
                use std::io::Write;
                let mut stderr = std::io::stderr();
                let mut buf1: Vec<u8> = Vec::new();
                let mut buf2: Vec<u8> = vec![b'\x1b', b'[', b'3', b'4', b'm'];
                while i < bytes.len() {
                    let marker: u8 = if i == range.start {b'^'} else if range.contains(&i) {b'~'} else {b' '};
                    match bytes[i] {
                        b'\r' => {
                            buf1.push(b'\r');
                            buf2.push(b'\r');
                        }
                        b'\n' => {
                            buf1.push(b'\n');
                            buf2.push(b'\n');
                            buf2.push(b'\x1b');
                            buf2.push(b'[');
                            buf2.push(b'0');
                            buf2.push(b'm');
                            stderr.write_all(&buf1).unwrap();
                            stderr.write_all(&buf2).unwrap();
                            buf1.clear();
                            buf2.truncate(5);
                            if i >= range.end {break;}
                        }
                        b'\t' => {
                            for _ in 0..4 {
                                buf1.push(b' ');
                                buf2.push(marker);
                            }
                        }
                        c => {
                            buf1.push(c);
                            buf2.push(marker);
                        }
                    }
                    i += 1;
                }
                has_errors = true;
            }
            else if cursor.goto_first_child() {continue;}
            if cursor.goto_next_sibling() {continue;}

            loop {
                if !cursor.goto_parent() {return has_errors;}
                if cursor.goto_next_sibling() {break;}
            }
        }
    }
}
