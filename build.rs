use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;
use std::f32::consts::PI;

fn main() {
    // Put the linker script somewhere the linker can find it
    let out = PathBuf::from(std::env::var_os("OUT_DIR").unwrap());
    let memory_x = include_bytes!("memory.x");
    let mut f = File::create(out.join("memory.x")).unwrap();
    f.write_all(memory_x).unwrap();
    println!("cargo:rustc-link-search={}", out.display());
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=memory.x");

    generate_sin_table();
}

fn generate_sin_table() {
    const TABLE_SIZE: usize = 512;
    const AMPLITUDE: f32 = i16::MAX as f32;

    let out_dir = std::env::var("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("sine_table.rs");
    let mut file = File::create(dest_path).unwrap();

    writeln!(file, "static SINE_TABLE: [u32; {}] = [", TABLE_SIZE).unwrap();

    for i in 0..TABLE_SIZE {
        let phase = (i as f32 / TABLE_SIZE as f32) * 2.0 * PI;
        let sample_i16 = (phase.sin() * AMPLITUDE).round() as i16;
        let sample_u16 = sample_i16 as u16;
        // duplicate into both halves
        let word = ((sample_u16 as u32) << 16) | sample_u16 as u32;
        writeln!(file, "    0x{:08X}, // Amplitude: {}", word, sample_i16).unwrap();
    }

    writeln!(file, "];").unwrap();
}