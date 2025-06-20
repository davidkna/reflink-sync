use std::{
    collections::HashSet,
    fs, io,
    path::{Path, PathBuf},
};

use clap::Parser;
use reflink::reflink_or_copy;
use same_file::is_same_file;
use walkdir::WalkDir;

#[derive(Parser, Debug)]
struct Opt {
    #[clap(name = "SRC")]
    src: PathBuf,
    #[clap(name = "DST")]
    dst: PathBuf,
}

fn files(root: &Path) -> HashSet<PathBuf> {
    WalkDir::new(root)
        .sort_by(|a, b| a.file_name().cmp(b.file_name()))
        .same_file_system(true)
        .into_iter()
        .filter_map(std::result::Result::ok)
        .filter(|i| i.file_type().is_file())
        .fold(HashSet::new(), |mut out, i| {
            let path = i.into_path();
            let path = path.strip_prefix(root).unwrap();
            out.insert(path.into());
            out
        })
}

fn sync(src_path: &Path, dst_path: &Path) -> io::Result<()> {
    assert!(!is_same_file(src_path, dst_path).unwrap());
    assert!(src_path.is_dir());

    let src = files(src_path);
    let dst = files(dst_path);

    let not_same: Vec<PathBuf> = src
        .intersection(&dst)
        .filter(|item| {
            let src_copy = src_path.join(item);
            let dst_copy = dst_path.join(item);
            let src_meta = fs::metadata(src_copy).unwrap();
            let dst_meta = fs::metadata(dst_copy).unwrap();

            src_meta.len() != dst_meta.len()
                || src_meta.modified().unwrap() > dst_meta.modified().unwrap()
        })
        .cloned()
        .collect();
    let to_copy = src.difference(&dst).chain(not_same.iter());
    let to_delete = dst.difference(&src).chain(not_same.iter());

    for item in to_delete {
        let item = dst_path.join(item);
        fs::remove_file(&item).unwrap();
        println!("Delete {:?}", &item);
    }

    for item in to_copy {
        let src = src_path.join(item);
        let dst = dst_path.join(item);

        if !dst.parent().is_some_and(std::path::Path::exists) {
            continue;
        }

        reflink_or_copy(&src, &dst)?;
        println!("Copy {:?} -> {:?}", &src, &dst);
    }

    Ok(())
}

fn main() -> io::Result<()> {
    let opt = Opt::parse();

    let src_path = opt.src;
    let dst_path = opt.dst;

    dst_path
        .read_dir()?
        .filter_map(std::result::Result::ok)
        .filter(|i| i.file_type().unwrap().is_dir())
        .for_each(|i| {
            let path = i.path();
            let prefix = path.strip_prefix(&dst_path).unwrap();
            let src = src_path.join(prefix);
            let dst = dst_path.join(prefix);

            if src.exists() && src.is_dir() {
                sync(&src, &dst).unwrap();
            }
        });

    Ok(())
}
