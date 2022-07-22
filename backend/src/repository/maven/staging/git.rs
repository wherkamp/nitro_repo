use crate::repository::maven::models::Pom;
use crate::repository::settings::RepositoryConfig;
use crate::storage::DynamicStorage;
use crate::system::user::database::Model as UserModel;
use git2::PushOptions;
use log::{error, trace};
use std::path::Path;
use std::sync::Arc;
use std::{fs, io};
use tempfile::tempdir;

pub async fn stage_to_git(
    username: String,
    password: String,
    url: String,
    branch: String,
    directory: String,
    storage: Arc<DynamicStorage>,
    repository: RepositoryConfig,
    model: UserModel,
) -> anyhow::Result<()> {
    let dir = tempdir()?;
    trace!("Cloning {} to {}", url, dir.path().display());
    let git = git2::Repository::clone(&url, dir.path())?;
    let working_directory = git.workdir().unwrap().join(&directory);

    match storage.as_ref() {
        DynamicStorage::LocalStorage(local) => {
            let buf = local
                .get_repository_folder(&repository.name)
                .join(&directory);
            fs::create_dir_all(&working_directory)?;
            copy_dir_all(&buf, &working_directory)?;
            if let Some(v) = buf.parent() {
                let maven = v.join("maven-metadata.xml");
                if maven.exists() {
                    fs::copy(
                        maven,
                        working_directory
                            .parent()
                            .unwrap()
                            .join("maven-metadata.xml"),
                    )?;
                }
            }
        }
        _ => {
            todo!("Support other storage types");
        }
    }
    let mut index = git.index()?;
    index.add_all(["*"].iter(), git2::IndexAddOption::DEFAULT, None)?;
    index.write().expect("Failed to write index");
    let v = index.write_tree()?;
    let tree = git.find_tree(v).unwrap();
    let sig = git2::Signature::now(model.name.as_str(), model.email.as_str())?;
    let me = git.signature()?;
    let parent_commit = git.head()?.peel_to_commit()?;
    let mut commit_name = "Released".to_string();
    for entry in fs::read_dir(working_directory).expect("Failed to read dir") {
        let entry = entry.expect("Failed to get entry");
        if entry.file_name().to_str().unwrap().ends_with("pom") {
            let pom: Result<Pom, serde_xml_rs::Error> =
                serde_xml_rs::from_reader(fs::OpenOptions::new().read(true).open(entry.path())?);
            match pom {
                Ok(ok) => {
                    commit_name = format!("{} {} - Nitro Repo", ok.artifact_id, ok.version);
                    break;
                }
                Err(error) => {
                    error!("Failed to parse pom: {}", error);
                }
            }
        }
    }

    git.commit(
        Some("HEAD"),
        &sig,
        &me,
        commit_name.as_str(),
        &tree,
        &[&parent_commit],
    )?;
    let mut options = PushOptions::new();
    let mut callbacks = git2::RemoteCallbacks::new();
    callbacks.credentials(|_, _, _| git2::Cred::userpass_plaintext(&username, &password));
    options.remote_callbacks(callbacks);

    git.find_remote("origin")?
        .push(&[format!("refs/heads/{}", branch)], Some(&mut options))?;
    Ok(())
}

fn copy_dir_all(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> io::Result<()> {
    trace!(
        "Copying {} to {}",
        src.as_ref().display(),
        dst.as_ref().display()
    );
    fs::create_dir_all(&dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        if ty.is_dir() {
            copy_dir_all(entry.path(), dst.as_ref().join(entry.file_name()))?;
        } else {
            let tof = dst.as_ref().join(entry.file_name());
            let srcf = entry.path();
            trace!("Copying {} to {}", srcf.display(), tof.display());
            fs::copy(srcf, tof)?;
        }
    }
    Ok(())
}
