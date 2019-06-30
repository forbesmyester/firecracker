use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::io;


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

#[cfg(test)]
mod tests {
    use std::error::Error;
    use super::*;

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

}


