use std::{collections::VecDeque, io::Write, path::PathBuf};

use crate::{
    template::{ReplacementCount, Template},
    Error, ProgramArgs,
};

pub type FromDir = PathBuf;
pub type ToDir = PathBuf;

#[derive(Debug, Clone)]
pub enum FileOp {
    Parse(FromDir, ToDir),
    #[allow(unused)]
    Skip(FromDir),
    #[allow(unused)]
    Simlink(FromDir, ToDir),
}

impl FileOp {
    pub fn execute(
        self,
        args: &ProgramArgs,
    ) -> Result<(ReplacementCount, FromDir, Option<ToDir>), (FromDir, Error)> {
        match self {
            FileOp::Parse(fin, fout) => fin
                .parse_into(fout.clone(), &args.open, &args.close, &args.replacements)
                .or_else(|e| match e {
                    Error::FileReadError => {
                        eprintln!("Cannot read file {fin:?}, trying copy");
                        fin.copy_into(fout.clone())
                            .map(|_| 0)
                            .map_err(|_| (fin.clone(), Error::FileCopyError))
                    }
                    _ => Err((fin.clone(), e)),
                })
                .map(|replacements| (replacements, fin, Some(fout))),
            FileOp::Simlink(_, _) => {
                unimplemented!()
            }
            FileOp::Skip(fin) => Ok((0, fin, None)),
        }
    }
}

pub struct DirectoryFiles {
    files: VecDeque<FileOp>,
}

impl Iterator for DirectoryFiles {
    type Item = FileOp;
    fn next(&mut self) -> Option<Self::Item> {
        self.files.pop_front()
    }
}

impl DirectoryFiles {
    pub fn len(&self) -> usize {
        self.files.len()
    }

    pub fn child_files_recursive<P: Into<PathBuf> + Clone>(from: P, to: P) -> Result<Self, Error> {
        let mut obj = Self {
            files: VecDeque::new(),
        };

        let base: PathBuf = from.into();
        let repl: PathBuf = to.into();

        let mut queue: VecDeque<PathBuf> = vec![base.clone()].into();

        while queue.len() > 0 {
            let next_dir_path = queue.pop_front().unwrap();
            let Ok(dir) = std::fs::read_dir(&next_dir_path) else {
                obj.files.push_back(FileOp::Skip(next_dir_path.clone()));
                continue;
            };
            for file in dir {
                let Ok(file) = file else {
                    continue;
                };

                let path = file.path();
                let base = path.strip_prefix(&base).unwrap();
                let mut new_path = repl.clone().into_os_string();
                new_path.push("/");
                new_path.push(base);
                let out_dir: PathBuf = new_path.into();

                let Ok(ft) = file.file_type() else {
                    obj.files.push_back(FileOp::Skip(path));
                    continue;
                };

                if ft.is_dir() {
                    queue.push_back(file.path());
                } else if ft.is_file() {
                    obj.files.push_back(FileOp::Parse(path, out_dir));
                } else {
                    obj.files.push_back(FileOp::Simlink(path, out_dir));
                }
            }
        }
        return Ok(obj);
    }
}

pub trait FileOps {
    fn write_into_ensure_dirs(&self, data: &[u8], into: PathBuf) -> Result<(), Error>;
    fn copy_into(&self, into: PathBuf) -> Result<(), Error>;
    fn parse_into(
        &self,
        into: PathBuf,
        od: &str,
        cd: &str,
        replacements: &yaml_rust::Yaml,
    ) -> Result<ReplacementCount, Error>;
}

impl FileOps for PathBuf {
    fn write_into_ensure_dirs(&self, data: &[u8], into: PathBuf) -> Result<(), Error> {
        let dir = into.parent().unwrap().to_path_buf();
        std::fs::create_dir_all(dir).map_err(|_| Error::DirectoryCreateError)?;

        let mut file = std::fs::File::options()
            .read(true)
            .write(true)
            .create_new(true)
            .open(into)
            .map_err(|_| Error::FileCreateError)?;
        file.write_all(data).map_err(|_| Error::FileWriteError)?;
        Ok(())
    }
    fn copy_into(&self, into: PathBuf) -> Result<(), Error> {
        let dir = into.parent().unwrap().to_path_buf();
        std::fs::create_dir_all(dir).map_err(|_| Error::DirectoryCreateError)?;
        std::fs::copy(self, into).map_err(|_| Error::FileCopyError)?;
        Ok(())
    }
    fn parse_into(
        &self,
        into: PathBuf,
        od: &str,
        cd: &str,
        replacements: &yaml_rust::Yaml,
    ) -> Result<ReplacementCount, Error> {
        let file = std::fs::read_to_string(self).map_err(|_| Error::FileReadError)?;
        let template = Template::from_str(&file, od, cd);
        let (replacements, new_file) = template.apply(&replacements);
        self.write_into_ensure_dirs(new_file.as_bytes(), into)
            .map(|_| replacements)
    }
}

#[test]
fn read_file_list() {
    let files = DirectoryFiles::child_files_recursive("./test", "./out").unwrap();

    for file in files {
        println!("{file:?}");
    }
}
