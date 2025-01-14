use std::io::Write;

fn main() {
    let syscall_in = match std::env::var("CARGO_CFG_TARGET_ARCH") {
        Ok(s) if s == "riscv64" => "src/riscv64_syscall.h.in",
        _ => "src/syscall.h.in",
    };
    println!("cargo:rerun-if-changed={}", syscall_in);

    let mut fout = std::fs::File::create(std::env::var("OUT_DIR").unwrap() + "/consts.rs").unwrap();
    writeln!(fout, "// Generated by build.rs. DO NOT EDIT.").unwrap();
    writeln!(fout, "use numeric_enum_macro::numeric_enum;\n").unwrap();
    writeln!(fout, "numeric_enum! {{").unwrap();
    writeln!(fout, "#[repr(u32)]").unwrap();
    writeln!(fout, "#[derive(Debug, Eq, PartialEq)]").unwrap();
    writeln!(fout, "#[allow(non_camel_case_types)]").unwrap();
    writeln!(fout, "pub enum SyscallType {{").unwrap();

    let data = std::fs::read_to_string(syscall_in).unwrap();

    for line in data.lines() {
        if !line.starts_with("#define") {
            continue;
        }
        let mut iter = line.split_whitespace();
        let _ = iter.next().unwrap();
        let name = iter.next().unwrap();
        let id = iter.next().unwrap();

        let name = &name[5..].to_uppercase();
        writeln!(fout, "    {} = {},", name, id).unwrap();
    }
    writeln!(fout, "}}").unwrap();
    writeln!(fout, "}}").unwrap();
}
