use std::{
    env, fs,
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
};

const APP_DIRECTORY: &str = "Cadence";

#[derive(Clone, Copy)]
pub enum DialogKind {
    OpenProject,
    SaveProject,
    OpenMusic,
    ExportFolder,
}

pub fn config_file(name: &str) -> PathBuf {
    let directory = config_directory();
    let path = directory.join(name);
    if !path.exists()
        && let Some(base) = directory.parent()
    {
        let previous = base.join("Marching Drill").join(name);
        if previous.is_file() {
            let _ = fs::copy(previous, &path);
        }
    }
    path
}

fn config_directory() -> PathBuf {
    #[cfg(target_os = "windows")]
    let base = env::var_os("APPDATA").map(PathBuf::from);

    #[cfg(target_os = "macos")]
    let base = env::var_os("HOME")
        .map(PathBuf::from)
        .map(|home| home.join("Library").join("Application Support"));

    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    let base = env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| {
            env::var_os("HOME")
                .map(PathBuf::from)
                .map(|home| home.join(".config"))
        });

    let directory = base
        .unwrap_or_else(|| env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
        .join(APP_DIRECTORY);
    let _ = fs::create_dir_all(&directory);
    directory
}

pub fn choose(kind: DialogKind, start: &Path) -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    return choose_windows(kind, start);

    #[cfg(target_os = "macos")]
    return choose_macos(kind, start);

    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    choose_linux(kind, start)
}

fn output_path(output: std::process::Output) -> Option<PathBuf> {
    if !output.status.success() {
        return None;
    }
    String::from_utf8(output.stdout)
        .ok()
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
}

#[allow(dead_code)]
fn choose_linux(kind: DialogKind, start: &Path) -> Option<PathBuf> {
    let (kdialog_action, filter) = match kind {
        DialogKind::OpenProject => (
            "--getopenfilename",
            "Cadence (*.mdrill *.drill.json *.json)",
        ),
        DialogKind::SaveProject => ("--getsavefilename", "Cadence (*.mdrill)"),
        DialogKind::OpenMusic => (
            "--getopenfilename",
            "Music (*.mp3 *.wav *.ogg *.flac *.m4a *.aac)",
        ),
        DialogKind::ExportFolder => ("--getexistingdirectory", ""),
    };
    let selected = Command::new("kdialog")
        .arg(kdialog_action)
        .arg(start)
        .args((!filter.is_empty()).then_some(filter))
        .output()
        .ok()
        .and_then(output_path);
    selected.or_else(|| {
        let mut command = Command::new("zenity");
        command.arg("--file-selection");
        match kind {
            DialogKind::SaveProject => {
                command.arg("--save").arg("--confirm-overwrite");
            }
            DialogKind::ExportFolder => {
                command.arg("--directory");
            }
            _ => {}
        }
        command.arg(format!("--filename={}", start.display()));
        match kind {
            DialogKind::OpenProject => {
                command.arg("--file-filter=Cadence | *.mdrill *.drill.json *.json");
            }
            DialogKind::SaveProject => {
                command.arg("--file-filter=Cadence | *.mdrill");
            }
            DialogKind::OpenMusic => {
                command.arg("--file-filter=Music | *.mp3 *.wav *.ogg *.flac *.m4a *.aac");
            }
            DialogKind::ExportFolder => {}
        }
        command.output().ok().and_then(output_path)
    })
}

#[allow(dead_code)]
fn choose_windows(kind: DialogKind, start: &Path) -> Option<PathBuf> {
    let (dialog_type, filter) = match kind {
        DialogKind::OpenProject => (
            "OpenFileDialog",
            "Cadence (*.mdrill;*.drill.json;*.json)|*.mdrill;*.drill.json;*.json",
        ),
        DialogKind::SaveProject => ("SaveFileDialog", "Cadence (*.mdrill)|*.mdrill"),
        DialogKind::OpenMusic => (
            "OpenFileDialog",
            "Music|*.mp3;*.wav;*.ogg;*.flac;*.m4a;*.aac",
        ),
        DialogKind::ExportFolder => ("FolderBrowserDialog", ""),
    };
    let script = if matches!(kind, DialogKind::ExportFolder) {
        "[Console]::OutputEncoding=[Text.UTF8Encoding]::new($false); Add-Type -AssemblyName System.Windows.Forms; $d=New-Object System.Windows.Forms.FolderBrowserDialog; $d.SelectedPath=$env:MARCHING_DRILL_DIALOG_PATH; if($d.ShowDialog() -eq [System.Windows.Forms.DialogResult]::OK){[Console]::Write($d.SelectedPath)}".to_owned()
    } else {
        format!(
            "[Console]::OutputEncoding=[Text.UTF8Encoding]::new($false); Add-Type -AssemblyName System.Windows.Forms; $d=New-Object System.Windows.Forms.{dialog_type}; $d.Filter='{filter}'; $p=$env:MARCHING_DRILL_DIALOG_PATH; if([IO.Directory]::Exists($p)){{$d.InitialDirectory=$p}}else{{$d.InitialDirectory=[IO.Path]::GetDirectoryName($p);$d.FileName=[IO.Path]::GetFileName($p)}}; if($d.ShowDialog() -eq [System.Windows.Forms.DialogResult]::OK){{[Console]::Write($d.FileName)}}"
        )
    };
    Command::new("powershell.exe")
        .args(["-NoProfile", "-STA", "-Command", &script])
        .env("MARCHING_DRILL_DIALOG_PATH", start)
        .output()
        .ok()
        .and_then(output_path)
}

