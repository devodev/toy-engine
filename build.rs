// ref: https://falseidolfactory.com/2018/06/23/compiling-glsl-to-spirv-at-build-time.html
// ref: https://github.com/google/shaderc-rs
use std::{error::Error, path::Path};

const SHADERS_SRC: &str = "assets/shaders";

// compile GLSL shaders located in SHADERS_SRC to SPIR-V
fn main() -> Result<(), Box<dyn Error>> {
    // Tell the build script to only run again if we change our source shaders
    println!("cargo:rerun-if-changed={SHADERS_SRC}");

    for entry in
        std::fs::read_dir(SHADERS_SRC).map_err(|e| format!("read shaders src dir: {e:?}"))?
    {
        let entry = entry?;

        if entry.file_type()?.is_file() {
            let in_path = entry.path();

            // determine shader type
            let shader_type =
                in_path
                    .extension()
                    .and_then(|ext| match ext.to_string_lossy().as_ref() {
                        "vert" => Some(shaderc::ShaderKind::Vertex),
                        "frag" => Some(shaderc::ShaderKind::Fragment),
                        _ => None,
                    });

            if let Some(shader_type) = shader_type {
                // read glsl into string
                let source_shader_text = std::fs::read_to_string(&in_path)
                    .map_err(|e| format!("read shader file to string: {e:?}"))?;

                // compile glsl string to spirv binary
                let compiler = shaderc::Compiler::new().ok_or("create shaderc compiler")?;
                let options =
                    shaderc::CompileOptions::new().ok_or("create shaderc compiler options")?;
                let compiled_shader_binary = compiler.compile_into_spirv(
                    &source_shader_text,
                    shader_type,
                    &in_path.display().to_string(),
                    "main",
                    Some(&options),
                )?;

                // Write compiled (binary) spirv shader
                let out_path = Path::new(SHADERS_SRC).join(format!(
                    "{}.spv",
                    in_path.file_name().unwrap().to_string_lossy()
                ));
                std::fs::write(&out_path, compiled_shader_binary.as_binary_u8())
                    .map_err(|e| format!("write compiled shader: {e:?}"))?;
            }
        }
    }

    Ok(())
}
