//! Production build runner (`yav build`).
//!
//! Orchestrates release compilation, minified Tailwind CSS generation,
//! asset fingerprinting, and manifest generation via `yav_build::BuildPipeline`.

use yav_build::BuildPipeline;

/// Executes the `yav build` command.
pub async fn run() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n  \x1b[1;36myav build\x1b[0m \x1b[90m— Production Build\x1b[0m\n");

    let pipeline = match BuildPipeline::from_root(".") {
        Ok(p) => p,
        Err(e) => {
            eprintln!("\x1b[1;31m✖ Build failed: invalid package.yavml:\x1b[0m\n  {e}\n");
            return Err(e.into());
        }
    };
    let config = pipeline.config();

    println!(
        "  ● Project: \x1b[1m{}\x1b[0m v{}",
        config.project.name, config.project.version
    );
    println!("  Compiling release target and optimizing assets...\n");

    let summary = pipeline.run_prod_build().await?;

    println!(
        "\n\x1b[1;32m✔ Build finished successfully in {:.2?}\x1b[0m",
        summary.duration
    );
    println!(
        "  • Tailwind CSS minified: {}",
        if summary.tailwind_built {
            "\x1b[32m✔ Yes\x1b[0m"
        } else {
            "\x1b[90m— Skipped/Disabled\x1b[0m"
        }
    );
    println!("  • Static assets processed: {}", summary.assets_copied);
    if let Some(ref manifest) = summary.manifest_path {
        println!("  • Asset manifest written to: {}", manifest.display());
    }
    println!("  • Output directory: {}", config.build.out_dir);
    println!("\n\x1b[1;32m✔ Production build ready for deployment\x1b[0m");

    Ok(())
}
