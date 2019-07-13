use std::env;
use std::ffi::OsString;
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};
use std::io;
use std::fmt;
use std::fs::{remove_file, File, OpenOptions};

const NUM_RETRIES: usize = 1 << 31;

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
        basename.push(".tmp_");
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

// TODO tempfile


pub struct TempPath {
    path: PathBuf,
}

impl fmt::Debug for TempPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.path.fmt(f)
    }
}

impl Drop for TempPath {
    fn drop(&mut self) {
        let _ = remove_file(&self.path);
    }
}

// impl Deref for TempPath {
//     type Target = Path;

//     fn deref(&self) -> &Path {
//         &self.path
//     }
// }

impl AsRef<Path> for TempPath {
    fn as_ref(&self) -> &Path {
        &self.path
    }
}

// impl AsRef<OsStr> for TempPath {
//     fn as_ref(&self) -> &OsStr {
//         self.path.as_os_str()
//     }
// }


pub struct NamedTempFile {
    path: TempPath,
    file: File,
}


impl fmt::Debug for NamedTempFile {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "NamedTempFile({:?})", self.path.path)
    }
}


impl NamedTempFile {
    /// Create a new named temporary file.
    ///
    /// # Security
    ///
    /// This will create a temporary file in the default temporary file
    /// directory (platform dependent). This has security implications on many
    /// platforms so please read the security section of this type's
    /// documentation.
    ///
    /// Reasons to use this method:
    ///
    ///   1. The file has a short lifetime and your temporary file cleaner is
    ///      sane (doesn't delete recently accessed files).
    ///
    ///   2. You trust every user on your system (i.e. you are the only user).
    ///
    ///   3. You have disabled your system's temporary file cleaner or verified
    ///      that your system doesn't have a temporary file cleaner.
    ///
    /// Reasons not to use this method:
    ///
    ///   1. You'll fix it later. No you won't.
    ///
    ///   2. You don't care about the security of the temporary file. If none of
    ///      the "reasons to use this method" apply, referring to a temporary
    ///      file by name may allow an attacker to create/overwrite your
    ///      non-temporary files. There are exceptions but if you don't already
    ///      know them, don't use this method.
    ///
    /// # Errors
    ///
    /// If the file can not be created, `Err` is returned.
    ///
    /// # Examples
    ///
    /// Create a named temporary file and write some data to it:
    ///
    /// ```no_run
    /// # use std::io::{self, Write};
    /// use tempfile::NamedTempFile;
    ///
    /// # fn main() {
    /// #     if let Err(_) = run() {
    /// #         ::std::process::exit(1);
    /// #     }
    /// # }
    /// # fn run() -> Result<(), ::std::io::Error> {
    /// let mut file = NamedTempFile::new()?;
    ///
    /// writeln!(file, "Brian was here. Briefly.")?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// [`Builder`]: struct.Builder.html
    // pub fn new() -> io::Result<NamedTempFile> {
    //     NamedTempFile::new_in(&env::temp_dir())
    // }

    /// Create a new named temporary file in the specified directory.
    ///
    /// See [`NamedTempFile::new()`] for details.
    ///
    /// [`NamedTempFile::new()`]: #method.new_in
    // pub fn new_in(path: &Path) -> io::Result<NamedTempFile> {
    //     NamedTempFile::new_full(path)
    // }

    pub fn new_full<G>(
        path: &Path,
        num_retries: usize,
        get_random: G,
        ) -> io::Result<NamedTempFile>
    where
        G: Fn() -> OsString
    {

        let tmp;
        let mut path = path;
        if !path.is_absolute() {
            // path = env::current_dir()?.join(path)
            let cur_dir = env::current_dir()?;
            tmp = cur_dir.join(path);
            path = &tmp;
        }

        let f = |fpath: PathBuf| {
            OpenOptions::new()
                .read(true)
                .write(true)
                .create_new(true)
                .mode(0o600)
                .open(&fpath)
                // .with_err_path(|| path.clone())
                .map(|file| NamedTempFile {
                    path: TempPath { path: fpath.to_path_buf() },
                    file,
                })
        };

        create_helper(
            path,
            num_retries,
            get_random,
            f
        )

    }

}



#[cfg(test)]
mod tests {
    use std::error::Error;
    use super::*;
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
            &Path::new("target"),
            || OsString::from("the_filename.txt"),
        ).unwrap();
        f.write_all(b"hello world").unwrap();
        f.sync_all().unwrap();
        assert_eq!(
            f.metadata().unwrap().len(),
            11
        );
    }

    #[test]
    fn test_namedtempfile_new_in() {
        if Path::new("target/.tmp_namedtempfile.txt").exists() {
            remove_file(Path::new("target/.tmp_namedtempfile.txt")).unwrap();
        }
        let mut namedtempfile = NamedTempFile::new_full(
            &Path::new("target"),
            1,
            || OsString::from("namedtempfile.txt"),
        ).unwrap();
        assert_eq!(
            namedtempfile.path.path.as_os_str(),
            env::current_dir().unwrap().join(
                OsString::from("target/.tmp_namedtempfile.txt")
            )
        );
        assert!(Path::new("target/.tmp_namedtempfile.txt").exists());
        namedtempfile.file.write_all(b"hello world").unwrap();
        namedtempfile.file.sync_all().unwrap();
        assert_eq!(
            namedtempfile.file.metadata().unwrap().len(),
            11
        );
        drop(namedtempfile);
        assert!(!Path::new("target/.tmp_namedtempfile.txt").exists());
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


