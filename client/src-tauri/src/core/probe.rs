use std::fs::File;
use std::io::BufReader;
use std::path::Path;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, State};
use tauri_plugin_shell::ShellExt;

use crate::{core::AppState, protocol::Snowflake};

#[derive(Debug, Serialize, Default)]
pub struct TrackInfo {
    pub index: i32,
    pub codec: String,
    pub language: Option<String>,
    pub title: Option<String>,
    pub is_default: bool,
}

#[derive(Debug, Serialize, Default)]
pub struct MediaProbe {
    pub audio_tracks: Vec<TrackInfo>,
    pub subtitle_tracks: Vec<TrackInfo>,
}

// What ffprobe actually returns
#[derive(Debug, Deserialize)]
struct FfprobeOutput {
    streams: Vec<FfprobeStream>,
}

#[derive(Debug, Deserialize)]
struct FfprobeStream {
    codec_type: String,
    codec_name: Option<String>,
    #[serde(default)]
    disposition: std::collections::HashMap<String, i32>,
    #[serde(default)]
    tags: std::collections::HashMap<String, String>,
}

// ---------- Matroska / WebM ----------
fn probe_matroska(path: &Path) -> Result<MediaProbe, String> {
    let mkv = matroska::open(path).map_err(|e| format!("matroska parse: {e}"))?;

    let mut probe = MediaProbe::default();
    let (mut a_idx, mut s_idx) = (0i32, 0i32);

    for track in &mkv.tracks {
        let language = track.language.as_ref().map(|lang| match lang {
            matroska::Language::ISO639(s) | matroska::Language::IETF(s) => s.clone(),
        });
        let codec = track
            .codec_name
            .clone()
            .unwrap_or_else(|| simplify_mkv_codec(&track.codec_id));

        let info = TrackInfo {
            index: 0,
            codec,
            language,
            title: track.name.clone(),
            is_default: track.default,
        };

        if track.is_audio() {
            probe.audio_tracks.push(TrackInfo {
                index: a_idx,
                ..info
            });
            a_idx += 1;
        } else if track.is_subtitle() {
            probe.subtitle_tracks.push(TrackInfo {
                index: s_idx,
                ..info
            });
            s_idx += 1;
        }
    }

    Ok(probe)
}

/// MKV codec IDs are like "A_AAC", "S_TEXT/ASS", "A_OPUS".
/// Strip the type prefix and `TEXT/` for readable display.
fn simplify_mkv_codec(codec_id: &str) -> String {
    let stripped = codec_id
        .strip_prefix("A_")
        .or_else(|| codec_id.strip_prefix("V_"))
        .or_else(|| codec_id.strip_prefix("S_"))
        .unwrap_or(codec_id);
    stripped
        .strip_prefix("TEXT/")
        .unwrap_or(stripped)
        .to_lowercase()
}

// ---------- MP4 / M4V / MOV ----------
fn probe_mp4(path: &Path) -> Result<MediaProbe, String> {
    use mp4::TrackType;

    let f = File::open(path).map_err(|e| format!("open: {e}"))?;
    let size = f.metadata().map_err(|e| format!("stat: {e}"))?.len();
    let mp4 = mp4::Mp4Reader::read_header(BufReader::new(f), size)
        .map_err(|e| format!("mp4 parse: {e}"))?;

    let mut probe = MediaProbe::default();
    let (mut a_idx, mut s_idx) = (0i32, 0i32);

    // tracks() is a HashMap; sort by id for deterministic ordering.
    let mut tracks: Vec<_> = mp4.tracks().values().collect();
    tracks.sort_by_key(|t| t.track_id());

    for t in tracks {
        let Ok(track_type) = t.track_type() else {
            continue;
        };
        let codec = t
            .box_type()
            .map(|b| b.to_string())
            .unwrap_or_else(|_| "unknown".into());
        let language = match t.language() {
            "und" | "" => None,
            s => Some(s.to_string()),
        };

        let info = TrackInfo {
            index: 0,
            codec,
            language,
            title: None,       // mp4 crate doesn't expose track names
            is_default: false, // patched after the loop
        };

        match track_type {
            TrackType::Audio => {
                probe.audio_tracks.push(TrackInfo {
                    index: a_idx,
                    ..info
                });
                a_idx += 1;
            }
            TrackType::Subtitle => {
                probe.subtitle_tracks.push(TrackInfo {
                    index: s_idx,
                    ..info
                });
                s_idx += 1;
            }
            _ => {}
        }
    }

    // MP4 has no explicit "default track" flag like MKV. Convention is
    // first-of-kind = default, which matches what most players do.
    if let Some(t) = probe.audio_tracks.first_mut() {
        t.is_default = true;
    }
    if let Some(t) = probe.subtitle_tracks.first_mut() {
        t.is_default = true;
    }

    Ok(probe)
}

// ---------- ffprobe fallback (your existing function, unchanged) ----------
async fn probe_ffprobe(app: &AppHandle, path: &Path) -> Result<MediaProbe, String> {
    let path_str = path.to_str().ok_or("Failed PathBuf::to_str()")?;
    let output = app
        .shell()
        .sidecar("ffprobe")
        .map_err(|e| format!("sidecar init: {e}"))?
        .args([
            "-v",
            "error",
            "-print_format",
            "json",
            "-show_streams",
            path_str,
        ])
        .output()
        .await
        .map_err(|e| format!("ffprobe spawn: {e}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("ffprobe failed: {stderr}"));
    }

    let parsed: FfprobeOutput = serde_json::from_slice(&output.stdout)
        .map_err(|e| format!("ffprobe json parse failed: {e}"))?;

    let mut audio_tracks = Vec::new();
    let mut subtitle_tracks = Vec::new();

    for stream in parsed.streams {
        let is_default = stream.disposition.get("default").copied().unwrap_or(0) == 1;
        let info = TrackInfo {
            index: 0, // we'll fix this below
            codec: stream.codec_name.unwrap_or_else(|| "unknown".into()),
            language: stream.tags.get("language").cloned(),
            title: stream.tags.get("title").cloned(),
            is_default,
        };
        match stream.codec_type.as_str() {
            "audio" => audio_tracks.push(info),
            "subtitle" => subtitle_tracks.push(info),
            _ => {}
        }
    }

    // Assign 0-indexed positions within each kind
    for (i, t) in audio_tracks.iter_mut().enumerate() {
        t.index = i as i32;
    }
    for (i, t) in subtitle_tracks.iter_mut().enumerate() {
        t.index = i as i32;
    }

    Ok(MediaProbe {
        audio_tracks,
        subtitle_tracks,
    })
}

// ---------- Dispatch ----------
#[tauri::command]
pub async fn probe_media(
    app: AppHandle,
    state: State<'_, AppState>,
    id: Snowflake,
) -> Result<MediaProbe, String> {
    let path = state
        .fm()
        .await?
        .local_path(id)
        .await
        .ok_or("File not on device.")?;

    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();

    let native = match ext.as_str() {
        "mkv" | "webm" => probe_matroska(&path),
        "mp4" | "m4v" | "mov" => probe_mp4(&path),
        other => Err(format!("no native prober for .{other}")),
    };

    match native {
        Ok(probe) => Ok(probe),
        Err(native_err) => match probe_ffprobe(&app, &path).await {
            Ok(probe) => Ok(probe),
            Err(ff_err) => Err(format!(
                "could not probe file (native: {native_err}; ffprobe: {ff_err})"
            )),
        },
    }
}
