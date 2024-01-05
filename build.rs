use std::path::PathBuf;

fn main() {
    println!("cargo:rerun-if-changed=opus-1.4");

    let mut config = cc::Build::new();

    config.define("OPUS_BUILD", None);
    config.define("DISABLE_FLOAT_API", None);
    config.define("FIXED_POINT", None);
    config.define("VAR_ARRAYS", None);
    config.define("OVERRIDE_OPUS_ALLOC", None);
    config.define("opus_alloc(x)", "0");
    config.define("OVERRIDE_OPUS_FREE", None);
    config.define("opus_free(x)", "0");

    config.includes([
        "opus-1.4/include",
        "opus-1.4/celt",
        "opus-1.4/silk",
        "opus-1.4/silk/fixed",
    ]);

    config.opt_level(2);

    config.flag_if_supported("-Wno-unused-parameter");
    config.flag_if_supported("-Wno-unused-value");
    config.flag_if_supported("-Wno-stringop-overread"); // TODO: investigate whether warning is valid
    config.flag_if_supported("-Wno-maybe-uninitialized"); // dito

    let mut files = vec![];

    add_sources(&mut files, "opus-1.4/celt/*.c");
    add_sources(&mut files, "opus-1.4/silk/*.c");
    add_sources(&mut files, "opus-1.4/silk/fixed/*.c");
    add_sources(&mut files, "opus-1.4/src/opus.c");
    add_sources(&mut files, "opus-1.4/src/opus_??coder.c");
    add_sources(&mut files, "opus-1.4/src/repacketizer.c");

    exclude(&mut files, "**/_custom*");

    config.files(files);

    config.compile("opus-cc");
}

fn add_sources(files: &mut Vec<PathBuf>, pattern: &str) {
    for file in glob::glob(pattern).unwrap() {
        files.push(file.unwrap());
    }
}

fn exclude(files: &mut Vec<PathBuf>, pattern: &str) {
    let pattern = glob::Pattern::new(pattern).unwrap();
    files.retain(|x| !pattern.matches_path(x));
}
