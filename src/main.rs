use same_file::is_same_file;
use std::{
    collections::HashSet,
    fs, io,
    path::{Path, PathBuf},
};
use structopt::StructOpt;
use walkdir::WalkDir;
use reflink::reflink_or_copy;

#[derive(StructOpt, Debug)]
struct Opt {
    #[structopt(name = "SRC", parse(from_os_str))]
    src: PathBuf,
    #[structopt(name = "DST", parse(from_os_str))]
    dst: PathBuf,
}

fn files(root: &Path) -> io::Result<HashSet<PathBuf>> {
    let mut out = HashSet::new();
    WalkDir::new(root)
        .sort_by(|a, b| a.file_name().cmp(b.file_name()))
        .same_file_system(true)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|i| i.file_type().is_file())
        .for_each(|i| {
            let path = i.into_path();
            let path = path.strip_prefix(root).unwrap();
            out.insert(path.into());
        });
    Ok(out)
}

fn main() -> io::Result<()> {
    let opt = Opt::from_args();

    let src_path = opt.src;
    let dst_path = opt.dst;

    assert!(src_path.is_dir());
    assert!(dst_path.is_dir());
    assert!(!is_same_file(&src_path, &dst_path).unwrap());

    let src = files(&src_path)?;
    let dst = files(&dst_path)?;

    let not_same = src.intersection(&dst).filter(|item| {
        let src_copy = src_path.join(&item);
        let dst_copy = dst_path.join(&item);
        let src_meta = fs::metadata(src_copy).unwrap();
        let dst_meta = fs::metadata(dst_copy).unwrap();

        src_meta.len() != dst_meta.len()
            && src_meta.modified().unwrap() <= dst_meta.modified().unwrap()
    });
    let to_copy = src.difference(&dst).chain(not_same.clone());
    let to_delete = dst.difference(&src).chain(not_same);

    for item in to_delete {
        let item = dst_path.join(&item);
        fs::remove_file(&item).unwrap();
        println!("Delete {:?}", &item);
    }

    for item in to_copy {
        let src = src_path.join(&item);
        let dst = dst_path.join(&item);
        dst.parent().map(|p| fs::create_dir_all(p).unwrap());
        reflink_or_copy(&src, &dst).unwrap();
        println!("Copy {:?} -> {:?}", &src, &dst);
    }

    Ok(())
}
