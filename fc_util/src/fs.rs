use std::env;
use std::ffi::OsString;
use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
use std::path::{Path, PathBuf};
use std::io;
use std::fmt;
use std::fs::{remove_file, File, OpenOptions};

const NUM_RETRIES: usize = 1 << 31;

pub struct TempPath {
    path: PathBuf,
}

pub struct NamedTempFile {
    path: TempPath,
    file: File,
}

impl fmt::Debug for NamedTempFile {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "NamedTempFile({:?})", self.path.path)
    }
}



fn create_helper<F, G, R>(
    parent: &Path,
    creation_attempts: usize,
    get_random: G,
    creator: F,
) -> io::Result<R>
where
    F: Fn(PathBuf) -> io::Result<R>,
    G: Fn() -> OsString,
{
    let mut i: usize = 0;
    while i < creation_attempts {
        i+=1;
        let rand: OsString = get_random();
        let mut basename = OsString::with_capacity(rand.len() + 4);
        basename.push(".tmp");
        basename.push(rand);
        let path = parent.join(basename);
        return match creator(path) {
            Err(ref e) if e.kind() == io::ErrorKind::AlreadyExists => continue,
            res => res,
        };
    }

    Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            match parent.to_str() {
                Some(s) => format!("Too many ({}) temporary creation attempts within {}", i, s),
                None => format!("Too many ({}) temporary creation attempts", i)
            }
    ))
}

fn create_unlinked(path: &Path) -> io::Result<File> {
    let tmp;
    // shadow this to decrease the lifetime. It can't live longer than `tmp`.
    let mut path = path;
    if !path.is_absolute() {
        let cur_dir = env::current_dir()?;
        tmp = cur_dir.join(path);
        path = &tmp;
    }

    let f = create_named(path)?;
    // don't care whether the path has already been unlinked,
    // but perhaps there are some IO error conditions we should send up?
    let _ = remove_file(path);
    Ok(f)
}


fn tempfile_in<G>(
    dir: &Path,
    get_random: G,
) -> io::Result<File>
where
    G: Fn() -> OsString,
{
    use libc::{EISDIR, ENOENT, EOPNOTSUPP, O_EXCL, O_TMPFILE};
    OpenOptions::new()
        .read(true)
        .write(true)
        .custom_flags(O_TMPFILE | O_EXCL) // do not mix with `create_new(true)`
        .open(dir)
        .or_else(|e| {
            match e.raw_os_error() {
                // These are the three "not supported" error codes for O_TMPFILE.
                Some(EOPNOTSUPP) | Some(EISDIR) | Some(ENOENT) => {
                    create_helper(
                        dir,
                        NUM_RETRIES,
                        get_random,
                        |path| create_unlinked(&path),
                    )
                }
                _ => Err(e),
            }
        })
}


#[cfg(test)]
mod tests {
    use std::error::Error;
    use super::*;
    use std::io::BufReader;
    use std::io::prelude::*;

    #[test]
    fn test_create_helper_success() {
        assert_eq!(
            create_helper(
                Path::new("/tmp"),
                1,
                || OsString::from("filename.txt"),
                |_p| Result::Ok("X")
            ).unwrap(),
            "X"
        );
    }

    #[test]
    fn test_create_helper_fail() {

        fn creator (_p: PathBuf) -> io::Result<String> {
            Result::Err(std::io::Error::new(io::ErrorKind::AlreadyExists, "Nope"))
        }

        assert_eq!(
            create_helper(
                Path::new("/tmp"),
                128,
                || OsString::from("filename.txt"),
                creator
            ).unwrap_err().description(),
            "Too many (128) temporary creation attempts within /tmp"
        );
    }

    #[test]
    fn test_tempfile_in() {
        let mut f = tempfile_in(
            &Path::new("./target"),
            || OsString::from("the_filename.txt"),
        ).unwrap();
        f.write_all(b"hello world").unwrap();
        f.sync_all().unwrap();
        assert_eq!(
            f.metadata().unwrap().len(),
            11
        );

    }
}


// pub fn create(dir: &Path) -> io::Result<File> {
//     use libc::{EISDIR, ENOENT, EOPNOTSUPP, O_EXCL, O_TMPFILE};
//     OpenOptions::new()
//         .read(true)
//         .write(true)
//         .custom_flags(O_TMPFILE | O_EXCL) // do not mix with `create_new(true)`
//         .open(dir)
//         .or_else(|e| {
//             match e.raw_os_error() {
//                 // These are the three "not supported" error codes for O_TMPFILE.
//                 Some(EOPNOTSUPP) | Some(EISDIR) | Some(ENOENT) => create_unix(dir),
//                 _ => Err(e),
//             }
//         })
// }

//     pub fn tempfile_in<P: AsRef<Path>>(&self, dir: P) -> io::Result<NamedTempFile> {
//         util::create_helper(
//             dir.as_ref(),
//             self.prefix,
//             self.suffix,
//             self.random_len,
//             file::create_named,
//         )
//     }

// fn create_unix(dir: &Path) -> io::Result<File> {
//     util::create_helper(
//         dir,
//         OsStr::new(".tmp"),
//         OsStr::new(""),
//         ::NUM_RAND_CHARS,
//         |path| create_unlinked(&path),
//     )
// }

// fn create_unlinked(path: &Path) -> io::Result<File> {
//     let tmp;
//     // shadow this to decrease the lifetime. It can't live longer than `tmp`.
//     let mut path = path;
//     if !path.is_absolute() {
//         let cur_dir = env::current_dir()?;
//         tmp = cur_dir.join(path);
//         path = &tmp;
//     }

//     let f = create_named(path)?;
//     // don't care whether the path has already been unlinked,
//     // but perhaps there are some IO error conditions we should send up?
//     let _ = fs::remove_file(path);
//     Ok(f)
// }

pub fn create_named(path: &Path) -> io::Result<File> {
    OpenOptions::new()
        .read(true)
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)
}


