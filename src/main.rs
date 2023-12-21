use std::{fs, io};
use std::path::{Path, PathBuf};
use anyhow::Context;
use chrono::Utc;

// Adapted from
// https://stackoverflow.com/questions/26958489/how-to-copy-a-folder-recursively-in-rust
fn copy_dir_recursively(src: &Path, dst: &Path) -> anyhow::Result<()> {
    fs::create_dir_all(&dst).context(format!("failed to create dst dir: {:?}", dst))?;
    for entry in fs::read_dir(src).context(format!("failed to read src dir: {:?}", src))? {
        let entry = entry?;
        let ty = entry.file_type()?;
        if ty.is_dir() {
            copy_dir_recursively(&entry.path(), &dst.join(entry.file_name())).context(
                format!("Failed trying to copy from {:?} to {:?}", entry.path(), dst.join(entry.file_name()))
            )?;
        } else {
            fs::copy(entry.path(), &dst.join(entry.file_name())).context(
                format!("Failed trying to copy from {:?} to {:?}", entry.path(), dst.join(entry.file_name()))
            )?;
        }
    }
    Ok(())
}

fn find_darkest_dungeon_2_app_data_dir() -> anyhow::Result<PathBuf> {
    let username = whoami::username();
    let expected_path = PathBuf::from(format!(
        "C:/Users/{}/AppData/LocalLow/RedHook/Darkest Dungeon II", username
    ));
    if !expected_path.exists() {
        return Err(anyhow::Error::new(io::Error::new(
            io::ErrorKind::NotFound,
            "Darkest Dungeon 2 app dir not found")
        ));
    }
    Ok(expected_path)
}

fn ensure_scumm_dir() -> anyhow::Result<PathBuf> {
    let dd2_app_dir = match find_darkest_dungeon_2_app_data_dir() {
        Err(e) => return Err(e.context("Failed to create scumm dir")),
        Ok(dir) => dir,
    };
    const SCUMM_DIR_NAME: &str = "scummed";
    let mut scumm_dir = dd2_app_dir.clone();
    scumm_dir.push(SCUMM_DIR_NAME);
    if scumm_dir.exists() {
        // We've already created the scumm dir before!
        return Ok(scumm_dir);
    }
    match fs::create_dir(&scumm_dir) {
        Err(err)=> {
            return Err(anyhow::Error::new(err).context("Failed to create scumm dir"))
        },
        Ok(_) => (),
    };
    Ok(scumm_dir)
}

fn find_save_dir() -> anyhow::Result<PathBuf> {
    let app_dir = match find_darkest_dungeon_2_app_data_dir() {
        Err(e) => return Err(e.context("Failed to find save dir")),
        Ok(app_dir) => app_dir,
    };
    let mut save_dir = app_dir;
    save_dir.push("SaveFiles");
    if !save_dir.exists() {
        return  Err(anyhow::Error::new(io::Error::new(
            io::ErrorKind::NotFound,
            "Darkest Dungeon 2 save dir not found in app dir")
        ));
    }
    Ok(save_dir)
}

fn find_user_id_dirs() -> anyhow::Result<Vec<PathBuf>> {
    // The interwebs suggest there should be only 1 sub dir corresponding to a user id.
    // https://www.pcgamingwiki.com/wiki/Darkest_Dungeon_II.
    // It seems possible if you had the game on both epic and steam you could end up with 2 (1 which
    // will be the steam ID, one the epic ID).
    let save_dir = match find_save_dir() {
        Err(e) => return Err(e.context("Failed to find user id dirs")),
        Ok(save_dir) => save_dir,
    };
    let read_dir = match fs::read_dir(save_dir) {
        Err(e) => return Err(
            anyhow::Error::new(e).context("Failed to read_dir while looking for user id dirs")
        ),
        Ok(read_dir) => read_dir
    };
    let mut sub_dirs= Vec::new();
    for potential_sub_dir in read_dir {
        match potential_sub_dir {
            Err(e) => return Err(
                anyhow::Error::new(e).context("Failed to read sub dir while looking for user id dir")
            ),
            Ok(dir) => sub_dirs.push(dir.path()),
        }
    }
    Ok(sub_dirs)
}

fn find_profiles_dirs() -> anyhow::Result<Vec<PathBuf>> {
    let user_id_dirs = find_user_id_dirs().context(
        "Failed to find user id dirs while looking for profile dirs"
    )?;
    let mut profile_dirs = Vec::new();
    for user_id_dir in user_id_dirs {
        let mut profiles_dir = user_id_dir;
        profiles_dir.push("profiles");
        if profiles_dir.exists() {
            profile_dirs.push(profiles_dir)
        } else {
            return Err(anyhow::Error::new(io::Error::new(
                io::ErrorKind::NotFound,
                format!("Profiles dir not found at {}", profiles_dir.to_str().expect(
                    "dir path should be a valid string"
                )),
            )));
        }
    }
    Ok(profile_dirs)
}

struct ScummedProfile {
    source_path: PathBuf,
    dest_path: PathBuf,
    time_scummed: chrono::DateTime<Utc>,
}

impl ScummedProfile {
    fn scumm_profile(
        profile_dir: &Path,
        scumm_dir: &Path,
    ) -> anyhow::Result<ScummedProfile> {
        if !profile_dir.exists() {
            return Err(anyhow::Error::new(io::Error::new(
                io::ErrorKind::NotFound,
                format!("Profiles dir not found at {}", profile_dir.to_str().expect(
                    "dir path should be a valid string"
                )),
            )));
        }

        let now = chrono::Utc::now();
        let mut dest_path = scumm_dir.to_path_buf();
        dest_path.push(now.format("%Y-%m-%dT%H-%M-%S.%f").to_string());

        copy_dir_recursively(profile_dir, &dest_path)?;

        Ok(ScummedProfile{
            source_path: profile_dir.to_path_buf(),
            dest_path,
            time_scummed: chrono::Utc::now(),
        })
    }
}

fn main() {
    let profile_dirs = find_profiles_dirs();
    let profile_dir = match profile_dirs {
        Err(e) => {
            println!("Failed to find profile dirs: {e}");
            return;
        },
        Ok(mut dirs) => {
            assert!(
                dirs.len() > 0,
                "if finding find_profiles_dirs didn't return err should have at least 1 dir",
            );
            if dirs.len() > 1 {
                println!("Found {} profile dirs, but currently only support 1 dir", dirs.len());
                return;
            }
            dirs.swap_remove(0)
        }
    };

    let scumm_dir = match ensure_scumm_dir() {
        Err(e) => {
            println!("failed to ensure scumm dir: {e}");
            return;
        },
        Ok(dir) => dir,
    };

    match ScummedProfile::scumm_profile(&profile_dir, &scumm_dir) {
        Err(e) => {
            println!("failed to scumm profile: {e}");
            return;
        },
        Ok(scummed) => println!(
            "successfully scummed current profile from {:?} to {:?}",
            scummed.source_path,
            scummed.dest_path,
        ),
    }
}