#[allow(dead_code)]
fn choose_macos(kind: DialogKind, start: &Path) -> Option<PathBuf> {
    let script = match kind {
        DialogKind::OpenProject => {
            "POSIX path of (choose file with prompt \"Open Drill\" of type {\"mdrill\", \"json\"})"
        }
        DialogKind::SaveProject => {
            "POSIX path of (choose file name with prompt \"Save Drill\" default name \"Untitled.mdrill\")"
        }
        DialogKind::OpenMusic => {
            "POSIX path of (choose file with prompt \"Choose Music\" of type {\"mp3\", \"wav\", \"ogg\", \"flac\", \"m4a\", \"aac\"})"
        }
        DialogKind::ExportFolder => {
            "POSIX path of (choose folder with prompt \"Export Performer Packets\")"
        }
    };
    Command::new("osascript")
        .args(["-e", script])
        .current_dir(if start.is_dir() {
            start
        } else {
            start.parent().unwrap_or_else(|| Path::new("."))
        })
        .output()
        .ok()
        .and_then(output_path)
}

pub fn start_audio(path: &Path, seek_seconds: f32) -> Result<Child, String> {
    #[cfg(target_os = "windows")]
    let mut command = audio_command_windows(path, seek_seconds);

    #[cfg(target_os = "macos")]
    let mut command = audio_command_macos(path, seek_seconds);

    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    let mut command = audio_command_linux(path, seek_seconds);

    command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|error| error.to_string())
}

#[allow(dead_code)]
fn audio_command_windows(path: &Path, seek_seconds: f32) -> Command {
    let script = "Add-Type -AssemblyName PresentationCore; $p=New-Object System.Windows.Media.MediaPlayer; $p.Open([Uri]$env:MARCHING_DRILL_AUDIO_PATH); $limit=(Get-Date).AddSeconds(10); while(!$p.NaturalDuration.HasTimeSpan -and (Get-Date) -lt $limit){Start-Sleep -Milliseconds 25}; if(!$p.NaturalDuration.HasTimeSpan){$p.Close();exit 2}; $p.Position=[TimeSpan]::FromSeconds([double]$env:MARCHING_DRILL_AUDIO_SEEK); $p.Play(); while($p.Position -lt $p.NaturalDuration.TimeSpan){Start-Sleep -Milliseconds 100}; $p.Close()";
    let mut command = Command::new("powershell.exe");
    command
        .args(["-NoProfile", "-STA", "-Command", script])
        .env("MARCHING_DRILL_AUDIO_PATH", path)
        .env("MARCHING_DRILL_AUDIO_SEEK", format!("{seek_seconds:.3}"));
    command
}

#[allow(dead_code)]
fn audio_command_macos(path: &Path, seek_seconds: f32) -> Command {
    let script = "ObjC.import('AVFoundation'); ObjC.import('Foundation'); function run(a){let e=Ref();let p=$.AVAudioPlayer.alloc.initWithContentsOfURLError($.NSURL.fileURLWithPath($(a[0])),e);if(!p){throw Error('Unable to open audio');}p.currentTime=parseFloat(a[1]);p.play;while(p.playing){$.NSRunLoop.currentRunLoop.runUntilDate($.NSDate.dateWithTimeIntervalSinceNow(0.1));}}";
    let mut command = Command::new("osascript");
    command
        .args(["-l", "JavaScript", "-e", script, "--"])
        .arg(path)
        .arg(format!("{seek_seconds:.3}"));
    command
}

#[allow(dead_code)]
fn audio_command_linux(path: &Path, seek_seconds: f32) -> Command {
    let mut command = Command::new("ffplay");
    command
        .args(["-nodisp", "-autoexit", "-loglevel", "quiet", "-ss"])
        .arg(format!("{seek_seconds:.3}"))
        .arg(path);
    command
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_file_has_application_directory() {
        assert!(config_file("test.json").ends_with(Path::new(APP_DIRECTORY).join("test.json")));
    }
}
